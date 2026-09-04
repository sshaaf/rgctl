//! Generic extraction helpers for language plugins

pub mod complexity;
pub mod ecmascript;
pub mod python;
pub mod tree_sitter;

pub use complexity::ComplexityCalculator;
pub use ecmascript::{
    extract_cjs_require_symbols, extract_class_extends_relations, extract_import_symbols,
    find_child_kind, simple_type_name, type_name_from_node,
};
pub use python::{
    containing_class_name, decorator_name_and_args, decorators_for_node,
    extract_class_extends_relations as extract_python_class_extends_relations,
    extract_decorator_relations, extract_import_symbols as extract_python_import_symbols,
    python_simple_type_name, source_location as python_source_location,
};
pub use tree_sitter::{
    extract_name_from_node, extract_parameters_generic, extract_symbols_by_kinds, node_to_location,
    parse_source, symbol_type_for_kind,
};
