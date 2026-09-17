use rgctl::cli::install_output::{
    INSTALL_SCHEMA_VERSION, InstallWrite, InstallWriteKind, InstallWriteStatus,
    build_install_response,
};

#[test]
fn test_install_json_schema_sanity() {
    let response = build_install_response(
        "/tmp/repo",
        "local",
        vec!["cursor".to_string()],
        true,
        false,
        false,
        vec![InstallWrite {
            agent: "cursor".into(),
            workflow: Some("gql".into()),
            kind: InstallWriteKind::Skill,
            path: "/tmp/repo/.cursor/skills/rgctl-gql/SKILL.md".into(),
            status: InstallWriteStatus::Created,
            host: None,
        }],
    );
    let doc = serde_json::to_value(&response).expect("serialize install fixture");

    assert_eq!(
        doc.get("schema_version").and_then(|v| v.as_u64()),
        Some(INSTALL_SCHEMA_VERSION as u64)
    );
    assert_eq!(doc.get("command").and_then(|v| v.as_str()), Some("install"));
    assert_eq!(doc.get("scope").and_then(|v| v.as_str()), Some("local"));
    for key in ["repo", "force", "writes", "agents", "with_commands"] {
        assert!(doc.get(key).is_some(), "install JSON missing '{key}'");
    }
    let writes = doc
        .get("writes")
        .and_then(|v| v.as_array())
        .expect("writes must be an array");
    assert!(!writes.is_empty());
    let write = &writes[0];
    assert_eq!(write.get("agent").and_then(|v| v.as_str()), Some("cursor"));
    assert_eq!(write.get("workflow").and_then(|v| v.as_str()), Some("gql"));
}
