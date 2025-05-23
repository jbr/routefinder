use crate::{RouteSpec, Segment};
use smartstring::alias::String;
use std::collections::BTreeMap;

#[derive(Default, Debug)]
pub(crate) struct Trie(TrieNode);

impl Trie {
    pub(crate) fn insert(&mut self, route_spec: RouteSpec) {
        self.0.insert(route_spec, 0);
    }

    pub(crate) fn matches<'trie, 'a>(&'trie self, path: &'a str) -> Option<TrieMatch<'trie, 'a>> {
        let mut captures = vec![];
        let mut wildcard = None;
        let path = path.trim_start_matches('/').trim_end_matches('/');
        #[cfg(feature = "log")]
        log::trace!("{path}");

        self.0
            .matches(path, &mut captures, &mut wildcard)
            .map(|route| {
                captures.reverse();
                TrieMatch(route, captures, wildcard)
            })
    }
}

#[derive(Debug)]
pub(crate) struct TrieMatch<'trie, 'a>(
    pub(crate) &'trie RouteSpec,
    pub(crate) Vec<&'a str>,
    pub(crate) Option<&'a str>,
);

#[derive(Default)]
struct TrieNode {
    slash: Option<Box<TrieNode>>,
    dot: Option<Box<TrieNode>>,
    statics: BTreeMap<String, TrieNode>,
    params: Option<Box<TrieNode>>,
    wildcard: bool,
    route: Option<RouteSpec>,
}

impl std::fmt::Debug for TrieNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut map = f.debug_map();
        if let Some(route) = &self.route {
            map.entry(
                if self.wildcard { &"*" } else { &"" },
                &format_args!("{route}"),
            );
        }
        if let Some(slash) = &self.slash {
            map.entry(&"/", slash);
        }

        if let Some(dot) = &self.dot {
            map.entry(&".", dot);
        }

        for (key, value) in &self.statics {
            map.entry(key, value);
        }

        if let Some(params) = &self.params {
            map.entry(&"[[:param]]", params);
        }

        map.finish()
    }
}

impl TrieNode {
    fn matches<'trie, 'a>(
        &'trie self,
        path: &'a str,
        captures: &mut Vec<&'a str>,
        wildcard: &mut Option<&'a str>,
    ) -> Option<&'trie RouteSpec> {
        #[cfg(feature = "log")]
        log::trace!("{path:?}, {:?}", self);

        if let Some(route) = &self.route {
            if !self.wildcard && path.is_empty() {
                #[cfg(feature = "log")]
                log::trace!("{path:?}, {route}");
                return Some(route);
            }
        }

        match path.as_bytes().first() {
            Some(b'/') => {
                let slashes = path.chars().take_while(|x| *x == '/').count();
                return self
                    .slash
                    .as_ref()?
                    .matches(&path[slashes..], captures, wildcard);
            }
            Some(b'.') => {
                return self.dot.as_ref()?.matches(&path[1..], captures, wildcard);
            }
            _ => (),
        }

        #[cfg(feature = "memchr")]
        let index = memchr::memchr2(b'.', b'/', path.as_bytes());
        #[cfg(not(feature = "memchr"))]
        let index = path.find(['.', '/']);

        let (component, rest) = if let Some(index) = index {
            path.split_at(index)
        } else {
            (path, "")
        };

        if let Some(route) = self
            .statics
            .get(component)
            .and_then(|f| f.matches(rest, captures, wildcard))
        {
            return Some(route);
        }

        if !component.is_empty() {
            if let Some(param) = &self.params {
                if let Some(route) = param.matches(rest, captures, wildcard) {
                    captures.push(component);
                    return Some(route);
                }
            }
        }

        if let Some(route) = &self.route {
            if self.wildcard {
                *wildcard = Some(path);
                return Some(route);
            }
        }

        if path.is_empty() {
            return self.slash.as_ref()?.matches(path, captures, wildcard);
        }

        None
    }

    fn insert(&mut self, route: RouteSpec, depth: usize) {
        let Some(segment) = route.segments().get(depth) else {
            #[cfg(feature = "log")]
            if let Some(previous) = &self.route {
                log::warn!("replacing {previous} with {route}");
            }
            self.route = Some(route);
            return;
        };

        match segment {
            Segment::Slash => self.slash.get_or_insert_default().insert(route, depth + 1),
            Segment::Dot => self.dot.get_or_insert_default().insert(route, depth + 1),
            Segment::Exact(string) => self
                .statics
                .entry(string.clone())
                .or_default()
                .insert(route, depth + 1),
            Segment::Param(_) => self.params.get_or_insert_default().insert(route, depth + 1),
            Segment::Wildcard => {
                self.wildcard = true;
                #[cfg(feature = "log")]
                if let Some(previous) = &self.route {
                    log::warn!("replacing {previous} with {route} for wildcard");
                }

                self.route = Some(route);
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::{Trie, TrieMatch};

    impl<const N: usize> PartialEq<(&str, [&str; N])> for TrieMatch<'_, '_> {
        fn eq(&self, other: &(&str, [&str; N])) -> bool {
            let (expected_route, expected_matches) = other;
            let TrieMatch(route, matches, _wildcard) = &self;
            expected_route == &route.to_string() && expected_matches == &**matches
        }
    }

    impl<const N: usize> PartialEq<(&str, [&str; N], &str)> for TrieMatch<'_, '_> {
        fn eq(&self, other: &(&str, [&str; N], &str)) -> bool {
            let (expected_route, expected_matches, expected_wildcard) = other;
            let TrieMatch(route, matches, wildcard) = &self;
            expected_route == &route.to_string()
                && expected_matches == &**matches
                && Some(expected_wildcard) == wildcard.as_ref()
        }
    }

    #[test]
    fn building() {
        let mut trie = Trie::default();
        trie.insert("/a/b/c".parse().unwrap());
        trie.insert("/a/:b/c".parse().unwrap());
        trie.insert("/a/:b".parse().unwrap());
        trie.insert("/a/*".parse().unwrap());
        trie.insert("/a/:b.:c".parse().unwrap());

        assert_eq!(trie.matches("a/b/c").unwrap(), ("/a/b/c", []));
        assert_eq!(trie.matches("a/d/c").unwrap(), ("/a/:b/c", ["d"]));
        assert_eq!(trie.matches("a/d").unwrap(), ("/a/:b", ["d"]));
        assert_eq!(trie.matches("a/d.1").unwrap(), ("/a/:b.:c", ["d", "1"]));
        assert_eq!(trie.matches("a/b/c/d").unwrap(), ("/a/*", [], "b/c/d"));
        assert_eq!(trie.matches("a/b/d").unwrap(), ("/a/*", [], "b/d"));
    }
}
