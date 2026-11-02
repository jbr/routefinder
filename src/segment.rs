use compact_str::CompactString;
use std::{fmt, ops::Deref};

/// The text payload of a [`Segment::Exact`] or [`Segment::Param`].
///
/// Dereferences to `str`; the storage is an implementation detail.
#[derive(PartialEq, Eq, Clone, Hash, Default)]
pub struct Text(CompactString);

impl Text {
    pub(crate) fn push(&mut self, c: char) {
        self.0.push(c);
    }
}

impl Deref for Text {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for Text {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Text {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

impl fmt::Display for Text {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl PartialEq<str> for Text {
    fn eq(&self, other: &str) -> bool {
        &*self.0 == other
    }
}

impl PartialEq<&str> for Text {
    fn eq(&self, other: &&str) -> bool {
        &*self.0 == *other
    }
}

impl From<&str> for Text {
    fn from(s: &str) -> Self {
        Self(CompactString::from(s))
    }
}

impl From<String> for Text {
    fn from(s: String) -> Self {
        Self(CompactString::from(s))
    }
}
/// the internal representation of a parsed component of a route
///
/// as an example, `/hello/:planet/*` would be represented as the
/// following sequence `[Exact("hello"), Slash, Param("planet"),
/// Slash, Wildcard]`
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Segment {
    /// represented by a / in the route spec and matching one /
    Slash,
    /// represented by a . in the route spec and matching one . in the path
    Dot,
    /// represented by any free text in the route spec, this matches
    /// exactly that text
    Exact(Text),
    /// represented by :name, where name is how the capture will be
    /// available in [`Captures`][crate::Captures]. Param captures up to the next slash
    /// or dot, whichever is next in the spec.
    Param(Text),
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

            (Dot, _) => Greater,
            (Exact(_), _) => Greater,
            (Param(_), Exact(_)) => Less,
            (Param(_), _) => Greater,
            (Wildcard, Exact(_)) | (Wildcard, Param(_)) => Less,
            (Wildcard, _) => Greater,
            _ => Less,
        }
    }
}
