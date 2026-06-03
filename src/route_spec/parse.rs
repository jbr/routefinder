use super::{RouteSpec, Segment};
use smartstring::alias::String as SmartString;
use std::{iter, str::FromStr};

impl FromStr for RouteSpec {
    type Err = String;

    fn from_str(source: &str) -> Result<Self, Self::Err> {
        let source_trimmed = source.trim_start_matches('/').trim_end_matches('/');
        let segments = parse_level(source_trimmed)?;

        Ok(Self {
            source: Some(SmartString::from(source)),
            segments,
            min_length: 0,
            dot_count: 0,
            segment_groups: vec![],
            handler_index: None,
        }
        .optimize())
    }
}

/// Parse one nesting level of a route. A level is a bracket-free prefix
/// optionally followed by a single trailing `[...]` optional group, which may
/// itself contain a nested level. Because closing `]`s are terminal, the group
/// (if present) is always the last thing at this level and nothing may follow
/// its `]`.
fn parse_level(source: &str) -> Result<Vec<Segment>, String> {
    let Some(open) = source.find('[') else {
        return parse_flat(source);
    };

    if !source.ends_with(']') {
        return Err(String::from(
            "an optional group `[` must be closed by a `]` at the end of the route; \
             nothing may follow the closing `]`",
        ));
    }

    let prefix = &source[..open];
    let inner = &source[open + 1..source.len() - 1];

    if inner.is_empty() {
        return Err(String::from("optional groups `[]` must not be empty"));
    }

    let mut segments = parse_flat(prefix)?;
    segments.push(Segment::Optional(parse_level(inner)?));
    Ok(segments)
}

/// Parse a bracket-free run of segments. Unlike [`FromStr`], this does not trim
/// leading/trailing slashes: a leading delimiter is meaningful (it becomes a
/// `Slash`/`Dot` segment), which is how optional groups retain their separator.
fn parse_flat(source: &str) -> Result<Vec<Segment>, String> {
    if source.contains(['[', ']']) {
        return Err(String::from("unbalanced or misplaced optional group brackets"));
    }

    let mut last_index = 0;

    #[cfg(feature = "memchr")]
    let index_iter = memchr::memchr2_iter(b'.', b'/', source.as_bytes());

    #[cfg(not(feature = "memchr"))]
    let index_iter = source.match_indices(['.', '/']).map(|(i, _)| i);

    index_iter
        .chain(iter::once_with(|| source.len()))
        .try_fold(vec![], |mut acc, index| {
            let first_char = if last_index == 0 {
                None
            } else {
                source.chars().nth(last_index - 1)
            };

            let section = &source[last_index..index];
            last_index = index + 1;

            let segment = match (section.chars().next(), section.len()) {
                (Some('*'), 1) => Some(Segment::Wildcard),
                (Some('*'), _) => {
                    return Err(format!(
                        concat!(
                            "since there can only be one wildcard,",
                            " it doesn't need a name. replace `{}` with `*`"
                        ),
                        section
                    ));
                }
                (Some(':'), 1) => {
                    return Err(String::from("params must be named"));
                }
                (Some(':'), _) => Some(Segment::Param(SmartString::from(&section[1..]))),
                (None, 0) => None,
                (_, _) => Some(Segment::Exact(SmartString::from(section))),
            };

            if first_char == Some('.') {
                acc.push(Segment::Dot);
            }

            if let Some(segment) = segment {
                if first_char == Some('/') {
                    acc.push(Segment::Slash);
                }

                acc.push(segment);
            }

            Ok(acc)
        })
}
