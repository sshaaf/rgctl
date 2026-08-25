use rgctl::cli::install_output::{
    INSTALL_SCHEMA_VERSION, InstallWrite, InstallWriteHost, InstallWriteStatus,
    build_install_response,
};

#[test]
fn test_install_json_schema_sanity() {
    let response = build_install_response(
        "/tmp/repo",
        false,
        vec![InstallWrite {
            host: InstallWriteHost::Claude,
            path: "/tmp/repo/.claude/skills/rgctl/SKILL.md".into(),
            status: InstallWriteStatus::Created,
        }],
    );
    let doc = serde_json::to_value(&response).expect("serialize install fixture");

    assert_eq!(
        doc.get("schema_version").and_then(|v| v.as_u64()),
        Some(INSTALL_SCHEMA_VERSION as u64)
    );
    assert_eq!(doc.get("command").and_then(|v| v.as_str()), Some("install"));
    assert_eq!(doc.get("skill").and_then(|v| v.as_str()), Some("rgctl"));
    for key in ["repo", "force", "writes"] {
        assert!(doc.get(key).is_some(), "install JSON missing '{key}'");
    }
    let writes = doc
        .get("writes")
        .and_then(|v| v.as_array())
        .expect("writes must be an array");
    assert!(!writes.is_empty());
    let write = &writes[0];
    assert_eq!(write.get("host").and_then(|v| v.as_str()), Some("claude"));
    assert_eq!(
        write.get("status").and_then(|v| v.as_str()),
        Some("created")
    );
    assert!(write.get("path").and_then(|v| v.as_str()).is_some());
}
