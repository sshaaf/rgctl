//! Language plugin crate for rgctl
//!
//! ## Honesty limits
//!
//! - **`tsconfig` paths / `baseUrl`:** Not resolved — imports are per-file only.
//! - **Type-only erasure:** Types inform hints (`to_type_hint`) but are not a full type checker.
//! - **Decorator factories:** `AnnotatedWith` targets the outer decorator identifier.

use rgctl_registry::LanguageRegistry;
use std::sync::Arc;

mod plugin;
pub use plugin::TypeScriptPlugin;

/// Register this language plugin.
pub fn register(registry: &mut LanguageRegistry) {
    registry.register_language_plugin(Arc::new(
        TypeScriptPlugin::new().expect("init TypeScriptPlugin"),
    ));
}
