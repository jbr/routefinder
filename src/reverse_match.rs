use std::ops::Deref;

use crate::{Captures, Route, Segment};

#[derive(Debug, Clone, Copy)]
pub struct ReverseMatch<'captures, 'route, T> {
    route: &'route Route<T>,
    captures: &'captures Captures,
}
impl<'captures, 'route, T> ReverseMatch<'captures, 'route, T> {
    pub fn new(captures: &'captures Captures, route: &'route Route<T>) -> Self {
        Self { route, captures }
    }

    pub fn route(&self) -> &Route<T> {
        &self.route
    }

    pub fn captures(&self) -> &Captures {
        &self.captures
    }
}

impl<'captures, 'route, T> std::fmt::Display for ReverseMatch<'captures, 'route, T> {
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

impl<'captures, 'route, T> Deref for ReverseMatch<'captures, 'route, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.route.handler()
    }
}
