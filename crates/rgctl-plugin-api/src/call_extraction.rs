//! Shared helpers for extracting `Calls` relations from tree-sitter ASTs.

use crate::{Relation, RelationType, SourceLocation, Symbol, SymbolType};
use std::path::Path;
use tree_sitter::Node;

/// Call node kinds per language grammar.
pub const RUST_CALL_KINDS: &[&str] = &["call_expression", "macro_invocation"];
pub const GO_CALL_KINDS: &[&str] = &["call_expression"];
pub const PYTHON_CALL_KINDS: &[&str] = &["call"];
pub const CSHARP_CALL_KINDS: &[&str] = &["invocation_expression"];
pub const C_CALL_KINDS: &[&str] = &["call_expression"];
pub const CPP_CALL_KINDS: &[&str] = &["call_expression"];
pub const JS_CALL_KINDS: &[&str] = &["call_expression"];
pub const TS_CALL_KINDS: &[&str] = &["call_expression"];

/// Find the innermost function symbol containing `node`.
///
/// `function_symbols` SHOULD already be filtered to [`SymbolType::Function`].
pub fn containing_function<'a>(node: Node, function_symbols: &[&'a Symbol]) -> Option<&'a Symbol> {
    let line = node.start_position().row + 1;
    function_symbols
        .iter()
        .copied()
        .filter(|s| line >= s.location.start_line && line <= s.location.end_line)
        .min_by_key(|s| s.location.end_line - s.location.start_line)
}

/// Best-effort callee name from a call expression subtree (iterative; bounded depth).
pub fn callee_name(root: Node, source: &[u8]) -> Option<String> {
    const MAX_DEPTH: usize = 512;
    let mut stack = vec![(root, 0usize)];

    while let Some((node, depth)) = stack.pop() {
        if depth > MAX_DEPTH {
            continue;
        }

        match node.kind() {
            // Go method/field selectors use `field_identifier` (not `identifier`).
            "identifier" | "type_identifier" | "field_identifier" | "property_identifier" => {
                return node.utf8_text(source).ok().map(str::to_string);
            }
            "field_expression" | "selector_expression" | "attribute" | "member_expression" => {
                if let Some(n) = node
                    .child_by_field_name("field")
                    .or_else(|| node.child_by_field_name("attribute"))
                    .or_else(|| node.child_by_field_name("property"))
                    .or_else(|| node.child_by_field_name("name"))
                {
                    stack.push((n, depth + 1));
                }
            }
            "scoped_identifier" | "qualified_type" => {
                if let Some(n) = node.child_by_field_name("name") {
                    stack.push((n, depth + 1));
                }
            }
            "parenthesized_expression" => {
                if let Some(inner) = node.named_child(0) {
                    stack.push((inner, depth + 1));
                }
            }
            "invocation_expression" => {
                if let Some(n) = node.named_child(0) {
                    stack.push((n, depth + 1));
                }
            }
            _ => {
                if let Some(func) = node.child_by_field_name("function") {
                    stack.push((func, depth + 1));
                } else if let Some(name) = node.child_by_field_name("name") {
                    stack.push((name, depth + 1));
                }
            }
        }
    }
    None
}

/// Push a `Calls` relation when `node` is a recognized call site.
pub fn push_call_relation(
    node: Node,
    source: &[u8],
    file_path: &Path,
    symbols: &[Symbol],
    function_symbols: &[&Symbol],
    call_kinds: &[&str],
    language: &str,
    relations: &mut Vec<Relation>,
) {
    if !call_kinds.contains(&node.kind()) {
        return;
    }

    let callee = node
        .child_by_field_name("function")
        .or_else(|| node.child_by_field_name("macro"))
        .or_else(|| node.child_by_field_name("name"))
        .and_then(|n| callee_name(n, source))
        .or_else(|| callee_name(node, source));

    let Some(callee) = callee else {
        return;
    };
    if callee.is_empty() {
        return;
    }

    let Some(from_fn) = containing_function(node, function_symbols) else {
        return;
    };

    let from = from_fn
        .qualified_name
        .clone()
        .unwrap_or_else(|| from_fn.name.clone());

    let (to_type_hint, to_qualified_hint) = if language == "go" {
        let ty = go_call_type_hint(node, source, symbols, from_fn);
        let qh = ty.as_ref().map(|t| format!("{t}.{callee}"));
        (ty, qh)
    } else {
        (None, None)
    };

    let mut meta = serde_json::json!({ "language": language });
    if language == "go" {
        if let Some((recv_ty, field)) = go_field_selector_meta(node, source, from_fn) {
            meta["go_recv_type"] = serde_json::Value::String(recv_ty);
            meta["go_field"] = serde_json::Value::String(field);
            meta["go_callee"] = serde_json::Value::String(callee.clone());
        }
    }

    // Prefer a unique same-file match; if ambiguous, keep bare name and rely on hints.
    let same_file_matches: Vec<_> = symbols
        .iter()
        .filter(|s| {
            s.name == callee
                && s.symbol_type == SymbolType::Function
                && s.location.file == file_path.to_string_lossy()
        })
        .collect();
    let local_target = match same_file_matches.as_slice() {
        [only] => only
            .qualified_name
            .clone()
            .unwrap_or_else(|| callee.clone()),
        _ => to_qualified_hint.clone().unwrap_or_else(|| callee.clone()),
    };

    relations.push(Relation {
        from,
        to: local_target,
        relation_type: RelationType::Calls,
        location: SourceLocation {
            file: file_path.to_string_lossy().to_string(),
            start_line: node.start_position().row + 1,
            end_line: node.end_position().row + 1,
            start_column: node.start_position().column,
            end_column: node.end_position().column,
        },
        metadata: meta,
        to_qualified_hint,
        to_type_hint,
    });
}

/// `recv.field.Method` → (receiver_type, field_name) for late resolution in GraphBuilder.
fn go_field_selector_meta(call: Node, source: &[u8], from_fn: &Symbol) -> Option<(String, String)> {
    let func = call.child_by_field_name("function")?;
    if func.kind() != "selector_expression" {
        return None;
    }
    let operand = func.child_by_field_name("operand")?;
    if operand.kind() != "selector_expression" {
        return None;
    }
    let inner_op = operand.child_by_field_name("operand")?;
    let field = operand.child_by_field_name("field")?;
    if inner_op.kind() != "identifier" {
        return None;
    }
    let recv_name = inner_op.utf8_text(source).ok()?;
    let field_name = field.utf8_text(source).ok()?.to_string();
    let recv_ok = from_fn
        .metadata
        .get("receiver_name")
        .and_then(|v| v.as_str())
        == Some(recv_name);
    if !recv_ok {
        return None;
    }
    let recv_ty = from_fn
        .metadata
        .get("receiver_type")
        .and_then(|v| v.as_str())?
        .trim_start_matches('*')
        .to_string();
    Some((recv_ty, field_name))
}

/// Best-effort Go receiver/field type for `x.Method` / `x.field.Method` call sites.
fn go_call_type_hint(
    call: Node,
    source: &[u8],
    symbols: &[Symbol],
    from_fn: &Symbol,
) -> Option<String> {
    let func = call.child_by_field_name("function")?;
    if func.kind() != "selector_expression" {
        return None;
    }
    let operand = func.child_by_field_name("operand")?;

    // `recv.Method` where recv is the method receiver variable.
    if operand.kind() == "identifier" {
        let recv_name = operand.utf8_text(source).ok()?;
        if let Some(rt) = from_fn
            .metadata
            .get("receiver_name")
            .and_then(|v| v.as_str())
        {
            if rt == recv_name {
                return from_fn
                    .metadata
                    .get("receiver_type")
                    .and_then(|v| v.as_str())
                    .map(|s| s.trim_start_matches('*').to_string());
            }
        }
        return None;
    }

    // `recv.field.Method` — resolve `field` on the receiver struct type.
    if operand.kind() == "selector_expression" {
        let inner_op = operand.child_by_field_name("operand")?;
        let field = operand.child_by_field_name("field")?;
        if inner_op.kind() != "identifier" {
            return None;
        }
        let recv_name = inner_op.utf8_text(source).ok()?;
        let field_name = field.utf8_text(source).ok()?;
        let recv_ok = from_fn
            .metadata
            .get("receiver_name")
            .and_then(|v| v.as_str())
            == Some(recv_name);
        if !recv_ok {
            return None;
        }
        let recv_ty = from_fn
            .metadata
            .get("receiver_type")
            .and_then(|v| v.as_str())?
            .trim_start_matches('*');
        let owner = symbols.iter().find(|s| {
            matches!(
                s.symbol_type,
                SymbolType::Struct
                    | SymbolType::Class
                    | SymbolType::Interface
                    | SymbolType::Annotation
            ) && s.name == recv_ty
        })?;
        if owner.symbol_type == SymbolType::Interface || owner.symbol_type == SymbolType::Annotation
        {
            return Some(owner.name.clone());
        }
        let ft = owner
            .fields
            .iter()
            .find(|f| f.name == field_name)?
            .field_type
            .as_deref()?;
        return Some(go_simple_type_name(ft));
    }

    None
}

/// `*pkg.Type` / `pkg.Type` → `Type` for Go resolution indexes.
fn go_simple_type_name(ty: &str) -> String {
    ty.trim_start_matches('*')
        .rsplit(['.', '/'])
        .next()
        .unwrap_or(ty)
        .to_string()
}

/// Iterative tree walk that records call relations (heap stack; bounded depth).
pub fn walk_calls(
    root: Node,
    source: &[u8],
    file_path: &Path,
    symbols: &[Symbol],
    call_kinds: &[&str],
    language: &str,
    relations: &mut Vec<Relation>,
) {
    const MAX_DEPTH: usize = 2048;
    let function_symbols: Vec<&Symbol> = symbols
        .iter()
        .filter(|s| s.symbol_type == SymbolType::Function)
        .collect();
    let mut stack = vec![(root, 0usize)];

    while let Some((node, depth)) = stack.pop() {
        if depth > MAX_DEPTH {
            tracing::warn!(
                file = ?file_path,
                depth = depth,
                "AST depth limit exceeded during walk_calls; skipping deep branches"
            );
            continue;
        }

        push_call_relation(
            node,
            source,
            file_path,
            symbols,
            &function_symbols,
            call_kinds,
            language,
            relations,
        );

        let child_count = node.child_count();
        for i in (0..child_count).rev() {
            if let Some(child) = node.child(i) {
                stack.push((child, depth + 1));
            }
        }
    }
}
