use crate::{Captures, RouteSpec, Segment};

/// The first `Param` name within `segments`, descending into optional groups.
/// An optional group is rendered in reverse iff its first param is present in
/// the captures; a param-less optional group is therefore never rendered.
fn first_param(segments: &[Segment]) -> Option<&str> {
    segments.iter().find_map(|segment| match segment {
        Segment::Param(name) => Some(name.as_str()),
        Segment::Optional(inner) => first_param(inner),
        _ => None,
    })
}

fn optional_included(inner: &[Segment], captures: &Captures<'_, '_>) -> bool {
    first_param(inner).is_some_and(|name| captures.get(name).is_some())
}

/// The `Param` names that will actually be rendered for these captures, in
/// order, respecting which optional groups are included.
fn rendered_params<'a>(
    segments: &'a [Segment],
    captures: &Captures<'_, '_>,
    out: &mut Vec<&'a str>,
) {
    for segment in segments {
        match segment {
            Segment::Param(name) => out.push(name),
            Segment::Optional(inner) if optional_included(inner, captures) => {
                rendered_params(inner, captures, out);
            }
            _ => {}
        }
    }
}

/// This struct represents the result of a reverse lookup from
/// [`Captures`] to a [`RouteSpec`]
#[derive(Debug, Clone, Copy)]
pub struct ReverseMatch<'keys, 'values, 'captures, 'route> {
    route: &'route RouteSpec,
    captures: &'captures Captures<'keys, 'values>,
}

impl<'keys, 'values, 'captures, 'route> ReverseMatch<'keys, 'values, 'captures, 'route> {
    /// Attempts to build a new ReverseMatch. Returns None if the
    /// match was unsuccessful.
    pub fn new(
        captures: &'captures Captures<'keys, 'values>,
        route: &'route RouteSpec,
    ) -> Option<Self> {
        let mut params = vec![];
        rendered_params(route.segments(), captures, &mut params);

        let all_params_matched = params
            .iter()
            .copied()
            .eq(captures.params().iter().map(|c| c.name()));

        if !all_params_matched {
            return None;
        }

        if captures.wildcard().is_some()
            && !route
                .capture_segments()
                .iter()
                .any(|s| matches!(s, Segment::Wildcard))
        {
            return None;
        }

        Some(Self { route, captures })
    }

    /// Returns the [`RouteSpec`] for this ReverseMatch
    pub fn route(&self) -> &RouteSpec {
        self.route
    }

    /// Returns the [`Captures`] for this ReverseMatch
    pub fn captures(&self) -> &Captures<'keys, 'values> {
        self.captures
    }
}

impl<'keys, 'values, 'captures, 'route> std::fmt::Display
    for ReverseMatch<'keys, 'values, 'captures, 'route>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("/")?;
        self.fmt_segments(self.route.segments(), f)
    }
}

impl<'keys, 'values, 'captures, 'route> ReverseMatch<'keys, 'values, 'captures, 'route> {
    fn fmt_segments(
        &self,
        segments: &[Segment],
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        for segment in segments {
            match segment {
                Segment::Slash => f.write_str("/")?,
                Segment::Dot => f.write_str(".")?,
                Segment::Exact(s) => f.write_str(s)?,
                Segment::Param(p) => f.write_str(self.captures.get(p).unwrap())?,
                Segment::Wildcard => f.write_str(self.captures.wildcard().unwrap_or_default())?,
                Segment::Optional(inner) => {
                    if optional_included(inner, self.captures) {
                        self.fmt_segments(inner, f)?;
                    }
                }
            };
        }
        Ok(())
    }
}
