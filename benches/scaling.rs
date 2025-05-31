// Scaling comparison: current `big-changes` trie vs `mainline` (origin/main's
// first trie) vs `published` (v0.5.4 linear matcher), across route-table sizes.
//
// Route sets use only static/param/wildcard segments so all three
// implementations match them identically (subsegment matching is branch-only
// and benched separately in bench.rs).

use criterion::{BenchmarkId, Criterion};
use std::{hint::black_box, time::Duration};

const SIZES: [usize; 4] = [10, 100, 1000, 10000];

/// Generate `n` REST-ish routes: each "resource" contributes four routes
/// (collection, member, sub-collection, sub-member).
fn routes(n: usize) -> Vec<(String, usize)> {
    let mut v = Vec::with_capacity(n + 4);
    let mut r = 0;
    while v.len() < n {
        let h = v.len();
        v.push((format!("/r{r}"), h));
        v.push((format!("/r{r}/:id"), h + 1));
        v.push((format!("/r{r}/:id/items"), h + 2));
        v.push((format!("/r{r}/:id/items/:item_id"), h + 3));
        r += 1;
    }
    v.truncate(n);
    v
}

/// Representative paths for a resource in the middle of the table, so the path
/// exists at every size: [static hit, param hit, deep hit, miss].
fn paths(n: usize) -> [String; 4] {
    let mid = (n / 4 / 2).max(0);
    [
        format!("/r{mid}"),
        format!("/r{mid}/42"),
        format!("/r{mid}/42/items/99"),
        "/does/not/exist/deeply".to_string(),
    ]
}

macro_rules! build {
    ($krate:ident, $routes:expr) => {{
        let mut router = $krate::Router::new();
        for (route, handler) in $routes {
            router.add(route.as_str(), *handler).unwrap();
        }
        router
    }};
}

fn matching(c: &mut Criterion) {
    struct Built {
        n: usize,
        current: routefinder::Router<usize>,
        mainline: mainline::Router<usize>,
        published: published::Router<usize>,
        paths: [String; 4],
    }

    let built: Vec<Built> = SIZES
        .iter()
        .map(|&n| {
            let rs = routes(n);
            Built {
                n,
                current: build!(routefinder, &rs),
                mainline: build!(mainline, &rs),
                published: build!(published, &rs),
                paths: paths(n),
            }
        })
        .collect();

    for (idx, scenario) in ["static_hit", "param_hit", "deep_hit", "miss"]
        .iter()
        .enumerate()
    {
        let mut g = c.benchmark_group(format!("match/{scenario}"));
        for b in &built {
            let path = &b.paths[idx];
            g.bench_with_input(BenchmarkId::new("current", b.n), &b.n, |bn, _| {
                bn.iter(|| b.current.best_match(black_box(path)))
            });
            g.bench_with_input(BenchmarkId::new("mainline", b.n), &b.n, |bn, _| {
                bn.iter(|| b.mainline.best_match(black_box(path)))
            });
            g.bench_with_input(BenchmarkId::new("published", b.n), &b.n, |bn, _| {
                bn.iter(|| b.published.best_match(black_box(path)))
            });
        }
        g.finish();
    }
}

fn construction(c: &mut Criterion) {
    let mut g = c.benchmark_group("construction");
    for &n in &[100usize, 1000, 10000] {
        let rs = routes(n);
        g.bench_with_input(BenchmarkId::new("current", n), &rs, |b, rs| {
            b.iter(|| build!(routefinder, rs))
        });
        g.bench_with_input(BenchmarkId::new("mainline", n), &rs, |b, rs| {
            b.iter(|| build!(mainline, rs))
        });
        g.bench_with_input(BenchmarkId::new("published", n), &rs, |b, rs| {
            b.iter(|| build!(published, rs))
        });
    }
    g.finish();
}

fn main() {
    let mut criterion = Criterion::default()
        .configure_from_args()
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(4))
        .sample_size(100);
    matching(&mut criterion);
    construction(&mut criterion);
    criterion.final_summary();
}
