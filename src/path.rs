/// A path to be tested against the router.
#[derive(Debug)]
pub(crate) struct Path<'a> {
    pub(crate) str: &'a str,
    pub(crate) trimmed: &'a str,
}

impl<'a> From<&'a str> for Path<'a> {
    fn from(str: &'a str) -> Self {
        let trimmed = str.trim_start_matches('/').trim_end_matches('/');
        Self { str, trimmed }
    }
}
