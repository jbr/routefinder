#![no_main]
// #[macro_use]
// extern crate libfuzzer_sys;
use libfuzzer_sys::fuzz_target;
use routefinder::arbitrary::FuzzPair;

fuzz_target!(|pairs: Vec<FuzzPair>| {
    let _ = env_logger::builder().is_test(true).try_init();

    let mut router = routefinder::Router::new();
    let mut paths = vec![];
    for (n, FuzzPair(route, path)) in pairs.into_iter().enumerate() {
        router.add(route, n).unwrap();
        paths.push(path);
    }

    for path in &paths {
        //        println!("--\n\n\n");
        let new = router.best_match(path);
        let old = router.match_iter(path).next();
        assert_eq!(
            new, old,
            "\n\npath: {path}\nrouter: {router:#?}\nnew: {new:?}\nold: {old:?}\n\n"
        );
    }
});
