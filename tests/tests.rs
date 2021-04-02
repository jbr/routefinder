type Result = std::result::Result<(), Box<dyn std::error::Error>>;
use routefinder::*;

#[test]
fn it_works() -> Result {
    let mut router = Router::new();
    router.add("/*", 1)?;
    router.add("/hello", 2)?;
    router.add("/:greeting", 3)?;
    router.add("/hey/:world", 4)?;
    router.add("/hey/earth", 5)?;
    router.add("/:greeting/:world/*", 6)?;

    assert_eq!(
        &format!("{:#?}", &router),
        r#"{
    Route(/*),
    Route(/:greeting/:world/*),
    Route(/:greeting),
    Route(/hey/:world),
    Route(/hey/earth),
    Route(/hello),
}"#
    );

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

    let m = router.best_match("/hey/earth/wildcard/stuff").unwrap();

    assert_eq!(*m, 6);
    let captures = m.captures();
    assert_eq!(captures.wildcard(), Some("wildcard/stuff"));
    assert_eq!(captures.get("greeting"), Some("hey"));
    assert_eq!(captures.get("world"), Some("earth"));

    Ok(())
}

#[test]
fn several_params() -> Result {
    let mut router = Router::new();
    router.add("/:a", 1)?;
    router.add("/:a/:b", 2)?;
    router.add("/:a/:b/:c", 3)?;
    router.add("/:param1/specific/:param2", 4)?;
    assert_eq!(*router.best_match("/hi").unwrap(), 1);
    assert_eq!(*router.best_match("/hi/there").unwrap(), 2);
    assert_eq!(*router.best_match("/hi/there/hey").unwrap(), 3);

    assert_eq!(router.matches("/hi/specific/anything").len(), 2);

    assert_eq!(*router.best_match("/hi/specific/anything").unwrap(), 4);

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
    let captures = best_match.captures();
    assert_eq!(captures.get("a"), Some("aaa"));
    assert_eq!(captures.get("b"), Some("bbb"));
    assert_eq!(captures.get("c"), Some("ccc"));
    assert_eq!(captures.get("not-present"), None);

    let mut router = Router::new();
    router.add("/*", ())?;
    let best_match = router.best_match("/hello/world").unwrap();
    assert_eq!(best_match.captures().wildcard(), Some("hello/world"));

    Ok(())
}

#[test]
fn errors_on_add() {
    let mut router = Router::new();

    assert!(router
        .add("*named_star", ())
        .unwrap_err()
        .contains("replace `*named_star` with `*`"));

    assert_eq!(router.add(":", ()).unwrap_err(), "params must be named");
}

#[test]
fn reverse_lookup() -> Result {
    let mut router = Router::new();
    router.add("/*", 1)?;
    router.add("/hello", 2)?;
    router.add("/:greeting", 3)?;
    router.add("/hey/:world", 4)?;
    router.add("/hey/earth", 5)?;
    router.add("/:greeting/:world/*", 6)?;
    router.add("/deeply/nested/:world/*", 7)?;

    // matching with a simple capture

    let mut captures = Captures::new();
    captures.push(Capture::new("world", "mars"));

    let reversed_match = router.best_reverse_match(&captures).unwrap();
    assert_eq!(reversed_match.to_string(), "/hey/mars");
    assert_eq!(*reversed_match, 4);

    let all_reverse_matches = router.reverse_matches(&captures);
    assert_eq!(2, all_reverse_matches.len());
    assert_eq!(
        all_reverse_matches.iter().map(|r| **r).collect::<Vec<_>>(),
        vec![4, 7]
    );

    assert_eq!(
        all_reverse_matches
            .iter()
            .map(|r| r.to_string())
            .collect::<Vec<_>>(),
        vec!["/hey/mars", "/deeply/nested/mars/"]
    );

    // matching with a wildcard

    let mut captures = Captures::new();
    captures.set_wildcard("hello/world");

    let reversed_match = router.best_reverse_match(&captures).unwrap();
    assert_eq!(reversed_match.to_string(), "/hello/world");
    assert_eq!(*reversed_match, 1);

    // matching with multiple params and a wildcard

    let mut captures = Captures::new();
    captures.extend(vec![("greeting", "howdy"), ("world", "mars")]);
    captures.set_wildcard("this/is/wildcard/stuff");

    let reversed_match = router.best_reverse_match(&captures).unwrap();
    assert_eq!(
        reversed_match.to_string(),
        "/howdy/mars/this/is/wildcard/stuff"
    );
    assert_eq!(*reversed_match, 6);

    Ok(())
}
