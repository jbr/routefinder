use crate::{Capture, Captures, RouteSpec, Segment};
use std::{cmp::Ordering, ops::Deref};

/// The output of a successful application of a [`RouteSpec`] to a str
/// path, as well as references to any captures.
///
/// It dereferences to the contained Handler type

#[derive(Debug)]
pub struct Match<'router, 'path, Handler> {
    pub(crate) path: &'path str,
    pub(crate) route: &'router RouteSpec,
    pub(crate) captures: Vec<&'path str>,
    pub(crate) handler: &'router Handler,
}

impl<'router, 'path, Handler> Match<'router, 'path, Handler> {
    /// Returns a reference to the handler associated with this route
    pub fn handler(&self) -> &'router Handler {
        self.handler
    }

    /// Returns the routespec for this route
    pub fn route(&self) -> &'router RouteSpec {
        self.route
    }

    /// returns the exact path that was matched
    pub fn path(&self) -> &'path str {
        self.path
    }

    /// Returns the [`Captures`] for this match
    pub fn captures(&self) -> Captures<'router, 'path> {
        self.route
            .segments()
            .iter()
            .filter(|s| matches!(s, Segment::Param(_) | Segment::Wildcard))
            .zip(&self.captures)
            .fold(
                Captures::default(),
                |mut captures, (segment, capture)| match segment {
                    Segment::Param(name) => {
                        captures.push(Capture::new(&**name, *capture));
                        captures
                    }

                    Segment::Wildcard => {
                        captures.set_wildcard(*capture);
                        captures
                    }
                    _ => captures,
                },
            )
    }
}

impl<'router, 'path, Handler> PartialEq for Match<'router, 'path, Handler> {
    fn eq(&self, other: &Self) -> bool {
        *other.route == *self.route
    }
}

impl<'router, 'path, Handler> Eq for Match<'router, 'path, Handler> {}

impl<'router, 'path, Handler> PartialOrd for Match<'router, 'path, Handler> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<'router, 'path, Handler> Ord for Match<'router, 'path, Handler> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.route.cmp(other.route)
    }
}

impl<'router, 'path, Handler> Deref for Match<'router, 'path, Handler> {
    type Target = Handler;

    fn deref(&self) -> &Self::Target {
        self.handler
    }
}
