use routefinder::{Captures, Router};

pub fn main() -> Result<(), String> {
    let mut router: Router<Box<dyn Fn(Captures) -> String>> = Router::new();

    router.add(
        "/*",
        Box::new(|captures| {
            format!(
                "/{} did not have an explicit match",
                captures.wildcard().unwrap_or_default()
            )
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
        println!(
            "\n\ninput: {}\nbest match: {}\noutput: {}\n\n",
            &path,
            m.route(),
            (m.handler())(m.captures()),
        );
    }

    println!(
        "all routes that match {}, in order of decreasing precedence: {:#?}",
        &path,
        router
            .matches(&path)
            .iter()
            .rev()
            .map(|m| m.route())
            .collect::<Vec<_>>()
    );

    Ok(())
}
