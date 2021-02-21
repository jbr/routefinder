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

    let matches = router.matches("/hello");
    assert_eq!(matches.len(), 3);
    assert_eq!(router.matches("/").len(), 1);
    assert_eq!(*router.best_match("/hey/earth").unwrap().handler(), 5);

    assert_eq!(
        router
            .best_match("/hey/mars")
            .unwrap()
            .captures()
            .get("world"),
        Some("mars")
    );

    let m = router.best_match("/hey/earth/wildcard/stuff").unwrap();

    assert_eq!(*m.handler(), 6);
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
