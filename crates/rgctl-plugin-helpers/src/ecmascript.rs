//! Shared ECMAScript module and class-heritage extraction for JS/TS plugins.

use rgctl_plugin_api::{
    Relation, RelationType, SourceLocation, Symbol, SymbolType,
};
use std::path::Path;
use tree_sitter::Node;

/// Extract `Import` symbols from `import_statement` nodes.
pub fn extract_import_symbols(
    node: Node,
    source: &[u8],
    file_path: &str,
    language: &str,
) -> Vec<Symbol> {
    if node.kind() != "import_statement" {
        return Vec::new();
    }

    let text = node.utf8_text(source).unwrap_or("").trim().to_string();
    let is_type_only = text.starts_with("import type")
        || text.starts_with("import typeof")
        || node
            .children(&mut node.walk())
            .any(|c| c.kind() == "type" && c.utf8_text(source).ok() == Some("type"));

    let mut symbols = Vec::new();
    let mut module = None;

    if let Some(src) = node.child_by_field_name("source") {
        module = src
            .utf8_text(source)
            .ok()
            .map(|s| s.trim_matches(['"', '\'']).to_string());
    } else {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "string" {
                module = child
                    .utf8_text(source)
                    .ok()
                    .map(|s| s.trim_matches(['"', '\'']).to_string());
            }
        }
    }

    let name = module.clone().unwrap_or_else(|| text.clone());
    symbols.push(import_symbol(
        &name,
        node,
        file_path,
        language,
        is_type_only,
        module.as_deref(),
        None,
    ));

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_import_clause_children(child, source, &name, file_path, language, is_type_only, module.as_deref(), &mut symbols);
    }

    symbols
}

fn collect_import_clause_children(
    node: Node,
    source: &[u8],
    name: &str,
    file_path: &str,
    language: &str,
    is_type_only: bool,
    module: Option<&str>,
    symbols: &mut Vec<Symbol>,
) {
    match node.kind() {
        "named_imports" => {
            let mut ic = node.walk();
            for spec in node.children(&mut ic) {
                if spec.kind() == "import_specifier" {
                    let local = import_specifier_local_name(spec, source);
                    symbols.push(import_symbol(
                        &format!("{name}:{local}"),
                        spec,
                        file_path,
                        language,
                        is_type_only,
                        module,
                        Some(&local),
                    ));
                }
            }
        }
        "namespace_import" => {
            if let Some(local) = node
                .child_by_field_name("name")
                .or_else(|| find_child_kind(node, "identifier"))
                .and_then(|n| n.utf8_text(source).ok())
            {
                symbols.push(import_symbol(
                    &format!("{name}:* as {local}"),
                    node,
                    file_path,
                    language,
                    is_type_only,
                    module,
                    Some(local),
                ));
            }
        }
        "import_clause" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                collect_import_clause_children(
                    child, source, name, file_path, language, is_type_only, module, symbols,
                );
            }
        }
        _ => {}
    }
}

/// `Extends` edges from `class_heritage` / `extends_clause` on a class declaration.
pub fn extract_class_extends_relations(
    class_node: Node,
    source: &[u8],
    file_path: &Path,
    class_name: &str,
    language: &str,
    relations: &mut Vec<Relation>,
) {
    let heritage = find_child_kind(class_node, "class_heritage");
    let Some(heritage) = heritage else {
        return;
    };

    let extends = heritage
        .children(&mut heritage.walk())
        .find(|c| c.kind() == "extends_clause");
    let Some(extends) = extends else {
        return;
    };

    let mut cursor = extends.walk();
    for child in extends.children(&mut cursor) {
        if child.kind() == "identifier" || child.kind() == "type_identifier" {
            if let Ok(base) = child.utf8_text(source) {
                push_relation(
                    class_name,
                    base,
                    RelationType::Extends,
                    child,
                    file_path,
                    language,
                    relations,
                );
            }
        } else if let Some(value) = child.child_by_field_name("value") {
            if let Some(base) = type_name_from_node(value, source) {
                push_relation(
                    class_name,
                    &base,
                    RelationType::Extends,
                    child,
                    file_path,
                    language,
                    relations,
                );
            }
        }
    }
}

fn import_symbol(
    name: &str,
    node: Node,
    file_path: &str,
    language: &str,
    is_type_only: bool,
    module: Option<&str>,
    local: Option<&str>,
) -> Symbol {
    let mut meta = serde_json::json!({ "language": language });
    if is_type_only {
        meta["is_type_only"] = serde_json::Value::Bool(true);
    }
    if let Some(module) = module {
        meta["module"] = serde_json::Value::String(module.to_string());
    }
    if let Some(local) = local {
        meta["local"] = serde_json::Value::String(local.to_string());
    }
    Symbol {
        name: name.to_string(),
        symbol_type: SymbolType::Import,
        qualified_name: None,
        location: source_location(node, file_path),
        signature: None,
        return_type: None,
        parameters: vec![],
        fields: vec![],
        modifiers: vec![],
        documentation: None,
        metadata: meta,
    }
}

fn import_specifier_local_name(spec: Node, source: &[u8]) -> String {
    if let Some(alias) = spec.child_by_field_name("alias") {
        if let Ok(name) = alias.utf8_text(source) {
            return name.to_string();
        }
    }
    if let Some(name) = spec.child_by_field_name("name") {
        if let Ok(text) = name.utf8_text(source) {
            return text.to_string();
        }
    }
    spec.utf8_text(source)
        .unwrap_or("default")
        .split(" as ")
        .last()
        .unwrap_or("default")
        .trim()
        .to_string()
}

pub fn type_name_from_node(node: Node, source: &[u8]) -> Option<String> {
    match node.kind() {
        "type_identifier" | "identifier" | "predefined_type" => {
            node.utf8_text(source).ok().map(str::to_string)
        }
        "generic_type" | "nested_type_identifier" => node
            .child_by_field_name("name")
            .or_else(|| find_child_kind(node, "type_identifier"))
            .and_then(|n| n.utf8_text(source).ok().map(str::to_string)),
        "member_expression" => node
            .child_by_field_name("property")
            .and_then(|n| n.utf8_text(source).ok().map(str::to_string)),
        _ => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if let Some(name) = type_name_from_node(child, source) {
                    return Some(name);
                }
            }
            None
        }
    }
}

pub fn simple_type_name(raw: &str) -> String {
    raw.split('<')
        .next()
        .unwrap_or(raw)
        .trim()
        .rsplit('.')
        .next()
        .unwrap_or(raw)
        .to_string()
}

pub fn find_child_kind<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == kind {
            return Some(child);
        }
        if let Some(found) = find_child_kind(child, kind) {
            return Some(found);
        }
    }
    None
}

fn push_relation(
    from: &str,
    to: &str,
    relation_type: RelationType,
    node: Node,
    file_path: &Path,
    language: &str,
    relations: &mut Vec<Relation>,
) {
    relations.push(Relation {
        from: from.to_string(),
        to: to.to_string(),
        relation_type,
        location: source_location(node, &file_path.to_string_lossy()),
        metadata: serde_json::json!({ "language": language }),
        to_qualified_hint: None,
        to_type_hint: None,
    });
}

fn source_location(node: Node, file_path: &str) -> SourceLocation {
    SourceLocation {
        file: file_path.to_string(),
        start_line: node.start_position().row + 1,
        end_line: node.end_position().row + 1,
        start_column: node.start_position().column,
        end_column: node.end_position().column,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use tree_sitter::Parser;

    fn parse_ts(source: &str) -> tree_sitter::Tree {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
            .unwrap();
        parser.parse(source, None).unwrap()
    }

    #[test]
    fn test_named_import_symbol() {
        let source = "import { foo, bar as Baz } from 'lodash';";
        let tree = parse_ts(source);
        let root = tree.root_node();
        let import = root.named_child(0).unwrap();
        let symbols = extract_import_symbols(import, source.as_bytes(), "a.ts", "typescript");
        assert!(symbols.iter().all(|s| s.symbol_type == SymbolType::Import));
        assert!(symbols.len() >= 2);
    }

    #[test]
    fn test_type_only_import_metadata() {
        let source = "import type { Foo } from './foo';";
        let tree = parse_ts(source);
        let import = tree.root_node().named_child(0).unwrap();
        let symbols = extract_import_symbols(import, source.as_bytes(), "a.ts", "typescript");
        assert!(
            symbols
                .first()
                .and_then(|s| s.metadata.get("is_type_only"))
                .and_then(|v| v.as_bool())
                == Some(true)
        );
    }

    #[test]
    fn test_class_extends_relation() {
        let source = "class AppError extends Error {}";
        let tree = parse_ts(source);
        let class = tree.root_node().named_child(0).unwrap();
        let mut relations = Vec::new();
        extract_class_extends_relations(
            class,
            source.as_bytes(),
            Path::new("a.ts"),
            "AppError",
            "typescript",
            &mut relations,
        );
        assert!(
            relations
                .iter()
                .any(|r| r.relation_type == RelationType::Extends && r.to == "Error")
        );
    }
}
