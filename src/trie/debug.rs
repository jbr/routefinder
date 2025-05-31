use super::{SubsegmentNode, TrieNode};
use std::fmt::{Debug, Display, Formatter, Result};

fn children(trie_node: &TrieNode) -> Vec<(String, Option<&TrieNode>)> {
    let mut children = vec![];
    if let Some(route) = &trie_node.route.as_ref() {
        children.push((
            format!(" \x1B[1m{}\x1B[0m", route.handler_index.unwrap_or_default()),
            None,
        ));
    }

    for (key, value) in &trie_node.statics {
        children.push((key.to_string(), Some(value)));
    }

    if let Some(subsegments) = &trie_node.subsegment_matcher {
        for (prefix, node) in &subsegments.prefixes {
            children.push((format!("{prefix}:param"), Some(node)));
        }
        for (suffix, node) in &subsegments.suffixes {
            children.push((format!(":param{suffix}"), Some(node)));
        }

        for (degenerate, node) in &subsegments.degenerate {
            children.push((degenerate.to_string(), Some(node)))
        }
    }

    if let Some(params) = trie_node.param.as_deref() {
        children.push((":param".into(), Some(params)));
    }

    if let Some(route) = trie_node.wildcard.as_ref() {
        children.push((
            format!(
                "* \x1b[1m{}\x1B[0m",
                route.handler_index.unwrap_or_default()
            ),
            None,
        ));
    }

    children
}

pub(super) fn fmt_with_indent(
    trie_node: &TrieNode,
    f: &mut Formatter<'_>,
    indents: Vec<(usize, bool)>,
) -> Result {
    let children_vec = children(trie_node);
    let count = children_vec.len();
    for (i, (mut line, mut child_opt)) in children_vec.into_iter().enumerate() {
        while let Some(child) = child_opt {
            let next_children = children(child);

            if next_children.len() != 1 {
                break;
            }

            let (next_label, next_child) = &next_children[0];

            line.push_str(next_label);

            if next_child.is_none() {
                child_opt = None;
                break;
            }

            child_opt = *next_child;
        }

        if child_opt.is_some() {
            line.push_str(" ╾─╮");
        }

        for (indent_number, (indent, last)) in indents.iter().enumerate() {
            write!(
                f,
                "{}{}",
                " ".repeat(*indent),
                if indent_number == indents.len() - 1 {
                    if i == count - 1 {
                        "╰─╼ "
                    } else {
                        "├─╼ "
                    }
                } else {
                    if *last {
                        " "
                    } else {
                        "│"
                    }
                }
            )?;
        }

        writeln!(f, "{}", line.replace(":param", "\x1B[3m:param\x1B[23m"))?;

        if let Some(child) = child_opt {
            let mut indents = indents.clone();
            if let Some((_, last)) = indents.last_mut() {
                *last = i == count - 1;
            }
            indents.push((line.len() - 4, false));
            fmt_with_indent(child, f, indents)?;
        }
    }

    Ok(())
}

impl Display for TrieNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        writeln!(f, "\n... ╾─╮")?;
        fmt_with_indent(self, f, vec![(6, false)])
    }
}

impl Debug for SubsegmentNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let mut debug_struct = f.debug_struct("SubsegmentNode");
        for (prefix, node) in &self.prefixes {
            debug_struct.field(&format!("{prefix}."), node);
        }
        for (suffix, node) in &self.suffixes {
            debug_struct.field(&format!(".{suffix}"), node);
        }
        for (route, node) in &self.degenerate {
            debug_struct.field(&route.to_string(), node);
        }

        debug_struct.finish()
    }
}

impl Debug for TrieNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let mut debug_struct = f.debug_struct("TrieNode");
        debug_struct
            .field("paths", &self.paths)
            .field("minimum_length", &self.minimum_length)
            .field("states", &self.states);

        for (exact, node) in &self.statics {
            debug_struct.field(&format!("{exact:?}"), node);
        }

        if let Some(node) = &self.param {
            debug_struct.field("param", node);
        };

        if let Some(route) = &self.wildcard {
            debug_struct.field(
                "wildcard",
                &format_args!("{route} ({:?})", route.handler_index),
            );
        };

        if let Some(route) = &self.route {
            debug_struct.field(
                "route",
                &format_args!("{route} ({:?})", route.handler_index),
            );
        }

        if let Some(node) = &self.subsegment_matcher {
            debug_struct.field("subsegment_matcher", node);
        };
        debug_struct.finish()
    }
}
