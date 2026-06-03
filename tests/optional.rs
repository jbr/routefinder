type Result = std::result::Result<(), Box<dyn std::error::Error>>;
use routefinder::*;
use std::str::FromStr;
use test_harness::test;

fn harness(f: impl Fn() -> Result) -> Result {
    let _ = env_logger::builder().is_test(true).try_init();
    f()
}

#[test(harness)]
fn motivating_example() -> Result {
    // /some matches, /some/thing matches with opt, /some/thing.html matches with opt+ext.
    let mut router = Router::new();
    router.add("/some[/:opt[.:ext]]", 1)?;

    assert_eq!(*router.best_match("/some").unwrap(), 1);
    assert_eq!(*router.best_match("/some/thing").unwrap(), 1);
    assert_eq!(*router.best_match("/some/thing.html").unwrap(), 1);

    // none of the absent params surface as captures
    let m = router.best_match("/some").unwrap();
    assert_eq!(m.captures().get("opt"), None);
    assert_eq!(m.captures().get("ext"), None);

    let m = router.best_match("/some/thing").unwrap();
    assert_eq!(m.captures().get("opt"), Some("thing"));
    assert_eq!(m.captures().get("ext"), None);

    let m = router.best_match("/some/thing.html").unwrap();
    assert_eq!(m.captures().get("opt"), Some("thing"));
    assert_eq!(m.captures().get("ext"), Some("html"));

    // a path that doesn't fit any variant doesn't match
    assert!(router.best_match("/some/a/b").is_none());

    Ok(())
}

#[test(harness)]
fn match_route_is_canonical() -> Result {
    // Match::route() reflects the as-registered optional spec, not an expanded
    // variant the user never typed.
    let mut router = Router::new();
    router.add("/some[/:opt[.:ext]]", 1)?;

    for path in ["/some", "/some/thing", "/some/thing.html"] {
        let m = router.best_match(path).unwrap();
        assert_eq!(m.route().to_string(), "/some[/:opt[.:ext]]");
        assert_eq!(m.route().source(), Some("/some[/:opt[.:ext]]"));
    }

    // a single optional route counts as one route, and each path matches once.
    assert_eq!(router.len(), 1);
    assert_eq!(router.matches("/some/thing").len(), 1);

    Ok(())
}

#[test(harness)]
fn optional_after_mandatory_params() -> Result {
    let mut router = Router::new();
    router.add("/a/:x[/:opt]", 1)?;

    let m = router.best_match("/a/foo").unwrap();
    assert_eq!(m.captures().get("x"), Some("foo"));
    assert_eq!(m.captures().get("opt"), None);

    let m = router.best_match("/a/foo/bar").unwrap();
    assert_eq!(m.captures().get("x"), Some("foo"));
    assert_eq!(m.captures().get("opt"), Some("bar"));

    assert!(router.best_match("/a").is_none());

    Ok(())
}

#[test(harness)]
fn specificity_against_other_routes() -> Result {
    // An expanded variant competes with other routes via the normal specificity
    // rules: a concrete route out-ranks the optional's param variant.
    let mut router = Router::new();
    router.add("/some[/:opt]", 1)?;
    router.add("/some/specific", 2)?;

    assert_eq!(*router.best_match("/some").unwrap(), 1);
    assert_eq!(*router.best_match("/some/specific").unwrap(), 2);
    assert_eq!(*router.best_match("/some/other").unwrap(), 1);

    Ok(())
}

#[test(harness)]
fn optional_wildcard() -> Result {
    let mut router = Router::new();
    router.add("/files[/*]", 1)?;

    assert_eq!(*router.best_match("/files").unwrap(), 1);
    let m = router.best_match("/files/a/b/c").unwrap();
    assert_eq!(*m, 1);
    assert_eq!(m.captures().wildcard(), Some("a/b/c"));

    Ok(())
}

#[test(harness)]
fn get_handler_distinguishes_optional() -> Result {
    let mut router = Router::new();
    router.add("/a[/:b]", 1)?;
    router.add("/a/:b", 2)?;

    // the optional spec and the plain spec are distinct registrations
    assert_eq!(router.get_handler("/a[/:b]"), Some(&1));
    assert_eq!(router.get_handler("/a/:b"), Some(&2));

    Ok(())
}

#[test(harness)]
fn direct_route_spec_matches() -> Result {
    // The cold-path RouteSpec::matches handles optionals by trying variants.
    let spec = RouteSpec::from_str("/some[/:opt[.:ext]]")?;
    assert!(spec.matches("/some").is_some());
    assert!(spec.matches("/some/thing").is_some());
    assert!(spec.matches("/some/thing.html").is_some());
    assert!(spec.matches("/some/a/b").is_none());

    assert_eq!(spec.matches("/some/thing.html").unwrap(), ["thing", "html"]);

    Ok(())
}

#[test(harness)]
fn reverse_routing() -> Result {
    let spec = RouteSpec::from_str("/some[/:opt[.:ext]]")?;

    let captures = Captures::from([("opt", "thing"), ("ext", "html")]);
    assert_eq!(spec.template(&captures).unwrap().to_string(), "/some/thing.html");

    let captures = Captures::from([("opt", "thing")]);
    assert_eq!(spec.template(&captures).unwrap().to_string(), "/some/thing");

    let captures = Captures::new();
    assert_eq!(spec.template(&captures).unwrap().to_string(), "/some");

    // ext without opt is not a valid variant
    let captures = Captures::from([("ext", "html")]);
    assert!(spec.template(&captures).is_none());

    Ok(())
}

#[test(harness)]
fn display_round_trips() -> Result {
    for source in [
        "/some[/:opt[.:ext]]",
        "/a/:x[/:opt]",
        "/files[/*]",
        "/some[/:opt]",
    ] {
        let spec = RouteSpec::from_str(source)?;
        assert_eq!(spec.to_string(), source);
    }
    Ok(())
}

#[test(harness)]
fn parse_errors() -> Result {
    assert!(RouteSpec::from_str("/some[]").is_err()); // empty group
    assert!(RouteSpec::from_str("/a[/:b]/c").is_err()); // content after ]
    assert!(RouteSpec::from_str("/a[/:b").is_err()); // unclosed
    assert!(RouteSpec::from_str("/a[/:b][/:c]").is_err()); // sibling groups
    assert!(RouteSpec::from_str("/a]b").is_err()); // stray ]
    Ok(())
}
