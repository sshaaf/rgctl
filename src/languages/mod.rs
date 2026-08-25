//! Language plugin registry wrapper (languages live in `rgctl-lang-*` crates).

pub use rgctl_config_formats as config;
pub use rgctl_lang_runtime as generic;
pub use rgctl_plugin_api as plugin_trait;
pub use rgctl_plugin_helpers as extraction;
pub use rgctl_registry::{plugin_abi, plugin_loader};

pub mod registry;

pub use registry::LanguageRegistry;

/// No-op alias; wiring happens in [`registry::ensure_initialized`].
pub fn ensure_registry_initialized() {
    registry::ensure_initialized();
}
