//! C# language plugin using Tree-sitter.

use rgctl_plugin_api::*;
use std::path::Path;
use tree_sitter::{Node, Parser};

/// C# language plugin.
pub struct CSharpPlugin {
    _parser: Parser,
}

impl CSharpPlugin {
    /// Create a new C# plugin.
    pub fn new() -> Result<Self> {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_c_sharp::LANGUAGE.into())
            .map_err(|e| Error::PluginError(format!("Failed to set C# grammar: {e}")))?;
        Ok(Self { _parser: parser })
    }

    fn parse(&self, file_path: &Path, source: &[u8]) -> Result<tree_sitter::Tree> {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_c_sharp::LANGUAGE.into())
            .map_err(|e| Error::PluginError(format!("Failed to set C# grammar: {e}")))?;
        parser.parse(source, None).ok_or_else(|| Error::ParseError {
            file: file_path.to_path_buf(),
            line: 0,
            message: "Failed to parse C# source".to_string(),
        })
    }

    fn extract_method(&self, node: Node, source: &[u8], file_path: &str) -> Result<Symbol> {
        let name = node
            .child_by_field_name("name")
            .or_else(|| find_child_kind(node, "identifier"))
            .and_then(|n| n.utf8_text(source).ok())
            .map(str::to_string)
            .ok_or_else(|| Error::ParseError {
                file: file_path.into(),
                line: node.start_position().row + 1,
                message: "Method missing name".to_string(),
            })?;

        let qualified_name = self
            .find_containing_type_name(node, source)
            .map(|ty| format!("{ty}.{name}"));

        let return_type = method_return_type(node, source);
        let modifiers = modifier_texts(node, source);
        let parameters = self.extract_parameters(node, source)?;

        Ok(Symbol {
            name: name.clone(),
            symbol_type: SymbolType::Function,
            qualified_name,
            location: source_location(node, file_path),
            signature: Some(first_line(node, source)),
            return_type,
            parameters,
            fields: vec![],
            modifiers,
            documentation: None,
            metadata: serde_json::json!({ "language": "csharp" }),
        })
    }

    fn extract_constructor(&self, node: Node, source: &[u8], file_path: &str) -> Result<Symbol> {
        let type_name = self
            .find_containing_type_name(node, source)
            .or_else(|| {
                node.child_by_field_name("name")
                    .and_then(|n| n.utf8_text(source).ok().map(str::to_string))
            })
            .ok_or_else(|| Error::ParseError {
                file: file_path.into(),
                line: node.start_position().row + 1,
                message: "Constructor missing containing type".to_string(),
            })?;

        let parameters = self.extract_parameters(node, source)?;
        let qualified_name = format!("{type_name}.<init>");

        Ok(Symbol {
            name: type_name,
            symbol_type: SymbolType::Function,
            qualified_name: Some(qualified_name),
            location: source_location(node, file_path),
            signature: Some(first_line(node, source)),
            return_type: None,
            parameters,
            fields: vec![],
            modifiers: modifier_texts(node, source),
            documentation: None,
            metadata: serde_json::json!({
                "language": "csharp",
                "is_constructor": true,
            }),
        })
    }

    fn extract_parameters(&self, node: Node, source: &[u8]) -> Result<Vec<Parameter>> {
        let mut parameters = Vec::new();
        let params_node = if let Some(p) = node.child_by_field_name("parameters") {
            p
        } else {
            let mut cursor = node.walk();
            let found = node
                .children(&mut cursor)
                .find(|c| c.kind() == "parameter_list");
            match found {
                Some(p) => p,
                None => return Ok(parameters),
            }
        };

        let mut cursor = params_node.walk();
        for child in params_node.children(&mut cursor) {
            if child.kind() != "parameter" {
                continue;
            }
            let name = child
                .child_by_field_name("name")
                .and_then(|n| n.utf8_text(source).ok())
                .map(str::to_string);
            let param_type = child
                .child_by_field_name("type")
                .and_then(|n| n.utf8_text(source).ok())
                .map(str::to_string);
            if let Some(name) = name {
                parameters.push(Parameter {
                    name,
                    param_type,
                    default_value: None,
                });
            }
        }
        Ok(parameters)
    }

    fn extract_type_fields(&self, type_node: Node, source: &[u8]) -> Result<Vec<Field>> {
        let mut fields = Vec::new();
        let body = type_node
            .child_by_field_name("body")
            .or_else(|| find_direct_child_kind(type_node, "declaration_list"));
        let Some(body) = body else {
            return Ok(fields);
        };

        let mut body_cursor = body.walk();
        for child in body.children(&mut body_cursor) {
            match child.kind() {
                "field_declaration" => {
                    let visibility = field_visibility(child, source);
                    let var_decl = find_direct_child_kind(child, "variable_declaration");
                    let Some(var_decl) = var_decl else {
                        continue;
                    };
                    let field_type = var_decl
                        .child_by_field_name("type")
                        .and_then(|n| n.utf8_text(source).ok())
                        .map(str::to_string);
                    let mut decl_cursor = var_decl.walk();
                    for declarator in var_decl.children(&mut decl_cursor) {
                        if declarator.kind() != "variable_declarator" {
                            continue;
                        }
                        if let Some(name_node) = declarator.child_by_field_name("name") {
                            fields.push(Field {
                                name: name_node.utf8_text(source)?.to_string(),
                                field_type: field_type.clone(),
                                visibility: visibility.clone(),
                            });
                        }
                    }
                }
                "property_declaration" => {
                    let name = child
                        .child_by_field_name("name")
                        .and_then(|n| n.utf8_text(source).ok())
                        .map(str::to_string);
                    let field_type = child
                        .child_by_field_name("type")
                        .and_then(|n| n.utf8_text(source).ok())
                        .map(str::to_string);
                    if let Some(name) = name {
                        fields.push(Field {
                            name,
                            field_type,
                            visibility: field_visibility(child, source),
                        });
                    }
                }
                _ => {}
            }
        }
        Ok(fields)
    }

    fn extract_type(
        &self,
        node: Node,
        source: &[u8],
        file_path: &str,
        symbol_type: SymbolType,
    ) -> Result<Symbol> {
        let name = type_name(node, source).ok_or_else(|| Error::ParseError {
            file: file_path.into(),
            line: node.start_position().row + 1,
            message: "Type missing name".to_string(),
        })?;

        let fields = if symbol_type == SymbolType::Class {
            self.extract_type_fields(node, source)?
        } else {
            vec![]
        };

        Ok(Symbol {
            name: name.clone(),
            symbol_type,
            qualified_name: None,
            location: source_location(node, file_path),
            signature: None,
            return_type: None,
            parameters: vec![],
            fields,
            modifiers: modifier_texts(node, source),
            documentation: None,
            metadata: serde_json::json!({ "language": "csharp" }),
        })
    }

    fn symbols_from_tree(
        &self,
        root: Node,
        source: &[u8],
        file_path: &Path,
    ) -> Result<Vec<Symbol>> {
        let mut symbols = Vec::new();
        self.traverse(root, source, &file_path.to_string_lossy(), &mut symbols)?;
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
            CSHARP_CALL_KINDS,
            "csharp",
            &mut relations,
        );
        self.extract_inheritance(root, source, file_path, &mut relations)?;
        Ok(relations)
    }

    fn traverse(
        &self,
        node: Node,
        source: &[u8],
        file_path: &str,
        symbols: &mut Vec<Symbol>,
    ) -> Result<()> {
        match node.kind() {
            "method_declaration" | "local_function_statement" => {
                symbols.push(self.extract_method(node, source, file_path)?);
            }
            "constructor_declaration" => {
                symbols.push(self.extract_constructor(node, source, file_path)?);
            }
            "class_declaration" | "struct_declaration" => {
                symbols.push(self.extract_type(node, source, file_path, SymbolType::Class)?);
            }
            "interface_declaration" => {
                symbols.push(self.extract_type(node, source, file_path, SymbolType::Interface)?);
            }
            "enum_declaration" => {
                symbols.push(self.extract_type(node, source, file_path, SymbolType::Enum)?);
            }
            "using_directive" => {
                let text = node.utf8_text(source)?.trim().to_string();
                symbols.push(Symbol {
                    name: text.clone(),
                    symbol_type: SymbolType::Import,
                    qualified_name: None,
                    location: source_location(node, file_path),
                    signature: None,
                    return_type: None,
                    parameters: vec![],
                    fields: vec![],
                    modifiers: vec![],
                    documentation: None,
                    metadata: serde_json::json!({ "language": "csharp" }),
                });
            }
            _ => {}
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.traverse(child, source, file_path, symbols)?;
        }
        Ok(())
    }

    fn extract_inheritance(
        &self,
        node: Node,
        source: &[u8],
        file_path: &Path,
        relations: &mut Vec<Relation>,
    ) -> Result<()> {
        match node.kind() {
            "class_declaration" => {
                let class_name = type_name(node, source).unwrap_or_default();
                if class_name.is_empty() {
                    return Ok(());
                }
                if let Some(base_list) = node.child_by_field_name("bases") {
                    collect_base_list_relations(
                        &class_name,
                        base_list,
                        source,
                        file_path,
                        true,
                        relations,
                    );
                } else if let Some(base_list) = find_child_kind(node, "base_list") {
                    collect_base_list_relations(
                        &class_name,
                        base_list,
                        source,
                        file_path,
                        true,
                        relations,
                    );
                }
            }
            "interface_declaration" => {
                let name = type_name(node, source).unwrap_or_default();
                if name.is_empty() {
                    return Ok(());
                }
                if let Some(base_list) = node.child_by_field_name("bases") {
                    collect_base_list_relations(
                        &name, base_list, source, file_path, false, relations,
                    );
                } else if let Some(base_list) = find_child_kind(node, "base_list") {
                    collect_base_list_relations(
                        &name, base_list, source, file_path, false, relations,
                    );
                }
            }
            _ => {}
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.extract_inheritance(child, source, file_path, relations)?;
        }
        Ok(())
    }

    fn find_containing_type_name(&self, node: Node, source: &[u8]) -> Option<String> {
        let mut current = node;
        while let Some(parent) = current.parent() {
            if matches!(
                parent.kind(),
                "class_declaration" | "struct_declaration" | "interface_declaration"
            ) {
                return type_name(parent, source);
            }
            current = parent;
        }
        None
    }
}

impl LanguagePlugin for CSharpPlugin {
    fn language_id(&self) -> &str {
        "csharp"
    }

    fn file_extensions(&self) -> Vec<&str> {
        vec!["cs"]
    }

    fn grammar(&self) -> Option<tree_sitter::Language> {
        Some(tree_sitter_c_sharp::LANGUAGE.into())
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
        _symbol: &Symbol,
        _source: &[u8],
    ) -> Result<Option<ComplexityMetrics>> {
        Ok(None)
    }
}

fn collect_base_list_relations(
    from: &str,
    base_list: Node,
    source: &[u8],
    file_path: &Path,
    class_style: bool,
    relations: &mut Vec<Relation>,
) {
    let bases: Vec<String> = base_list
        .children(&mut base_list.walk())
        .filter(|c| c.is_named() && c.kind() == "identifier")
        .filter_map(|c| c.utf8_text(source).ok().map(str::to_string))
        .collect();

    if bases.is_empty() {
        return;
    }

    if class_style {
        if let Some(base) = bases.first() {
            relations.push(relation(
                from,
                base,
                RelationType::Extends,
                base_list,
                file_path,
            ));
        }
        for iface in bases.iter().skip(1) {
            relations.push(relation(
                from,
                iface,
                RelationType::Implements,
                base_list,
                file_path,
            ));
        }
    } else {
        for iface in &bases {
            relations.push(relation(
                from,
                iface,
                RelationType::Implements,
                base_list,
                file_path,
            ));
        }
    }
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
        metadata: serde_json::json!({ "language": "csharp" }),
        to_qualified_hint: None,
        to_type_hint: None,
    }
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
        .unwrap_or("")
        .lines()
        .next()
        .unwrap_or("")
        .trim()
        .to_string()
}

fn modifier_texts(node: Node, source: &[u8]) -> Vec<String> {
    node.children(&mut node.walk())
        .filter(|c| c.kind() == "modifier")
        .filter_map(|c| c.utf8_text(source).ok().map(str::to_string))
        .collect()
}

fn type_name(node: Node, source: &[u8]) -> Option<String> {
    node.child_by_field_name("name")
        .or_else(|| find_child_kind(node, "identifier"))
        .and_then(|n| n.utf8_text(source).ok().map(str::to_string))
}

fn method_return_type(node: Node, source: &[u8]) -> Option<String> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if matches!(
            child.kind(),
            "predefined_type" | "identifier" | "generic_name" | "nullable_type"
        ) {
            return child.utf8_text(source).ok().map(str::to_string);
        }
    }
    None
}

fn find_child_kind<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
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

fn find_direct_child_kind<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == kind {
            return Some(child);
        }
    }
    None
}

fn field_visibility(node: Node, source: &[u8]) -> Option<String> {
    let mods = modifier_texts(node, source);
    if mods.is_empty() {
        None
    } else {
        Some(mods.join(" "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_extract_csharp_class_and_method() {
        let source = br#"
using System;

public class UserService {
    public string Authenticate(string token) {
        return token;
    }
}
"#;
        let plugin = CSharpPlugin::new().unwrap();
        let symbols = plugin
            .extract_symbols(Path::new("UserService.cs"), source)
            .unwrap();
        assert!(symbols.iter().any(|s| s.name == "UserService"));
        assert!(symbols.iter().any(|s| s.name == "Authenticate"));
        let auth = symbols.iter().find(|s| s.name == "Authenticate").unwrap();
        assert_eq!(auth.parameters.len(), 1);
        assert_eq!(auth.parameters[0].name, "token");
        assert!(
            auth.parameters[0]
                .param_type
                .as_deref()
                .is_some_and(|t| !t.is_empty()),
            "expected typed parameter, got {:?}",
            auth.parameters[0].param_type
        );
    }

    #[test]
    fn test_extract_csharp_fields_and_constructor() {
        let source = br#"
public class OrderDTO {
    private string orderId;
    private string status;

    public OrderDTO(string orderId, string status) {
        this.orderId = orderId;
        this.status = status;
    }

    public void MarkProcessed() {
        this.status = "PROCESSED";
    }
}
"#;
        let plugin = CSharpPlugin::new().unwrap();
        let symbols = plugin
            .extract_symbols(Path::new("OrderDTO.cs"), source)
            .unwrap();
        let class = symbols
            .iter()
            .find(|s| s.name == "OrderDTO" && s.symbol_type == SymbolType::Class)
            .expect("class");
        let status = class
            .fields
            .iter()
            .find(|f| f.name == "status")
            .expect("status field");
        assert!(
            status.field_type.as_deref().is_some_and(|t| !t.is_empty()),
            "expected status field type, got {:?}",
            status.field_type
        );
        let ctor = symbols
            .iter()
            .find(|s| {
                s.symbol_type == SymbolType::Function
                    && s.metadata.get("is_constructor").and_then(|v| v.as_bool()) == Some(true)
            })
            .expect("constructor");
        assert!(
            ctor.qualified_name
                .as_deref()
                .is_some_and(|qn| qn.ends_with(".<init>")),
            "expected .<init> qn, got {:?}",
            ctor.qualified_name
        );
        assert_eq!(ctor.parameters.len(), 2);
        let method = symbols.iter().find(|s| s.name == "MarkProcessed").unwrap();
        assert!(method.parameters.is_empty());
    }

    #[test]
    fn test_extract_relations_calls() {
        let source = br#"
public class Example {
    public void Foo() {
        Bar();
    }
    public void Bar() {}
}
"#;
        let plugin = CSharpPlugin::new().unwrap();
        let path = Path::new("Example.cs");
        let symbols = plugin.extract_symbols(path, source).unwrap();
        let relations = plugin.extract_relations(path, source, &symbols).unwrap();
        assert!(
            relations
                .iter()
                .any(|r| matches!(r.relation_type, RelationType::Calls)),
            "expected Calls relations, got {relations:?}"
        );
    }

    #[test]
    fn test_extract_relations_inheritance() {
        let source = br#"public class ServiceImpl : BaseService, IService {}"#;
        let plugin = CSharpPlugin::new().unwrap();
        let path = Path::new("Service.cs");
        let symbols = plugin.extract_symbols(path, source).unwrap();
        let relations = plugin.extract_relations(path, source, &symbols).unwrap();
        assert!(
            relations
                .iter()
                .any(|r| matches!(r.relation_type, RelationType::Extends)),
            "missing Extends: {relations:?}"
        );
        assert!(
            relations
                .iter()
                .any(|r| matches!(r.relation_type, RelationType::Implements)),
            "missing Implements: {relations:?}"
        );
    }
}
