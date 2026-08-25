//! Definition and use extraction from tree-sitter AST nodes.

use std::collections::HashSet;
use tree_sitter::Node;

/// Extract variables defined and used in a statement node.
pub fn extract_def_use(node: Node, source: &[u8]) -> (HashSet<String>, HashSet<String>) {
    let mut defined = HashSet::new();
    let mut used = HashSet::new();
    collect_def_use(node, source, &mut defined, &mut used, false);
    (defined, used)
}

fn is_field_access_kind(kind: &str) -> bool {
    matches!(
        kind,
        "field_access"
            | "field_expression"
            | "member_expression"
            | "member_access_expression"
            | "selector_expression"
            | "attribute"
    )
}

/// Build `base.member` (or full node text) for a field-access style AST node.
fn field_access_compound(node: Node, source: &[u8]) -> Option<String> {
    let field = node
        .child_by_field_name("field")
        .or_else(|| node.child_by_field_name("property"))
        .or_else(|| node.child_by_field_name("attribute"))
        .or_else(|| node.child_by_field_name("name"));
    let object = node
        .child_by_field_name("object")
        .or_else(|| node.child_by_field_name("argument"))
        .or_else(|| node.child_by_field_name("value"))
        .or_else(|| node.child_by_field_name("expression"));
    match (object, field) {
        (Some(obj), Some(fld)) => {
            let obj_txt = obj.utf8_text(source).ok()?;
            let fld_txt = fld.utf8_text(source).ok()?;
            Some(format!("{obj_txt}.{fld_txt}"))
        }
        _ => node.utf8_text(source).ok().map(|s| s.to_string()),
    }
}

fn collect_field_access_base_uses(node: Node, source: &[u8], used: &mut HashSet<String>) {
    if let Some(object) = node
        .child_by_field_name("object")
        .or_else(|| node.child_by_field_name("argument"))
        .or_else(|| node.child_by_field_name("value"))
        .or_else(|| node.child_by_field_name("expression"))
    {
        collect_def_use(object, source, &mut HashSet::new(), used, false);
    }
}

fn collect_assignment_lhs(
    left: Node,
    source: &[u8],
    defined: &mut HashSet<String>,
    used: &mut HashSet<String>,
) {
    if is_field_access_kind(left.kind()) {
        if let Some(compound) = field_access_compound(left, source) {
            defined.insert(compound);
        }
        collect_field_access_base_uses(left, source, used);
    } else {
        collect_pattern_defs(left, source, defined);
    }
}

/// Go `var_spec`: `name` / `name_list` + optional `value` / `value_list`.
fn collect_go_var_spec(
    spec: Node,
    source: &[u8],
    defined: &mut HashSet<String>,
    used: &mut HashSet<String>,
) {
    if let Some(name) = spec.child_by_field_name("name") {
        collect_pattern_defs(name, source, defined);
    }
    let mut cursor = spec.walk();
    for child in spec.children(&mut cursor) {
        match child.kind() {
            "identifier" => {
                collect_pattern_defs(child, source, defined);
            }
            "expression_list" => {
                // Could be names or values — tree-sitter-go uses name field when present.
                if spec.child_by_field_name("name").is_none()
                    && spec.child_by_field_name("value").is_none()
                {
                    // Fallback: first expression_list is often names in older grammars
                }
                collect_def_use(child, source, defined, used, false);
            }
            _ => {}
        }
    }
    if let Some(value) = spec
        .child_by_field_name("value")
        .or_else(|| spec.child_by_field_name("right"))
    {
        collect_def_use(value, source, defined, used, false);
    }
}

fn collect_def_use(
    node: Node,
    source: &[u8],
    defined: &mut HashSet<String>,
    used: &mut HashSet<String>,
    is_def_target: bool,
) {
    let kind = node.kind();

    match kind {
        // Rust
        "let_declaration" | "let_statement" => {
            if let Some(pattern) = node.child_by_field_name("pattern") {
                collect_pattern_defs(pattern, source, defined);
            }
            if let Some(value) = node.child_by_field_name("value") {
                collect_def_use(value, source, defined, used, false);
            }
        }
        "assignment_expression" | "augmented_assignment_expression" => {
            if let Some(left) = node.child_by_field_name("left") {
                collect_assignment_lhs(left, source, defined, used);
            }
            if let Some(right) = node.child_by_field_name("right") {
                collect_def_use(right, source, defined, used, false);
            }
        }
        "compound_assignment_expr" => {
            if let Some(left) = node.child_by_field_name("left") {
                collect_assignment_lhs(left, source, defined, used);
            }
            if let Some(right) = node.child_by_field_name("right") {
                collect_def_use(right, source, defined, used, false);
            }
        }

        // Python
        "assignment" => {
            if let Some(left) = node.child_by_field_name("left") {
                collect_assignment_lhs(left, source, defined, used);
            }
            if let Some(right) = node.child_by_field_name("right") {
                collect_def_use(right, source, defined, used, false);
            }
        }
        "augmented_assignment" => {
            if let Some(left) = node.child_by_field_name("left") {
                collect_assignment_lhs(left, source, defined, used);
                collect_def_use(left, source, defined, used, false);
            }
            if let Some(right) = node.child_by_field_name("right") {
                collect_def_use(right, source, defined, used, false);
            }
        }
        "for_statement" => {
            if let Some(left) = node.child_by_field_name("left") {
                collect_pattern_defs(left, source, defined);
            }
            if let Some(init) = node.child_by_field_name("initializer") {
                collect_def_use(init, source, defined, used, false);
            }
            if let Some(body) = node.child_by_field_name("body") {
                collect_def_use(body, source, defined, used, false);
            }
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if matches!(child.kind(), "range_clause" | "for_clause") {
                    collect_def_use(child, source, defined, used, false);
                }
            }
        }

        // Go
        "short_var_declaration" | "assignment_statement" => {
            if let Some(left) = node.child_by_field_name("left") {
                collect_assignment_lhs(left, source, defined, used);
            }
            if let Some(right) = node.child_by_field_name("right") {
                collect_def_use(right, source, defined, used, false);
            }
        }
        // `var` / `var ( ... )` — children are var_spec / var_spec_list (no left/right fields).
        "var_declaration" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                match child.kind() {
                    "var_spec" => collect_go_var_spec(child, source, defined, used),
                    "var_spec_list" => {
                        let mut c2 = child.walk();
                        for spec in child.children(&mut c2) {
                            if spec.kind() == "var_spec" {
                                collect_go_var_spec(spec, source, defined, used);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        "range_clause" => {
            if let Some(left) = node.child_by_field_name("left") {
                collect_pattern_defs(left, source, defined);
            }
            if let Some(right) = node.child_by_field_name("right") {
                collect_def_use(right, source, defined, used, false);
            }
        }
        "expression_list" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.is_named() {
                    if is_def_target {
                        collect_pattern_defs(child, source, defined);
                    } else {
                        collect_pattern_defs(child, source, defined);
                        collect_def_use(child, source, defined, used, false);
                    }
                }
            }
        }
        "parameter_declaration" => {
            if let Some(name) = node.child_by_field_name("name") {
                collect_pattern_defs(name, source, defined);
            }
        }

        // C# / Java / JS / TS local declarations
        "variable_declaration"
        | "local_declaration_statement"
        | "local_variable_declaration"
        | "lexical_declaration"
        | "variable_declaration_list" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "variable_declarator" {
                    if let Some(name) = child.child_by_field_name("name") {
                        collect_pattern_defs(name, source, defined);
                    }
                    if let Some(value) = child.child_by_field_name("value") {
                        collect_def_use(value, source, defined, used, false);
                    }
                } else if matches!(
                    child.kind(),
                    "variable_declaration" | "variable_declaration_list"
                ) {
                    collect_def_use(child, source, defined, used, false);
                }
            }
        }
        "variable_declarator" => {
            if let Some(name) = node.child_by_field_name("name") {
                collect_pattern_defs(name, source, defined);
            }
            if let Some(value) = node.child_by_field_name("value") {
                collect_def_use(value, source, defined, used, false);
            }
        }

        // C
        "declaration" => {
            if let Some(decl) = node.child_by_field_name("declarator") {
                collect_declarator_defs(decl, source, defined);
            }
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "init_declarator" {
                    if let Some(name) = child.child_by_field_name("declarator") {
                        collect_declarator_defs(name, source, defined);
                    }
                    if let Some(value) = child.child_by_field_name("value") {
                        collect_def_use(value, source, defined, used, false);
                    }
                }
            }
        }
        "init_declarator" => {
            if let Some(name) = node.child_by_field_name("declarator") {
                collect_declarator_defs(name, source, defined);
            }
            if let Some(value) = node.child_by_field_name("value") {
                collect_def_use(value, source, defined, used, false);
            }
        }

        // Field / member access (Java field_access, Rust field_expression, …)
        k if is_field_access_kind(k) => {
            if is_def_target {
                if let Some(compound) = field_access_compound(node, source) {
                    defined.insert(compound);
                }
                collect_field_access_base_uses(node, source, used);
            } else {
                collect_field_access_base_uses(node, source, used);
                if let Some(compound) = field_access_compound(node, source) {
                    used.insert(compound);
                }
            }
        }

        // Shared identifiers
        "identifier" | "shorthand_field_identifier" | "field_identifier" | "type_identifier" => {
            if is_def_target {
                if let Ok(name) = node.utf8_text(source) {
                    if kind == "identifier" || kind == "shorthand_field_identifier" {
                        defined.insert(name.to_string());
                    }
                }
            } else if kind == "identifier" || kind == "shorthand_field_identifier" {
                if let Ok(name) = node.utf8_text(source) {
                    used.insert(name.to_string());
                }
            }
        }

        "scoped_identifier" => {
            if let Some(name) = node.child_by_field_name("name") {
                if is_def_target {
                    collect_pattern_defs(name, source, defined);
                } else {
                    collect_def_use(name, source, defined, used, false);
                }
            }
        }

        _ if is_def_target && is_binding_pattern(kind) => {
            collect_pattern_defs(node, source, defined);
        }

        _ => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                collect_def_use(child, source, defined, used, false);
            }
        }
    }
}

fn is_binding_pattern(kind: &str) -> bool {
    matches!(
        kind,
        "identifier"
            | "shorthand_field_identifier"
            | "tuple_pattern"
            | "tuple_struct_pattern"
            | "struct_pattern"
            | "pattern"
            | "list_pattern"
            | "attribute"
            | "rest_pattern"
            | "wildcard_pattern"
    )
}

fn collect_declarator_defs(node: Node, source: &[u8], defined: &mut HashSet<String>) {
    match node.kind() {
        "identifier" => {
            if let Ok(name) = node.utf8_text(source) {
                defined.insert(name.to_string());
            }
        }
        "pointer_declarator"
        | "function_declarator"
        | "array_declarator"
        | "parenthesized_declarator" => {
            if let Some(inner) = node.child_by_field_name("declarator") {
                collect_declarator_defs(inner, source, defined);
            }
        }
        _ => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.is_named() {
                    collect_declarator_defs(child, source, defined);
                }
            }
        }
    }
}

fn collect_pattern_defs(node: Node, source: &[u8], defined: &mut HashSet<String>) {
    match node.kind() {
        "identifier" | "shorthand_field_identifier" => {
            if let Ok(name) = node.utf8_text(source) {
                defined.insert(name.to_string());
            }
        }
        "tuple_pattern"
        | "tuple_struct_pattern"
        | "struct_pattern"
        | "pattern"
        | "list_pattern"
        | "attribute"
        | "rest_pattern" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                collect_pattern_defs(child, source, defined);
            }
        }
        _ => {
            collect_def_use(node, source, defined, &mut HashSet::new(), true);
        }
    }
}

/// Collect all identifier uses under a subtree.
pub fn extract_used_variables(node: Node, source: &[u8]) -> HashSet<String> {
    let (_, used) = extract_def_use(node, source);
    used
}

#[cfg(test)]
mod tests {
    use super::*;
    use tree_sitter::Parser;

    fn parse_rust(source: &str) -> tree_sitter::Tree {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .unwrap();
        parser.parse(source, None).unwrap()
    }

    fn parse_python(source: &str) -> tree_sitter::Tree {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_python::LANGUAGE.into())
            .unwrap();
        parser.parse(source, None).unwrap()
    }

    fn parse_go(source: &str) -> tree_sitter::Tree {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_go::LANGUAGE.into())
            .unwrap();
        parser.parse(source, None).unwrap()
    }

    fn find_kind<'a>(node: tree_sitter::Node<'a>, kind: &str) -> Option<tree_sitter::Node<'a>> {
        if node.kind() == kind {
            return Some(node);
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if let Some(found) = find_kind(child, kind) {
                return Some(found);
            }
        }
        None
    }

    #[test]
    fn test_rust_let_def_use() {
        let source = "fn f(a: i32) { let x = a + 1; x }";
        let tree = parse_rust(source);
        let let_node = find_kind(tree.root_node(), "let_declaration").unwrap();
        let (defs, uses) = extract_def_use(let_node, source.as_bytes());
        assert!(defs.contains("x"));
        assert!(uses.contains("a"));
    }

    #[test]
    fn test_python_assignment_def_use() {
        let source = "def f(a):\n    x = a + 1\n    return x\n";
        let tree = parse_python(source);
        let assign = find_kind(tree.root_node(), "assignment").unwrap();
        let (defs, uses) = extract_def_use(assign, source.as_bytes());
        assert!(defs.contains("x"));
        assert!(uses.contains("a"));
    }

    #[test]
    fn test_identifier_use_only() {
        let source = "fn f() { x + y }";
        let tree = parse_rust(source);
        let root = tree.root_node();
        let uses = extract_used_variables(root, source.as_bytes());
        assert!(uses.contains("x"));
        assert!(uses.contains("y"));
    }

    #[test]
    fn test_go_short_var_and_range_def_use() {
        let source = "package demo\nfunc f(m map[string]int) {\n    x := 1\n    for k, v := range m {\n        use(k, v, x)\n    }\n}\n";
        let tree = parse_go(source);
        let assign = find_kind(tree.root_node(), "short_var_declaration").unwrap();
        let (defs, _uses) = extract_def_use(assign, source.as_bytes());
        assert!(defs.contains("x"));

        let for_node = find_kind(tree.root_node(), "for_statement").unwrap();
        let (for_defs, for_uses) = extract_def_use(for_node, source.as_bytes());
        assert!(
            for_defs.contains("k") || for_defs.contains("v"),
            "for defs: {for_defs:?}"
        );
        assert!(for_uses.contains("m"));
    }

    #[test]
    fn test_java_field_assignment_def_use() {
        let source = "class C { void m(OrderDTO order) { order.status = \"X\"; } }";
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_java::LANGUAGE.into())
            .unwrap();
        let tree = parser.parse(source, None).unwrap();
        let assign = find_kind(tree.root_node(), "assignment_expression").expect("assignment");
        let (defs, uses) = extract_def_use(assign, source.as_bytes());
        assert!(
            defs.contains("order.status"),
            "defs should include order.status, got {defs:?}"
        );
        assert!(
            uses.contains("order"),
            "uses should include order, got {uses:?}"
        );
    }

    #[test]
    fn test_java_local_variable_declaration_def_use() {
        let source = "class C { void m(OrderDTO order) { OrderDTO other = order; } }";
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_java::LANGUAGE.into())
            .unwrap();
        let tree = parser.parse(source, None).unwrap();
        let decl = find_kind(tree.root_node(), "local_variable_declaration").expect("local decl");
        let (defs, uses) = extract_def_use(decl, source.as_bytes());
        assert!(
            defs.contains("other"),
            "defs should include other, got {defs:?}"
        );
        assert!(
            uses.contains("order"),
            "uses should include order, got {uses:?}"
        );
        assert!(
            !uses.contains("other"),
            "declarator name must not be a use, got {uses:?}"
        );
    }
}
