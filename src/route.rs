use std::cmp::Ordering;
use std::convert::{TryFrom, TryInto};

use crate::{Match, Segment};
#[derive(Debug)]
pub struct Route<T> {
    definition: RouteDefinition<'static>,
    handler: T,
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
    pub(crate) fn new<R>(
        route: R,
        handler: T,
    ) -> Result<Self, <R as TryInto<RouteDefinition<'static>>>::Error>
    where
        R: TryInto<RouteDefinition<'static>>,
    {
        Ok(Self {
            definition: route.try_into()?,
            handler,
        })
    }

    pub fn definition(&self) -> &RouteDefinition {
        &self.definition
    }

    pub fn handler(&self) -> &T {
        &self.handler
    }
    pub fn segments(&self) -> &[Segment] {
        &self.definition.segments[..]
    }

    pub fn is_match<'a, 'b>(&'a self, path: &'b str) -> Option<Match<'a, 'b, T>> {
        let mut p = path.trim_start_matches('/').trim_end_matches('/');
        let mut captures = vec![];

        let mut peek = self.definition.segments.iter().peekable();
        while let Some(segment) = peek.next() {
            p = match *segment {
                Segment::Exact(e) => {
                    if p.starts_with(e) {
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

#[derive(Debug, PartialEq, Eq)]
pub struct RouteDefinition<'s> {
    source: &'s str,
    segments: Vec<Segment<'s>>,
}

impl TryFrom<&'static str> for RouteDefinition<'static> {
    type Error = String;

    fn try_from(s: &'static str) -> Result<Self, Self::Error> {
        let segments = s
            .trim_start_matches('/')
            .trim_end_matches('/')
            .split('/')
            .try_fold(vec![], |mut acc, section| {
                let segment =                 match (section.chars().next(), section.len()) {
                    (Some('*'), 1) => Some(Segment::Wildcard),
                    (Some('*'), _) => return Err(format!("since there can only be one wildcard, it doesn't need a name. replace \"*{}\" with \"*\"", section)),
                    (Some(':'), 1) => return Err(String::from("params must be named")),
                    (Some(':'), _) => Some(Segment::Param(&section[1..])),
                    (None, 0) => None,
                    (_, _) => Some(Segment::Exact(section)),
                };
                if let Some(segment) = segment {
                    if !acc.is_empty() {acc.push(Segment::Slash); }
                    acc.push(segment);
                }
                Ok(acc)
            })?;

        Ok(RouteDefinition {
            source: s,
            segments,
        })
    }
}

impl<'s> PartialOrd for RouteDefinition<'s> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<'s> Ord for RouteDefinition<'s> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.segments
            .iter()
            .zip(&other.segments)
            .map(|(mine, theirs)| mine.cmp(theirs))
            .chain(std::iter::once_with(|| {
                other.segments.len().cmp(&self.segments.len())
            }))
            .find(|c| *c != Ordering::Equal)
            .unwrap_or_else(|| Ordering::Less)
    }
}
