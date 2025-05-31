use super::{Trie, TrieMatch};

impl<const N: usize> PartialEq<(&str, [&str; N])> for TrieMatch<'_, '_> {
    fn eq(&self, other: &(&str, [&str; N])) -> bool {
        let (expected_route, expected_captures) = other;
        let TrieMatch {
            route, captures, ..
        } = &self;
        expected_route == &route.to_string() && expected_captures == &**captures
    }
}

impl<const N: usize> PartialEq<(&str, [&str; N], &str)> for TrieMatch<'_, '_> {
    fn eq(&self, other: &(&str, [&str; N], &str)) -> bool {
        let (expected_route, expected_captures, expected_wildcard) = other;
        let TrieMatch {
            route,
            captures,
            wildcard,
        } = &self;
        expected_route == &route.to_string()
            && expected_captures == &**captures
            && Some(expected_wildcard) == wildcard.as_ref()
    }
}

#[test]
fn building() {
    let _ = env_logger::builder().is_test(true).try_init();

    let mut trie = Trie::default();
    trie.insert("/a/b/c".parse().unwrap());
    trie.insert("/a/:b/c".parse().unwrap());
    trie.insert("/a/:b".parse().unwrap());
    trie.insert("/a/*".parse().unwrap());
    trie.insert("/a/:b.:c".parse().unwrap());
    assert_eq!(trie.best_match("a/b/c").unwrap(), ("/a/b/c", []));
    assert_eq!(trie.best_match("a/d/c").unwrap(), ("/a/:b/c", ["d"]));
    assert_eq!(trie.best_match("a/d").unwrap(), ("/a/:b", ["d"]));
    assert_eq!(trie.best_match("a/d.1").unwrap(), ("/a/:b.:c", ["d", "1"]));
    assert_eq!(trie.best_match("a/b/c/d").unwrap(), ("/a/*", [], "b/c/d"));
    assert_eq!(trie.best_match("a/b/d").unwrap(), ("/a/*", [], "b/d"));
}
