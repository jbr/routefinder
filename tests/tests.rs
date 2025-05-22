type Result = std::result::Result<(), Box<dyn std::error::Error>>;
use std::{iter::FromIterator, str::FromStr};

use routefinder::*;

#[test]
fn it_works() -> Result {
    let router = Router::new_with_routes([
        ("/*", 1),
        ("/hello", 2),
        ("/:greeting", 3),
        ("/hey/:world", 4),
        ("/hey/earth", 5),
        ("/:greeting/:world/*", 6),
    ])?;

    assert_eq!(
        &format!("{:#?}", &router),
        r#"{
    /hello,
    /hey/earth,
    /hey/:world,
    /:greeting,
    /:greeting/:world/*,
    /*,
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

#[test]
fn wildcard_matches_root() -> Result {
    let router = Router::new_with_routes([("*", ())])?;
    assert!(router.best_match("/").is_some());

    let router = Router::new_with_routes([("/something/:anything/*", ())])?;
    assert!(router.best_match("/something/1/").is_some());

    let router = Router::new_with_routes([("/something/:anything/*", ())])?;
    assert!(router.best_match("/something/1").is_some());

    Ok(())
}

#[test]
fn trailing_slashes_are_ignored() -> Result {
    let router = Router::new_with_routes([("/a", ())])?;
    assert!(router.best_match("/a/").is_some());
    assert!(router.best_match("/a").is_some());

    let router = Router::new_with_routes([("/a/", ())])?;
    assert!(router.best_match("/a").is_some());
    assert!(router.best_match("/a/").is_some());

    Ok(())
}

#[test]
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
            .matches("/hi/planet.txt")
            .into_iter()
            .map(|x| *x)
            .collect::<Vec<_>>(),
        vec![4, 2, 3]
    );

    assert!(router.matches("/").is_empty());
    assert!(router.matches("/a/b/c/d").is_empty());

    Ok(())
}

#[test]
fn parse() -> Result {
    assert_eq!(
        RouteSpec::from_str("a.:b")?.matches("a.hello"),
        Some(vec!["hello"])
    );

    assert_eq!(
        RouteSpec::from_str(":a.:b")?.matches("a.hello"),
        Some(vec!["a", "hello"])
    );
    Ok(())
}

#[test]
fn multiple_slashes() -> Result {
    assert!(RouteSpec::from_str("a/b/c")?
        .matches("/a////b///c//")
        .is_some());
    Ok(())
}

#[test]
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

#[test]
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

#[test]
fn priority() -> Result {
    assert!(RouteSpec::from_str("exact")? < RouteSpec::from_str(":param")?);
    assert!(RouteSpec::from_str("a")? < RouteSpec::from_str("a/b")?);
    assert!(RouteSpec::from_str(":a.:b")? < RouteSpec::from_str(":a")?);
    Ok(())
}

#[test]
fn extend_captures() {
    let mut captures = Captures::from_iter([("key", "value")]);
    let other_captures = Captures::from_iter([("key2", "value2")]);

    captures.extend(other_captures);

    assert_eq!(
        captures.iter().collect::<Vec<_>>(),
        [("key", "value"), ("key2", "value2")]
    );
}

#[test]
fn append_captures() {
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
}
