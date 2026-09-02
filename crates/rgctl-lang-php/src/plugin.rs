//! PHP language plugin using Tree-sitter.

use rgctl_plugin_api::{PHP_CALL_KINDS, *};
use rgctl_plugin_helpers::ComplexityCalculator;
use std::path::Path;
use tree_sitter::{Node, Parser};

pub const PHP_FUNCTION_KINDS: &[&str] = &[
    "function_definition",
    "method_declaration",
    "arrow_function",
    "anonymous_function",
];

const PHP_COMPLEXITY_BRANCH_KINDS: &[&str] = &[
    "if_statement",
    "else_if_clause",
    "while_statement",
    "for_statement",
    "foreach_statement",
    "switch_statement",
    "match_expression",
    "catch_clause",
];

/// PHP Tier 1 language plugin.
pub struct PhpPlugin {
    _parser: Parser,
}

impl PhpPlugin {
    /// Create a new PHP plugin.
    pub fn new() -> Result<Self> {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_php::LANGUAGE_PHP.into())
            .map_err(|e| Error::PluginError(format!("Failed to set PHP grammar: {e}")))?;
        Ok(Self { _parser: parser })
    }

    fn parse(&self, file_path: &Path, source: &[u8]) -> Result<tree_sitter::Tree> {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_php::LANGUAGE_PHP.into())
            .map_err(|e| Error::PluginError(format!("Failed to set PHP grammar: {e}")))?;
        parser.parse(source, None).ok_or_else(|| Error::ParseError {
            file: file_path.to_path_buf(),
            line: 0,
            message: "Failed to parse PHP source".to_string(),
        })
    }

    fn symbols_from_tree(
        &self,
        root: Node,
        source: &[u8],
        file_path: &Path,
    ) -> Result<Vec<Symbol>> {
        let mut symbols = Vec::new();
        let mut namespace = None;
        let file_path_str = file_path.to_string_lossy();
        self.traverse_symbols(
            root,
            source,
            &file_path_str,
            &mut namespace,
            &mut symbols,
        )?;
        Ok(symbols)
    }

    fn traverse_symbols(
        &self,
        node: Node,
        source: &[u8],
        file_path: &str,
        namespace: &mut Option<String>,
        symbols: &mut Vec<Symbol>,
    ) -> Result<()> {
        if node.kind() == "namespace_definition" {
            if let Some(ns) = node
                .child_by_field_name("name")
                .and_then(|n| normalize_namespace(n, source))
            {
                *namespace = Some(ns);
            }
        }

        match node.kind() {
            "function_definition" => {
                if let Some(sym) =
                    self.extract_function(node, source, file_path, namespace.as_deref())?
                {
                    symbols.push(sym);
                }
            }
            "method_declaration" => {
                if self.find_enclosing_class_name(node, source).is_none() {
                    if let Some(sym) =
                        self.extract_method(node, source, file_path, namespace.as_deref(), None)?
                    {
                        symbols.push(sym);
                    }
                }
            }
            "arrow_function" | "anonymous_function" => {
                if let Some(sym) =
                    self.extract_anonymous_function(node, source, file_path, namespace.as_deref())?
                {
                    symbols.push(sym);
                }
            }
            "class_declaration" => {
                symbols.push(self.extract_class(node, source, file_path, namespace.as_deref())?);
                self.traverse_class_members(node, source, file_path, namespace.as_deref(), symbols)?;
            }
            "interface_declaration" => {
                symbols.push(self.extract_interface(node, source, file_path, namespace.as_deref())?);
            }
            "trait_declaration" => {
                symbols.push(self.extract_trait(node, source, file_path, namespace.as_deref())?);
            }
            "enum_declaration" => {
                symbols.push(self.extract_enum(node, source, file_path, namespace.as_deref())?);
            }
            _ => {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    self.traverse_symbols(child, source, file_path, namespace, symbols)?;
                }
            }
        }
        Ok(())
    }

    fn traverse_class_members(
        &self,
        class_node: Node,
        source: &[u8],
        file_path: &str,
        namespace: Option<&str>,
        symbols: &mut Vec<Symbol>,
    ) -> Result<()> {
        let class_name = type_name(class_node, source).unwrap_or_default();
        let qclass = qualify(namespace, &class_name);
        let body = class_node
            .child_by_field_name("body")
            .or_else(|| find_child_kind(class_node, "declaration_list"));
        let Some(body) = body else {
            return Ok(());
        };

        let mut cursor = body.walk();
        for child in body.children(&mut cursor) {
            if child.kind() == "method_declaration" {
                if let Some(sym) = self.extract_method(
                    child,
                    source,
                    file_path,
                    namespace,
                    Some(qclass.as_str()),
                )? {
                    symbols.push(sym);
                }
            }
        }
        Ok(())
    }

    fn extract_function(
        &self,
        node: Node,
        source: &[u8],
        file_path: &str,
        namespace: Option<&str>,
    ) -> Result<Option<Symbol>> {
        let name = node_name(node, source).ok_or_else(|| Error::ParseError {
            file: file_path.into(),
            line: node.start_position().row + 1,
            message: "Function missing name".to_string(),
        })?;
        let parameters = self.extract_parameters(node, source)?;
        let return_type = node
            .child_by_field_name("return_type")
            .and_then(|n| n.utf8_text(source).ok())
            .map(str::to_string);
        let qualified_name = qualify(namespace, &name);
        Ok(Some(Symbol {
            name,
            symbol_type: SymbolType::Function,
            qualified_name: Some(qualified_name),
            location: source_location(node, file_path),
            signature: Some(first_line(node, source)),
            return_type,
            parameters,
            fields: vec![],
            modifiers: visibility_modifiers(node, source),
            documentation: None,
            metadata: serde_json::json!({ "language": "php" }),
        }))
    }

    fn extract_method(
        &self,
        node: Node,
        source: &[u8],
        file_path: &str,
        namespace: Option<&str>,
        enclosing_class: Option<&str>,
    ) -> Result<Option<Symbol>> {
        let name = node_name(node, source).ok_or_else(|| Error::ParseError {
            file: file_path.into(),
            line: node.start_position().row + 1,
            message: "Method missing name".to_string(),
        })?;

        let class_name = enclosing_class
            .map(str::to_string)
            .or_else(|| self.find_enclosing_class_name(node, source));
        let parameters = self.extract_parameters(node, source)?;
        let return_type = node
            .child_by_field_name("return_type")
            .and_then(|n| n.utf8_text(source).ok())
            .map(str::to_string);

        let is_constructor = name == "__construct";
        let qualified_name = if is_constructor {
            class_name
                .as_ref()
                .map(|c| format!("{c}.<init>"))
                .or_else(|| Some(format!("{}.<init>", name)))
        } else {
            class_name
                .as_ref()
                .map(|c| format!("{c}.{name}"))
                .or_else(|| Some(qualify(namespace, &name)))
        };

        let mut metadata = serde_json::json!({ "language": "php" });
        if is_constructor {
            metadata["is_constructor"] = serde_json::json!(true);
        }

        Ok(Some(Symbol {
            name: name.clone(),
            symbol_type: SymbolType::Function,
            qualified_name,
            location: source_location(node, file_path),
            signature: Some(first_line(node, source)),
            return_type,
            parameters,
            fields: vec![],
            modifiers: visibility_modifiers(node, source),
            documentation: None,
            metadata,
        }))
    }

    fn extract_anonymous_function(
        &self,
        node: Node,
        source: &[u8],
        file_path: &str,
        namespace: Option<&str>,
    ) -> Result<Option<Symbol>> {
        let line = node.start_position().row + 1;
        let name = if node.kind() == "arrow_function" {
            format!("$arrow${line}")
        } else {
            format!("$anonymous${line}")
        };
        let parameters = self.extract_parameters(node, source)?;
        let return_type = node
            .child_by_field_name("return_type")
            .and_then(|n| n.utf8_text(source).ok())
            .map(str::to_string);
        Ok(Some(Symbol {
            name: name.clone(),
            symbol_type: SymbolType::Function,
            qualified_name: Some(qualify(namespace, &name)),
            location: source_location(node, file_path),
            signature: Some(first_line(node, source)),
            return_type,
            parameters,
            fields: vec![],
            modifiers: vec![],
            documentation: None,
            metadata: serde_json::json!({ "language": "php", "is_anonymous": true }),
        }))
    }

    fn extract_class(
        &self,
        node: Node,
        source: &[u8],
        file_path: &str,
        namespace: Option<&str>,
    ) -> Result<Symbol> {
        let name = type_name(node, source).ok_or_else(|| Error::ParseError {
            file: file_path.into(),
            line: node.start_position().row + 1,
            message: "Class missing name".to_string(),
        })?;
        let fields = self.extract_type_fields(node, source)?;
        Ok(Symbol {
            name: name.clone(),
            symbol_type: SymbolType::Class,
            qualified_name: Some(qualify(namespace, &name)),
            location: source_location(node, file_path),
            signature: None,
            return_type: None,
            parameters: vec![],
            fields,
            modifiers: visibility_modifiers(node, source),
            documentation: None,
            metadata: serde_json::json!({ "language": "php" }),
        })
    }

    fn extract_interface(
        &self,
        node: Node,
        source: &[u8],
        file_path: &str,
        namespace: Option<&str>,
    ) -> Result<Symbol> {
        let name = type_name(node, source).ok_or_else(|| Error::ParseError {
            file: file_path.into(),
            line: node.start_position().row + 1,
            message: "Interface missing name".to_string(),
        })?;
        Ok(Symbol {
            name: name.clone(),
            symbol_type: SymbolType::Interface,
            qualified_name: Some(qualify(namespace, &name)),
            location: source_location(node, file_path),
            signature: None,
            return_type: None,
            parameters: vec![],
            fields: vec![],
            modifiers: visibility_modifiers(node, source),
            documentation: None,
            metadata: serde_json::json!({ "language": "php" }),
        })
    }

    fn extract_trait(
        &self,
        node: Node,
        source: &[u8],
        file_path: &str,
        namespace: Option<&str>,
    ) -> Result<Symbol> {
        let name = type_name(node, source).ok_or_else(|| Error::ParseError {
            file: file_path.into(),
            line: node.start_position().row + 1,
            message: "Trait missing name".to_string(),
        })?;
        Ok(Symbol {
            name: name.clone(),
            symbol_type: SymbolType::Class,
            qualified_name: Some(qualify(namespace, &name)),
            location: source_location(node, file_path),
            signature: None,
            return_type: None,
            parameters: vec![],
            fields: vec![],
            modifiers: visibility_modifiers(node, source),
            documentation: None,
            metadata: serde_json::json!({ "language": "php", "is_trait": true }),
        })
    }

    fn extract_enum(
        &self,
        node: Node,
        source: &[u8],
        file_path: &str,
        namespace: Option<&str>,
    ) -> Result<Symbol> {
        let name = type_name(node, source).ok_or_else(|| Error::ParseError {
            file: file_path.into(),
            line: node.start_position().row + 1,
            message: "Enum missing name".to_string(),
        })?;
        Ok(Symbol {
            name: name.clone(),
            symbol_type: SymbolType::Enum,
            qualified_name: Some(qualify(namespace, &name)),
            location: source_location(node, file_path),
            signature: None,
            return_type: None,
            parameters: vec![],
            fields: vec![],
            modifiers: visibility_modifiers(node, source),
            documentation: None,
            metadata: serde_json::json!({ "language": "php" }),
        })
    }

    fn extract_type_fields(&self, type_node: Node, source: &[u8]) -> Result<Vec<Field>> {
        let mut fields = Vec::new();
        let body = type_node
            .child_by_field_name("body")
            .or_else(|| find_child_kind(type_node, "declaration_list"));
        let Some(body) = body else {
            return Ok(fields);
        };

        let mut body_cursor = body.walk();
        for child in body.children(&mut body_cursor) {
            if child.kind() == "property_declaration" {
                let field_type = child
                    .child_by_field_name("type")
                    .and_then(|n| n.utf8_text(source).ok())
                    .map(str::to_string);
                let visibility = visibility_modifiers(child, source).join(" ");
                let vis = if visibility.is_empty() {
                    None
                } else {
                    Some(visibility)
                };
                let mut elem_cursor = child.walk();
                for elem in child.children(&mut elem_cursor) {
                    if elem.kind() != "property_element" {
                        continue;
                    }
                    if let Some(name_node) = elem.child_by_field_name("name") {
                        if let Some(name) = variable_text(name_node, source) {
                            fields.push(Field {
                                name,
                                field_type: field_type.clone(),
                                visibility: vis.clone(),
                            });
                        }
                    }
                }
            }
        }

        let body = type_node
            .child_by_field_name("body")
            .or_else(|| find_child_kind(type_node, "declaration_list"));
        if let Some(body) = body {
            let mut body_cursor = body.walk();
            for child in body.children(&mut body_cursor) {
                if child.kind() == "method_declaration" {
                    if node_name(child, source).as_deref() == Some("__construct") {
                        if let Some(params) = child.child_by_field_name("parameters") {
                            self.collect_promoted_fields(params, source, &mut fields)?;
                        }
                    }
                }
            }
        }

        Ok(fields)
    }

    fn collect_promoted_fields(
        &self,
        params_node: Node,
        source: &[u8],
        fields: &mut Vec<Field>,
    ) -> Result<()> {
        let mut cursor = params_node.walk();
        for child in params_node.children(&mut cursor) {
            if child.kind() != "property_promotion_parameter" {
                continue;
            }
            let name = child
                .child_by_field_name("name")
                .and_then(|n| variable_text(n, source));
            let field_type = child
                .child_by_field_name("type")
                .and_then(|n| n.utf8_text(source).ok())
                .map(str::to_string);
            let visibility = visibility_modifiers(child, source).join(" ");
            if let Some(name) = name {
                fields.push(Field {
                    name,
                    field_type,
                    visibility: if visibility.is_empty() {
                        None
                    } else {
                        Some(visibility)
                    },
                });
            }
        }
        Ok(())
    }

    fn extract_parameters(&self, node: Node, source: &[u8]) -> Result<Vec<Parameter>> {
        let mut parameters = Vec::new();
        let Some(params_node) = node.child_by_field_name("parameters") else {
            return Ok(parameters);
        };

        let mut cursor = params_node.walk();
        for child in params_node.children(&mut cursor) {
            match child.kind() {
                "simple_parameter" | "property_promotion_parameter" | "optional_parameter"
                | "variadic_parameter" => {
                    let name = child
                        .child_by_field_name("name")
                        .and_then(|n| variable_text(n, source));
                    let param_type = child
                        .child_by_field_name("type")
                        .and_then(|n| n.utf8_text(source).ok())
                        .map(str::to_string);
                    let default_value = child
                        .child_by_field_name("default_value")
                        .and_then(|n| n.utf8_text(source).ok())
                        .map(str::to_string);
                    if let Some(name) = name {
                        parameters.push(Parameter {
                            name,
                            param_type,
                            default_value,
                        });
                    }
                }
                _ => {}
            }
        }
        Ok(parameters)
    }

    fn find_enclosing_class_name(&self, node: Node, source: &[u8]) -> Option<String> {
        let mut current = node;
        while let Some(parent) = current.parent() {
            if parent.kind() == "class_declaration" {
                return type_name(parent, source);
            }
            current = parent;
        }
        None
    }

    fn relations_from_tree(
        &self,
        root: Node,
        source: &[u8],
        file_path: &Path,
        symbols: &[Symbol],
    ) -> Result<Vec<Relation>> {
        let mut relations = Vec::new();
        walk_calls(
            root,
            source,
            file_path,
            symbols,
            PHP_CALL_KINDS,
            "php",
            &mut relations,
        );
        self.extract_inheritance(root, source, file_path, &mut relations)?;
        Ok(relations)
    }

    fn extract_inheritance(
        &self,
        node: Node,
        source: &[u8],
        file_path: &Path,
        relations: &mut Vec<Relation>,
    ) -> Result<()> {
        if node.kind() == "class_declaration" {
            let name = type_name(node, source).unwrap_or_default();
            if !name.is_empty() {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    match child.kind() {
                        "base_clause" => {
                            for base in type_refs(child, source) {
                                relations.push(relation(
                                    &name,
                                    &base,
                                    RelationType::Extends,
                                    child,
                                    file_path,
                                ));
                            }
                        }
                        "class_interface_clause" => {
                            for iface in type_refs(child, source) {
                                relations.push(relation(
                                    &name,
                                    &iface,
                                    RelationType::Implements,
                                    child,
                                    file_path,
                                ));
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.extract_inheritance(child, source, file_path, relations)?;
        }
        Ok(())
    }
}

impl Default for PhpPlugin {
    fn default() -> Self {
        Self::new().expect("Failed to create PhpPlugin")
    }
}

impl LanguagePlugin for PhpPlugin {
    fn language_id(&self) -> &str {
        "php"
    }

    fn file_extensions(&self) -> Vec<&str> {
        vec!["php"]
    }

    fn grammar(&self) -> Option<tree_sitter::Language> {
        Some(tree_sitter_php::LANGUAGE_PHP.into())
    }

    fn extract_symbols(&self, file_path: &Path, source: &[u8]) -> Result<Vec<Symbol>> {
        let tree = self.parse(file_path, source)?;
        self.symbols_from_tree(tree.root_node(), source, file_path)
    }

    fn extract_relations(
        &self,
        file_path: &Path,
        source: &[u8],
        symbols: &[Symbol],
    ) -> Result<Vec<Relation>> {
        let tree = self.parse(file_path, source)?;
        self.relations_from_tree(tree.root_node(), source, file_path, symbols)
    }

    fn extract_all(&self, file_path: &Path, source: &[u8]) -> Result<ExtractAllResult> {
        let tree = self.parse(file_path, source)?;
        let root = tree.root_node();
        let symbols = self.symbols_from_tree(root, source, file_path)?;
        let relations = self.relations_from_tree(root, source, file_path, &symbols)?;
        Ok(ExtractAllResult::from_parts(symbols, relations))
    }

    fn calculate_complexity(
        &self,
        symbol: &Symbol,
        source: &[u8],
    ) -> Result<Option<ComplexityMetrics>> {
        if symbol.symbol_type != SymbolType::Function {
            return Ok(None);
        }

        let tree = self.parse(Path::new(&symbol.location.file), source)?;
        let target_line = symbol.location.start_line.saturating_sub(1);
        let kinds: Vec<&str> = PHP_FUNCTION_KINDS.to_vec();

        if let Some(func_node) = find_node_at_line(tree.root_node(), target_line, &kinds) {
            Ok(Some(ComplexityMetrics {
                cyclomatic: ComplexityCalculator::cyclomatic(func_node, PHP_COMPLEXITY_BRANCH_KINDS),
                cognitive: ComplexityCalculator::cognitive(func_node, PHP_COMPLEXITY_BRANCH_KINDS),
                loc: ComplexityCalculator::loc(func_node),
                parameters: symbol.parameters.len(),
                nesting_depth: ComplexityCalculator::nesting_depth(
                    func_node,
                    &["compound_statement", "declaration_list"],
                ),
                returns: ComplexityCalculator::return_count(func_node, "return_statement"),
            }))
        } else {
            Ok(None)
        }
    }
}

fn find_node_at_line<'a>(node: Node<'a>, line: usize, kinds: &[&str]) -> Option<Node<'a>> {
    if kinds.contains(&node.kind()) && node.start_position().row == line {
        return Some(node);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(found) = find_node_at_line(child, line, kinds) {
            return Some(found);
        }
    }
    None
}

fn qualify(namespace: Option<&str>, name: &str) -> String {
    match namespace {
        Some(ns) if !ns.is_empty() => format!("{ns}\\{name}"),
        _ => name.to_string(),
    }
}

fn normalize_namespace(node: Node, source: &[u8]) -> Option<String> {
    let text = node.utf8_text(source).ok()?;
    Some(text.trim().trim_start_matches('\\').to_string())
}

fn type_name(node: Node, source: &[u8]) -> Option<String> {
    node.child_by_field_name("name")
        .and_then(|n| n.utf8_text(source).ok())
        .map(|s| s.trim_start_matches('\\').to_string())
}

fn node_name(node: Node, source: &[u8]) -> Option<String> {
    node.child_by_field_name("name")
        .and_then(|n| n.utf8_text(source).ok())
        .map(str::to_string)
}

fn variable_text(node: Node, source: &[u8]) -> Option<String> {
    node.utf8_text(source)
        .ok()
        .map(|s| s.trim_start_matches('$').to_string())
}

fn visibility_modifiers(node: Node, source: &[u8]) -> Vec<String> {
    let mut mods = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "visibility_modifier"
            || child.kind() == "static_modifier"
            || child.kind() == "abstract_modifier"
            || child.kind() == "final_modifier"
            || child.kind() == "readonly_modifier"
        {
            if let Ok(text) = child.utf8_text(source) {
                mods.push(text.to_string());
            }
        }
    }
    mods
}

fn type_refs(node: Node, source: &[u8]) -> Vec<String> {
    let mut refs = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if matches!(
            child.kind(),
            "name" | "qualified_name" | "relative_name"
        ) {
            if let Ok(text) = child.utf8_text(source) {
                refs.push(text.trim_start_matches('\\').to_string());
            }
        }
    }
    refs
}

fn find_child_kind<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .find(|c| c.kind() == kind)
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

fn first_line(node: Node, source: &[u8]) -> String {
    node.utf8_text(source)
        .ok()
        .and_then(|s| s.lines().next())
        .unwrap_or("")
        .trim()
        .to_string()
}

fn relation(
    from: &str,
    to: &str,
    relation_type: RelationType,
    node: Node,
    file_path: &Path,
) -> Relation {
    Relation {
        from: from.to_string(),
        to: to.to_string(),
        relation_type,
        location: SourceLocation {
            file: file_path.to_string_lossy().to_string(),
            start_line: node.start_position().row + 1,
            end_line: node.end_position().row + 1,
            start_column: node.start_position().column,
            end_column: node.end_position().column,
        },
        metadata: serde_json::json!({}),
        to_qualified_hint: None,
        to_type_hint: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_php_plugin_language_id() {
        let plugin = PhpPlugin::new().unwrap();
        assert_eq!(plugin.language_id(), "php");
    }

    #[test]
    fn test_extract_class_method_namespace_and_construct() {
        let source = br#"<?php
namespace App\Service;

class UserService {
    public function __construct(private UserRepository $repo) {}

    public function findById(int $id): ?User {
        return $this->repo->find($id);
    }
}
"#;
        let plugin = PhpPlugin::new().unwrap();
        let symbols = plugin
            .extract_symbols(Path::new("UserService.php"), source)
            .unwrap();

        let class_sym = symbols
            .iter()
            .find(|s| s.name == "UserService" && s.symbol_type == SymbolType::Class)
            .expect("class");
        assert_eq!(
            class_sym.qualified_name.as_deref(),
            Some("App\\Service\\UserService")
        );
        assert!(class_sym.fields.iter().any(|f| f.name == "repo"));

        let ctor = symbols
            .iter()
            .find(|s| {
                s.metadata
                    .get("is_constructor")
                    .and_then(|v| v.as_bool())
                    == Some(true)
            })
            .expect("constructor");
        assert_eq!(ctor.name, "__construct");
        assert_eq!(
            ctor.qualified_name.as_deref(),
            Some("App\\Service\\UserService.<init>")
        );

        let method = symbols
            .iter()
            .find(|s| s.name == "findById")
            .expect("method");
        assert_eq!(
            method.qualified_name.as_deref(),
            Some("App\\Service\\UserService.findById")
        );
        let id_param = method
            .parameters
            .iter()
            .find(|p| p.name == "id")
            .expect("id param");
        assert_eq!(id_param.param_type.as_deref(), Some("int"));
    }

    #[test]
    fn test_extract_relations_calls_and_implements() {
        let source = br#"<?php
namespace App;

class User extends BaseUser implements JsonSerializable {
    public function save(): void {
        $this->persist();
        parent::save();
    }
}
"#;
        let plugin = PhpPlugin::new().unwrap();
        let path = Path::new("User.php");
        let symbols = plugin.extract_symbols(path, source).unwrap();
        let relations = plugin.extract_relations(path, source, &symbols).unwrap();

        assert!(relations
            .iter()
            .any(|r| r.relation_type == RelationType::Extends && r.to == "BaseUser"));
        assert!(relations.iter().any(|r| {
            r.relation_type == RelationType::Implements && r.to == "JsonSerializable"
        }));
        assert!(relations
            .iter()
            .any(|r| r.relation_type == RelationType::Calls));
    }

    #[test]
    fn test_calculate_complexity() {
        let source = br#"<?php
function check($x) {
    if ($x > 0) {
        foreach ($items as $item) {
            if ($item > 10) return true;
        }
    }
    return false;
}
"#;
        let plugin = PhpPlugin::new().unwrap();
        let symbols = plugin
            .extract_symbols(Path::new("check.php"), source)
            .unwrap();
        let func = symbols
            .iter()
            .find(|s| s.name == "check")
            .expect("check function");
        let metrics = plugin.calculate_complexity(func, source).unwrap().unwrap();
        assert!(metrics.cyclomatic > 1);
    }
}
