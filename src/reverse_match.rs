use std::ops::Deref;

use crate::{Captures, Route, Segment};
/// This struct represents the result of a reverse lookup from
/// Captures to a Route
#[derive(Debug, Clone, Copy)]
pub struct ReverseMatch<'keys, 'values, 'captures, 'route, T> {
    route: &'route Route<T>,
    captures: &'captures Captures<'keys, 'values>,
}

impl<'keys, 'values, 'captures, 'route, T> ReverseMatch<'keys, 'values, 'captures, 'route, T> {
    /// attempts to build a new ReverseMatch. Returns None if the
    /// match was unsuccessful.
    pub fn new(
        captures: &'captures Captures<'keys, 'values>,
        route: &'route Route<T>,
    ) -> Option<Self> {
        let all_params_matched = route
            .segments()
            .iter()
            .filter_map(|s| match s {
                Segment::Param(s) => Some(s),
                _ => None,
            })
            .eq(captures.params().iter().map(|c| c.name()));

        if !all_params_matched {
            return None;
        }

        if captures.wildcard().is_some()
            && !matches!(route.segments().last(), Some(Segment::Wildcard))
        {
            return None;
        }

        Some(Self { captures, route })
    }

    /// returns the Route for this ReverseMatch
    pub fn route(&self) -> &Route<T> {
        &self.route
    }

    /// returns the Captures for this ReverseMatch
    pub fn captures(&self) -> &Captures {
        &self.captures
    }
}

impl<'keys, 'values, 'captures, 'route, T> std::fmt::Display
    for ReverseMatch<'keys, 'values, 'captures, 'route, T>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("/")?;
        for segment in self.route.segments() {
            match segment {
                Segment::Slash => f.write_str("/")?,
                Segment::Dot => f.write_str(".")?,
                Segment::Exact(s) => f.write_str(s)?,
                Segment::Param(p) => f.write_str(self.captures.get(p).unwrap())?,
                Segment::Wildcard => f.write_str(self.captures.wildcard().unwrap_or_default())?,
            };
        }
        Ok(())
    }
}

impl<'keys, 'values, 'captures, 'route, T> Deref
    for ReverseMatch<'keys, 'values, 'captures, 'route, T>
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.route.handler()
    }
}
