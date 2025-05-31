mod best_match;
mod debug;
mod insert;
mod non_recursive;
#[cfg(test)]
mod test;

use crate::{RouteSpec, TrieIter};
pub(crate) use non_recursive::TrieSearcher;
use smallvec::SmallVec;
use smartstring::alias::String;
use std::{collections::BTreeMap, fmt::Formatter};

#[derive(Default, Debug)]
pub(crate) struct Trie(TrieNode);

impl std::fmt::Display for Trie {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "\n/ ╾─╮")?;
        debug::fmt_with_indent(&self.0, f, vec![(4, false)])
    }
}

impl Trie {
    pub(crate) fn insert(&mut self, route_spec: RouteSpec) {
        self.0.insert(route_spec, 0);
    }

    pub(crate) fn best_match<'trie, 'a>(
        &'trie self,
        path: &'a str,
    ) -> Option<TrieMatch<'trie, 'a>> {
        let path = path.trim_start_matches('/').trim_end_matches('/');
        #[cfg(feature = "log")]
        log::trace!("{path}");

        self.0.best_match(path).map(|mut trie_match| {
            trie_match.captures.reverse();
            trie_match
        })
    }

    pub(crate) fn match_iter<'trie, 'path>(
        &'trie self,
        path: &'path str,
    ) -> TrieIter<'trie, 'path> {
        let path = path.trim_start_matches('/').trim_end_matches('/');
        TrieIter(TrieSearcher::new(self, path))
    }
}

#[derive(Debug)]
pub(crate) struct TrieMatch<'trie, 'a> {
    pub(crate) route: &'trie RouteSpec,
    pub(crate) captures: SmallVec<[&'a str; 5]>,
    pub(crate) wildcard: Option<&'a str>,
}

#[derive(Default)]
pub(crate) struct TrieNode {
    paths: usize,
    statics: BTreeMap<String, TrieNode>,
    param: Option<Box<TrieNode>>,
    wildcard: Option<RouteSpec>,
    route: Option<RouteSpec>,
    subsegment_matcher: Option<SubsegmentNode>,
    minimum_length: Option<usize>,
    /// (min, max) length of the keys in `statics`, maintained incrementally as
    /// static children are added so `compute_state_sequence` stays O(1) per
    /// insert rather than folding over every static child each time.
    static_len_range: Option<(usize, usize)>,
    states: SmallVec<[State; 10]>,
}

#[derive(Default)]
struct SubsegmentNode {
    prefixes: BTreeMap<String, TrieNode>,
    suffixes: BTreeMap<String, TrieNode>,
    prefix_lengths: SmallVec<[usize; 2]>,
    suffix_lengths: SmallVec<[usize; 2]>,
    degenerate: BTreeMap<RouteSpec, TrieNode>,
}

#[derive(Default, Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq)]
pub(crate) enum State {
    #[default]
    Start,
    Terminal,
    FindSlash,
    Exact {
        min: usize,
        max: usize,
    },
    Prefixes {
        min: usize,
    },
    Suffixes {
        min: usize,
    },
    Degenerate {
        index: usize,
        min: usize,
    },
    Captures,
    Wildcard,
    Done,
}

fn find_slash(path: &str) -> (&str, &str) {
    #[cfg(feature = "memchr")]
    let slash = memchr::memchr(b'/', path.as_bytes());
    #[cfg(not(feature = "memchr"))]
    let slash = path.find('/');

    if let Some(slash) = slash {
        let (s, r) = path.split_at(slash);
        (s, r.trim_start_matches('/'))
    } else {
        (path, "")
    }
}
