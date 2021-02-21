/// the internal representation of a parsed component of a route as an
/// example, `/hello/:planet/*` would be represented as the following
/// sequence `[Exact("hello"), Slash, Param("planet"), Slash,
/// Wildcard]`
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Segment {
    /// represented by a / in the route spec and matching one /
    Slash,
    /// represented by a . in the route spec and matching one . in the path
    Dot,
    /// represented by any free text in the route spec, this matches
    /// exactly that text
    Exact(String),
    /// represented by :name, where name is how the capture will be
    /// available in [`Captures`]. Param captures up to the next slash
    /// or dot, whichever is next in the spec.
    Param(String),
    /// represented by * in the spec, this will capture everything up
    /// to the end of the path. a wildcard will also match nothing
    /// (similar to the regex `(.*)$`). There can only be one wildcard
    /// per route spec
    Wildcard,
}
impl PartialOrd for Segment {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Segment {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        use std::cmp::Ordering::*;
        use Segment::*;
        match (self, other) {
            (Exact(_), Exact(_))
            | (Slash, Slash)
            | (Dot, Slash)
            | (Slash, Dot)
            | (Dot, Dot)
            | (Param(_), Param(_))
            | (Wildcard, Wildcard) => Equal,

            (Exact(_), _) => Greater,
            (Param(_), Exact(_)) => Less,
            (Param(_), _) => Greater,
            (Wildcard, Exact(_)) | (Wildcard, Param(_)) => Less,
            (Wildcard, _) => Greater,
            _ => Less,
        }
    }
}
