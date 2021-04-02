use std::ops::Deref;

use crate::{Captures, Route, Segment};

#[derive(Debug, Clone, Copy)]
pub struct ReverseMatch<'keys, 'values, 'captures, 'route, T> {
    route: &'route Route<T>,
    captures: &'captures Captures<'keys, 'values>,
}

impl<'keys, 'values, 'captures, 'route, T> ReverseMatch<'keys, 'values, 'captures, 'route, T> {
    pub fn new(captures: &'captures Captures<'keys, 'values>, route: &'route Route<T>) -> Self {
        Self { route, captures }
    }

    pub fn route(&self) -> &Route<T> {
        &self.route
    }

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
