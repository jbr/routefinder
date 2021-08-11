use crate::{Captures, Match, ReverseMatch, Segment};
use smartstring::alias::String as SmartString;
use std::{
    cmp::Ordering,
    convert::{TryFrom, TryInto},
    fmt::{self, Debug, Display, Formatter},
    iter,
    str::FromStr,
};

/// A parsed [`RouteSpec`] and associated handler
pub struct Route<T> {
    definition: RouteSpec,
    handler: T,
}

impl<T> Debug for Route<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("Route({})", &self.definition))
    }
}

impl<T> PartialEq for Route<T> {
    fn eq(&self, other: &Self) -> bool {
        self.definition == other.definition
    }
}

impl<T> Eq for Route<T> {}

impl<T> PartialOrd for Route<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Ord for Route<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.definition.cmp(&other.definition)
    }
}

impl<T> Route<T> {
    /// create a new standalone route from the provided [`TryInto<RouteSpec>`] and handler
    pub fn new<R>(route: R, handler: T) -> Result<Self, <R as TryInto<RouteSpec>>::Error>
    where
        R: TryInto<RouteSpec>,
    {
        Ok(Self {
            definition: route.try_into()?,
            handler,
        })
    }

    /// the [`RouteSpec`] for this [`Route`]
    pub fn definition(&self) -> &RouteSpec {
        &self.definition
    }

    /// borrow whatever handler T is contained in this route
    pub fn handler(&self) -> &T {
        &self.handler
    }

    /// a slice of [`RouteSpec`] [`Segment`]s that represents this route
    pub fn segments(&self) -> &[Segment] {
        &self.definition.segments[..]
    }

    /// attempts to build a `Match` against the provided path. Returns
    /// None if there is not a match.
    pub fn matches<'router, 'path>(
        &'router self,
        path: &'path str,
    ) -> Option<Match<'router, 'path, T>> {
        self.definition.matches(path).map(|captures| Match {
            route: self,
            captures,
            path,
        })
    }

    /// populate this route with the params and/or wildcard from a
    /// [`Captures`], if it matches.
    pub fn template<'route, 'keys, 'captures, 'values>(
        &'route self,
        captures: &'captures Captures<'keys, 'values>,
    ) -> Option<ReverseMatch<'keys, 'values, 'captures, 'route, T>> {
        ReverseMatch::new(captures, self)
    }
}

/// Routefinder's representation of the parsed route
///
/// This contains both the source string (or unique description) and
/// an ordered sequence of [`Segment`]s
#[derive(PartialEq, Eq, Debug, Clone)]
pub struct RouteSpec {
    source: SmartString,
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
    pub(crate) fn new(source: &str, segments: Vec<Segment>) -> Self {
        Self {
            source: SmartString::from(source),
            segments,
        }
    }

    fn dots(&self) -> usize {
        self.segments
            .iter()
            .filter(|c| matches!(c, Segment::Dot))
            .count()
    }

    #[inline]
    fn inner_match<'router, 'path>(
        &'router self,
        mut path: &'path str,
        captures: &mut Vec<&'path str>,
    ) -> Option<&'path str> {
        let mut peek = self.segments.iter().peekable();
        while let Some(segment) = peek.next() {
            path = match &*segment {
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
                            let capture = path.split('/').next()?;
                            captures.push(capture);
                            &path[capture.len()..]
                        }

                        Some(Segment::Dot) => {
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
    pub fn matches<'router, 'path>(&'router self, path: &'path str) -> Option<Vec<&'path str>> {
        let mut p = path.trim_start_matches('/').trim_end_matches('/');
        let mut captures = vec![];
        p = self.inner_match(p, &mut captures)?;
        if p.is_empty() || p == "/" {
            Some(captures)
        } else {
            None
        }
    }
}

impl FromStr for RouteSpec {
    type Err = String;

    fn from_str(source: &str) -> Result<Self, Self::Err> {
        let mut last_index = 0;
        let source_trimmed = source.trim_start_matches('/').trim_end_matches('/');
        let segments = source_trimmed
            .match_indices(|c| c == '.' || c == '/')
            .chain(iter::once_with(|| (source_trimmed.len(), "/")))
            .try_fold(vec![], |mut acc, (index, _c)| {
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
                        let message = format!(
                            concat!(
                                "since there can only be one wildcard,",
                                " it doesn't need a name. replace `{}` with `*`"
                            ),
                            section
                        );
                        return Err(message);
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

        Ok(RouteSpec::new(source, segments))
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
            source: SmartString::new(),
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
    }
}
