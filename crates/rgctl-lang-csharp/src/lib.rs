//! Language plugin crate for rgctl
//!
//! ## Honesty limits
//!
//! - **DI ambiguity:** Constructor injection (`IServiceCollection`, `IHostApplicationBuilder`)
//!   is not resolved at discover time — field/property type hints are best-effort from
//!   declarations in the same compilation unit. See [issue #75](https://github.com/sshaaf/rgctl/issues/75).
//! - **`global using`:** Project-wide usings are invisible when indexing a single file.
//! - **Source generators / partial types:** Cross-file partial class merge is out of scope.

use rgctl_registry::LanguageRegistry;
use std::sync::Arc;

mod plugin;
pub use plugin::CSharpPlugin;

/// Register this language plugin.
pub fn register(registry: &mut LanguageRegistry) {
    registry.register_language_plugin(Arc::new(CSharpPlugin::new().expect("init CSharpPlugin")));
}
