use super::{RouteSpec, Segment};
use crate::segment::{arbitrary_exact_segment, arbitrary_param, SegmentType};
use arbitrary::{Arbitrary, Unstructured};
#[derive(Debug)]
#[doc(hidden)]
pub struct FuzzPair(pub RouteSpec, pub String);

fn gen_segment(u: &mut Unstructured<'_>) -> arbitrary::Result<String> {
    let len = u.int_in_range(1..=10)?;
    let literal_chars = ('0'..='9')
        .chain('A'..='Z')
        .chain('a'..='z')
        .chain(['-', '_', '~'])
        .chain(['!', '$', '&', '\'', '(', ')', '*', '+', ',', ';', '='])
        .collect::<Vec<_>>();
    std::iter::repeat_with(|| u.choose_iter(&literal_chars).copied())
        .take(len)
        .collect::<arbitrary::Result<String>>()
}

impl<'a> Arbitrary<'a> for FuzzPair {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        let route_spec = RouteSpec::arbitrary(u)?;
        let mut path = String::new();
        for segment in route_spec.segments() {
            match segment {
                Segment::Slash => path.push('/'),
                Segment::Dot => path.push('.'),
                Segment::Exact(smart_string) => path.push_str(smart_string),
                Segment::Param(_) => {
                    let str = gen_segment(u)?;
                    path.push_str(&str);
                }
                Segment::Wildcard => {
                    for _ in 0..u.int_in_range(0..=10)? {
                        match path.chars().last() {
                            None | Some('/' | '.') => path.push_str(&gen_segment(u)?),
                            _ if u.ratio(5, 7)? => path.push('/'),
                            _ => path.push('.'),
                        }
                    }
                }
            }
        }

        Ok(Self(route_spec, path))
    }
}
impl<'a> Arbitrary<'a> for RouteSpec {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        let mut segments = vec![];

        for _ in 0..u.arbitrary_len::<Segment>()? {
            let segment_type = match segments.last() {
                None | Some(Segment::Slash) => u.choose(&[
                    SegmentType::Exact,
                    SegmentType::Param,
                    SegmentType::Wildcard,
                ])?,

                Some(Segment::Dot) => u.choose(&[SegmentType::Exact, SegmentType::Param])?,

                Some(Segment::Exact(_) | Segment::Param(_)) => {
                    if u.ratio(5, 7)? {
                        &SegmentType::Slash
                    } else {
                        &SegmentType::Dot
                    }
                }

                Some(Segment::Wildcard) => break,
            };

            let segment = match segment_type {
                SegmentType::Exact => arbitrary_exact_segment(u)?,
                SegmentType::Param => arbitrary_param(u)?,
                SegmentType::Dot => Segment::Dot,
                SegmentType::Slash => Segment::Slash,
                SegmentType::Wildcard => Segment::Wildcard,
            };

            segments.push(segment);
        }

        Ok(Self {
            segments,
            source: None,
            min_length: 0,
            dot_count: 0,
            segment_groups: vec![],
            handler_index: None,
        }
        .optimize())
    }
}
