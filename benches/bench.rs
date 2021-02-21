use criterion::{criterion_group, criterion_main, Criterion};

use routefinder::*;

fn benchmark(c: &mut Criterion) {
    let mut router = Router::new();
    router.add("/posts/:post_id/comments/:id", 1).unwrap();
    router.add("/posts/:post_id/comments", 2).unwrap();
    router.add("/posts/:post_id", 3).unwrap();
    router.add("/posts", 4).unwrap();
    router.add("/comments", 5).unwrap();
    router.add("/comments/:id", 6).unwrap();
    router.add("/*", 7).unwrap();

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

criterion_group!(benches, benchmark);
criterion_main!(benches);
