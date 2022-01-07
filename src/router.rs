use crate::{Match, RouteSpec};
use std::{
    collections::{
        btree_map::{IntoIter, Iter, IterMut},
        BTreeMap,
    },
    convert::TryInto,
    fmt::{self, Debug, Formatter},
    iter::FromIterator,
};

/// The top level struct for routefinder
///
/// A router represents an ordered set of routes which can be applied
/// to a given request path, and any handler T that is associated with
/// each route

pub struct Router<Handler> {
    routes: BTreeMap<RouteSpec, Handler>,
}

impl<Handler> Debug for Router<Handler> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut debug_set = f.debug_set();
        for route in self.routes.keys() {
            debug_set.entry(&format_args!("{}", route));
        }
        debug_set.finish()
    }
}

impl<Handler> Default for Router<Handler> {
    fn default() -> Self {
        Self {
            routes: Default::default(),
        }
    }
}

impl<Handler> IntoIterator for Router<Handler> {
    type Item = (RouteSpec, Handler);
    type IntoIter = IntoIter<RouteSpec, Handler>;

    fn into_iter(self) -> Self::IntoIter {
        self.routes.into_iter()
    }
}

impl<'a, Handler: 'a> IntoIterator for &'a Router<Handler> {
    type Item = (&'a RouteSpec, &'a Handler);

    type IntoIter = Iter<'a, RouteSpec, Handler>;

    fn into_iter(self) -> Self::IntoIter {
        self.routes.iter()
    }
}

impl<'a, Handler: 'a> IntoIterator for &'a mut Router<Handler> {
    type Item = (&'a RouteSpec, &'a mut Handler);

    type IntoIter = IterMut<'a, RouteSpec, Handler>;

    fn into_iter(self) -> Self::IntoIter {
        self.routes.iter_mut()
    }
}

impl<Handler> FromIterator<(RouteSpec, Handler)> for Router<Handler> {
    fn from_iter<T: IntoIterator<Item = (RouteSpec, Handler)>>(iter: T) -> Self {
        Self {
            routes: iter.into_iter().collect(),
        }
    }
}

impl<Handler> Router<Handler> {
    /// Builds a new router
    ///
    /// ```rust
    /// let mut router = routefinder::Router::new();
    /// router.add("/", ()).unwrap(); // here we use () as the handler
    /// assert!(router.best_match("/").is_some());
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a route to the router, accepting any type that implements TryInto<[`RouteSpec`]>. In most circumstances, this will be a &str or a String.
    ///
    /// ```rust
    /// let mut router = routefinder::Router::new();
    /// assert!(router.add("*named_wildcard", ()).is_err());
    /// assert!(router.add("*", ()).is_ok());
    /// assert!(router.add(format!("/dynamic/{}", "route"), ()).is_ok());
    /// ```
    pub fn add<R>(
        &mut self,
        route: R,
        handler: Handler,
    ) -> Result<(), <R as TryInto<RouteSpec>>::Error>
    where
        R: TryInto<RouteSpec>,
    {
        self.routes.insert(route.try_into()?, handler);
        Ok(())
    }

    /// Returns the single best route match as defined by the sorting
    /// rules. To compare any two routes, step through each
    /// [`Segment`][crate::Segment] and find the first pair that are not equal,
    /// according to: `Exact > Param > Wildcard > (dots and slashes)`
    /// As a result, `/hello` > `/:param` > `/*`.  Because we can sort
    /// the routes before encountering a path, we evaluate them from
    /// highest to lowest weight and an early return as soon as we
    /// find a match.
    ///
    /// ```rust
    /// let mut router = routefinder::Router::new();
    /// router.add("*", 0).unwrap();
    /// router.add("/:param", 1).unwrap();
    /// router.add("/hello", 2).unwrap();
    /// assert_eq!(*router.best_match("/hello").unwrap(), 2);
    /// assert_eq!(*router.best_match("/hey").unwrap(), 1);
    /// assert_eq!(router.best_match("/hey").unwrap().captures().get("param"), Some("hey"));
    /// assert_eq!(*router.best_match("/hey/there").unwrap(), 0);
    /// assert_eq!(*router.best_match("/").unwrap(), 0);
    /// ```
    pub fn best_match<'a, 'b>(&'a self, path: &'b str) -> Option<Match<'a, 'b, Handler>> {
        self.match_iter(path).next()
    }

    /// Returns _all_ of the matching routes for a given path. This is
    /// probably not what you want, as [`Router::best_match`] is more
    /// efficient. The primary reason you'd want to use `matches` is
    /// to implement different route precedence rules or for
    /// testing.
    ///
    /// ```rust
    /// let mut router = routefinder::Router::new();
    /// router.add("*", ()).unwrap();
    /// router.add("/:param", ()).unwrap();
    /// router.add("/hello", ()).unwrap();
    /// assert_eq!(router.matches("/").len(), 1);
    /// assert_eq!(router.matches("/hello").len(), 3);
    /// assert_eq!(router.matches("/hey").len(), 2);
    /// assert_eq!(router.matches("/hey/there").len(), 1);
    /// ```
    pub fn matches<'a, 'b>(&'a self, path: &'b str) -> Vec<Match<'a, 'b, Handler>> {
        self.match_iter(path).collect()
    }

    /// Returns an iterator over the possible matches for this
    /// particular path. Because rust iterators are lazy, this is
    /// useful for some filtering operations that might otherwise use
    /// [`Router::matches`], which is this iterator collected into a
    /// vec.
    pub fn match_iter<'a, 'b>(&'a self, path: &'b str) -> MatchIter<'a, 'b, Handler> {
        MatchIter {
            iter: self.routes.iter(),
            path,
        }
    }

    /// Returns an iterator of references to `(&RouteSpec, &Handler)`
    ///
    /// ```
    /// let mut router = routefinder::Router::new();
    /// router.add("*", 1).unwrap();
    /// router.add("/:param", 2).unwrap();
    /// router.add("/hello", 3).unwrap();
    /// let (route, handler) = router.iter().next().unwrap();
    /// assert_eq!(route.to_string(), "/hello");
    /// assert_eq!(handler, &3);
    /// ```
    pub fn iter(&self) -> impl Iterator<Item = (&RouteSpec, &Handler)> {
        self.into_iter()
    }

    /// returns an iterator of `(&RouteSpec, &mut Handler)`
    ///
    /// ```
    /// let mut router = routefinder::Router::new();
    /// router.add("*", 1).unwrap();
    /// router.add("/:param", 2).unwrap();
    /// router.add("/hello", 3).unwrap();
    ///
    /// assert_eq!(*router.best_match("/hello").unwrap(), 3);
    /// let (route, handler) = router.iter_mut().next().unwrap();
    /// assert_eq!(route.to_string(), "/hello");
    /// *handler = 10;
    /// assert_eq!(*router.best_match("/hello").unwrap(), 10);
    /// ```
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&RouteSpec, &mut Handler)> {
        self.into_iter()
    }

    /// returns the number of routes that have been added
    pub fn len(&self) -> usize {
        self.routes.len()
    }

    /// returns true if no routes have been added
    pub fn is_empty(&self) -> bool {
        self.routes.is_empty()
    }

    /// get a reference to the handler for the given route spec
    pub fn get_handler(&self, spec: impl TryInto<RouteSpec>) -> Option<&Handler> {
        spec.try_into().ok().and_then(|sp| self.routes.get(&sp))
    }

    /// get a mut reference to the handler for the given route spec
    pub fn get_handler_mut(&mut self, spec: impl TryInto<RouteSpec>) -> Option<&mut Handler> {
        spec.try_into()
            .ok()
            .and_then(move |sp| self.routes.get_mut(&sp))
    }
}

/// an iterator over matches for a given path. returned by [`Router::match_iter`]
#[derive(Debug)]
pub struct MatchIter<'a, 'b, Handler> {
    iter: Iter<'a, RouteSpec, Handler>,
    path: &'b str,
}
impl<'a, 'b, Handler> Iterator for MatchIter<'a, 'b, Handler> {
    type Item = Match<'a, 'b, Handler>;

    fn next(&mut self) -> Option<Self::Item> {
        let path = self.path;
        self.iter.find_map(|(route, handler)| {
            route.matches(path).map(|captures| Match {
                path,
                route,
                captures,
                handler,
            })
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}
