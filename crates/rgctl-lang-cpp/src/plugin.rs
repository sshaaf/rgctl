//! C++ language plugin using Tree-sitter.

use rgctl_plugin_api::*;
use std::path::Path;
use tree_sitter::{Node, Parser};

/// C++ language plugin.
pub struct CppPlugin {
    _parser: Parser,
}

impl CppPlugin {
    /// Create a new C++ plugin.
    pub fn new() -> Result<Self> {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_cpp::LANGUAGE.into())
            .map_err(|e| Error::PluginError(format!("Failed to set C++ grammar: {e}")))?;
        Ok(Self { _parser: parser })
    }

    fn parse(&self, file_path: &Path, source: &[u8]) -> Result<tree_sitter::Tree> {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_cpp::LANGUAGE.into())
            .map_err(|e| Error::PluginError(format!("Failed to set C++ grammar: {e}")))?;
        parser.parse(source, None).ok_or_else(|| Error::ParseError {
            file: file_path.to_path_buf(),
            line: 0,
            message: "Failed to parse C++ source".to_string(),
        })
    }

    fn extract_function(
        &self,
        node: Node,
        source: &[u8],
        file_path: &str,
        scope: Option<&str>,
    ) -> Result<Option<Symbol>> {
        let Some(name) = function_name_from_node(node, source) else {
            return Ok(None);
        };

        // Skip destructors for constructor detection (`~Foo`).
        let is_destructor = name.starts_with('~');
        let is_constructor = !is_destructor && scope.is_some_and(|s| s == name) && !name.is_empty();

        let parameters = extract_parameters(node, source)?;

        let qualified_name = if is_constructor {
            scope.map(|s| format!("{s}::<init>"))
        } else {
            scope.map(|s| format!("{s}::{name}"))
        };

        let metadata = if is_constructor {
            serde_json::json!({ "language": "cpp", "is_constructor": true })
        } else {
            serde_json::json!({ "language": "cpp" })
        };

        Ok(Some(Symbol {
            name: name.clone(),
            symbol_type: SymbolType::Function,
            qualified_name,
            location: source_location(node, file_path),
            signature: Some(first_line(node, source)),
            return_type: None,
            parameters,
            fields: vec![],
            modifiers: vec![],
            documentation: None,
            metadata,
        }))
    }

    fn extract_class(
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

        // F1: fields from field_declaration in class/struct body.
        // Enums have no fields; leave empty.
        let fields = if matches!(symbol_type, SymbolType::Class | SymbolType::Struct) {
            extract_type_fields(node, source)?
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
            modifiers: vec![],
            documentation: None,
            metadata: serde_json::json!({ "language": "cpp" }),
        })
    }

    fn symbols_from_tree(
        &self,
        root: Node,
        source: &[u8],
        file_path: &Path,
    ) -> Result<Vec<Symbol>> {
        let mut symbols = Vec::new();
        self.traverse(
            root,
            source,
            &file_path.to_string_lossy(),
            &mut symbols,
            None,
        )?;
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
            CPP_CALL_KINDS,
            "cpp",
            &mut relations,
        );
        self.extract_inheritance(root, source, file_path, &mut relations)?;
        Ok(relations)
    }

    /// Iterative tree traversal using an explicit stack to prevent stack overflows on deep ASTs.
    fn traverse(
        &self,
        root: Node,
        source: &[u8],
        file_path: &str,
        symbols: &mut Vec<Symbol>,
        initial_scope: Option<String>,
    ) -> Result<()> {
        const MAX_DEPTH: usize = 2048;
        let mut stack = vec![(root, 0usize, initial_scope)];

        while let Some((node, depth, scope)) = stack.pop() {
            if depth > MAX_DEPTH {
                tracing::warn!(
                    file = %file_path,
                    depth = depth,
                    "AST depth limit exceeded during C++ traversal; skipping deep branches"
                );
                continue;
            }

            let next_scope = match node.kind() {
                "namespace_definition" => node
                    .child_by_field_name("name")
                    .and_then(|n| n.utf8_text(source).ok().map(str::to_string))
                    .map(|name| {
                        scope
                            .as_ref()
                            .map(|s| format!("{s}::{name}"))
                            .unwrap_or(name)
                    }),
                "class_specifier" | "struct_specifier" => type_name(node, source),
                _ => None,
            };
            let child_scope = next_scope.clone().or(scope);
            let active_scope = child_scope.as_deref();

            match node.kind() {
                "function_definition" | "declaration" => {
                    if let Some(sym) =
                        self.extract_function(node, source, file_path, active_scope)?
                    {
                        symbols.push(sym);
                    }
                }
                "class_specifier" => {
                    if type_name(node, source).is_some() {
                        symbols.push(self.extract_class(
                            node,
                            source,
                            file_path,
                            SymbolType::Class,
                        )?);
                    }
                }
                "struct_specifier" => {
                    if type_name(node, source).is_some() {
                        symbols.push(self.extract_class(
                            node,
                            source,
                            file_path,
                            SymbolType::Struct,
                        )?);
                    }
                }
                "enum_specifier" => {
                    if type_name(node, source).is_some() {
                        symbols.push(self.extract_class(
                            node,
                            source,
                            file_path,
                            SymbolType::Enum,
                        )?);
                    }
                }
                "type_definition" => {
                    let mut cursor = node.walk();
                    for child in node.children(&mut cursor) {
                        if matches!(child.kind(), "class_specifier" | "struct_specifier")
                            && type_name(child, source).is_some()
                        {
                            let st = if child.kind() == "class_specifier" {
                                SymbolType::Class
                            } else {
                                SymbolType::Struct
                            };
                            symbols.push(self.extract_class(child, source, file_path, st)?);
                        }
                    }
                }
                "preproc_include" | "using_declaration" => {
                    let text = node.utf8_text(source)?.trim().to_string();
                    symbols.push(Symbol {
                        name: text,
                        symbol_type: SymbolType::Import,
                        qualified_name: None,
                        location: source_location(node, file_path),
                        signature: None,
                        return_type: None,
                        parameters: vec![],
                        fields: vec![],
                        modifiers: vec![],
                        documentation: None,
                        metadata: serde_json::json!({ "language": "cpp" }),
                    });
                }
                _ => {}
            }

            let mut cursor = node.walk();
            let children: Vec<Node> = node.children(&mut cursor).collect();
            for child in children.into_iter().rev() {
                stack.push((child, depth + 1, child_scope.clone()));
            }
        }
        Ok(())
    }

    /// Iterative inheritance extraction (heap stack; bounded depth).
    fn extract_inheritance(
        &self,
        root: Node,
        source: &[u8],
        file_path: &Path,
        relations: &mut Vec<Relation>,
    ) -> Result<()> {
        const MAX_DEPTH: usize = 2048;
        let mut stack = vec![(root, 0usize)];

        while let Some((node, depth)) = stack.pop() {
            if depth > MAX_DEPTH {
                tracing::warn!(
                    file = %file_path.display(),
                    depth = depth,
                    "AST depth limit exceeded during C++ inheritance walk; skipping deep branches"
                );
                continue;
            }

            if matches!(node.kind(), "class_specifier" | "struct_specifier") {
                let class_name = type_name(node, source).unwrap_or_default();
                if !class_name.is_empty() {
                    let mut cursor = node.walk();
                    for child in node.children(&mut cursor) {
                        if child.kind() == "base_class_clause" {
                            collect_base_relations(
                                &class_name,
                                child,
                                source,
                                file_path,
                                relations,
                            );
                        }
                    }
                }
            }

            let mut cursor = node.walk();
            let children: Vec<Node> = node.children(&mut cursor).collect();
            for child in children.into_iter().rev() {
                stack.push((child, depth + 1));
            }
        }
        Ok(())
    }
}

impl LanguagePlugin for CppPlugin {
    fn language_id(&self) -> &str {
        "cpp"
    }

    fn file_extensions(&self) -> Vec<&str> {
        vec!["cpp", "cc", "cxx", "hpp", "hh", "hxx"]
    }

    fn grammar(&self) -> Option<tree_sitter::Language> {
        Some(tree_sitter_cpp::LANGUAGE.into())
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

fn collect_base_relations(
    from: &str,
    base_clause: Node,
    source: &[u8],
    file_path: &Path,
    relations: &mut Vec<Relation>,
) {
    let mut cursor = base_clause.walk();
    let mut access_public = false;
    for child in base_clause.children(&mut cursor) {
        match child.kind() {
            "access_specifier" => {
                access_public = child
                    .utf8_text(source)
                    .ok()
                    .is_some_and(|t| t.contains("public"));
            }
            "type_identifier" | "qualified_identifier" => {
                let base = child
                    .utf8_text(source)
                    .ok()
                    .map(str::to_string)
                    .or_else(|| {
                        child
                            .child_by_field_name("name")
                            .and_then(|n| n.utf8_text(source).ok().map(str::to_string))
                    });
                if let Some(base) = base {
                    let relation_type = if access_public {
                        RelationType::Extends
                    } else {
                        RelationType::Implements
                    };
                    relations.push(relation(from, &base, relation_type, child, file_path));
                }
            }
            _ => {}
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
        metadata: serde_json::json!({ "language": "cpp" }),
        to_qualified_hint: None,
        to_type_hint: None,
    }
}

fn function_name_from_node(node: Node, source: &[u8]) -> Option<String> {
    if matches!(node.kind(), "function_definition" | "declaration") {
        if let Some(decl) = node.child_by_field_name("declarator") {
            return name_from_declarator(decl, source);
        }
    }
    None
}

fn find_parameter_list(node: Node) -> Option<Node> {
    let mut current = node.child_by_field_name("declarator")?;
    const MAX_DEPTH: usize = 512;
    for _ in 0..MAX_DEPTH {
        if current.kind() == "function_declarator" {
            return current.child_by_field_name("parameters");
        }
        match current.child_by_field_name("declarator") {
            Some(inner) => current = inner,
            None => return None,
        }
    }
    None
}

/// F3: typed parameters from `parameter_list` / `parameter_declaration`.
fn extract_parameters(node: Node, source: &[u8]) -> Result<Vec<Parameter>> {
    let mut parameters = Vec::new();
    let Some(params_node) = find_parameter_list(node) else {
        return Ok(parameters);
    };

    let mut cursor = params_node.walk();
    for child in params_node.children(&mut cursor) {
        if !matches!(
            child.kind(),
            "parameter_declaration" | "optional_parameter_declaration"
        ) {
            continue;
        }
        let param_type = child
            .child_by_field_name("type")
            .and_then(|n| n.utf8_text(source).ok().map(str::to_string));
        let name = child
            .child_by_field_name("declarator")
            .and_then(|d| name_from_declarator(d, source));
        let default_value = child
            .child_by_field_name("default_value")
            .and_then(|n| n.utf8_text(source).ok().map(str::to_string));
        if let Some(name) = name {
            parameters.push(Parameter {
                name,
                param_type,
                default_value,
            });
        }
    }
    Ok(parameters)
}

fn extract_type_fields(type_node: Node, source: &[u8]) -> Result<Vec<Field>> {
    let mut fields = Vec::new();
    let Some(body) = type_node.child_by_field_name("body") else {
        return Ok(fields);
    };
    if body.kind() != "field_declaration_list" {
        return Ok(fields);
    }

    let mut cursor = body.walk();
    for child in body.children(&mut cursor) {
        if child.kind() != "field_declaration" {
            continue;
        }
        let field_type = child
            .child_by_field_name("type")
            .and_then(|n| n.utf8_text(source).ok().map(str::to_string));

        for i in 0..child.child_count() {
            if child.field_name_for_child(i as u32) != Some("declarator") {
                continue;
            }
            let Some(declarator) = child.child(i) else {
                continue;
            };
            if let Some(name) = name_from_declarator(declarator, source) {
                fields.push(Field {
                    name,
                    field_type: field_type.clone(),
                    visibility: None,
                });
            }
        }
    }
    Ok(fields)
}

fn name_from_declarator(root: Node, source: &[u8]) -> Option<String> {
    const MAX_DEPTH: usize = 512;
    let mut stack = vec![(root, 0usize)];

    while let Some((node, depth)) = stack.pop() {
        if depth > MAX_DEPTH {
            continue;
        }

        match node.kind() {
            "identifier" | "type_identifier" | "field_identifier" | "destructor_name" => {
                return node.utf8_text(source).ok().map(str::to_string);
            }
            "qualified_identifier" => {
                return node
                    .child_by_field_name("name")
                    .and_then(|n| n.utf8_text(source).ok().map(str::to_string));
            }
            "function_declarator"
            | "pointer_declarator"
            | "reference_declarator"
            | "parenthesized_declarator"
            | "array_declarator" => {
                if let Some(inner) = node.child_by_field_name("declarator") {
                    stack.push((inner, depth + 1));
                } else {
                    let mut cursor = node.walk();
                    let children: Vec<Node> = node.children(&mut cursor).collect();
                    for child in children.into_iter().rev() {
                        if child.is_named() {
                            stack.push((child, depth + 1));
                        }
                    }
                }
            }
            _ => {
                let mut cursor = node.walk();
                let children: Vec<Node> = node.children(&mut cursor).collect();
                for child in children.into_iter().rev() {
                    if child.is_named() {
                        stack.push((child, depth + 1));
                    }
                }
            }
        }
    }
    None
}

fn type_name(node: Node, source: &[u8]) -> Option<String> {
    node.child_by_field_name("name")
        .and_then(|n| n.utf8_text(source).ok().map(str::to_string))
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_extract_cpp_class_and_method() {
        let source = br#"
#include <string>

class UserService {
public:
    std::string authenticate(const std::string& token) {
        return token;
    }
};
"#;
        let plugin = CppPlugin::new().unwrap();
        let symbols = plugin
            .extract_symbols(Path::new("UserService.cpp"), source)
            .unwrap();
        assert!(symbols.iter().any(|s| s.name == "UserService"));
        assert!(symbols.iter().any(|s| s.name == "authenticate"));
    }

    #[test]
    fn test_extract_cpp_fields_ctor_and_typed_params() {
        let source = br#"
class Order {
public:
    int orderId;

    Order(int id) : orderId(id) {}

    int get(int x) {
        return x;
    }
};
"#;
        let plugin = CppPlugin::new().unwrap();
        let symbols = plugin
            .extract_symbols(Path::new("Order.cpp"), source)
            .unwrap();

        let class = symbols
            .iter()
            .find(|s| s.name == "Order" && s.symbol_type == SymbolType::Class)
            .expect("Order class");
        assert!(
            class
                .fields
                .iter()
                .any(|f| f.name == "orderId" && f.field_type.as_deref() == Some("int")),
            "fields: {:?}",
            class.fields
        );

        let ctor = symbols
            .iter()
            .find(|s| {
                s.symbol_type == SymbolType::Function
                    && s.metadata.get("is_constructor").and_then(|v| v.as_bool()) == Some(true)
            })
            .expect("constructor");
        assert_eq!(ctor.qualified_name.as_deref(), Some("Order::<init>"));
        assert_eq!(ctor.parameters.len(), 1);
        assert_eq!(ctor.parameters[0].name, "id");
        assert_eq!(ctor.parameters[0].param_type.as_deref(), Some("int"));

        let get = symbols.iter().find(|s| s.name == "get").expect("get");
        assert_eq!(get.parameters.len(), 1);
        assert_eq!(get.parameters[0].name, "x");
        assert_eq!(get.parameters[0].param_type.as_deref(), Some("int"));
    }

    #[test]
    fn test_extract_relations_calls() {
        let source = br#"
void foo() {
    bar();
    baz(1);
}

void bar() {}
int baz(int x) { return x; }
"#;
        let plugin = CppPlugin::new().unwrap();
        let path = Path::new("example.cpp");
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
        let source = br#"class ServiceImpl : public BaseService, IService {};"#;
        let plugin = CppPlugin::new().unwrap();
        let path = Path::new("Service.cpp");
        let symbols = plugin.extract_symbols(path, source).unwrap();
        let relations = plugin.extract_relations(path, source, &symbols).unwrap();
        assert!(
            relations
                .iter()
                .any(|r| matches!(r.relation_type, RelationType::Extends)),
            "missing Extends: {relations:?}"
        );
    }
}
