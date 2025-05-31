use super::{State, TrieMatch, TrieNode};
use crate::{trie::find_slash, Path};
use smallvec::{smallvec, SmallVec};

macro_rules! try_match {
    ($result:expr) => {
        if let Some(route) = $result {
            return Some(route);
        }
    };
}

impl TrieNode {
    pub(super) fn best_match<'trie, 'path>(
        &'trie self,
        path: &'path str,
    ) -> Option<TrieMatch<'trie, 'path>> {
        #[cfg(feature = "log")]
        log::trace!("testing {path:?} at \n{self:#?}");

        let mut segment = "";
        let mut rest = "";

        for state in &self.states {
            match state {
                State::Start => {
                    let len = path.len();
                    if self.minimum_length.is_some_and(|ml| len < ml) {
                        #[cfg(feature = "log")]
                        log::trace!(
                            "abandoning due to path length of {len} and minimum of {}",
                            self.minimum_length.unwrap_or_default(),
                        );

                        return None;
                    }
                }
                State::Terminal => try_match!(self.terminal(path)),
                State::FindSlash => (segment, rest) = find_slash(path),
                State::Exact { min, max } => {
                    try_match!(self.exact(segment, rest, *min, *max))
                }
                State::Prefixes { min } => try_match!(self.prefixes(segment, rest, *min)),
                State::Suffixes { min } => try_match!(self.suffixes(segment, rest, *min)),
                State::Degenerate { index, min } => {
                    try_match!(self.degenerate(segment, rest, *index, *min))
                }
                State::Captures => try_match!(self.captures(segment, rest)),
                State::Wildcard => {
                    return Some(TrieMatch {
                        route: self.wildcard.as_ref().unwrap(),
                        captures: smallvec![],
                        wildcard: Some(path),
                    });
                }
                State::Done => {}
            }
        }
        None
    }

    fn terminal<'trie, 'path>(&'trie self, path: &'path str) -> Option<TrieMatch<'trie, 'path>> {
        if path.is_empty() {
            let route = self.route.as_ref().unwrap();
            #[cfg(feature = "log")]
            log::trace!("reached terminal node for {route}");
            Some(TrieMatch {
                route,
                captures: smallvec![],
                wildcard: None,
            })
        } else {
            None
        }
    }

    fn exact<'trie, 'path>(
        &'trie self,
        segment: &'path str,
        rest: &'path str,
        min: usize,
        max: usize,
    ) -> Option<TrieMatch<'trie, 'path>> {
        if segment.len() < min || segment.len() > max {
            return None;
        }

        if let Some(node) = self.statics.get(segment) {
            if let Some(trie_match) = node.best_match(rest) {
                return Some(trie_match);
            }
        }
        None
    }

    fn prefixes<'trie, 'path>(
        &'trie self,
        segment: &'path str,
        rest: &'path str,
        min: usize,
    ) -> Option<TrieMatch<'trie, 'path>> {
        if segment.len() < min {
            return None;
        }

        let subsegment_matcher = self.subsegment_matcher.as_ref().unwrap();
        for prefix_length in &subsegment_matcher.prefix_lengths {
            if *prefix_length >= segment.len() {
                break;
            }

            if let Some(node) = subsegment_matcher.prefixes.get(&segment[0..*prefix_length]) {
                if let Some(mut trie_match) = node.best_match(rest) {
                    trie_match.captures.push(&segment[*prefix_length..]);
                    return Some(trie_match);
                }

                break;
            }
        }
        None
    }

    fn suffixes<'trie, 'path>(
        &'trie self,
        segment: &'path str,
        rest: &'path str,
        min: usize,
    ) -> Option<TrieMatch<'trie, 'path>> {
        if segment.len() < min {
            return None;
        }

        let subsegment_matcher = self.subsegment_matcher.as_ref().unwrap();
        for suffix_length in &subsegment_matcher.suffix_lengths {
            if *suffix_length >= segment.len() {
                break;
            }

            let suffix_start = segment.len() - suffix_length;
            if let Some(node) = subsegment_matcher.suffixes.get(&segment[suffix_start..]) {
                if let Some(mut trie_match) = node.best_match(rest) {
                    trie_match.captures.push(&segment[..suffix_start]);
                    return Some(trie_match);
                }

                break;
            }
        }
        None
    }

    fn degenerate<'trie, 'path>(
        &'trie self,
        segment: &'path str,
        rest: &'path str,
        index: usize,
        min: usize,
    ) -> Option<TrieMatch<'trie, 'path>> {
        if segment.len() < min {
            return None;
        }

        let degenerate = &self.subsegment_matcher.as_ref().unwrap().degenerate;
        let path = Path::from(segment);

        let (segment_spec, node) = degenerate.iter().nth(index).unwrap();
        let mut new_captures: SmallVec<[&'path str; 2]> = smallvec![];
        if segment_spec.matches_path(&path, &mut new_captures) {
            if let Some(mut trie_match) = node.best_match(rest) {
                trie_match.captures.extend(new_captures.drain(..).rev());
                return Some(trie_match);
            }
        }
        None
    }

    fn captures<'trie, 'path>(
        &'trie self,
        segment: &'path str,
        rest: &'path str,
    ) -> Option<TrieMatch<'trie, 'path>> {
        let node = self.param.as_deref().unwrap();
        if !segment.is_empty() {
            if let Some(mut trie_match) = node.best_match(rest) {
                trie_match.captures.push(segment);
                return Some(trie_match);
            }
        }
        None
    }
}
