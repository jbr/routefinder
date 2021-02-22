use std::cmp::Ordering;
use std::convert::{TryFrom, TryInto};
use std::str::FromStr;

use crate::{Match, Segment};

/// A parsed [`RouteSpec`] and associated handler
pub struct Route<T> {
    definition: RouteSpec,
    handler: T,
}

impl<T> std::fmt::Debug for Route<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("Route({:?})", &self.definition))
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
    pub(crate) fn new<R>(route: R, handler: T) -> Result<Self, <R as TryInto<RouteSpec>>::Error>
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

    /// performs the test of whether this route matches a given path str
    pub fn is_match<'a, 'b>(&'a self, path: &'b str) -> Option<Match<'a, 'b, T>> {
        let mut p = path.trim_start_matches('/').trim_end_matches('/');
        let mut captures = vec![];

        let mut peek = self.definition.segments.iter().peekable();
        while let Some(segment) = peek.next() {
            p = match &*segment {
                Segment::Exact(e) => {
                    if p.starts_with(&*e) {
                        &p[e.len()..]
                    } else {
                        return None;
                    }
                }

                Segment::Param(_) => {
                    if p.is_empty() { return None; }
                    match peek.peek() {
                        None | Some(Segment::Slash) => {
                            let capture = p.split('/').next()?;
                            captures.push(capture);
                            &p[capture.len()..]
                        }
                        Some(Segment::Dot) => {
                            let index = p.find(|c| c == '.' || c == '/')?;
                            if p.chars().nth(index) == Some('.') {
                                captures.push(&p[..index]);
                                &p[index + 1..]
                            } else {
                                return None;
                            }
                        }
                        _ => panic!("param must be followed by a dot, a slash, or the end of the route"),
                    }
                }

                Segment::Wildcard => {
                    match peek.peek() {
                        Some(_) => panic!("wildcard must currently be the terminal segment, please file an issue if you have a use case for a mid-route *"),
                        None => {
                            captures.push(p);
                            ""
                        }
                    }
                }

                Segment::Slash => match (p.chars().next(), peek.peek()) {
                    (Some('/'),Some(_)) => &p[1..],
                    (None, None) => p,
                    (None, Some(Segment::Wildcard)) => p,
                    _ => return None,
                }

                Segment::Dot => match p.chars().next() {
                    Some('.') => &p[1..],
                    _ => return None,
                }
            }
        }

        if p.is_empty() || p == "/" {
            Some(Match::new(path, &self, captures))
        } else {
            None
        }
    }
}

/// the internal representation of a route, containing both the source
/// string (or unique description) and a Vec of [`Segment`]s
#[derive(PartialEq, Eq)]
pub struct RouteSpec {
    source: String,
    segments: Vec<Segment>,
}

impl std::fmt::Display for RouteSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("/")?;
        for segment in &self.segments {
            match segment {
                Segment::Slash => f.write_str("/")?,
                Segment::Dot => f.write_str(".")?,
                Segment::Exact(s) => f.write_str(&s)?,
                Segment::Param(p) => f.write_fmt(format_args!(":{}", p))?,
                Segment::Wildcard => f.write_str("*")?,
            };
        }
        Ok(())
    }
}

impl std::fmt::Debug for RouteSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self))
    }
}

impl RouteSpec {
    pub fn new(source: &str, segments: Vec<Segment>) -> Self {
        Self {
            source: String::from(source),
            segments,
        }
    }
}

impl FromStr for RouteSpec {
    type Err = String;

    fn from_str(source: &str) -> Result<Self, Self::Err> {
        let segments = source
            .trim_start_matches('/')
            .trim_end_matches('/')
            .split('/')
            .try_fold(vec![], |mut acc, section| {
                let segment =                 match (section.chars().next(), section.len()) {
                    (Some('*'), 1) => Some(Segment::Wildcard),
                    (Some('*'), _) => return Err(format!("since there can only be one wildcard, it doesn't need a name. replace `{}` with `*`", section)),
                    (Some(':'), 1) => return Err(String::from("params must be named")),
                    (Some(':'), _) => Some(Segment::Param(String::from(&section[1..]))),
                    (None, 0) => None,
                    (_, _) => Some(Segment::Exact(String::from(section))),
                };
                if let Some(segment) = segment {
                    if !acc.is_empty() {acc.push(Segment::Slash); }
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
            .chain(std::iter::once_with(|| {
                other.segments.len().cmp(&self.segments.len())
            }))
            .find(|c| *c != Ordering::Equal)
            .unwrap_or(Ordering::Less)
    }
}
