use routefinder::{RouteSpec, Router, Segment};

#[test]
fn slash_boundaries_are_required() {
    for (pattern, path) in [
        ("/files/*", "/filesomething"),
        ("/files/*", "/filesomething/else"),
        ("/foo/bar", "/foobar"),
        // A missing slash must not be hidden by the minimum-length check.
        ("/foo/bar/*", "/foobar/long-enough"),
        ("/foo/bar/baz", "/foobar//baz"),
        ("/foo/:bar", "/foobarbaz"),
        ("/foo/bar/:baz", "/foobar/long-enough"),
        ("/foo/bar/*", "/foo/barbaz"),
    ] {
        let spec: RouteSpec = pattern.parse().unwrap();
        assert_eq!(spec.matches(path), None, "{pattern:?} vs {path:?}");
        let router = Router::new_with_routes([(pattern, ())]).unwrap();
        assert!(
            router.best_match(path).is_none(),
            "{:?} vs {:?}",
            pattern,
            path
        );
        assert!(
            router.matches(path).is_empty(),
            "{:?} vs {:?}",
            pattern,
            path
        );
    }
}

#[test]
fn wildcard_slash_is_optional_only_at_end_of_path() {
    let spec: RouteSpec = "/files/*".parse().unwrap();
    for (path, capture) in [
        ("/files", ""),
        ("/files/", ""),
        ("//files////", ""),
        ("/files/something", "something"),
        ("/files////something/else", "something/else"),
    ] {
        assert_eq!(spec.matches(path), Some(vec![capture]), "{path:?}");
    }
    let spec: RouteSpec = "/foo/bar".parse().unwrap();
    assert_eq!(spec.matches("/foo/bar"), Some(vec![]));
    assert_eq!(spec.matches("//foo///bar//"), Some(vec![]));
}

#[test]
fn unicode_route_separators_use_byte_offsets() {
    for (pattern, path, segments) in [
        (
            "/é/x",
            "/é/x",
            vec![
                Segment::Exact("é".into()),
                Segment::Slash,
                Segment::Exact("x".into()),
            ],
        ),
        (
            "/🦀/x",
            "/🦀/x",
            vec![
                Segment::Exact("🦀".into()),
                Segment::Slash,
                Segment::Exact("x".into()),
            ],
        ),
        (
            "/é.txt",
            "/é.txt",
            vec![Segment::Exact("é.".into()), Segment::Exact("txt".into())],
        ),
    ] {
        let parsed: RouteSpec = pattern.parse().unwrap();
        assert_eq!(parsed.segments(), segments, "{pattern:?}");
        assert_eq!(parsed.matches(path), Some(vec![]), "{pattern:?}");
        assert_eq!(RouteSpec::from(segments).matches(path), Some(vec![]));
    }
}

#[test]
fn unicode_parameter_before_dot_uses_byte_offsets() {
    let spec: RouteSpec = "/:name.:ext".parse().unwrap();
    for name in ["a", "é", "🦀", "東京"] {
        let path = format!("/{name}.txt");
        assert_eq!(spec.matches(&path), Some(vec![name, "txt"]), "{path:?}");
        // A slash cannot be mistaken for the expected dot after Unicode.
        let path = format!("/{name}/.txt");
        assert_eq!(spec.matches(&path), None, "{path:?}");
    }
}
