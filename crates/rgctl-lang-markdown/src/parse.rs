//! Two-grammar markdown parse (block `LANGUAGE` + inline `INLINE_LANGUAGE`).

use rgctl_plugin_api::{Error, Result};
use std::collections::HashMap;
use std::path::Path;
use tree_sitter::{Node, Parser, Range, Tree};

/// Combined block tree plus per-inline-node inline trees.
pub struct ParsedMarkdown {
    /// Block-structure tree.
    pub block: Tree,
    inline_trees: Vec<Tree>,
    inline_indices: HashMap<usize, usize>,
}

impl ParsedMarkdown {
    /// Inline tree for a block `inline` or `pipe_table_cell` node.
    pub fn inline_tree(&self, node: Node<'_>) -> Option<&Tree> {
        let index = *self.inline_indices.get(&node.id())?;
        self.inline_trees.get(index)
    }
}

/// Parse markdown source with both official grammars.
pub fn parse_markdown(source: &[u8], file_path: &Path) -> Result<ParsedMarkdown> {
    let mut parser = Parser::new();
    let block_language = tree_sitter_md::LANGUAGE.into();
    let inline_language = tree_sitter_md::INLINE_LANGUAGE.into();

    parser
        .set_included_ranges(&[])
        .map_err(|e| Error::PluginError(format!("markdown included ranges: {e}")))?;
    parser
        .set_language(&block_language)
        .map_err(|e| Error::PluginError(format!("markdown block grammar: {e}")))?;

    let block = parser
        .parse(source, None)
        .ok_or_else(|| Error::ParseError {
            file: file_path.to_path_buf(),
            line: 0,
            message: "Failed to parse markdown (block grammar)".to_string(),
        })?;

    parser
        .set_language(&inline_language)
        .map_err(|e| Error::PluginError(format!("markdown inline grammar: {e}")))?;

    let mut inline_trees = Vec::new();
    let mut inline_indices = HashMap::new();
    let mut tree_cursor = block.walk();

    let mut i = 0usize;
    'outer: loop {
        let node = loop {
            if tree_cursor.node().kind() == "inline"
                || tree_cursor.node().kind() == "pipe_table_cell"
                || !tree_cursor.goto_first_child()
            {
                while !tree_cursor.goto_next_sibling() {
                    if !tree_cursor.goto_parent() {
                        break 'outer;
                    }
                }
            }
            let kind = tree_cursor.node().kind();
            if kind == "inline" || kind == "pipe_table_cell" {
                break tree_cursor.node();
            }
        };

        let mut range = node.range();
        let mut ranges = Vec::new();
        if tree_cursor.goto_first_child() {
            while tree_cursor.goto_next_sibling() {
                if !tree_cursor.node().is_named() {
                    continue;
                }
                let child_range = tree_cursor.node().range();
                ranges.push(Range {
                    start_byte: range.start_byte,
                    start_point: range.start_point,
                    end_byte: child_range.start_byte,
                    end_point: child_range.start_point,
                });
                range.start_byte = child_range.end_byte;
                range.start_point = child_range.end_point;
            }
            tree_cursor.goto_parent();
        }
        ranges.push(range);

        parser
            .set_included_ranges(&ranges)
            .map_err(|e| Error::PluginError(format!("markdown inline ranges: {e}")))?;
        let inline_tree = parser
            .parse(source, None)
            .ok_or_else(|| Error::ParseError {
                file: file_path.to_path_buf(),
                line: node.start_position().row + 1,
                message: "Failed to parse markdown (inline grammar)".to_string(),
            })?;
        inline_trees.push(inline_tree);
        inline_indices.insert(node.id(), i);
        i += 1;
    }
    drop(tree_cursor);

    Ok(ParsedMarkdown {
        block,
        inline_trees,
        inline_indices,
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn grammar_loads() {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_md::LANGUAGE.into())
            .expect("block LANGUAGE");
        parser
            .set_language(&tree_sitter_md::INLINE_LANGUAGE.into())
            .expect("INLINE_LANGUAGE");
    }
}
