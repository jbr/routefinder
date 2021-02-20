#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Segment<'s> {
    Slash,
    Dot,
    Exact(&'s str),
    Param(&'s str),
    Wildcard,
}
impl PartialOrd for Segment<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Segment<'_> {
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
