type Result = std::result::Result<(), Box<dyn std::error::Error>>;
use routefinder::*;
use std::{iter::FromIterator, str::FromStr};
use test_harness::test;

fn harness(f: impl Fn() -> Result) -> Result {
    let _ = env_logger::builder().is_test(true).try_init();
    f()
}

#[test(harness)]
fn it_works() -> Result {
    let router = Router::new_with_routes([
        ("/*", 1),
        ("/hello", 2),
        ("/:greeting", 3),
        ("/hey/:world", 4),
        ("/:salutation/:world/*", 6),
        ("/hey/:param/:second/*", 11),
        ("/hey/earth", 5),
    ])?;

    let matches = router.matches("/hello");
    assert_eq!(matches.len(), 3);
    assert_eq!(router.matches("/").len(), 1);
    assert_eq!(*router.best_match("/hey/earth").unwrap(), 5);

    assert_eq!(
        router
            .best_match("/hey/mars")
            .unwrap()
            .captures()
            .get("world"),
        Some("mars")
    );

    let m = router.best_match("/hey/earth/some/wildcard/stuff").unwrap();

    assert_eq!(*m, 11);
    let captures = m.captures();
    assert_eq!(captures.wildcard(), Some("wildcard/stuff"));
    assert_eq!(captures.get("param"), Some("earth"));
    assert_eq!(captures.get("second"), Some("some"));

    Ok(())
}

#[test(harness)]
fn several_params() -> Result {
    let router = Router::new_with_routes([
        ("/:a", 1),
        ("/:a/:b", 2),
        ("/:a/:b/:c", 3),
        ("/:param1/specific/:param2", 4),
    ])?;
    assert_eq!(*router.best_match("/hi").unwrap(), 1);
    assert_eq!(*router.best_match("/hi/there").unwrap(), 2);
    assert_eq!(*router.best_match("/hi/there/hey").unwrap(), 3);

    assert_eq!(router.matches("/hi/specific/anything").len(), 2);

    assert_eq!(*router.best_match("/hi/specific/anything").unwrap(), 4);

    assert!(router.matches("/").is_empty());
    assert!(router.matches("/a/b/c/d").is_empty());

    Ok(())
}

#[test(harness)]
fn wildcard_matches_root() -> Result {
    let router = Router::new_with_routes([("*", ())])?;
    assert!(router.best_match("/").is_some());

    let router = Router::new_with_routes([("/something/:anything/*", ())])?;
    assert!(router.best_match("/something/1/").is_some());

    let router = Router::new_with_routes([("/something/:anything/*", ())])?;
    assert!(router.best_match("/something/1").is_some());

    Ok(())
}

#[test(harness)]
fn trailing_slashes_are_ignored() -> Result {
    let router = Router::new_with_routes([("/a", ())])?;
    assert!(router.best_match("/a/").is_some());
    assert!(router.best_match("/a").is_some());

    let router = Router::new_with_routes([("/a/", ())])?;
    assert!(router.best_match("/a").is_some());
    assert!(router.best_match("/a/").is_some());

    Ok(())
}

#[test(harness)]
fn captures() -> Result {
    let router = Router::new_with_routes([("/:a/:b/:c", ())])?;
    let best_match = router.best_match("/aaa/bbb/ccc").unwrap();
    let captures = best_match.captures();
    assert_eq!(captures.get("a"), Some("aaa"));
    assert_eq!(captures.get("b"), Some("bbb"));
    assert_eq!(captures.get("c"), Some("ccc"));
    assert_eq!(captures.get("not-present"), None);

    let router = Router::new_with_routes([("/*", ())])?;
    let best_match = router.best_match("/hello/world").unwrap();
    assert_eq!(best_match.captures().wildcard(), Some("hello/world"));

    Ok(())
}

#[test(harness)]
fn errors_on_add() -> Result {
    let mut router = Router::new();

    assert!(router
        .add("*named_star", ())
        .unwrap_err()
        .contains("replace `*named_star` with `*`"));

    assert_eq!(router.add(":", ()).unwrap_err(), "params must be named");
    Ok(())
}

#[test(harness)]
fn dots() -> Result {
    let router = Router::new_with_routes([
        ("/:a.:b", 1),
        ("/:a/:b.:c", 2),
        ("/:a/:b", 3),
        ("/:a/:b.txt", 4),
    ])?;

    assert_eq!(*router.best_match("/hello.world").unwrap(), 1);
    assert_eq!(*router.best_match("/hi/there.world").unwrap(), 2);
    assert_eq!(*router.best_match("/hi/yep").unwrap(), 3);
    assert_eq!(*router.best_match("/hi/planet.txt").unwrap(), 4);

    assert_eq!(
        router
            .match_iter("/hi/planet.txt")
            .map(|x| *x)
            .collect::<Vec<_>>(),
        vec![4, 2, 3]
    );

    assert!(router.matches("/").is_empty());
    assert!(router.matches("/a/b/c/d").is_empty());

    Ok(())
}

#[test(harness)]
fn parse() -> Result {
    assert_eq!(
        RouteSpec::from_str("a.:b")?.matches("a.hello"),
        Some(vec!["hello"])
    );

    assert_eq!(
        RouteSpec::from_str("/a/b/c")?.matches("/a/b/c"),
        Some(vec![])
    );
    assert_eq!(
        RouteSpec::from_str("/a.b.c")?.matches("/a.b.c"),
        Some(vec![])
    );

    assert_eq!(
        RouteSpec::from_str(":a.:b")?.matches("a.hello"),
        Some(vec!["a", "hello"])
    );
    assert_eq!(
        RouteSpec::from_str("a.:b")?.matches("a.hello"),
        Some(vec!["hello"])
    );
    assert_eq!(RouteSpec::from_str(":a.b")?.matches("a.b"), Some(vec!["a"]));

    Ok(())
}

#[test(harness)]
fn multiple_slashes() -> Result {
    assert!(RouteSpec::from_str("a/b/c")?
        .matches("/a////b///c//")
        .is_some());
    Ok(())
}

#[test(harness)]
fn templating() -> Result {
    assert_eq!(
        RouteSpec::from_str(":a/:b.:c")?
            .template(&[("a", "users"), ("b", "jbr"), ("c", "txt")].into())
            .unwrap()
            .to_string(),
        "/users/jbr.txt"
    );

    Ok(())
}

#[test(harness)]
fn specific_matches() -> Result {
    assert_eq!(
        RouteSpec::from_str(":param")?.matches("/a.b.c.d").unwrap(),
        vec!["a.b.c.d"]
    );

    assert_eq!(
        RouteSpec::from_str(":a.:b")?.matches("/a.b.c.d").unwrap(),
        vec!["a", "b.c.d"]
    );

    assert_eq!(
        RouteSpec::from_str(":a.:b.:c")?
            .matches("/a.b.c.d")
            .unwrap(),
        vec!["a", "b", "c.d"]
    );

    assert_eq!(
        RouteSpec::from_str(":a.:b.:c.:d")?
            .matches("/a.b.c.d")
            .unwrap(),
        vec!["a", "b", "c", "d"]
    );

    assert!(RouteSpec::from_str(":a.:b")?.matches("/a").is_none());

    Ok(())
}

#[test(harness)]
fn priority() -> Result {
    assert!(RouteSpec::from_str("exact")? < RouteSpec::from_str(":param")?);
    assert!(RouteSpec::from_str("a")? < RouteSpec::from_str("a/b")?);
    assert!(RouteSpec::from_str(":a.:b")? < RouteSpec::from_str(":a")?);
    Ok(())
}

#[test(harness)]
fn extend_captures() -> Result {
    let mut captures = Captures::from_iter([("key", "value")]);
    let other_captures = Captures::from_iter([("key2", "value2")]);

    captures.extend(other_captures);

    assert_eq!(
        captures.iter().collect::<Vec<_>>(),
        [("key", "value"), ("key2", "value2")]
    );
    Ok(())
}

#[test(harness)]
fn append_captures() -> Result {
    let mut captures = Captures::from_iter([("key", "value")]);
    captures.set_wildcard("something");

    let mut other_captures = Captures::from_iter([("key2", "value2")]);
    other_captures.set_wildcard("other");

    captures.append(other_captures);

    assert_eq!(
        captures.iter().collect::<Vec<_>>(),
        [("key", "value"), ("key2", "value2")]
    );

    assert_eq!(Some("other"), captures.wildcard());
    Ok(())
}

#[test(harness)]
fn wildcard_vs_param() -> Result {
    let router = Router::new_with_routes([("/foo/*", "wildcard"), ("/foo/:bar", "param")])?;
    assert_eq!(*router.best_match("/foo/123").unwrap(), "param");
    Ok(())
}

#[test(harness)]
fn trailing_slashes() -> Result {
    let router = Router::new_with_routes([("/foo/bar", "no-trailing-slash")])?;
    assert_eq!(
        *router.best_match("/foo/bar/").unwrap(),
        "no-trailing-slash"
    );
    Ok(())
}

#[test(harness)]
fn prefix_ambiguity() -> Result {
    let router = Router::new_with_routes([("/foo/bar", "exact"), ("/foo/:bar", "param")])?;
    assert_eq!(*router.best_match("/foo/bar").unwrap(), "exact");
    assert_eq!(*router.best_match("/foo/qux").unwrap(), "param");
    Ok(())
}

#[test(harness)]
fn slashes() -> Result {
    let router = Router::new_with_routes([("/foo/bar", "one-slash")])?;
    assert_eq!(*router.best_match("//foo///bar").unwrap(), "one-slash");
    assert_eq!(*router.best_match("/foo//bar").unwrap(), "one-slash");
    Ok(())
}

#[test(harness)]
fn wildcard() -> Result {
    let router = Router::new_with_routes([("/foo/*", "wildcard")])?;
    assert_eq!(*router.best_match("/foo/bar/baz/qux").unwrap(), "wildcard");
    Ok(())
}

#[test(harness)]
fn root() -> Result {
    let router = Router::new_with_routes([("/", "root")])?;
    assert_eq!(*router.best_match("").unwrap(), "root");
    Ok(())
}

#[test(harness)]
fn simple_exact_match() -> Result {
    let router = Router::new_with_routes([("foo", "foo")])?;
    assert_eq!(*router.best_match("foo").unwrap(), "foo");
    Ok(())
}

#[test(harness)]
fn param_then_dot_or_slash() -> Result {
    let router = Router::new_with_routes([("/foo/:bar.baz", "dot")])?;
    assert_eq!(*router.best_match("foo/qux.baz").unwrap(), "dot");
    Ok(())
}

#[test(harness)]
fn both_param_and_wildcard_at_root() -> Result {
    let router = Router::new_with_routes([("*", 0), ("/:param", 1)])?;
    assert_eq!(*router.best_match("/").unwrap(), 0);
    Ok(())
}

#[test(harness)]
fn exact_and_param_and_wildcard_precedence() -> Result {
    let router = Router::new_with_routes([
        ("/", "root"),
        ("*", "wildcard"),
        ("/:param", "param"),
        ("/prefix/*", "prefixed-wildcard"),
        ("/prefix", "prefix"),
        ("/prefix/:param", "prefixed-param"),
    ])?;
    assert_eq!(*router.best_match("/").unwrap(), "root");
    assert_eq!(*router.best_match("/foo").unwrap(), "param");
    assert_eq!(*router.best_match("/foo/bar").unwrap(), "wildcard");

    assert_eq!(*router.best_match("/prefix").unwrap(), "prefix");
    assert_eq!(*router.best_match("/prefix/foo").unwrap(), "prefixed-param");
    assert_eq!(
        *router.best_match("/prefix/foo/bar").unwrap(),
        "prefixed-wildcard"
    );

    Ok(())
}

#[test(harness)]
fn more_regression_testing() -> Result {
    let routes = [
        (
            vec![":abc.:def"],
            "abc.def",
            ":abc.:def",
            vec![("abc", "abc"), ("def", "def")],
        ),
        (
            vec![":abc.:def", ":abc"],
            "abc.def",
            ":abc.:def",
            vec![("abc", "abc"), ("def", "def")],
        ),
        (vec!["abc.def", "abc/*"], "abc.def", "abc.def", vec![]),
        (
            vec![":param"],
            "abc.def",
            ":param",
            vec![("param", "abc.def")],
        ),
        (
            vec![":a.:b.:c"],
            "abc.def.ghi",
            ":a.:b.:c",
            vec![("a", "abc"), ("b", "def"), ("c", "ghi")],
        ),
        (vec!["a.:b", ":a.b", ":a"], "z.b", ":a.b", vec![("a", "z")]),
        (vec!["a.:b", ":a.b", ":a"], "a.z", "a.:b", vec![("b", "z")]),
        (vec!["a.:b", ":a.b", "a.b"], "a.b", "a.b", vec![]),
        (vec!["a."], "a.", "a.", vec![]),
        (vec![".a"], ".a", ".a", vec![]),
        (vec![".:a"], ".a", ".:a", vec![("a", "a")]),
        (vec![":a."], "a.", ":a.", vec![("a", "a")]),
        (vec!["a.b.:c/x"], "a.b.c/x", "a.b.:c/x", vec![("c", "c")]),
        (
            vec![":a.b.:c/x"],
            "a.b.c/x",
            ":a.b.:c/x",
            vec![("a", "a"), ("c", "c")],
        ),
        (
            vec![":a.b.:c.d/x"],
            "a.b.c.d/x",
            ":a.b.:c.d/x",
            vec![("a", "a"), ("c", "c")],
        ),
        (
            vec![":a.b.:c.:d/x"],
            "a.b.c.d.e/x",
            ":a.b.:c.:d/x",
            vec![("a", "a"), ("c", "c"), ("d", "d.e")],
        ),
        (
            vec![":just_param", ":param.exact."],
            "anything.exact.",
            ":param.exact.",
            vec![("param", "anything")],
        ),
        (
            vec!["/0", "/:s", "/:e.88!S8888.", "/"],
            "/11.88!S8888.",
            "/:e.88!S8888.",
            vec![("e", "11")],
        ),
        (
            vec!["/exact", "/:s", "/:param.prefix"],
            "/param.prefix",
            "/:param.prefix",
            vec![("param", "param")],
        ),
    ];
    for (i, (route, path, expected, captures)) in routes.iter().enumerate() {
        let router = Router::new_with_routes(route.iter().map(|r| (*r, *r)))?;
        // for (route, _) in &router {
        //     let mut keys = router.iter().map(|(x, _)| x).collect::<Vec<_>>();
        //     keys.sort();
        //     dbg!(&keys);
        //     // assert_eq!(key.cmp(route), Ordering::Equal);
        //     // assert_eq!(route.cmp(key), Ordering::Equal);

        //     assert!(
        //         router.get_handler(route.clone()).is_some(),
        //         "could not find {}",
        //         route
        //     );
        // }

        let new = router.best_match(path);

        let old = router.match_iter(path).next();
        assert_eq!(new, old);
        let new = new.unwrap();
        let old = old.unwrap();
        assert_eq!(new.handler(), old.handler());
        assert_eq!(new.handler(), expected, "{i} {path}");
        assert_eq!(old.handler(), expected, "{i} {path}");
        for (param, expected_capture) in captures {
            assert_eq!(
                new.captures().get(param),
                Some(*expected_capture),
                "{i} NEW {path} param {param}"
            );
            assert_eq!(
                old.captures().get(param),
                Some(*expected_capture),
                "{i} OLD {path} param {param}"
            );
        }

        assert_eq!(new.captures().len(), captures.len(), "{i} {path}");
        assert_eq!(old.captures().len(), captures.len(), "{i} {path}");
    }
    Ok(())
}
