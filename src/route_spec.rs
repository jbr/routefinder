use crate::{Captures, ReverseMatch, Segment};
use smartstring::alias::String as SmartString;
use std::{
    cmp::Ordering,
    convert::TryFrom,
    fmt::{self, Debug, Display, Formatter},
    iter,
    str::FromStr,
};

/// Routefinder's representation of the parsed route
///
/// This contains both an optional source string (or unique description) and
/// an ordered sequence of [`Segment`]s
#[derive(PartialEq, Eq, Debug, Clone)]
pub struct RouteSpec {
    source: Option<SmartString>,
    segments: Vec<Segment>,
}

impl Display for RouteSpec {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("/")?;
        for segment in &self.segments {
            match segment {
                Segment::Slash => f.write_str("/")?,
                Segment::Dot => f.write_str(".")?,
                Segment::Exact(s) => f.write_str(s)?,
                Segment::Param(p) => f.write_fmt(format_args!(":{}", p))?,
                Segment::Wildcard => f.write_str("*")?,
            };
        }
        Ok(())
    }
}

impl RouteSpec {
    fn dots(&self) -> usize {
        self.segments
            .iter()
            .filter(|c| matches!(c, Segment::Dot))
            .count()
    }

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
    fn inner_match<'path>(
        &self,
        mut path: &'path str,
        captures: &mut Vec<&'path str>,
    ) -> Option<&'path str> {
        let mut peek = self.segments.iter().peekable();
        while let Some(segment) = peek.next() {
            path = match segment {
                Segment::Exact(e) => {
                    if path.starts_with(&**e) {
                        &path[e.len()..]
                    } else {
                        return None;
                    }
                }

                Segment::Param(_) => {
                    if path.is_empty() {
                        return None;
                    }
                    match peek.peek() {
                        None | Some(Segment::Slash) => {
                            #[cfg(feature = "memchr")]
                            let capture = memchr::memchr(b'/', path.as_bytes())
                                .map(|index| &path[..index])
                                .unwrap_or(path);
                            #[cfg(not(feature = "memchr"))]
                            let capture = path.split('/').next()?;

                            captures.push(capture);
                            &path[capture.len()..]
                        }

                        Some(Segment::Dot) => {
                            #[cfg(feature = "memchr")]
                            let index = memchr::memchr2(b'.', b'/', path.as_bytes())?;
                            #[cfg(not(feature = "memchr"))]
                            let index = path.find(|c| c == '.' || c == '/')?;

                            if path.chars().nth(index) == Some('.') {
                                captures.push(&path[..index]);
                                &path[index..] // we leave the dot so it can be matched by the Segment::Dot
                            } else {
                                return None;
                            }
                        }
                        _ => panic!(
                            "param must be followed by a dot, a slash, or the end of the route"
                        ),
                    }
                }

                Segment::Wildcard => match peek.peek() {
                    Some(_) => panic!(concat!(
                        "wildcard must currently be the terminal segment, ",
                        "please file an issue if you have a use case for a mid-route *"
                    )),
                    None => {
                        captures.push(path);
                        ""
                    }
                },

                Segment::Slash => match (path.chars().next(), peek.peek()) {
                    (Some('/'), Some(_)) => &path[1..],
                    (None, None) => path,
                    (None, Some(Segment::Wildcard)) => path,
                    _ => return None,
                },

                Segment::Dot => match path.chars().next() {
                    Some('.') => &path[1..],
                    _ => return None,
                },
            }
        }

        Some(path)
    }

    /// Returns a vec of captured str slices for this routespec
    #[inline]
    pub fn matches<'path>(&self, path: &'path str) -> Option<Vec<&'path str>> {
        let mut p = path.trim_start_matches('/').trim_end_matches('/');
        let mut captures = vec![];
        p = self.inner_match(p, &mut captures)?;
        if p.is_empty() || p == "/" {
            Some(captures)
        } else {
            None
        }
    }

    /// populate this route spec with the params and/or wildcard from
    /// a [`Captures`], if it matches.
    pub fn template<'route, 'keys, 'captures, 'values>(
        &'route self,
        captures: &'captures Captures<'keys, 'values>,
    ) -> Option<ReverseMatch<'keys, 'values, 'captures, 'route>> {
        ReverseMatch::new(captures, self)
    }
}

impl FromStr for RouteSpec {
    type Err = String;

    fn from_str(source: &str) -> Result<Self, Self::Err> {
        let mut last_index = 0;
        let source_trimmed = source.trim_start_matches('/').trim_end_matches('/');
        #[cfg(feature = "memchr")]
        let index_iter = memchr::memchr2_iter(b'.', b'/', source_trimmed.as_bytes());

        #[cfg(not(feature = "memchr"))]
        let index_iter = source_trimmed
            .match_indices(|c| c == '.' || c == '/')
            .map(|(i, _)| i);

        let segments = index_iter
            .chain(iter::once_with(|| source_trimmed.len()))
            .try_fold(vec![], |mut acc, index| {
                let first_char = if last_index == 0 {
                    None
                } else {
                    source_trimmed.chars().nth(last_index - 1)
                };

                let section = &source_trimmed[last_index..index];
                last_index = index + 1;

                let segment = match (section.chars().next(), section.len()) {
                    (Some('*'), 1) => Some(Segment::Wildcard),
                    (Some('*'), _) => {
                        return Err(format!(
                            concat!(
                                "since there can only be one wildcard,",
                                " it doesn't need a name. replace `{}` with `*`"
                            ),
                            section
                        ));
                    }
                    (Some(':'), 1) => {
                        return Err(String::from("params must be named"));
                    }
                    (Some(':'), _) => Some(Segment::Param(SmartString::from(&section[1..]))),
                    (None, 0) => None,
                    (_, _) => Some(Segment::Exact(SmartString::from(section))),
                };

                if first_char == Some('.') {
                    if let Some(Segment::Exact(s)) = acc.last_mut() {
                        s.push('.');
                    } else {
                        acc.push(Segment::Dot);
                    }
                }

                if let Some(segment) = segment {
                    if first_char == Some('/') {
                        acc.push(Segment::Slash);
                    }
                    acc.push(segment);
                }

                Ok(acc)
            })?;

        Ok(Self {
            source: Some(SmartString::from(source)),
            segments,
        })
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
        }
    }
}

impl PartialOrd for RouteSpec {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for RouteSpec {
    fn cmp(&self, other: &Self) -> Ordering {
        self.segments
            .iter()
            .zip(&other.segments)
            .map(|(mine, theirs)| mine.cmp(theirs))
            .chain(iter::once_with(|| self.dots().cmp(&other.dots())))
            .chain(iter::once_with(|| {
                other.segments.len().cmp(&self.segments.len())
            }))
            .find(|c| *c != Ordering::Equal)
            .unwrap_or(Ordering::Less)
            .reverse()
    }
}
