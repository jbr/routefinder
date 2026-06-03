use crate::{trie::TrieMatch, Match, RouteSpec, Trie, TrieIter};
use std::{
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
    /// Routes and their handlers, in insertion order. The trie stores route
    /// specs carrying a `handler_index` into this vec, so matching resolves a
    /// handler with a single O(1) index rather than a second lookup.
    routes: Vec<(RouteSpec, Handler)>,
    trie: Trie,
}

#[cfg(feature = "arbitrary")]
impl<'a> arbitrary::Arbitrary<'a> for Router<usize> {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        let mut router = Self::new();

        for n in 0..u.arbitrary_len::<RouteSpec>()? {
            router.add(RouteSpec::arbitrary(u)?, n).unwrap();
        }
        Ok(router)
    }
}

impl<Handler> Debug for Router<Handler> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut debug_set = f.debug_set();
        for (route, _) in &self.routes {
            debug_set.entry(&format_args!("{route}"));
        }
        debug_set.finish()
    }
}

impl<Handler> Default for Router<Handler> {
    fn default() -> Self {
        Self {
            routes: Default::default(),
            trie: Default::default(),
        }
    }
}

impl<Handler> IntoIterator for Router<Handler> {
    type Item = (RouteSpec, Handler);
    type IntoIter = std::vec::IntoIter<(RouteSpec, Handler)>;

    fn into_iter(self) -> Self::IntoIter {
        self.routes.into_iter()
    }
}

impl<'a, Handler> IntoIterator for &'a Router<Handler> {
    type Item = (&'a RouteSpec, &'a Handler);
    type IntoIter = std::iter::Map<
        std::slice::Iter<'a, (RouteSpec, Handler)>,
        fn(&'a (RouteSpec, Handler)) -> (&'a RouteSpec, &'a Handler),
    >;

    fn into_iter(self) -> Self::IntoIter {
        self.routes.iter().map(|(route, handler)| (route, handler))
    }
}

impl<'a, Handler> IntoIterator for &'a mut Router<Handler> {
    type Item = (&'a RouteSpec, &'a mut Handler);
    type IntoIter = std::iter::Map<
        std::slice::IterMut<'a, (RouteSpec, Handler)>,
        fn(&'a mut (RouteSpec, Handler)) -> (&'a RouteSpec, &'a mut Handler),
    >;

    fn into_iter(self) -> Self::IntoIter {
        self.routes
            .iter_mut()
            .map(|(route, handler)| (&*route, handler))
    }
}

impl<Handler> FromIterator<(RouteSpec, Handler)> for Router<Handler> {
    fn from_iter<T: IntoIterator<Item = (RouteSpec, Handler)>>(iter: T) -> Self {
        let mut router = Self::new();
        for (route_spec, handler) in iter {
            router.insert_route_spec(route_spec, handler);
        }
        router
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

    /// Constructs a new router with the provided routes
    pub fn new_with_routes<RS, I>(iter: I) -> Result<Self, <RS as TryInto<RouteSpec>>::Error>
    where
        I: IntoIterator<Item = (RS, Handler)>,
        RS: TryInto<RouteSpec>,
    {
        let mut router = Self::new();
        for (rs, handler) in iter {
            router.add(rs, handler)?;
        }
        Ok(router)
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
        let route_spec = route.try_into()?;
        self.insert_route_spec(route_spec, handler);
        Ok(())
    }

    fn insert_route_spec(&mut self, mut route_spec: RouteSpec, handler: Handler) {
        route_spec.handler_index = Some(self.routes.len());

        // A route with optional groups is expanded into flat variant specs that
        // all share this route's handler_index; the canonical (optional) spec is
        // what we store and hand back as `Match::route`. Non-optional routes
        // insert a single spec, as before.
        if route_spec.has_optional() {
            for variant in route_spec.expand() {
                self.trie.insert(variant);
            }
        } else {
            self.trie.insert(route_spec.clone());
        }

        self.routes.push((route_spec, handler));
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
        let TrieMatch {
            route,
            mut captures,
            wildcard,
        } = self.trie.best_match(path)?;

        #[cfg(feature = "log")]
        log::trace!(
            "route: {route}\ncaptures: {captures:?}\nwildcard: {}",
            wildcard.unwrap_or_default()
        );

        if let Some(wildcard) = wildcard {
            captures.push(wildcard);
        }

        // Hand back the canonical (as-registered) spec, not the expanded trie
        // variant, so `Match::route()` reflects what the user typed. Capture
        // names still resolve correctly because the captured values are a
        // positional prefix of the canonical's params (see `capture_segments`).
        let (canonical, handler) = self.routes.get(route.handler_index?)?;

        Some(Match {
            path,
            route: canonical,
            captures,
            handler,
        })
    }

    /// returns the number of routes that have been added
    pub fn len(&self) -> usize {
        self.routes.len()
    }

    /// returns true if no routes have been added
    pub fn is_empty(&self) -> bool {
        self.routes.is_empty()
    }

    /// Returns an iterator of references to `(&RouteSpec, &Handler)`, in the
    /// order the routes were added.
    ///
    /// ```
    /// let mut router = routefinder::Router::new();
    /// router.add("*", 1).unwrap();
    /// router.add("/:param", 2).unwrap();
    /// router.add("/hello", 3).unwrap();
    /// let routes: Vec<_> = router.iter().map(|(r, h)| (r.to_string(), *h)).collect();
    /// assert_eq!(routes, [("/*".into(), 1), ("/:param".into(), 2), ("/hello".into(), 3)]);
    /// ```
    pub fn iter(&self) -> impl Iterator<Item = (&RouteSpec, &Handler)> {
        self.into_iter()
    }

    /// Returns an iterator of `(&RouteSpec, &mut Handler)`, in the order the
    /// routes were added.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&RouteSpec, &mut Handler)> {
        self.into_iter()
    }

    /// Returns a reference to the handler previously associated with the
    /// provided route spec, if any.
    ///
    /// ```
    /// let mut router = routefinder::Router::new();
    /// router.add("/hello", 3).unwrap();
    /// assert_eq!(router.get_handler("/hello"), Some(&3));
    /// assert_eq!(router.get_handler("/missing"), None);
    /// ```
    pub fn get_handler(&self, spec: impl TryInto<RouteSpec>) -> Option<&Handler> {
        let spec = spec.try_into().ok()?;
        self.routes
            .iter()
            .find_map(|(route, handler)| (route == &spec).then_some(handler))
    }

    /// Returns a mutable reference to the handler previously associated with the
    /// provided route spec, if any.
    pub fn get_handler_mut(&mut self, spec: impl TryInto<RouteSpec>) -> Option<&mut Handler> {
        let spec = spec.try_into().ok()?;
        self.routes
            .iter_mut()
            .find_map(|(route, handler)| (route == &spec).then_some(handler))
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
    pub fn match_iter<'router, 'path>(
        &'router self,
        path: &'path str,
    ) -> MatchIter<'router, 'path, Handler> {
        let trie_iter = self.trie.match_iter(path);
        MatchIter {
            router: self,
            path,
            trie_iter,
        }
    }
}

#[derive(Debug)]
pub struct MatchIter<'router, 'path, Handler> {
    router: &'router Router<Handler>,
    path: &'path str,
    trie_iter: TrieIter<'router, 'path>,
}

impl<'router, 'path, Handler> Iterator for MatchIter<'router, 'path, Handler> {
    type Item = Match<'router, 'path, Handler>;

    fn next(&mut self) -> Option<Self::Item> {
        let TrieMatch {
            route,
            mut captures,
            wildcard,
        } = self.trie_iter.next()?;
        let handler_index = route.handler_index?;
        let (canonical, handler) = self.router.routes.get(handler_index)?;

        if let Some(wildcard) = wildcard {
            captures.push(wildcard);
        }
        Some(Match {
            path: self.path,
            route: canonical,
            captures,
            handler,
        })
    }
}
