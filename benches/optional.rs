// Does an optional route cost anything that hand-writing its expansion wouldn't?
//
// Approach 1 (parse-time expansion) inserts an optional route's variants into
// the trie exactly as if they had been added separately, so matching should be
// indistinguishable from a router built with the hand-written equivalents, and
// construction should differ only by the (one-time) expansion work. This bench
// puts the two side by side at several route-table sizes.

use criterion::{BenchmarkId, Criterion};
use routefinder::Router;
use std::{hint::black_box, time::Duration};

const SIZES: [usize; 4] = [10, 100, 1000, 10000];

/// `n` optional routes, one per resource: `/r{i}[/:id[.:fmt]]`.
fn optional_routes(n: usize) -> Vec<(String, usize)> {
    (0..n).map(|i| (format!("/r{i}[/:id[.:fmt]]"), i)).collect()
}

/// The same route set written out as its three flat variants per resource.
fn expanded_routes(n: usize) -> Vec<(String, usize)> {
    let mut v = Vec::with_capacity(n * 3);
    for i in 0..n {
        v.push((format!("/r{i}"), i));
        v.push((format!("/r{i}/:id"), i));
        v.push((format!("/r{i}/:id.:fmt"), i));
    }
    v
}

fn build(routes: &[(String, usize)]) -> Router<usize> {
    let mut router = Router::new();
    for (route, handler) in routes {
        router.add(route.as_str(), *handler).unwrap();
    }
    router
}

/// [absent, param present, param+ext present, miss] for a mid-table resource.
fn paths(n: usize) -> [String; 4] {
    let mid = (n / 2).max(0);
    [
        format!("/r{mid}"),
        format!("/r{mid}/42"),
        format!("/r{mid}/42.json"),
        "/does/not/exist".to_string(),
    ]
}

fn matching(c: &mut Criterion) {
    struct Built {
        n: usize,
        optional: Router<usize>,
        expanded: Router<usize>,
        paths: [String; 4],
    }

    let built: Vec<Built> = SIZES
        .iter()
        .map(|&n| Built {
            n,
            optional: build(&optional_routes(n)),
            expanded: build(&expanded_routes(n)),
            paths: paths(n),
        })
        .collect();

    for (idx, scenario) in ["absent", "param", "param_ext", "miss"].iter().enumerate() {
        let mut g = c.benchmark_group(format!("match/{scenario}"));
        for b in &built {
            let path = &b.paths[idx];
            g.bench_with_input(BenchmarkId::new("optional", b.n), &b.n, |bn, _| {
                bn.iter(|| b.optional.best_match(black_box(path)))
            });
            g.bench_with_input(BenchmarkId::new("expanded", b.n), &b.n, |bn, _| {
                bn.iter(|| b.expanded.best_match(black_box(path)))
            });
        }
        g.finish();
    }
}

fn construction(c: &mut Criterion) {
    let mut g = c.benchmark_group("construction");
    for &n in &[100usize, 1000, 10000] {
        let opt = optional_routes(n);
        let exp = expanded_routes(n);
        g.bench_with_input(BenchmarkId::new("optional", n), &opt, |b, rs| {
            b.iter(|| build(rs))
        });
        g.bench_with_input(BenchmarkId::new("expanded", n), &exp, |b, rs| {
            b.iter(|| build(rs))
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
