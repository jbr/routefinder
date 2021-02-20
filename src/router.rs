use std::convert::TryInto;

use crate::{Match, Matches, Route, RouteDefinition};

#[derive(Debug)]
pub struct Router<T> {
    routes: Vec<Route<T>>,
}

impl<T> Default for Router<T> {
    fn default() -> Self {
        Self { routes: vec![] }
    }
}

impl<T> Router<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add<R>(
        &mut self,
        route: R,
        handler: T,
    ) -> Result<(), <R as TryInto<RouteDefinition<'static>>>::Error>
    where
        R: TryInto<RouteDefinition<'static>>,
    {
        self.routes.push(Route::new(route, handler)?);
        Ok(())
    }

    pub fn matches<'a, 'b>(&'a self, path: &'b str) -> Matches<'a, 'b, T> {
        Matches::for_routes_and_path(&self.routes[..], path)
    }

    pub fn best_match<'a, 'b>(&'a self, path: &'b str) -> Option<Match<'a, 'b, T>> {
        self.routes.iter().filter_map(|r| r.is_match(path)).max()
    }
}
