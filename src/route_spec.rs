use crate::{Captures, Path, ReverseMatch, Segment};
use smallvec::{smallvec, Array, SmallVec};
use smartstring::alias::String as SmartString;
use std::{
    cmp::Ordering,
    convert::TryFrom,
    fmt::{self, Debug, Display, Formatter},
};

#[cfg(feature = "arbitrary")]
#[doc(hidden)] // fuzzing support; public only so the `fuzz` crate can reach it
pub mod arbitrary;
mod r#match;
mod parse;

/// Routefinder's representation of the parsed route
///
/// This contains both an optional source string (or unique description) and
/// an ordered sequence of [`Segment`]s
#[derive(Eq, Debug, Clone)]
pub struct RouteSpec {
    source: Option<SmartString>,
    segments: Vec<Segment>,
    /// A per-path-segment classification derived from `segments` (via
    /// [`RouteSpec::populate_segment_groups`] in `optimize`). This is the
    /// canonical sort key for [`Ord`]/[`PartialEq`]: it groups each
    /// slash-delimited segment into exact/prefix/suffix/capture/wildcard/complex
    /// so route specificity can be compared a group at a time. `segments`
    /// remains the source of truth that the trie is built from; this is a cache
    /// kept consistent whenever the spec is constructed.
    segment_groups: Vec<SegmentGroup>,
    pub(crate) min_length: usize,
    dot_count: usize,
    pub(crate) handler_index: Option<usize>,
}

#[derive(Eq, PartialEq, Debug, Clone)]
enum SegmentGroup {
    Exact(SmartString),
    Capture(SmartString),
    Prefix(SmartString, SmartString),
    Suffix(SmartString, SmartString),
    Wildcard,
    Complex(Vec<Segment>),
}

impl PartialEq for RouteSpec {
    fn eq(&self, other: &Self) -> bool {
        self.segment_groups == other.segment_groups
    }
}

impl Display for RouteSpec {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("/")?;
        for segment in &self.segments {
            match segment {
                Segment::Slash => f.write_str("/")?,
                Segment::Dot => f.write_str(".")?,
                Segment::Exact(s) => f.write_str(s)?,
                Segment::Param(p) => f.write_fmt(format_args!(":{p}"))?,
                Segment::Wildcard => f.write_str("*")?,
            };
        }
        Ok(())
    }
}

impl RouteSpec {
    /// Retrieve a reference to the original route definition, if this
    /// routespec was parsed from a string representation. If this
    /// routespec was created another way, this will return None.
    pub fn source(&self) -> Option<&str> {
        self.source.as_deref()
    }

    /// Slice accessor for the component [`Segment`]s in this RouteSpec
    pub fn segments(&self) -> &[Segment] {
        self.segments.as_slice()
    }

    #[inline]
    fn passes_optimization_criteria(&self, path: &Path<'_>) -> bool {
        self.min_length <= path.trimmed.len()
    }

    /// Returns a vec of captured str slices for this routespec
    #[inline]
    pub fn matches<'path>(&self, path: &'path str) -> Option<Vec<&'path str>> {
        let mut captures: SmallVec<[&'path str; 5]> = smallvec![];

        if self.matches_path(&path.into(), &mut captures) {
            Some(captures.into_vec())
        } else {
            None
        }
    }

    #[inline]
    pub(crate) fn matches_path<'path>(
        &self,
        path: &Path<'path>,
        captures: &mut SmallVec<impl Array<Item = &'path str>>,
    ) -> bool {
        if !self.passes_optimization_criteria(path) {
            return false;
        }
        let Some(p) = self.inner_match(path, captures) else {
            return false;
        };
        p.is_empty() || p == "/"
    }

    /// populate this route spec with the params and/or wildcard from
    /// a [`Captures`], if it matches.
    pub fn template<'route, 'keys, 'captures, 'values>(
        &'route self,
        captures: &'captures Captures<'keys, 'values>,
    ) -> Option<ReverseMatch<'keys, 'values, 'captures, 'route>> {
        ReverseMatch::new(captures, self)
    }

    fn compute_optimizations(&mut self) {
        self.dot_count = 0;
        self.min_length = 0;

        for segment in &self.segments {
            match segment {
                Segment::Slash => {
                    if self.min_length != 0 {
                        self.min_length += 1;
                    }
                }
                Segment::Dot => {
                    self.min_length += 1;
                    self.dot_count += 1;
                }
                Segment::Exact(s) => {
                    self.min_length += s.len();
                }
                Segment::Param(_) => {
                    self.min_length += 1;
                }
                Segment::Wildcard => {}
            }
        }

        if matches!(self.segments.first(), Some(Segment::Slash)) {
            self.min_length = self.min_length.saturating_sub(1);
        }

        if matches!(
            self.segments.last(),
            Some(Segment::Slash | Segment::Wildcard)
        ) {
            self.min_length = self.min_length.saturating_sub(1);
        }
    }

    fn compact(&mut self) {
        let mut index = 1;
        let segments = &mut self.segments;
        use Segment::*;
        loop {
            let new_previous = {
                let current = segments.get(index);
                let next = segments.get(index + 1);
                let previous = segments.get(index - 1);
                match (previous, current, next) {
                    (Some(Exact(a)), Some(Dot), Some(Exact(b))) => Some((format!("{a}.{b}"), true)),
                    (Some(Exact(a)), Some(Exact(b)), _) => Some((format!("{a}{b}"), false)),
                    _ => None,
                }
            };
            if let Some((new_previous, skip_next)) = new_previous {
                segments[index - 1] = Exact(new_previous.into());
                if skip_next {
                    segments.remove(index + 1);
                }
                segments.remove(index);
            } else if index < segments.len() {
                index += 1;
            } else {
                break;
            }
        }
    }

    fn populate_segment_groups(&mut self) {
        use Segment::*;
        self.segment_groups.clear();
        let mut segments = &self.segments[..];
        loop {
            let index = segments
                .iter()
                .position(|x| x == &Slash)
                .unwrap_or(segments.len());
            let (current, rest) = segments.split_at(index);
            self.segment_groups.push(match current {
                [] => {
                    break;
                }
                [Exact(e)] => SegmentGroup::Exact(e.clone()),
                [Param(e)] => SegmentGroup::Capture(e.clone()),
                [Wildcard] => SegmentGroup::Wildcard,
                [Exact(e), Dot, Param(c)] => SegmentGroup::Prefix(e.clone(), c.clone()),
                [Param(c), Dot, Exact(e)] => SegmentGroup::Suffix(e.clone(), c.clone()),
                other => SegmentGroup::Complex(other.to_vec()),
            });
            if let Some(rest) = rest.get(1..) {
                segments = rest;
            } else {
                break;
            }
        }
    }

    fn optimize(mut self) -> Self {
        self.compact();
        self.populate_segment_groups();
        self.compute_optimizations();
        self
    }
}

impl TryFrom<&str> for RouteSpec {
    type Error = String;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        s.parse()
    }
}

impl TryFrom<String> for RouteSpec {
    type Error = String;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        s.parse()
    }
}

impl From<Vec<Segment>> for RouteSpec {
    fn from(segments: Vec<Segment>) -> Self {
        Self {
            segments,
            source: None,
            min_length: 0,
            dot_count: 0,
            segment_groups: vec![],
            handler_index: None,
        }
        .optimize()
    }
}

impl PartialOrd for RouteSpec {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

macro_rules! return_if_unequal {
    ($a:expr, $b:expr) => {
        match ($a, $b) {
            (a, b) => match a.cmp(b) {
                Ordering::Equal => (),
                other => {
                    return other;
                }
            },
        }
    };
}

impl Ord for RouteSpec {
    fn cmp(&self, other: &Self) -> Ordering {
        use Ordering::*;
        use SegmentGroup::*;
        for (a, b) in self.segment_groups.iter().zip(&other.segment_groups) {
            match (a, b) {
                (Exact(_), _) => return Less,
                (_, Exact(_)) => return Greater,
                (Prefix(_, _), _) => return Less,
                (_, Prefix(_, _)) => return Greater,
                (Suffix(_, _), _) => return Less,
                (_, Suffix(_, _)) => return Greater,
                (Complex(_), _) => return Less,
                (_, Complex(_)) => return Greater,
                (Capture(_), _) => return Less,
                (_, Capture(_)) => return Greater,
                _ => (),
            }
        }

        return_if_unequal!(self.segment_groups.len(), &other.segment_groups.len());

        for (a, b) in self.segment_groups.iter().zip(&other.segment_groups) {
            match (a, b) {
                (Exact(a), Exact(b)) => return_if_unequal!(a, b),
                (Prefix(exact_a, param_a), Prefix(exact_b, param_b)) => {
                    return_if_unequal!(exact_a, exact_b);
                    return_if_unequal!(param_a, param_b);
                }
                (Suffix(exact_a, param_a), Suffix(exact_b, param_b)) => {
                    return_if_unequal!(exact_a, exact_b);
                    return_if_unequal!(param_a, param_b);
                }
                (Complex(a), Complex(b)) => return_if_unequal!(b.len(), &a.len()),
                (Capture(a), Capture(b)) => return_if_unequal!(a, b),
                _ => (),
            }
        }

        Equal
    }
}
