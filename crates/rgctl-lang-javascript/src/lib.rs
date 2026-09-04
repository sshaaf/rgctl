//! Language plugin crate for rgctl
//!
//! ## Honesty limits
//!
//! - **Bundler aliases:** `import from '@/foo'` and package.json `exports` are not resolved.
//! - **Dynamic `import()` / `require(expr)`:** Only static string literal `require('mod')` is indexed.
//! - **Re-exports:** Export metadata records the statement text; chained re-export targets are not followed.

use rgctl_registry::LanguageRegistry;
use std::sync::Arc;

mod plugin;
pub use plugin::JavaScriptPlugin;

/// Register this language plugin.
pub fn register(registry: &mut LanguageRegistry) {
    registry.register_language_plugin(Arc::new(
        JavaScriptPlugin::new().expect("init JavaScriptPlugin"),
    ));
}
