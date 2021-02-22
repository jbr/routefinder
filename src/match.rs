use std::cmp::Ordering;

use crate::{Captures, Route, Segment};

/// This struct represents the output of a successful application of a
/// [`Route`] to a str path, as well as references to any captures
/// such as params and wildcards.
#[derive(Debug)]
pub struct Match<'router, 'path, T> {
    path: &'path str,
    route: &'router Route<T>,
    captures: Vec<&'path str>,
}

impl<'router, 'path, T> Match<'router, 'path, T> {
    pub(crate) fn new(
        path: &'path str,
        route: &'router Route<T>,
        captures: Vec<&'path str>,
    ) -> Self {
        Self {
            path,
            route,
            captures,
        }
    }

    /// Returns a reference to the handler associated with this route
    pub fn handler(&self) -> &'router T {
        self.route.handler()
    }

    /// Returns the [`Captures`] for this match
    pub fn captures(&self) -> Captures {
        self.route
            .segments()
            .iter()
            .filter(|s| matches!(s, Segment::Param(_) | Segment::Wildcard))
            .zip(&self.captures)
            .fold(
                Captures::default(),
                |mut captures, (segment, capture)| match segment {
                    Segment::Param(name) => {
                        captures.0.push((name.clone(), String::from(*capture)));
                        captures
                    }

                    Segment::Wildcard => {
                        captures.1 = Some(String::from(*capture));
                        captures
                    }
                    _ => captures,
                },
            )
    }
}

impl<'router, 'path, T> PartialEq for Match<'router, 'path, T> {
    fn eq(&self, other: &Self) -> bool {
        *other.route == *self.route
    }
}

impl<'router, 'path, T> Eq for Match<'router, 'path, T> {}

impl<'router, 'path, T> PartialOrd for Match<'router, 'path, T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<'router, 'path, T> Ord for Match<'router, 'path, T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.route.cmp(&other.route)
    }
}
