use crate::{Captures, ReverseMatch, Segment};
use std::{
    cmp::Ordering,
    convert::{TryFrom, TryInto},
    fmt::{self, Debug, Display, Formatter},
    iter,
    str::FromStr,
};

/// A parsed [`RouteSpec`] and associated handler
pub struct Route<T, N = ()> {
    name: Option<N>,
    definition: RouteSpec,
    handler: T,
}

impl<T, N: Debug> Debug for Route<T, N> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if let Some(ref name) = self.name {
            f.write_fmt(format_args!("{:?}: ", name))?;
        }
        f.write_fmt(format_args!("Route({:?})", &self.definition))
    }
}

impl<T, N> PartialEq for Route<T, N> {
    fn eq(&self, other: &Self) -> bool {
        self.definition == other.definition
    }
}

impl<T, N> Eq for Route<T, N> {}

impl<T, N> PartialOrd for Route<T, N> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T, N> Ord for Route<T, N> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.definition.cmp(&other.definition)
    }
}

impl<T, N: Debug> Route<T, N> {
    pub(crate) fn new<R>(
        route: R,
        handler: T,
        name: Option<N>,
    ) -> Result<Self, <R as TryInto<RouteSpec>>::Error>
    where
        R: TryInto<RouteSpec>,
    {
        Ok(Self {
            name,
            definition: route.try_into()?,
            handler,
        })
    }

    /// the name for this route, if provided
    pub fn name(&self) -> Option<&N> {
        self.name.as_ref()
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

    /// templates out a path with the provided captures
    pub fn template(&self, captures: &Captures<'_, '_>) -> Result<String, String> {
        ReverseMatch::new(&captures, &self)
            .map(|m| m.to_string())
            .ok_or_else(|| format!("could not template {:?} with {:?}", &self, &captures))
    }
}

/// Routefinder's representation of the parsed route
///
/// This contains both the source string (or unique description) and
/// an ordered sequence of [`Segment`]s
#[derive(PartialEq, Eq)]
pub struct RouteSpec {
    source: String,
    segments: Vec<Segment>,
}

impl Display for RouteSpec {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
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

impl Debug for RouteSpec {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
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
            .chain(iter::once_with(|| {
                other.segments.len().cmp(&self.segments.len())
            }))
            .find(|c| *c != Ordering::Equal)
            .unwrap_or(Ordering::Less)
    }
}
