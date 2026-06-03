use smallvec::{Array, SmallVec};

use super::{Path, RouteSpec, Segment};

impl RouteSpec {
    #[inline]
    pub(crate) fn inner_match<'path>(
        &self,
        path: &Path<'path>,
        captures: &mut SmallVec<impl Array<Item = &'path str>>,
    ) -> Option<&'path str> {
        let mut path_str = path.trimmed;
        let mut peek = self.segments.iter().peekable();

        while let Some(segment) = peek.next() {
            path_str = match segment {
                Segment::Exact(e) => {
                    if path_str.starts_with(&**e) {
                        &path_str[e.len()..]
                    } else {
                        return None;
                    }
                }

                Segment::Param(_) => {
                    if path_str.is_empty() {
                        return None;
                    }
                    match peek.peek() {
                        None | Some(Segment::Slash) => {
                            #[cfg(feature = "memchr")]
                            let capture = memchr::memchr(b'/', path_str.as_bytes())
                                .map(|index| &path_str[..index])
                                .unwrap_or(path_str);
                            #[cfg(not(feature = "memchr"))]
                            let capture = path_str.split('/').next()?;

                            captures.push(capture);
                            &path_str[capture.len()..]
                        }

                        Some(Segment::Dot) => {
                            #[cfg(feature = "memchr")]
                            let index = memchr::memchr2(b'.', b'/', path_str.as_bytes())?;
                            #[cfg(not(feature = "memchr"))]
                            let index = path_str.find(['.', '/'])?;

                            if path_str.chars().nth(index) == Some('.') {
                                captures.push(&path_str[..index]);
                                &path_str[index..] // we leave the dot so it can be matched by the Segment::Dot
                            } else {
                                return None;
                            }
                        }
                        _ => panic!(
                            "param must be followed by a dot, a slash, or the end of the route"
                        ),
                    }
                }

                Segment::Wildcard => match peek.peek() {
                    Some(_) => panic!(concat!(
                        "wildcard must currently be the terminal segment, ",
                        "please file an issue if you have a use case for a mid-route *"
                    )),
                    None => {
                        captures.push(path_str);
                        ""
                    }
                },

                Segment::Slash => {
                    match (
                        path_str.chars().take_while(|c| *c == '/').count(),
                        peek.peek(),
                    ) {
                        (0, None | Some(Segment::Wildcard)) if path_str.is_empty() => path_str,
                        (n, Some(_)) if n > 0 => &path_str[n..],
                        _ => return None,
                    }
                }

                Segment::Dot => match path_str.chars().next() {
                    Some('.') => &path_str[1..],
                    _ => return None,
                },

                // Optional specs are expanded into flat variants before reaching
                // inner_match (see RouteSpec::matches_path), so this is never hit.
                Segment::Optional(_) => {
                    unreachable!("optional segments are expanded before matching")
                }
            }
        }

        Some(path_str)
    }
}
