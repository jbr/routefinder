use super::{RouteSpec, Segment};
use smartstring::alias::String as SmartString;
use std::{iter, str::FromStr};

impl FromStr for RouteSpec {
    type Err = String;

    fn from_str(source: &str) -> Result<Self, Self::Err> {
        let mut last_index = 0;
        let source_trimmed = source.trim_start_matches('/').trim_end_matches('/');
        #[cfg(feature = "memchr")]
        let index_iter = memchr::memchr2_iter(b'.', b'/', source_trimmed.as_bytes());

        #[cfg(not(feature = "memchr"))]
        let index_iter = source_trimmed.match_indices(['.', '/']).map(|(i, _)| i);

        let segments = index_iter
            .chain(iter::once_with(|| source_trimmed.len()))
            .try_fold(vec![], |mut acc, index| {
                let first_char = if last_index == 0 {
                    None
                } else {
                    source_trimmed.chars().nth(last_index - 1)
                };

                let section = &source_trimmed[last_index..index];
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
            })?;

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
