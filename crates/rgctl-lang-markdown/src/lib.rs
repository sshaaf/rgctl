//! Markdown language support via tree-sitter-md.

mod extract;
mod parse;
mod plugin;
mod slug;

use rgctl_registry::LanguageRegistry;
use std::sync::Arc;

pub use plugin::MarkdownPlugin;

/// Register the markdown language plugin.
pub fn register(registry: &mut LanguageRegistry) {
    registry.register_language_plugin(Arc::new(
        MarkdownPlugin::new().expect("init MarkdownPlugin"),
    ));
}
