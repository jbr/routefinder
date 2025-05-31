// Profiling target: a tight best_match loop over a realistic route table.
// Build release with debug symbols and profile, e.g.:
//   cargo instruments -t time --example profile_match --release
//   cargo flamegraph --example profile_match
use routefinder::Router;
use std::hint::black_box;

fn main() {
    let mut router = Router::new();
    let mut h = 0usize;
    let mut next = || {
        h += 1;
        h
    };
    // ~1000 REST-ish routes
    for r in 0..250 {
        router.add(format!("/r{r}"), next()).unwrap();
        router.add(format!("/r{r}/:id"), next()).unwrap();
        router.add(format!("/r{r}/:id/items"), next()).unwrap();
        router
            .add(format!("/r{r}/:id/items/:item_id"), next())
            .unwrap();
    }

    let paths = [
        "/r125",                  // static hit
        "/r125/42",               // param hit
        "/r125/42/items",         // deeper static
        "/r125/42/items/99",      // deep param hit
        "/does/not/exist/deeply", // miss
    ];

    let iterations: u64 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(20_000_000);

    let mut acc = 0usize;
    for _ in 0..iterations {
        for path in paths {
            if let Some(m) = router.best_match(black_box(path)) {
                acc = acc.wrapping_add(*m.handler());
            }
        }
    }
    println!("checksum: {acc}");
}
