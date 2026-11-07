use routefinder::{Captures, Router};

pub fn main() -> Result<(), String> {
    let mut router = Router::<Box<dyn Fn(Captures) -> String>>::new();

    router.add(
        "/*",
        Box::new(|captures| {
            let path = captures.wildcard().unwrap_or_default();
            format!("/{path} did not have an explicit match",)
        }),
    )?;

    router.add(
        "/hello/:planet",
        Box::new(|captures| format!("hello, {}", captures.get("planet").unwrap())),
    )?;

    router.add(
        "/hello/earth",
        Box::new(|_| "hello! this is your home planet so it gets a dedicated route".into()),
    )?;

    router.add(
        "/nested/*",
        Box::new(|captures| format!("wildcard: {}", captures.wildcard().unwrap_or_default())),
    )?;

    println!("router: {:#?}", router);

    let path = std::env::args().nth(1).unwrap_or_default();

    if let Some(m) = router.best_match(&path) {
        let route = m.route();
        let captures = m(m.captures());
        println!("\n\ninput: {path}\nbest match: {route}\noutput: {captures}\n\n");
    }

    let matches = router
        .matches(&path)
        .iter()
        .map(|m| m.route())
        .collect::<Vec<_>>();

    println!("all routes that match {path}, in order of decreasing precedence: {matches:#?}");

    Ok(())
}
