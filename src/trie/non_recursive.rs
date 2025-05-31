use super::{find_slash, State, Trie, TrieMatch, TrieNode};
use crate::Path;
use smallvec::{smallvec, Array, SmallVec};

macro_rules! try_match {
    ($result:expr) => {
        if let Some(route) = $result {
            return Some(route);
        }
    };
}

#[derive(Debug)]
pub(crate) struct SearchFrame<'trie, 'path> {
    node: &'trie TrieNode,
    path: &'path str,
    capture_count: usize,
    visits: usize,
    state_index: usize,
    segment: &'path str,
    rest: &'path str,
}

#[derive(Default, Debug)]
pub(crate) struct Stack<'trie, 'path>(SmallVec<[SearchFrame<'trie, 'path>; 5]>);

#[derive(Debug)]
pub(crate) struct TrieSearcher<'trie, 'path> {
    pub(crate) captures: SmallVec<[&'path str; 5]>,
    pub(crate) current: SearchFrame<'trie, 'path>,
    pub(crate) stack: Stack<'trie, 'path>,
}

impl<'trie, 'path> TrieSearcher<'trie, 'path> {
    pub(crate) fn next(&mut self) -> Option<TrieMatch<'trie, 'path>> {
        if self.current.state_index != 0 {
            self.increment_visits()?
        }

        loop {
            let state = self
                .current
                .node
                .states
                .get(self.current.state_index)
                .unwrap_or(&State::Done);

            self.current.state_index += 1;

            match state {
                State::Start => self.start(),
                State::Terminal => try_match!(self.terminal()),
                State::FindSlash => {
                    (self.current.segment, self.current.rest) = find_slash(self.current.path);
                }
                State::Exact { min, max } => self.exact(*min, *max),
                State::Prefixes { min } => self.prefixes(*min),
                State::Suffixes { min } => self.suffixes(*min),
                State::Degenerate { index, min } => self.degenerate(*index, *min),
                State::Captures => self.captures(),
                State::Wildcard => try_match!(self.wildcard()),
                State::Done => self.done()?,
            }
        }
    }

    #[inline(always)]
    pub(crate) fn new(trie: &'trie Trie, path: &'path str) -> Self {
        Self {
            current: SearchFrame::new(&trie.0, 0, path),
            stack: Stack::default(),
            captures: smallvec![],
        }
    }

    #[inline(always)]
    fn start(&mut self) {
        self.current.visits += 1;
        let len = self.current.path.len();
        if self.current.node.minimum_length.is_some_and(|ml| len < ml) {
            self.current.state_index = self.current.node.states.len();
            #[cfg(feature = "log")]
            log::trace!(
                "abandoning due to length of {len} and a minimum of {:?}",
                self.current.node.minimum_length,
            );
        }
    }

    #[inline(always)]
    fn terminal(&mut self) -> Option<TrieMatch<'trie, 'path>> {
        if self.current.path.is_empty() {
            let route = &self.current.node.route.as_ref().unwrap();

            #[cfg(feature = "log")]
            log::trace!("reached terminal node for {route}");
            return Some(TrieMatch {
                route,
                captures: self.captures.clone(),
                wildcard: None,
            });
        }
        None
    }

    #[inline(always)]
    fn exact(&mut self, min: usize, max: usize) {
        let segment = self.current.segment;
        let rest = self.current.rest;

        if segment.len() < min || segment.len() > max {
            return;
        }

        if let Some(node) = self.current.node.statics.get(segment) {
            self.stack.push(&mut self.current, node, 0, rest);
        }
    }

    #[inline(always)]
    fn prefixes(&mut self, min: usize) {
        let segment = self.current.segment;
        let rest = self.current.rest;

        if segment.len() < min {
            return;
        }

        let subsegment_matcher = self.current.node.subsegment_matcher.as_ref().unwrap();
        for prefix_length in &subsegment_matcher.prefix_lengths {
            if *prefix_length >= segment.len() {
                break;
            }

            if let Some(node) = subsegment_matcher.prefixes.get(&segment[0..*prefix_length]) {
                self.captures.push(&segment[*prefix_length..]);
                self.stack.push(&mut self.current, node, 1, rest);
                break;
            }
        }
    }

    #[inline(always)]
    fn suffixes(&mut self, min: usize) {
        let segment = self.current.segment;
        let rest = self.current.rest;

        if segment.len() < min {
            return;
        }

        let subsegment_matcher = self.current.node.subsegment_matcher.as_ref().unwrap();
        for suffix_length in &subsegment_matcher.suffix_lengths {
            if *suffix_length >= segment.len() {
                break;
            }

            let suffix_start = segment.len() - suffix_length;
            if let Some(node) = subsegment_matcher.suffixes.get(&segment[suffix_start..]) {
                self.captures.push(&segment[..suffix_start]);
                self.stack.push(&mut self.current, node, 1, rest);
                break;
            }
        }
    }

    #[inline(always)]
    fn degenerate(&mut self, index: usize, min: usize) {
        let segment = self.current.segment;
        let rest = self.current.rest;

        if segment.len() < min {
            return;
        }

        let degenerate = &self
            .current
            .node
            .subsegment_matcher
            .as_ref()
            .unwrap()
            .degenerate;
        let path = Path::from(segment);

        let (segment_spec, node) = degenerate.iter().nth(index).unwrap();
        let mut new_captures: SmallVec<[&'path str; 2]> = smallvec![];
        if segment_spec.matches_path(&path, &mut new_captures) {
            self.captures.extend_from_slice(&new_captures);
            self.stack
                .push(&mut self.current, node, new_captures.len(), rest);
        }
    }

    #[inline(always)]
    fn captures(&mut self) {
        let segment = self.current.segment;
        let rest = self.current.rest;

        let node = self.current.node.param.as_deref().unwrap();
        if !segment.is_empty() {
            self.captures.push(segment);
            self.stack.push(&mut self.current, node, 1, rest);
        }
    }

    #[inline(always)]
    fn wildcard(&mut self) -> Option<TrieMatch<'trie, 'path>> {
        Some(TrieMatch {
            wildcard: Some(self.current.path),
            route: self.current.node.wildcard.as_ref().unwrap(),
            captures: self.captures.clone(),
        })
    }

    #[inline(always)]
    fn done(&mut self) -> Option<()> {
        self.captures
            .truncate(self.captures.len() - self.current.capture_count);
        self.current = self.stack.pop()?;
        self.increment_visits()
    }

    #[inline(always)]
    fn increment_visits(&mut self) -> Option<()> {
        self.stack
            .increment_visits(&mut self.captures, &mut self.current)
    }
}

impl<'trie, 'path> SearchFrame<'trie, 'path> {
    #[inline(always)]
    fn new(node: &'trie TrieNode, capture_count: usize, rest: &'path str) -> Self {
        SearchFrame {
            node,
            path: rest,
            capture_count,
            visits: 0,
            state_index: 0,
            segment: "",
            rest: "",
        }
    }
}

impl<'trie, 'path> Stack<'trie, 'path> {
    #[inline(always)]
    fn push(
        &mut self,
        current: &mut SearchFrame<'trie, 'path>,
        node: &'trie TrieNode,
        capture_count: usize,
        rest: &'path str,
    ) {
        let mut next = SearchFrame::new(node, capture_count, rest);
        if current.visits < current.node.paths {
            self.0.push(std::mem::replace(current, next));
        } else {
            next.capture_count += current.capture_count;
            #[cfg(feature = "log")]
            log::trace!("not pushing {current:?} to stack, just replacing with {next:?}");
            *current = next;
        }
    }

    #[inline(always)]
    fn increment_visits(
        &mut self,
        captures: &mut SmallVec<impl Array<Item = &'path str>>,
        current: &mut SearchFrame<'trie, 'path>,
    ) -> Option<()> {
        current.visits += 1;
        while current.visits > current.node.paths {
            #[cfg(feature = "log")]
            log::trace!("done with {current:?}");

            captures.truncate(captures.len() - current.capture_count);
            *current = self.0.pop()?;
            current.visits += 1;
        }
        Some(())
    }

    fn pop(&mut self) -> Option<SearchFrame<'trie, 'path>> {
        self.0.pop()
    }
}
