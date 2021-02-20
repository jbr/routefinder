use std::collections::BTreeSet;
use std::convert::{TryFrom, TryInto};
use std::ops::Deref;

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
        self.routes.push(Route {
            definition: route.try_into()?,
            handler,
        });
        Ok(())
    }

    pub fn matches<'a, 'b>(&'a self, path: &'b str) -> Matches<'a, 'b, T> {
        Matches {
            matches: self
                .routes
                .iter()
                .filter_map(|r| r.is_match(path))
                .collect(),
        }
    }

    pub fn best_match<'a, 'b>(&'a self, path: &'b str) -> Option<Match<'a, 'b, T>> {
        self.routes.iter().filter_map(|r| r.is_match(path)).max()
    }
}

#[derive(Debug)]
pub struct Captures(Vec<(String, String)>, Option<String>);
impl Captures {
    pub fn wildcard(&self) -> Option<&str> {
        self.1.as_deref()
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.0
            .iter()
            .find_map(|(k, v)| if k == key { Some(&**v) } else { None })
    }
}

impl Deref for Captures {
    type Target = Vec<(String, String)>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug)]
pub struct Match<'router, 'path, T> {
    path: &'path str,
    route: &'router Route<T>,
    captures: Vec<&'path str>,
}

impl<'router, 'path, T> Match<'router, 'path, T> {
    pub fn handler(&self) -> &'router T {
        &self.route.handler
    }

    pub fn into_route(self) -> &'router Route<T> {
        self.route
    }

    pub fn captures(&self) -> Captures {
        self.route
            .definition
            .segments
            .iter()
            .filter(|s| matches!(s, Segment::Param(_) | Segment::Wildcard))
            .zip(&self.captures)
            .fold(
                Captures(vec![], None),
                |mut captures, (segment, capture)| match segment {
                    Segment::Param(name) => {
                        captures
                            .0
                            .push((String::from(*name), String::from(*capture)));
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
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<'router, 'path, T> Ord for Match<'router, 'path, T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.route
            .definition
            .segments
            .iter()
            .zip(&other.route.definition.segments)
            .map(|(mine, theirs)| mine.cmp(theirs))
            .find(|c| *c != std::cmp::Ordering::Equal)
            .unwrap_or(std::cmp::Ordering::Equal)
    }
}

#[derive(Debug)]
pub struct Matches<'router, 'path, T> {
    matches: BTreeSet<Match<'router, 'path, T>>,
}

impl<'router, 'path, T> Deref for Matches<'router, 'path, T> {
    type Target = BTreeSet<Match<'router, 'path, T>>;

    fn deref(&self) -> &Self::Target {
        &self.matches
    }
}

impl<'router, 'path, T> Matches<'router, 'path, T> {
    pub fn len(&self) -> usize {
        self.matches.len()
    }

    pub fn best(&self) -> Option<&Match<'router, 'path, T>> {
        self.matches.iter().last()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct RouteDefinition<'s> {
    source: &'s str,
    segments: Vec<Segment<'s>>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Segment<'s> {
    Slash,
    Dot,
    Exact(&'s str),
    Param(&'s str),
    Wildcard,
}
impl PartialOrd for Segment<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Segment<'_> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        use std::cmp::Ordering::*;
        use Segment::*;
        match (self, other) {
            (Exact(_), Exact(_))
            | (Slash, Slash)
            | (Dot, Slash)
            | (Slash, Dot)
            | (Dot, Dot)
            | (Param(_), Param(_))
            | (Wildcard, Wildcard) => Equal,

            (Exact(_), _) => Greater,
            (Param(_), Exact(_)) => Less,
            (Param(_), _) => Greater,
            (Wildcard, Exact(_)) | (Wildcard, Param(_)) => Less,
            (Wildcard, _) => Greater,
            _ => Less,
        }
    }
}

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

impl<T> Route<T> {
    pub fn handler(&self) -> &T {
        &self.handler
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
            Some(Match {
                path,
                route: &self,
                captures,
            })
        } else {
            None
        }
    }
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

#[cfg(test)]
mod tests {
    use std::error::Error;
    type Result = std::result::Result<(), Box<dyn Error>>;
    use super::*;
    #[test]
    fn it_works() -> Result {
        let mut router = Router::new();
        router.add("/*", 1)?;
        router.add("/hello", 2)?;
        router.add("/:greeting", 3)?;
        router.add("/hey/:world", 4)?;
        router.add("/hey/earth", 5)?;
        router.add("/hey/:world/*", 6)?;
        let matches = router.matches("/hello");
        assert_eq!(matches.len(), 3);
        assert_eq!(router.matches("/hi/world").len(), 1);
        assert_eq!(*router.matches("/hey/earth").best().unwrap().handler(), 5);
        assert_eq!(
            *router.matches("/hey/mars").best().unwrap().captures,
            vec!["mars"]
        );

        assert_eq!(
            *router
                .matches("/hey/earth/wildcard")
                .best()
                .unwrap()
                .handler(),
            6
        );

        assert_eq!(
            *router
                .matches("/hey/earth/wildcard")
                .best()
                .unwrap()
                .handler(),
            6
        );
        Ok(())
    }

    #[test]
    fn several_params() -> Result {
        let mut router = Router::new();
        router.add(":anything/specific/:anything", 4)?;
        router.add("/:a", 1)?;
        router.add("/:a/:b", 2)?;
        router.add("/:a/:b/:c", 3)?;

        assert_eq!(router.best_match("/hi").unwrap().handler(), &1);
        assert_eq!(router.best_match("/hi/there").unwrap().handler(), &2);
        assert_eq!(router.best_match("/hi/there/hey").unwrap().handler(), &3);
        assert_eq!(router.matches("/hi/specific/anything").len(), 2);
        assert_eq!(
            router
                .best_match("/hi/specific/anything")
                .unwrap()
                .handler(),
            &4
        );

        assert!(router.matches("/").is_empty());
        assert!(router.matches("/a/b/c/d").is_empty());

        Ok(())
    }

    #[test]
    fn wildcard_matches_root() -> Result {
        let mut router = Router::new();
        router.add("*", ())?;
        assert!(router.best_match("/").is_some());

        let mut router = Router::new();
        router.add("/something/:anything/*", ())?;
        assert!(router.best_match("/something/1/").is_some());

        let mut router = Router::new();
        router.add("/something/:anything/*", ())?;
        assert!(router.best_match("/something/1").is_some());

        Ok(())
    }

    #[test]
    fn trailing_slashes_are_ignored() -> Result {
        let mut router = Router::new();
        router.add("/a", ())?;
        assert!(router.best_match("/a/").is_some());
        assert!(router.best_match("/a").is_some());

        let mut router = Router::new();
        router.add("/a/", ())?;
        assert!(router.best_match("/a").is_some());
        assert!(router.best_match("/a/").is_some());

        Ok(())
    }

    #[test]
    fn captures() -> Result {
        let mut router = Router::new();
        router.add("/:a/:b/:c", ())?;
        let best_match = router.best_match("/aaa/bbb/ccc").unwrap();
        assert_eq!(best_match.captures, vec!["aaa", "bbb", "ccc"]);

        let captures = best_match.captures();
        assert_eq!(captures.get("a"), Some("aaa"));

        let captures = best_match.captures();
        assert_eq!(captures.get("b"), Some("bbb"));

        let mut router = Router::new();
        router.add("/*", ())?;
        let best_match = router.best_match("/hello/world").unwrap();
        assert_eq!(best_match.captures, vec!["hello/world"]);
        assert_eq!(best_match.captures().wildcard(), Some("hello/world"));

        Ok(())
    }
}
