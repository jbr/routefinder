use criterion::Criterion;
use routefinder::Router;
use std::time::Duration;

fn benchmark1(c: &mut Criterion) {
    let router = Router::new_with_routes([
        ("/posts/:post_id/comments/:id", 1),
        ("/posts/:post_id/comments", 2),
        ("/posts/:post_id", 3),
        ("/posts", 4),
        ("/comments", 5),
        ("/comments/:id", 6),
        ("/*", 7),
    ])
    .unwrap();

    c.bench_function("/posts/n/comments/n", |b| {
        b.iter(|| router.best_match("/posts/100/comments/200"))
    });

    c.bench_function("/posts/n/comments", |b| {
        b.iter(|| router.best_match("/posts/100/comments"))
    });

    c.bench_function("/posts/n", |b| b.iter(|| router.best_match("/posts/100")));

    c.bench_function("/posts", |b| b.iter(|| router.best_match("/posts")));

    c.bench_function("/comments", |b| b.iter(|| router.best_match("/comments")));

    c.bench_function("/comments/n", |b| {
        b.iter(|| router.best_match("/comments/100"))
    });

    c.bench_function("fallthrough", |b| {
        b.iter(|| router.best_match("/a/b/c/d/e/f"))
    });
}

const ROUTES: [(&str, &str); 10] = [
    // Static routes
    ("/home", "home"),
    ("/about", "about"),
    ("/contact", "contact"),
    // Parameterized routes
    ("/user/:id", "user_profile"),
    ("/post/:year/:month/:slug", "blog_post"),
    // Wildcard routes
    ("/static/*", "static_files"),
    ("/files/*", "file_handler"),
    // Mixed routes with greater depth
    ("/a/b/c/d/e/f", "deep_static"),
    ("/deep/:one/:two/:three", "deep_params"),
    ("/a/:b/*", "greedy_middle"),
];

const PATHS: [&str; 9] = [
    "/home",                      // static
    "/user/123",                  // param
    "/post/2023/10/rust-routing", // multi-param
    "/static/css/style.css",      // wildcard
    "/files/docs/readme.md",      // wildcard
    "/a/b/c/d/e/f",               // deep static
    "/deep/one/two/three",        // deep param
    "/greedy/any/arbitrary/tail", // greedy middle
    "/not/found/path",            // non-matching
];

fn benchmark2(c: &mut Criterion) {
    c.bench_function("static router creation", |b| {
        b.iter(|| Router::new_with_routes(ROUTES).unwrap())
    });

    c.bench_function("best match various route patterns", |b| {
        let router = Router::new_with_routes(ROUTES).unwrap();
        b.iter(|| {
            for path in PATHS {
                let _ = router.best_match(path);
            }
        })
    });

    c.bench_function("next match various route patterns", |b| {
        let router = Router::new_with_routes(ROUTES).unwrap();
        b.iter(|| {
            for path in PATHS {
                let _ = router.match_iter(path).next();
            }
        })
    });

    c.bench_function("insert 100 dynamic routes", |b| {
        b.iter(|| {
            let mut router = Router::new();
            for i in 0..100 {
                router.add(format!("/dyn/route/{i}"), "dynamic").unwrap();
            }
        })
    });
}

const ROUTES2: [(&str, &str); 13] = [
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

const PATHS2: [&str; 14] = [
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

fn benchmark3(c: &mut Criterion) {
    c.bench_function("bench 3 static router creation", |b| {
        b.iter(|| Router::new_with_routes(ROUTES2).unwrap())
    });

    c.bench_function("bench 3 best match", |b| {
        let router = Router::new_with_routes(ROUTES2).unwrap();
        b.iter(|| {
            for path in PATHS2 {
                let _ = router.best_match(path);
            }
        })
    });

    c.bench_function("bench 3 next", |b| {
        let router = Router::new_with_routes(ROUTES2).unwrap();
        b.iter(|| {
            for path in PATHS2 {
                let _ = router.match_iter(path).next();
            }
        })
    });

    c.bench_function("bench 3 all matches", |b| {
        let router = Router::new_with_routes(ROUTES2).unwrap();
        b.iter(|| {
            for path in PATHS2 {
                let _ = router.matches(path);
            }
        })
    });
}
fn benchmark3_comparison(c: &mut Criterion) {
    c.bench_function("bench 3 comparison static router creation", |b| {
        b.iter(|| {
            let mut router = published::Router::new();
            for (route, handler) in ROUTES2 {
                router.add(route, handler).unwrap();
            }
        })
    });

    c.bench_function("bench 3 comparison best match", |b| {
        let mut router = published::Router::new();
        for (route, handler) in ROUTES2 {
            router.add(route, handler).unwrap();
        }
        b.iter(|| {
            for path in PATHS2 {
                let _ = router.best_match(path);
            }
        })
    });

    c.bench_function("bench 3 comparison next", |b| {
        let mut router = published::Router::new();
        for (route, handler) in ROUTES2 {
            router.add(route, handler).unwrap();
        }
        b.iter(|| {
            for path in PATHS2 {
                let _ = router.match_iter(path).next();
            }
        })
    });

    c.bench_function("bench 3 comparison all matches", |b| {
        let mut router = published::Router::new();
        for (route, handler) in ROUTES2 {
            router.add(route, handler).unwrap();
        }

        b.iter(|| {
            for path in PATHS2 {
                let _ = router.matches(path);
            }
        })
    });
}

fn main() {
    let mut criterion = Criterion::default()
        .configure_from_args()
        .measurement_time(Duration::from_secs(20))
        .sample_size(500);
    benchmark1(&mut criterion);
    benchmark2(&mut criterion);
    benchmark3(&mut criterion);
    benchmark3_comparison(&mut criterion);
    Criterion::default().configure_from_args().final_summary();
}
