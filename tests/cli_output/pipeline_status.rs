use rgctl::cli::pipeline_status::{
    PIPELINE_STATUS_SCHEMA_VERSION, STAGE_BASIC, STAGE_DEEP, STAGE_SEMANTIC,
    fixture_pipeline_status_json,
};

#[test]
fn test_pipeline_status_json_schema_sanity() {
    let doc = fixture_pipeline_status_json();
    assert_eq!(
        doc.get("schema_version").and_then(|v| v.as_u64()),
        Some(PIPELINE_STATUS_SCHEMA_VERSION as u64)
    );
    assert_eq!(
        doc.get("command").and_then(|v| v.as_str()),
        Some("pipeline_status")
    );
    let plan = doc.get("plan").and_then(|v| v.as_array()).expect("plan");
    assert_eq!(plan.len(), 3);
    assert_eq!(plan[0]["id"], STAGE_BASIC);
    assert_eq!(plan[1]["id"], STAGE_DEEP);
    assert_eq!(plan[2]["id"], STAGE_SEMANTIC);
    assert!(
        doc.get("dashboard_ready")
            .and_then(|v| v.as_bool())
            .is_some()
    );
    assert!(doc.get("cfg_ready").and_then(|v| v.as_bool()).is_some());
    assert!(
        doc.get("semantic_ready")
            .and_then(|v| v.as_bool())
            .is_some()
    );
}
