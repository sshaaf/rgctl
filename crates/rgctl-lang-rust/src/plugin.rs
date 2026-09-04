//! Rust language plugin
//!
//! Extracts symbols, relationships, and complexity metrics from Rust source code
//! using TreeSitter.

use rgctl_plugin_api::*;
use rgctl_plugin_api::{Error, Result};

use crate::extract_depth::{
    extract_mod_symbol, extract_trait_symbols, extract_use_symbols, module_prefix_from_path,
    push_children, qualify_with_module, walk_depth_relations, AST_MAX_DEPTH,
};
use std::path::Path;
use tree_sitter::{Node, Parser};

/// Rust language plugin
pub struct RustPlugin;

impl RustPlugin {
    /// Create a new Rust plugin
    pub fn new() -> Result<Self> {
        Ok(Self)
    }

    /// Extract function node details
    fn extract_function(&self, node: Node, source: &[u8], file_path: &str) -> Result<Symbol> {
        let mut cursor = node.walk();
        let mut name = None;
        let mut parameters = Vec::new();
        let mut return_type = None;
        let mut modifiers = Vec::new();
        let mut documentation = None;

        // Look for function identifier
        for child in node.children(&mut cursor) {
            match child.kind() {
                "identifier" => {
                    name = Some(child.utf8_text(source)?.to_string());
                }
                "parameters" => {
                    parameters = self.extract_parameters(child, source)?;
                }
                "visibility_modifier" => {
                    modifiers.push(child.utf8_text(source)?.to_string());
                }
                "function_modifiers" => {
                    let mods = child.utf8_text(source)?.to_string();
                    modifiers.extend(mods.split_whitespace().map(|s| s.to_string()));
                }
                _ if child.kind().contains("type") => {
                    return_type = Some(child.utf8_text(source)?.to_string());
                }
                _ => {}
            }
        }

        // Prefer explicit return type field when present
        if let Some(rt) = node
            .child_by_field_name("return_type")
            .and_then(|n| n.utf8_text(source).ok())
            .map(|s| s.trim_start_matches("->").trim().to_string())
        {
            return_type = Some(rt);
        }

        // Look for doc comments
        if let Some(prev_sibling) = node.prev_sibling() {
            if prev_sibling.kind() == "line_comment" {
                let comment = prev_sibling.utf8_text(source)?;
                if comment.starts_with("///") || comment.starts_with("//!") {
                    documentation = Some(
                        comment
                            .trim_start_matches("///")
                            .trim_start_matches("//!")
                            .trim()
                            .to_string(),
                    );
                }
            }
        }

        let raw_name = name.ok_or_else(|| Error::ParseError {
            file: file_path.into(),
            line: node.start_position().row + 1,
            message: "Function missing name".to_string(),
        })?;

        let impl_type = self.find_containing_impl_type(node, source);
        let module_prefix = module_prefix_from_path(Path::new(file_path));
        let is_constructor = raw_name == "new" && impl_type.is_some();
        let (qualified_name, metadata) = if is_constructor {
            let ty = impl_type.clone().unwrap_or_else(|| "Unknown".to_string());
            let local = format!("{ty}::<init>");
            (
                Some(qualify_with_module(module_prefix.as_deref(), &local)),
                serde_json::json!({ "language": "rust", "is_constructor": true }),
            )
        } else if let Some(ty) = impl_type {
            let local = format!("{ty}::{raw_name}");
            (
                Some(qualify_with_module(module_prefix.as_deref(), &local)),
                serde_json::json!({ "language": "rust" }),
            )
        } else if let Some(prefix) = module_prefix.as_deref() {
            (
                Some(qualify_with_module(Some(prefix), &raw_name)),
                serde_json::json!({ "language": "rust" }),
            )
        } else {
            (None, serde_json::json!({ "language": "rust" }))
        };

        Ok(Symbol {
            name: raw_name,
            symbol_type: SymbolType::Function,
            qualified_name,
            location: SourceLocation {
                file: file_path.to_string(),
                start_line: node.start_position().row + 1,
                end_line: node.end_position().row + 1,
                start_column: node.start_position().column,
                end_column: node.end_position().column,
            },
            signature: Some(
                node.utf8_text(source)?
                    .split('{')
                    .next()
                    .unwrap_or("")
                    .trim()
                    .to_string(),
            ),
            return_type,
            parameters,
            fields: vec![],
            modifiers,
            documentation,
            metadata,
        })
    }

    fn find_containing_impl_type(&self, node: Node, source: &[u8]) -> Option<String> {
        let mut current = node;
        while let Some(parent) = current.parent() {
            if parent.kind() == "impl_item" {
                if let Some(ty) = parent
                    .child_by_field_name("type")
                    .and_then(|n| n.utf8_text(source).ok())
                    .map(str::to_string)
                {
                    return Some(ty);
                }
                let mut cursor = parent.walk();
                for child in parent.children(&mut cursor) {
                    if child.kind() == "type_identifier" {
                        return child.utf8_text(source).ok().map(str::to_string);
                    }
                }
            }
            current = parent;
        }
        None
    }

    /// Extract function parameters
    fn extract_parameters(&self, params_node: Node, source: &[u8]) -> Result<Vec<Parameter>> {
        let mut parameters = Vec::new();
        let mut cursor = params_node.walk();

        for child in params_node.children(&mut cursor) {
            if child.kind() == "parameter" {
                let mut name = None;
                let mut param_type = child
                    .child_by_field_name("type")
                    .and_then(|n| n.utf8_text(source).ok())
                    .map(str::to_string);
                let mut param_cursor = child.walk();

                for param_child in child.children(&mut param_cursor) {
                    match param_child.kind() {
                        "identifier" => {
                            name = Some(param_child.utf8_text(source)?.to_string());
                        }
                        "self" | "mutable_self" => {
                            name = Some(param_child.utf8_text(source)?.to_string());
                        }
                        _ if param_type.is_none() && param_child.kind().contains("type") => {
                            param_type = Some(param_child.utf8_text(source)?.to_string());
                        }
                        _ => {}
                    }
                }

                // Pattern field may wrap the identifier (e.g. `mut name`)
                if name.is_none() {
                    if let Some(pattern) = child.child_by_field_name("pattern") {
                        name = pattern
                            .utf8_text(source)
                            .ok()
                            .map(|s| s.trim_start_matches("mut ").trim().to_string());
                    }
                }

                if let Some(name) = name {
                    parameters.push(Parameter {
                        name,
                        param_type,
                        default_value: None, // Rust doesn't have default params
                    });
                }
            } else if child.kind() == "self_parameter" {
                parameters.push(Parameter {
                    name: child.utf8_text(source)?.to_string(),
                    param_type: None,
                    default_value: None,
                });
            }
        }

        Ok(parameters)
    }

    /// Extract struct or enum definition
    fn extract_type_definition(
        &self,
        node: Node,
        source: &[u8],
        file_path: &str,
        is_enum: bool,
    ) -> Result<Symbol> {
        let mut cursor = node.walk();
        let mut name = None;
        let mut fields = Vec::new();
        let mut modifiers = Vec::new();

        for child in node.children(&mut cursor) {
            match child.kind() {
                "type_identifier" => {
                    name = Some(child.utf8_text(source)?.to_string());
                }
                "visibility_modifier" => {
                    modifiers.push(child.utf8_text(source)?.to_string());
                }
                "field_declaration_list" => {
                    fields = self.extract_fields(child, source)?;
                }
                _ => {}
            }
        }

        let name = name.ok_or_else(|| Error::ParseError {
            file: file_path.into(),
            line: node.start_position().row + 1,
            message: format!("{} missing name", if is_enum { "Enum" } else { "Struct" }),
        })?;

        let module_prefix = module_prefix_from_path(Path::new(file_path));
        let qn = qualify_with_module(module_prefix.as_deref(), &name);

        Ok(Symbol {
            name: name.clone(),
            symbol_type: if is_enum {
                SymbolType::Enum
            } else {
                SymbolType::Struct
            },
            qualified_name: Some(qn),
            location: SourceLocation {
                file: file_path.to_string(),
                start_line: node.start_position().row + 1,
                end_line: node.end_position().row + 1,
                start_column: node.start_position().column,
                end_column: node.end_position().column,
            },
            signature: None,
            return_type: None,
            parameters: vec![],
            fields,
            modifiers,
            documentation: None,
            metadata: serde_json::json!({}),
        })
    }

    /// Extract struct fields
    fn extract_fields(&self, field_list_node: Node, source: &[u8]) -> Result<Vec<Field>> {
        let mut fields = Vec::new();
        let mut cursor = field_list_node.walk();

        for child in field_list_node.children(&mut cursor) {
            if child.kind() == "field_declaration" {
                let mut name = None;
                let mut field_type = None;
                let mut visibility = None;
                let mut field_cursor = child.walk();

                for field_child in child.children(&mut field_cursor) {
                    match field_child.kind() {
                        "field_identifier" => {
                            name = Some(field_child.utf8_text(source)?.to_string());
                        }
                        "visibility_modifier" => {
                            visibility = Some(field_child.utf8_text(source)?.to_string());
                        }
                        _ if field_type.is_none() && field_child.kind().contains("type") => {
                            field_type = Some(field_child.utf8_text(source)?.to_string());
                        }
                        _ => {}
                    }
                }

                if field_type.is_none() {
                    field_type = child
                        .child_by_field_name("type")
                        .and_then(|n| n.utf8_text(source).ok())
                        .map(str::to_string);
                }

                if let Some(name) = name {
                    fields.push(Field {
                        name,
                        field_type,
                        visibility,
                    });
                }
            }
        }

        Ok(fields)
    }

    /// Calculate cyclomatic complexity for a function node
    fn calculate_cyclomatic(&self, node: Node, _source: &[u8]) -> usize {
        let mut complexity = 1;
        let mut stack = vec![(node, 0usize)];

        while let Some((node, depth)) = stack.pop() {
            if depth > AST_MAX_DEPTH {
                continue;
            }
            if matches!(
                node.kind(),
                "if_expression"
                    | "match_expression"
                    | "while_expression"
                    | "for_expression"
                    | "loop_expression"
                    | "match_arm"
            ) {
                complexity += 1;
            }
            push_children(&mut stack, node, depth);
        }

        complexity
    }

    /// Calculate cognitive complexity (weighted by nesting)
    fn calculate_cognitive(&self, node: Node, _source: &[u8]) -> usize {
        let mut cognitive = 0;
        let mut stack = vec![(node, 0usize, 0usize)];

        while let Some((node, depth, nesting)) = stack.pop() {
            if depth > AST_MAX_DEPTH {
                continue;
            }
            let mut cursor = node.walk();
            let children: Vec<Node> = node.children(&mut cursor).collect();
            for child in children.into_iter().rev() {
                let child_nesting = match child.kind() {
                    "if_expression" | "match_expression" | "while_expression"
                    | "for_expression" | "loop_expression" => {
                        cognitive += 1 + nesting;
                        nesting + 1
                    }
                    "match_arm" => {
                        cognitive += 1 + nesting;
                        nesting
                    }
                    _ => nesting,
                };
                stack.push((child, depth + 1, child_nesting));
            }
        }

        cognitive
    }

    /// Count lines of code for a node
    fn count_loc(&self, node: Node) -> usize {
        (node.end_position().row - node.start_position().row + 1).max(1)
    }

    /// Count nesting depth
    fn count_nesting_depth(&self, node: Node) -> usize {
        let mut max_depth = 0;
        let mut stack = vec![(node, 0usize, 0usize)];

        while let Some((node, depth, current_depth)) = stack.pop() {
            if depth > AST_MAX_DEPTH {
                continue;
            }
            max_depth = max_depth.max(current_depth);
            let mut cursor = node.walk();
            let children: Vec<Node> = node.children(&mut cursor).collect();
            for child in children.into_iter().rev() {
                let child_depth = if matches!(
                    child.kind(),
                    "if_expression"
                        | "match_expression"
                        | "while_expression"
                        | "for_expression"
                        | "loop_expression"
                        | "block"
                ) {
                    current_depth + 1
                } else {
                    current_depth
                };
                stack.push((child, depth + 1, child_depth));
            }
        }

        max_depth
    }

    /// Count return statements
    fn count_returns(&self, node: Node) -> usize {
        let mut count = 0;
        let mut stack = vec![(node, 0usize)];

        while let Some((node, depth)) = stack.pop() {
            if depth > AST_MAX_DEPTH {
                continue;
            }
            if node.kind() == "return_expression" {
                count += 1;
            }
            push_children(&mut stack, node, depth);
        }

        count
    }

    fn parse(&self, file_path: &Path, source: &[u8]) -> Result<tree_sitter::Tree> {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .map_err(|e| Error::PluginError(format!("Failed to set Rust grammar: {e}")))?;
        parser.parse(source, None).ok_or_else(|| Error::ParseError {
            file: file_path.to_path_buf(),
            line: 0,
            message: "Failed to parse Rust source".to_string(),
        })
    }

    fn symbols_from_tree(
        &self,
        root: Node,
        source: &[u8],
        file_path: &Path,
    ) -> Result<Vec<Symbol>> {
        let mut symbols = Vec::new();
        let file_path_str = file_path.to_string_lossy();
        let module_prefix = module_prefix_from_path(file_path);
        let mut stack = vec![(root, 0usize)];
        let mut depth_warned = false;

        while let Some((node, depth)) = stack.pop() {
            if depth > AST_MAX_DEPTH {
                if !depth_warned {
                    tracing::warn!(
                        file = %file_path.display(),
                        depth = AST_MAX_DEPTH,
                        "AST depth limit exceeded during symbol extraction; skipping deep branches"
                    );
                    depth_warned = true;
                }
                continue;
            }

            match node.kind() {
                "function_item" => {
                    symbols.push(self.extract_function(node, source, &file_path_str)?);
                }
                "struct_item" => {
                    symbols.push(self.extract_type_definition(node, source, &file_path_str, false)?);
                }
                "enum_item" => {
                    symbols.push(self.extract_type_definition(node, source, &file_path_str, true)?);
                }
                "use_declaration" => {
                    symbols.extend(extract_use_symbols(node, source, &file_path_str));
                }
                "mod_item" => {
                    if let Some(m) = extract_mod_symbol(node, source, &file_path_str) {
                        symbols.push(m);
                    }
                }
                "trait_item" => {
                    symbols.extend(extract_trait_symbols(
                        node,
                        source,
                        &file_path_str,
                        module_prefix.as_deref(),
                    ));
                }
                _ => {}
            }

            push_children(&mut stack, node, depth);
        }

        Ok(symbols)
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
            RUST_CALL_KINDS,
            "rust",
            &mut relations,
        );
        walk_depth_relations(root, source, file_path, symbols, &mut relations);
        Ok(relations)
    }
}

impl Default for RustPlugin {
    fn default() -> Self {
        Self::new().expect("Failed to create RustPlugin")
    }
}

impl LanguagePlugin for RustPlugin {
    fn language_id(&self) -> &str {
        "rust"
    }

    fn file_extensions(&self) -> Vec<&str> {
        vec!["rs"]
    }

    fn grammar(&self) -> Option<tree_sitter::Language> {
        Some(tree_sitter_rust::LANGUAGE.into())
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

        // Re-parse to find the function node
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .map_err(|e| Error::PluginError(format!("Failed to set Rust grammar: {}", e)))?;

        let tree = parser
            .parse(source, None)
            .ok_or_else(|| Error::ParseError {
                file: symbol.location.file.clone().into(),
                line: symbol.location.start_line,
                message: "Failed to parse source for complexity analysis".to_string(),
            })?;

        // Find the function node by location
        let root = tree.root_node();
        let target_line = symbol.location.start_line - 1; // TreeSitter uses 0-indexed lines

        let mut stack = vec![(root, 0usize)];
        while let Some((node, depth)) = stack.pop() {
            if depth > AST_MAX_DEPTH {
                continue;
            }
            if node.kind() == "function_item" && node.start_position().row == target_line {
                return Ok(Some(ComplexityMetrics {
                    cyclomatic: self.calculate_cyclomatic(node, source),
                    cognitive: self.calculate_cognitive(node, source),
                    loc: self.count_loc(node),
                    parameters: symbol.parameters.len(),
                    nesting_depth: self.count_nesting_depth(node),
                    returns: self.count_returns(node),
                }));
            }
            push_children(&mut stack, node, depth);
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_plugin_language_id() {
        let plugin = RustPlugin::new().unwrap();
        assert_eq!(plugin.language_id(), "rust");
    }

    #[test]
    fn test_rust_plugin_file_extensions() {
        let plugin = RustPlugin::new().unwrap();
        assert_eq!(plugin.file_extensions(), vec!["rs"]);
    }

    #[test]
    fn test_rust_plugin_can_handle() {
        let plugin = RustPlugin::new().unwrap();
        assert!(plugin.can_handle(Path::new("test.rs")));
        assert!(plugin.can_handle(Path::new("src/lib.rs")));
        assert!(!plugin.can_handle(Path::new("test.py")));
    }

    #[test]
    fn test_extract_simple_function() {
        let plugin = RustPlugin::new().unwrap();
        let source = b"fn add(a: i32, b: i32) -> i32 { a + b }";
        let symbols = plugin
            .extract_symbols(Path::new("test.rs"), source)
            .unwrap();

        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "add");
        assert_eq!(symbols[0].symbol_type, SymbolType::Function);
        assert_eq!(symbols[0].parameters.len(), 2);
        assert_eq!(symbols[0].parameters[0].name, "a");
        assert_eq!(symbols[0].parameters[1].name, "b");
        assert!(symbols[0].return_type.is_some());
    }

    #[test]
    fn test_extract_function_with_modifiers() {
        let plugin = RustPlugin::new().unwrap();
        let source = b"pub async fn fetch_data() -> Result<String> { Ok(String::new()) }";
        let symbols = plugin
            .extract_symbols(Path::new("test.rs"), source)
            .unwrap();

        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "fetch_data");
        assert!(symbols[0].modifiers.contains(&"pub".to_string()));
    }

    #[test]
    fn test_extract_struct() {
        let plugin = RustPlugin::new().unwrap();
        let source = b"pub struct User { pub name: String, age: u32 }";
        let symbols = plugin
            .extract_symbols(Path::new("test.rs"), source)
            .unwrap();

        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "User");
        assert_eq!(symbols[0].symbol_type, SymbolType::Struct);
        assert_eq!(symbols[0].fields.len(), 2);
        assert_eq!(symbols[0].fields[0].name, "name");
        assert_eq!(symbols[0].fields[1].name, "age");
    }

    #[test]
    fn test_extract_struct_fields_and_new_constructor() {
        let source = br#"
pub struct User {
    pub name: String,
    age: u32,
}

impl User {
    pub fn new(name: String, age: u32) -> Self {
        Self { name, age }
    }
}
"#;
        let plugin = RustPlugin::new().unwrap();
        let symbols = plugin
            .extract_symbols(Path::new("user.rs"), source)
            .unwrap();
        let st = symbols
            .iter()
            .find(|s| s.name == "User" && s.symbol_type == SymbolType::Struct)
            .expect("struct");
        assert_eq!(st.fields.len(), 2);
        assert_eq!(st.fields[0].field_type.as_deref(), Some("String"));
        assert_eq!(st.fields[1].field_type.as_deref(), Some("u32"));
        let ctor = symbols
            .iter()
            .find(|s| {
                s.symbol_type == SymbolType::Function
                    && s.metadata.get("is_constructor").and_then(|v| v.as_bool()) == Some(true)
            })
            .expect("constructor");
        assert_eq!(ctor.name, "new");
        assert_eq!(ctor.qualified_name.as_deref(), Some("User::<init>"));
        assert!(
            ctor.parameters
                .iter()
                .any(|p| { p.name == "name" && p.param_type.as_deref() == Some("String") })
        );
        assert!(
            ctor.parameters
                .iter()
                .any(|p| p.name == "age" && p.param_type.as_deref() == Some("u32"))
        );
    }

    #[test]
    fn test_extract_enum() {
        let plugin = RustPlugin::new().unwrap();
        let source = b"enum Status { Active, Inactive, Pending }";
        let symbols = plugin
            .extract_symbols(Path::new("test.rs"), source)
            .unwrap();

        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "Status");
        assert_eq!(symbols[0].symbol_type, SymbolType::Enum);
    }

    #[test]
    fn test_extract_multiple_symbols() {
        let plugin = RustPlugin::new().unwrap();
        let source = br#"
            fn helper() -> i32 { 42 }

            struct Config {
                port: u16,
            }

            fn main() {
                let c = Config { port: 8080 };
            }
        "#;
        let symbols = plugin
            .extract_symbols(Path::new("test.rs"), source)
            .unwrap();

        assert_eq!(symbols.len(), 3); // helper, Config, main
        assert_eq!(symbols[0].name, "helper");
        assert_eq!(symbols[1].name, "Config");
        assert_eq!(symbols[2].name, "main");
    }

    #[test]
    fn test_calculate_complexity_simple() {
        let plugin = RustPlugin::new().unwrap();
        let source = b"fn simple() { println!(\"hello\"); }";
        let symbols = plugin
            .extract_symbols(Path::new("test.rs"), source)
            .unwrap();

        assert_eq!(symbols.len(), 1);
        let complexity = plugin.calculate_complexity(&symbols[0], source).unwrap();

        assert!(complexity.is_some());
        let metrics = complexity.unwrap();
        assert_eq!(metrics.cyclomatic, 1); // No branches
        assert_eq!(metrics.parameters, 0);
    }

    #[test]
    fn test_calculate_complexity_with_branches() {
        let plugin = RustPlugin::new().unwrap();
        let source = br#"
            fn check(x: i32) -> bool {
                if x > 0 {
                    if x < 100 {
                        return true;
                    }
                }
                false
            }
        "#;
        let symbols = plugin
            .extract_symbols(Path::new("test.rs"), source)
            .unwrap();

        assert_eq!(symbols.len(), 1);
        let complexity = plugin.calculate_complexity(&symbols[0], source).unwrap();

        assert!(complexity.is_some());
        let metrics = complexity.unwrap();
        assert_eq!(metrics.cyclomatic, 3); // Base + 2 if statements
        assert!(metrics.cognitive >= 2); // Nested conditions should have cognitive cost
        assert_eq!(metrics.parameters, 1);
        assert_eq!(metrics.returns, 1);
    }

    #[test]
    fn test_calculate_complexity_with_match() {
        let plugin = RustPlugin::new().unwrap();
        let source = br#"
            fn handle(x: Option<i32>) -> i32 {
                match x {
                    Some(v) => v,
                    None => 0,
                }
            }
        "#;
        let symbols = plugin
            .extract_symbols(Path::new("test.rs"), source)
            .unwrap();

        assert_eq!(symbols.len(), 1);
        let complexity = plugin.calculate_complexity(&symbols[0], source).unwrap();

        assert!(complexity.is_some());
        let metrics = complexity.unwrap();
        assert!(metrics.cyclomatic >= 2); // Match + arms
    }

    #[test]
    fn test_complexity_not_calculated_for_structs() {
        let plugin = RustPlugin::new().unwrap();
        let source = b"struct Data { value: i32 }";
        let symbols = plugin
            .extract_symbols(Path::new("test.rs"), source)
            .unwrap();

        assert_eq!(symbols.len(), 1);
        let complexity = plugin.calculate_complexity(&symbols[0], source).unwrap();
        assert!(complexity.is_none()); // Structs don't have complexity
    }

    #[test]
    fn test_source_location_accuracy() {
        let plugin = RustPlugin::new().unwrap();
        let source = br#"
fn first() {}

fn second() {}
"#;
        let symbols = plugin
            .extract_symbols(Path::new("test.rs"), source)
            .unwrap();

        assert_eq!(symbols.len(), 2);
        assert_eq!(symbols[0].location.start_line, 2); // first() on line 2
        assert_eq!(symbols[1].location.start_line, 4); // second() on line 4
    }

    #[test]
    fn test_extract_relations_calls() {
        let source = br#"
fn caller() {
    helper();
}

fn helper() {}
"#;
        let plugin = RustPlugin::new().unwrap();
        let path = Path::new("test.rs");
        let symbols = plugin.extract_symbols(path, source).unwrap();
        let relations = plugin.extract_relations(path, source, &symbols).unwrap();
        assert!(
            relations
                .iter()
                .any(|r| matches!(r.relation_type, RelationType::Calls) && r.to == "helper"),
            "expected Calls -> helper, got {relations:?}"
        );
    }

    #[test]
    fn test_extract_use_imports() {
        let source = br#"use crate::services::order;
use uuid::Uuid as Id;

fn main() {}
"#;
        let plugin = RustPlugin::new().unwrap();
        let symbols = plugin
            .extract_symbols(Path::new("src/main.rs"), source)
            .unwrap();
        let imports: Vec<_> = symbols
            .iter()
            .filter(|s| s.symbol_type == SymbolType::Import)
            .collect();
        assert!(imports.len() >= 2, "{imports:?}");
        assert!(imports.iter().any(|i| i.name == "order"));
        assert!(imports.iter().any(|i| i.name == "Id"));
    }

    #[test]
    fn test_extract_impl_implements_and_derive() {
        let source = br#"
#[derive(Debug)]
struct Item { v: i32 }

impl Default for Item {
    fn default() -> Self { Item { v: 0 } }
}
"#;
        let plugin = RustPlugin::new().unwrap();
        let path = Path::new("src/item.rs");
        let symbols = plugin.extract_symbols(path, source).unwrap();
        let relations = plugin.extract_relations(path, source, &symbols).unwrap();
        assert!(
            relations.iter().any(|r| {
                r.relation_type == RelationType::Implements
                    && r.from.ends_with("Item")
                    && r.to == "Default"
            }),
            "Implements: {relations:?}"
        );
        assert!(
            relations.iter().any(|r| {
                r.relation_type == RelationType::AnnotatedWith
                    && r.from.ends_with("Item")
                    && r.to == "derive"
            }),
            "AnnotatedWith: {relations:?}"
        );
        let item = symbols
            .iter()
            .find(|s| s.name == "Item" && s.symbol_type == SymbolType::Struct)
            .expect("struct");
        assert_eq!(
            item.qualified_name.as_deref(),
            Some("item::Item")
        );
    }
}
