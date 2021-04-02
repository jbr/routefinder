use std::collections::BTreeSet;
use std::convert::TryInto;

use crate::{Captures, Match, ReverseMatch, Route, RouteSpec};

/// a router represents an ordered set of routes which can be applied
/// to a given request path, and any handler T that is associated with
/// each route

pub struct Router<T> {
    routes: BTreeSet<Route<T>>,
}

impl<T> std::fmt::Debug for Router<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_set().entries(self.routes.iter()).finish()
    }
}

impl<T> Default for Router<T> {
    fn default() -> Self {
        Self {
            routes: BTreeSet::new(),
        }
    }
}

impl<T> Router<T> {
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
    pub fn add<R>(&mut self, route: R, handler: T) -> Result<(), <R as TryInto<RouteSpec>>::Error>
    where
        R: TryInto<RouteSpec>,
    {
        self.routes.insert(Route::new(route, handler)?);
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
    pub fn best_match<'a, 'b>(&'a self, path: &'b str) -> Option<Match<'a, 'b, T>> {
        self.routes
            .iter()
            .rev()
            .find_map(|route| Match::new(path, route))
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
    /// assert!(!router.matches("/").is_empty());
    /// assert_eq!(router.matches("/hello").len(), 3);
    /// assert_eq!(router.matches("/hey").len(), 2);
    /// assert_eq!(router.matches("/hey/there").len(), 1);
    /// ```
    pub fn matches<'a, 'b>(&'a self, path: &'b str) -> Vec<Match<'a, 'b, T>> {
        self.routes
            .iter()
            .filter_map(|route| Match::new(path, route))
            .collect()
    }

    /// Returns the single best [`ReverseMatch`] for the provided captures
    ///
    /// ```rust
    /// use routefinder::{Router, Captures};
    /// let mut router = Router::new();
    /// router.add("/users/:user_id/posts/:post_id", ()).unwrap();
    ///
    /// let captures = Captures::from(vec![("user_id", "1"), ("post_id", "10")]);
    /// let reverse_match = router.best_reverse_match(&captures).unwrap();
    /// assert_eq!("/users/1/posts/10", reverse_match.to_string());
    /// ```
    pub fn best_reverse_match<'keys, 'values, 'router, 'captures>(
        &'router self,
        captures: &'captures Captures<'keys, 'values>,
    ) -> Option<ReverseMatch<'keys, 'values, 'captures, 'router, T>> {
        self.routes
            .iter()
            .find_map(|route| ReverseMatch::new(captures, route))
    }

    /// Returns all valid [`ReverseMatch`]es for the provided captures
    ///
    /// ```rust
    /// use routefinder::{Router, Captures};
    /// let mut router = Router::new();
    /// router.add("/settings/:user_id/*", ()).unwrap();
    /// router.add("/profiles/:user_id", ()).unwrap();
    ///
    /// let captures = Captures::from(vec![("user_id", "1")]);
    /// let reverse_matches = router.reverse_matches(&captures);
    /// assert_eq!(2, reverse_matches.len());
    /// assert_eq!(
    ///     reverse_matches.iter().map(|r| r.to_string()).collect::<Vec<_>>(),
    ///     vec!["/settings/1/", "/profiles/1"]
    /// );
    /// ```
    pub fn reverse_matches<'keys, 'values, 'router, 'captures>(
        &'router self,
        captures: &'captures Captures<'keys, 'values>,
    ) -> Vec<ReverseMatch<'keys, 'values, 'captures, 'router, T>> {
        self.routes
            .iter()
            .filter_map(|route| ReverseMatch::new(captures, route))
            .collect()
    }
}
