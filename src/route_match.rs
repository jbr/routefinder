#[derive(Debug)]
pub struct Match<'router, 'path, T> {
    path: &'path str,
    route: &'router Route<T>,
    captures: Vec<&'path str>,
}

impl<'router, 'path, T> Match<'router, 'path, T> {
    pub fn handler(&self) -> &'router T {
        &self.route.handler
    }

    pub fn into_route(self) -> &'router Route<T> {
        self.route
    }

    pub fn captures(&self) -> Captures {
        self.route
            .definition
            .segments
            .iter()
            .filter(|s| matches!(s, Segment::Param(_) | Segment::Wildcard))
            .zip(&self.captures)
            .fold(
                Captures::default(),
                |mut captures, (segment, capture)| match segment {
                    Segment::Param(name) => {
                        captures
                            .0
                            .push((String::from(*name), String::from(*capture)));
                        captures
                    }

                    Segment::Wildcard => {
                        captures.1 = Some(String::from(*capture));
                        captures
                    }
                    _ => captures,
                },
            )
    }
}

impl<'router, 'path, T> PartialEq for Match<'router, 'path, T> {
    fn eq(&self, other: &Self) -> bool {
        *other.route == *self.route
    }
}

impl<'router, 'path, T> Eq for Match<'router, 'path, T> {}

impl<'router, 'path, T> PartialOrd for Match<'router, 'path, T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<'router, 'path, T> Ord for Match<'router, 'path, T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.route
            .definition
            .segments
            .iter()
            .zip(&other.route.definition.segments)
            .map(|(mine, theirs)| mine.cmp(theirs))
            .find(|c| *c != std::cmp::Ordering::Equal)
            .unwrap_or(std::cmp::Ordering::Equal)
    }
}
