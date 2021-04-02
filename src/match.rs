use std::cmp::Ordering;
use std::ops::Deref;

use crate::{Capture, Captures, Route, RouteSpec, Segment};

/// This struct represents the output of a successful application of a
/// [`Route`] to a str path, as well as references to any captures
/// such as params and wildcards. It dereferences to the contained type T

#[derive(Debug)]
pub struct Match<'router, 'path, T> {
    path: &'path str,
    route: &'router Route<T>,
    captures: Vec<&'path str>,
}

impl<'router, 'path, T> Match<'router, 'path, T> {
    /// Attempts to build a new Match from the provided path string
    /// and route. Returns None if the match was unsuccessful
    pub fn new(path: &'path str, route: &'router Route<T>) -> Option<Self> {
        let mut p = path.trim_start_matches('/').trim_end_matches('/');
        let mut captures = vec![];

        let mut peek = route.segments().iter().peekable();
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
            Some(Self {
                path,
                route,
                captures,
            })
        } else {
            None
        }
    }

    /// Returns a reference to the handler associated with this route
    pub fn handler(&self) -> &'router T {
        self.route.handler()
    }

    /// Returns the routespec for this route
    pub fn route(&self) -> &'router RouteSpec {
        self.route.definition()
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
                        captures.push(Capture::new(name, *capture));
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

impl<'router, 'path, T> Deref for Match<'router, 'path, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.route.handler()
    }
}
