//! PHP language plugin for rgctl (Tier 1).
//!
//! Honesty limits: no `include`/`require` resolution, no trait linearization or
//! `insteadof` alias precedence, no magic-method dispatch (`__call`, `__get`),
//! best-effort dynamic call targets (`metadata.unresolved`), no assignment tracking
//! for first-class callables beyond direct patterns, no Composer autoload graph.
//! Set `RGCTL_PHP_ONLY=1` to parse `.php` with `LANGUAGE_PHP_ONLY` (pure PHP, no HTML).

use rgctl_registry::LanguageRegistry;
use std::sync::Arc;

mod plugin;
pub use plugin::PhpPlugin;

/// Register the PHP language plugin.
pub fn register(registry: &mut LanguageRegistry) {
    registry.register_language_plugin(Arc::new(PhpPlugin::new().expect("init PhpPlugin")));
}
