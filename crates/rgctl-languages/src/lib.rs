//! Built-in Tier 1 language plugin registration.

use rgctl_registry::LanguageRegistry;

/// Register all Tier 1 language plugins.
pub fn register_languages(registry: &mut LanguageRegistry) {
    rgctl_lang_rust::register(registry);
    rgctl_lang_python::register(registry);
    rgctl_lang_javascript::register(registry);
    rgctl_lang_typescript::register(registry);
    rgctl_lang_go::register(registry);
    rgctl_lang_java::register(registry);
    rgctl_lang_csharp::register(registry);
    rgctl_lang_c::register(registry);
    rgctl_lang_cpp::register(registry);
    rgctl_lang_markdown::register(registry);
}

/// Default registry with config formats and all built-in languages.
pub fn default_registry() -> LanguageRegistry {
    let mut registry = LanguageRegistry::with_config_formats();
    register_languages(&mut registry);
    registry
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn default_registry_registers_markdown() {
        let registry = default_registry();
        assert!(registry.has_plugin("markdown"));
        let plugin = registry
            .get_plugin_for_file(Path::new("docs/guide.md"))
            .expect("md plugin");
        assert_eq!(plugin.language_id(), "markdown");
        let mdx = registry
            .get_plugin_for_file(Path::new("docs/overview.mdx"))
            .expect("mdx plugin");
        assert_eq!(mdx.language_id(), "markdown");
    }

    #[test]
    fn default_registry_can_process_markdown_files() {
        let registry = default_registry();
        assert!(registry.can_process_file(Path::new("README.md")));
        assert!(registry.can_process_file(Path::new("notes/page.mdx")));
        let langs = registry.supported_languages();
        assert!(langs.iter().any(|l| l == "markdown"));
    }

    #[test]
    fn markdown_not_treated_as_yaml_config() {
        let registry = default_registry();
        assert!(
            registry
                .get_config_plugin_for_file(Path::new("readme.md"))
                .is_err()
        );
    }
}
