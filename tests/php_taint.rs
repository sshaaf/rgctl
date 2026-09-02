//! PHP taint integration against inline fixture code.

use rgctl::analysis::{
    ProgramDependenceGraph, TaintAnalyzer, TaintSink, TaintSource, build_cfg_for_function,
};

#[test]
fn php_taint_get_to_sql_query() {
    let code = r#"<?php
function handle_request() {
    $username = $_GET['username'];
    $query = "SELECT * FROM users WHERE name = '" . $username . "'";
    mysqli_query($conn, $query);
}
"#;
    let cfg = build_cfg_for_function("php", code, "handle_request").unwrap();
    let pdg = ProgramDependenceGraph::build(&cfg, code.as_bytes()).unwrap();
    let mut analyzer = TaintAnalyzer::new(&pdg, &cfg);
    analyzer.detect_patterns("php");
    let flows = analyzer.vulnerable_flows();
    assert!(!flows.is_empty(), "expected $_GET -> mysqli_query flow");
    assert_eq!(flows[0].source_type, TaintSource::HttpParameter);
    assert_eq!(flows[0].sink_type, TaintSink::SqlQuery);
}
