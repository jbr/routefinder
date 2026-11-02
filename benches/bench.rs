use criterion::{criterion_group, criterion_main, Criterion};
use routefinder::Router;

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

    c.bench_function("match various route patterns", |b| {
        let router = Router::new_with_routes(ROUTES).unwrap();
        b.iter(|| {
            for path in PATHS {
                let _ = router.best_match(path);
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

criterion_group!(benches, benchmark1, benchmark2);
criterion_main!(benches);
