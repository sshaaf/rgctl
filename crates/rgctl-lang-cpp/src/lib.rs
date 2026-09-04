//! Language plugin crate for rgctl — C++ (tree-sitter-cpp).
//!
//! ## Honesty limits
//!
//! - **Templates:** `template_declaration` symbols carry `metadata.is_template`; no
//!   per-instantiation matrix or SFINAE-aware resolution.
//! - **ADL / overloads:** `Calls` use best-effort callee names and `to_qualified_hint`;
//!   argument-dependent lookup and overload sets are not modeled.
//! - **Separate compilation:** symbols and relations are per translation unit; ODR
//!   merge across `.cpp`/`.hpp` pairs is not performed.
//! - **Friends / concepts:** not extracted as first-class graph semantics.

use rgctl_registry::LanguageRegistry;
use std::sync::Arc;

mod plugin;
pub use plugin::CppPlugin;

/// Register this language plugin.
pub fn register(registry: &mut LanguageRegistry) {
    registry.register_language_plugin(Arc::new(CppPlugin::new().expect("init CppPlugin")));
}
