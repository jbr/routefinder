use smartstring::alias::String as SmartString;
/// the internal representation of a parsed component of a route
///
/// as an example, `/hello/:planet/*` would be represented as the
/// following sequence `[Exact("hello"), Slash, Param("planet"),
/// Slash, Wildcard]`
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum Segment {
    /// represented by a / in the route spec and matching one /
    Slash,
    /// represented by a . in the route spec and matching one . in the path
    Dot,
    /// represented by any free text in the route spec, this matches
    /// exactly that text
    Exact(SmartString),
    /// represented by :name, where name is how the capture will be
    /// available in [`Captures`][crate::Captures]. Param captures up to the next slash
    /// or dot, whichever is next in the spec.
    Param(SmartString),
    /// represented by * in the spec, this will capture everything up
    /// to the end of the path. a wildcard will also match nothing
    /// (similar to the regex `(.*)$`). There can only be one wildcard
    /// per route spec
    Wildcard,
    /// represented by `[...]` in the spec, this marks a trailing,
    /// possibly-nested optional group. For example `/some[/:opt[.:ext]]`
    /// parses to `[Exact("some"), Optional([Slash, Param("opt"),
    /// Optional([Dot, Param("ext")])])]`. An `Optional` is always the final
    /// segment at its level (closing `]`s are terminal), and the contained
    /// segments include their own leading delimiter. The trie never sees this
    /// variant: a route containing it is expanded into flat variant specs at
    /// insertion time (see [`RouteSpec::expand`][crate::RouteSpec::expand]).
    Optional(Vec<Segment>),
}

#[cfg(feature = "arbitrary")]
#[derive(arbitrary::Arbitrary)]
pub(crate) enum SegmentType {
    Slash,
    Dot,
    Exact,
    Param,
    Wildcard,
}

#[cfg(feature = "arbitrary")]
pub(crate) fn arbitrary_exact_segment(
    u: &mut arbitrary::Unstructured<'_>,
) -> arbitrary::Result<Segment> {
    let len = u.int_in_range(1..=10)?;
    let literal_chars = ('0'..='9')
        .chain('A'..='Z')
        .chain('a'..='z')
        .chain(['-', '_', '~'])
        .chain(['!', '$', '&', '\'', '(', ')', '*', '+', ',', ';', '='])
        .collect::<Vec<_>>();
    Ok(Segment::Exact(
        std::iter::repeat_with(|| u.choose_iter(&literal_chars).copied())
            .take(len)
            .collect::<arbitrary::Result<SmartString>>()?,
    ))
}

#[cfg(feature = "arbitrary")]
pub(crate) fn arbitrary_param(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<Segment> {
    let alpha = ('a'..='z').collect::<Vec<_>>();
    let len = u.int_in_range(1..=10)?;
    Ok(Segment::Param(
        std::iter::repeat_with(|| u.choose_iter(&alpha).copied())
            .take(len)
            .collect::<arbitrary::Result<SmartString>>()?,
    ))
}

#[cfg(feature = "arbitrary")]
impl<'a> arbitrary::Arbitrary<'a> for Segment {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        Ok(match SegmentType::arbitrary(u)? {
            SegmentType::Exact => arbitrary_exact_segment(u)?,
            SegmentType::Param => arbitrary_param(u)?,
            SegmentType::Dot => Segment::Dot,
            SegmentType::Slash => Segment::Slash,
            SegmentType::Wildcard => Segment::Wildcard,
        })
    }
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
            // Optional only appears in canonical specs (never in the trie), and
            // ranks as least specific so the order stays total and consistent
            // with the derived Eq.
            (Optional(l), Optional(r)) => l.cmp(r),
            (Optional(_), _) => Less,
            (_, Optional(_)) => Greater,
            (Exact(l), Exact(r)) => l.cmp(r),
            (Param(l), Param(r)) => l.cmp(r),
            (Slash, Slash) | (Dot, Slash) | (Slash, Dot) | (Dot, Dot) | (Wildcard, Wildcard) => {
                Equal
            }
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
