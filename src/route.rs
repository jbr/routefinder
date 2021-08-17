use crate::{Captures, Match, ReverseMatch, RouteSpec, Segment};

use std::{
    cmp::Ordering,
    convert::{TryInto},
    fmt::{self, Debug, Formatter},
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
        &self.definition.segments()
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
