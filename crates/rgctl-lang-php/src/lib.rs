//! PHP language plugin for rgctl (Tier 1).
//!
//! Honesty limits: no `include`/`require` resolution, no trait linearization,
//! no magic-method dispatch (`__call`, `__get`), best-effort dynamic call targets.

use rgctl_registry::LanguageRegistry;
use std::sync::Arc;

mod plugin;
pub use plugin::PhpPlugin;

/// Register the PHP language plugin.
pub fn register(registry: &mut LanguageRegistry) {
    registry.register_language_plugin(Arc::new(PhpPlugin::new().expect("init PhpPlugin")));
}
