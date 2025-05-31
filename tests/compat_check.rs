//! Regression guard: this crate must match the matching behavior of the last
//! published release (`published` dev-dependency) for every route/path pair.
//!
//! This caught a real specificity regression in an intermediate trie
//! implementation, so it is kept as a permanent test. It compares which route
//! wins and the captured wildcard tail; if this crate ever diverges from the
//! released semantics, this fails.

/// Subsegment routes — the patterns where specificity ordering is subtle
/// (prefix vs suffix vs exact vs degenerate within a single dotted segment).
const SUBSEGMENT_ROUTES: [(&str, &str); 13] = [
    ("first/prefix.suffix", "full exact"),
    ("first/prefix.:suffix", "exact prefix"),
    ("first/:prefix.suffix", "exact suffix"),
    ("first/:prefix.:suffix", "exact degenerate"),
    ("first/:param", "exact param"),
    ("first/*", "exact star"),
    (":first/prefix.suffix", "param exact"),
    (":first/prefix.:suffix", "param prefix"),
    (":first/:prefix.suffix", "param suffix"),
    (":first/:prefix.:suffix", "param degenerate"),
    (":first/:param", "full param"),
    (":first/*", "param star"),
    ("*", "star"),
];

const SUBSEGMENT_PATHS: [&str; 14] = [
    "/first/prefix.suffix",
    "/first/prefix.different",
    "/first/different.suffix",
    "/first/diff1.diff2",
    "/first/different",
    "/first/different/path",
    "/first",
    "/different/prefix.suffix",
    "/different/prefix.different",
    "/different/different.suffix",
    "/different/different.different",
    "/different/different",
    "/different/different/different",
    "/different",
];

/// General static/param/wildcard routes with overlapping specificity.
const GENERAL_ROUTES: [(&str, &str); 9] = [
    ("/*", "root star"),
    ("/hello", "hello"),
    ("/:greeting", "greeting"),
    ("/hey/:world", "hey world"),
    ("/:salutation/:world/*", "two params star"),
    ("/hey/:param/:second/*", "hey deep star"),
    ("/hey/earth", "hey earth"),
    ("/a/b/c/d/e/f", "deep static"),
    ("/a/:b/*", "greedy middle"),
];

const GENERAL_PATHS: [&str; 11] = [
    "/",
    "/hello",
    "/world",
    "/hey/earth",
    "/hey/mars",
    "/hey/earth/some/wildcard/stuff",
    "/a/b/c/d/e/f",
    "/a/x/y/z",
    "/one/two/three",
    "/nothing/here/at/all",
    "/hey",
];

macro_rules! best {
    ($krate:ident, $routes:expr, $path:expr) => {{
        let mut router = $krate::Router::new();
        for (route, handler) in $routes {
            router.add(route, handler).unwrap();
        }
        router.best_match($path).map(|m| {
            let captures: Vec<(String, String)> = m
                .captures()
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();
            let wildcard = m.captures().wildcard().map(str::to_string);
            (*m.handler(), captures, wildcard)
        })
    }};
}

fn check(name: &str, routes: &[(&str, &str)], paths: &[&str]) {
    let routes = routes.to_vec();
    let mut mismatches = vec![];
    for &path in paths {
        let current = best!(routefinder, routes.clone(), path);
        let released = best!(published, routes.clone(), path);
        if current != released {
            mismatches.push(format!(
                "  {path}\n    released: {released:?}\n    current:  {current:?}"
            ));
        }
    }
    assert!(
        mismatches.is_empty(),
        "{name}: {} path(s) diverge from the published release:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

#[test]
fn subsegment_matches_release() {
    check("subsegment", &SUBSEGMENT_ROUTES, &SUBSEGMENT_PATHS);
}

#[test]
fn general_matches_release() {
    check("general", &GENERAL_ROUTES, &GENERAL_PATHS);
}
