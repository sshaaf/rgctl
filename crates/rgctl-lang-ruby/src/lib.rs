//! Ruby language plugin for rgctl (Tier 1).
//!
//! Honesty limits: no `$LOAD_PATH`/Bundler resolution, no method lookup or `super`
//! targets, dynamic `send`/`method_missing` marked `metadata.unresolved`, blocks/yield
//! CFG is conservative. See `docs/ruby-extract-honesty.md`.

use rgctl_registry::LanguageRegistry;
use std::sync::Arc;

mod ast_coverage;
mod plugin;
pub use plugin::RubyPlugin;

/// Register the Ruby language plugin.
pub fn register(registry: &mut LanguageRegistry) {
    registry.register_language_plugin(Arc::new(RubyPlugin::new().expect("init RubyPlugin")));
}
