//! Markdown / MDX context files — graph indexing only (no CFG/PDG).

use std::path::Path;

const MARKUP_CFG_HINT: &str = "Markdown context files (.md/.mdx) are indexed for GQL only (headings, links). \
Use `rgctl -f json gql` with `n.kind = 'heading'` — see docs/markdown-context.md.";

/// True when the path is handled by `rgbuilder-lang-markdown` (not CFG-capable).
pub fn is_markup_context_path(path: &Path) -> bool {
    matches!(path.extension().and_then(|e| e.to_str()), Some(ext) if ext.eq_ignore_ascii_case("md") || ext.eq_ignore_ascii_case("mdx"))
}

/// Error message when a CFG/PDG command is invoked on a markup file.
pub fn markup_context_unsupported(command: &str, path: &Path) -> Option<String> {
    if is_markup_context_path(path) {
        Some(format!("{command}: {MARKUP_CFG_HINT}"))
    } else {
        None
    }
}
