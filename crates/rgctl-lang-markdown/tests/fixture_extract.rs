//! Integration tests against the checked-in `tests/fixtures/markdown-context` corpus.

use rgctl_lang_markdown::MarkdownPlugin;
use rgctl_plugin_api::{LanguagePlugin, RelationType, SymbolType};
use std::fs;
use std::path::{Path, PathBuf};

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/markdown-context")
}

fn extract_fixture(rel: &str) -> rgctl_plugin_api::ExtractAllResult {
    let root = fixture_root();
    let path = root.join(rel);
    let source = fs::read(&path).expect("read fixture");
    let plugin = MarkdownPlugin::new().expect("plugin");
    plugin.extract_all(&path, &source).expect("extract_all")
}

#[test]
fn fixture_guide_checkout_flow_section_body() {
    let symbols = extract_fixture("docs/guide.md").symbols;
    let checkout = symbols
        .iter()
        .find(|s| s.name == "Checkout Flow")
        .expect("Checkout Flow heading");
    let body = checkout
        .metadata
        .get("body_text")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    assert!(
        body.contains("End-to-end checkout"),
        "checkout intro body: {body}"
    );
    assert!(
        !body.contains("## Cart"),
        "section body must not include nested headings"
    );
    assert!(checkout.metadata.get("body_hash").is_some());
}

#[test]
fn fixture_guide_fenced_block_body_text() {
    let symbols = extract_fixture("docs/guide.md").symbols;
    let block = symbols
        .iter()
        .find(|s| {
            s.metadata.get("kind") == Some(&serde_json::json!("code_block"))
                && s.metadata.get("language").and_then(|v| v.as_str()) == Some("java")
        })
        .expect("java code block");
    assert!(
        block
            .metadata
            .get("body_text")
            .and_then(|v| v.as_str())
            .is_some_and(|b| b.contains("cart.validate")),
        "fence body_text: {:?}",
        block.metadata.get("body_text")
    );
}

#[test]
fn fixture_readme_frontmatter_values() {
    let symbols = extract_fixture("README.md").symbols;
    let author = symbols
        .iter()
        .find(|s| s.name == "metadata.author")
        .expect("metadata.author");
    assert_eq!(
        author.metadata.get("value").and_then(|v| v.as_str()),
        Some("rgctl-fixture")
    );
    assert!(
        author
            .metadata
            .get("body_text")
            .and_then(|v| v.as_str())
            .is_some_and(|v| v.contains("rgctl-fixture"))
    );
}

#[test]
fn fixture_guide_checkout_flow_heading() {
    let out = extract_fixture("docs/guide.md");
    let symbols = &out.symbols;
    let relations = &out.relations;
    let checkout = symbols
        .iter()
        .find(|s| s.name == "Checkout Flow")
        .expect("Checkout Flow heading");
    assert_eq!(checkout.symbol_type, SymbolType::Module);
    assert!(
        checkout
            .qualified_name
            .as_ref()
            .is_some_and(|qn| qn.ends_with("docs/guide.md#checkout-flow")),
        "qn: {:?}",
        checkout.qualified_name
    );
    assert!(
        relations.iter().any(|r| {
            r.relation_type == RelationType::References
                && r.to.ends_with("adr.md#payments")
                && r.to_type_hint.as_deref() == Some("module")
                && r.from.ends_with("#checkout-flow")
        }),
        "payments ADR link from Checkout Flow"
    );
    assert!(
        relations.iter().any(|r| {
            r.relation_type == RelationType::References
                && r.to.ends_with("adr.md")
                && !r.to.contains('#')
                && r.to_type_hint.as_deref() == Some("file")
        }),
        "adr.md file link"
    );
    assert!(
        relations.iter().any(|r| {
            r.relation_type == RelationType::References
                && r.to.ends_with("CheckoutService.java")
                && r.to_type_hint.as_deref() == Some("file")
        }),
        "Java file link"
    );
}

#[test]
fn fixture_guide_nested_cart_contains() {
    let relations = extract_fixture("docs/guide.md").relations;
    assert!(
        relations.iter().any(|r| {
            r.relation_type == RelationType::Defines
                && r.from.ends_with("#checkout-flow")
                && r.to.ends_with("#cart")
        }),
        "Checkout Flow CONTAINS Cart"
    );
}

#[test]
fn fixture_readme_yaml_frontmatter() {
    let symbols = extract_fixture("README.md").symbols;
    assert!(
        symbols.iter().any(|s| {
            s.name == "metadata.author"
                && s.symbol_type == SymbolType::Variable
                && s.qualified_name
                    .as_ref()
                    .is_some_and(|qn| qn.ends_with("README.md#fm.metadata.author"))
        }),
        "README frontmatter metadata.author"
    );
    assert!(
        symbols.iter().any(|s| {
            s.name == "metadata.team"
                && s.qualified_name
                    .as_ref()
                    .is_some_and(|qn| qn.ends_with("README.md#fm.metadata.team"))
        }),
        "README frontmatter metadata.team"
    );
}

#[test]
fn fixture_validation_rules_section_links_use_child_heading_as_from() {
    let relations = extract_fixture("docs/guide.md").relations;
    let overview = relations
        .iter()
        .find(|r| r.to.ends_with("adr.md#overview"))
        .expect("overview link from validation rules section");
    assert!(
        overview.from.ends_with("#validation-rules"),
        "link after ### Validation rules must use that heading as from, got {}",
        overview.from
    );
}

#[test]
fn fixture_guide_has_fenced_code_block_symbol() {
    let symbols = extract_fixture("docs/guide.md").symbols;
    assert!(
        symbols.iter().any(|s| {
            s.metadata.get("kind") == Some(&serde_json::json!("code_block"))
                && s.metadata.get("language").and_then(|v| v.as_str()) == Some("java")
        }),
        "java fenced block in guide.md"
    );
}

#[test]
fn fixture_guide_external_link_has_no_reference_edge() {
    let out = extract_fixture("docs/guide.md");
    let symbols = &out.symbols;
    let relations = &out.relations;
    assert!(
        symbols.iter().any(|s| {
            s.metadata.get("url").and_then(|v| v.as_str()) == Some("https://stripe.com/docs/api")
        }),
        "stripe link symbol"
    );
    assert!(
        !relations.iter().any(|r| r.to.contains("stripe.com")),
        "external URL must not create REFERENCES"
    );
}

#[test]
fn fixture_adr_nested_headings_define_tree() {
    let relations = extract_fixture("docs/adr.md").relations;
    assert!(
        relations.iter().any(|r| {
            r.relation_type == RelationType::Defines
                && r.from.ends_with("adr.md#architecture-decisions")
                && r.to.ends_with("adr.md#payments")
        }),
        "ADR root CONTAINS Payments via Defines"
    );
}

#[test]
fn fixture_adr_links_back_to_guide() {
    let relations = extract_fixture("docs/adr.md").relations;
    assert!(
        relations.iter().any(|r| {
            r.relation_type == RelationType::References
                && r.to.ends_with("guide.md#checkout-flow")
                && r.to_type_hint.as_deref() == Some("module")
        }),
        "adr links to guide checkout-flow fragment"
    );
}

#[test]
fn fixture_mdx_indexed_with_same_plugin() {
    let symbols = extract_fixture("docs/overview.mdx").symbols;
    assert!(
        symbols.iter().any(|s| s.name == "MDX overview"),
        "mdx heading extracted"
    );
}

#[test]
fn plugin_handles_mdx_path() {
    let plugin = MarkdownPlugin::new().expect("plugin");
    assert!(plugin.can_handle(Path::new("notes/page.mdx")));
    assert_eq!(plugin.language_id(), "markdown");
    assert!(plugin.file_extensions().contains(&"mdx"));
}
