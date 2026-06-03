use super::{State, SubsegmentNode, TrieNode};
use crate::{RouteSpec, Segment};

impl TrieNode {
    pub(super) fn insert(&mut self, route: RouteSpec, depth: usize) {
        self.update_minimum_length(&route, depth);
        self.paths += 1;

        let Some(segment) = route.segments().get(depth) else {
            if let Some(previous) = &self.route {
                if previous > &route {
                    #[cfg(feature = "log")]
                    log::warn!("replacing {previous} with {route}");
                    self.route = Some(route);
                    self.paths = self.paths.saturating_sub(1);
                } else {
                    #[cfg(feature = "log")]
                    log::warn!("NOT replacing {previous} with {route}");
                }
            } else {
                self.route = Some(route);
            }
            self.compute_state_sequence();
            return;
        };

        let next = route.segments().get(depth + 1);

        match (segment, next) {
            (Segment::Slash, _) => {
                self.paths = self.paths.saturating_sub(1);
                self.insert(route, depth + 1);
            }
            (Segment::Exact(string), Some(Segment::Slash) | None) => {
                let len = string.len();
                self.static_len_range = Some(match self.static_len_range {
                    Some((min, max)) => (min.min(len), max.max(len)),
                    None => (len, len),
                });
                self.statics
                    .entry(string.clone())
                    .or_default()
                    .insert(route, depth + 1);
            }
            (Segment::Param(_), Some(Segment::Slash) | None) => {
                self.param.get_or_insert_default().insert(route, depth + 1)
            }
            (Segment::Wildcard, None) => {
                if let Some(previous) = &self.wildcard {
                    if previous > &route {
                        #[cfg(feature = "log")]
                        log::warn!("replacing {previous} with {route} for wildcard");
                        self.paths = self.paths.saturating_sub(1);
                        self.wildcard = Some(route);
                    } else {
                        #[cfg(feature = "log")]
                        log::warn!("NOT replacing {previous} with {route} for wildcard");
                    }
                } else {
                    self.wildcard = Some(route);
                }
            }

            (Segment::Param(_) | Segment::Exact(_) | Segment::Dot, _) => {
                self.subsegment_matcher
                    .get_or_insert_default()
                    .insert(route, depth);
            }

            _ => {
                unimplemented!("UNIMPLEMENTED {segment:?} {next:?}")
            }
        }

        self.compute_state_sequence();
    }

    fn update_minimum_length(&mut self, route: &RouteSpec, depth: usize) {
        let mut new_minimum_length = 0;
        let segments = route.segments().get(depth..);

        if let Some(segments) = segments {
            for segment in segments {
                match segment {
                    Segment::Slash | Segment::Dot | Segment::Param(_) => {
                        new_minimum_length += 1;
                    }

                    Segment::Exact(s) => {
                        new_minimum_length += s.len();
                    }

                    Segment::Wildcard => {}

                    // The trie is only ever fed expanded (flat) variant specs.
                    Segment::Optional(_) => {
                        unreachable!("optional segments are expanded before insertion")
                    }
                }
            }

            if matches!(segments.last(), Some(Segment::Slash | Segment::Wildcard)) {
                new_minimum_length = new_minimum_length.saturating_sub(1);
            }
        }

        self.minimum_length = Some(if let Some(ml) = self.minimum_length {
            new_minimum_length.min(ml)
        } else {
            new_minimum_length
        });
    }

    fn compute_state_sequence(&mut self) {
        let states = &mut self.states;
        states.clear();
        states.push(State::Start);

        if self.route.is_some() {
            states.push(State::Terminal);
        }

        if !self.statics.is_empty() || self.subsegment_matcher.is_some() || self.param.is_some() {
            states.push(State::FindSlash);
        }

        if let Some((min, max)) = self.static_len_range {
            states.push(State::Exact { min, max });
        }

        if let Some(subsegment) = &self.subsegment_matcher {
            // `prefix_lengths`/`suffix_lengths` are kept sorted, so the first
            // entry is the minimum key length (the prefix/suffix keys are the
            // literal plus its dot, matching those stored lengths).
            if let Some(&min) = subsegment.prefix_lengths.first() {
                states.push(State::Prefixes { min });
            }

            if let Some(&min) = subsegment.suffix_lengths.first() {
                states.push(State::Suffixes { min });
            }

            states.extend(
                subsegment
                    .degenerate
                    .keys()
                    .enumerate()
                    .map(|(index, route)| State::Degenerate {
                        index,
                        min: route.min_length,
                    }),
            );
        }

        if self.param.is_some() {
            states.push(State::Captures);
        }

        if self.wildcard.is_some() {
            states.push(State::Wildcard);
        }
    }
}

impl SubsegmentNode {
    pub(super) fn insert(&mut self, route: RouteSpec, depth: usize) {
        let next_slash = route.segments()[depth..]
            .iter()
            .position(|segment| segment == &Segment::Slash)
            .map(|x| depth + x)
            .unwrap_or_else(|| route.segments().len());

        let segment = &route.segments()[depth..next_slash];

        match segment {
            [Segment::Exact(e), Segment::Dot, Segment::Param(_)] => {
                self.prefix_lengths.push(e.len() + 1);
                self.prefix_lengths.sort_unstable();
                self.prefix_lengths.dedup();
                self.prefixes
                    .entry(format!("{e}.").into())
                    .or_default()
                    .insert(route, next_slash);
            }

            [Segment::Param(_), Segment::Dot, Segment::Exact(e)] => {
                self.suffix_lengths.push(e.len() + 1);
                self.suffix_lengths.sort_unstable();
                self.suffix_lengths.dedup();
                self.suffixes
                    .entry(format!(".{e}").into())
                    .or_default()
                    .insert(route, next_slash);
            }

            other => {
                let segment_spec = RouteSpec::from(other.to_vec());
                self.degenerate
                    .entry(segment_spec)
                    .or_default()
                    .insert(route, next_slash);
            }
        }
    }
}
