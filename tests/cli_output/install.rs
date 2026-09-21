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

#[test]
fn test_install_json_schema_hosts_serialization() {
    for (agent, expected) in [
        ("claude", "claude"),
        ("codex", "codex"),
        ("agents", "codex"),
        ("cursor", "cursor"),
    ] {
        let host = rgctl::cli::install_output::host_compat(agent);
        let response = build_install_response(
            "/tmp/repo",
            "local",
            vec![agent.to_string()],
            false,
            false,
            false,
            vec![InstallWrite {
                agent: agent.into(),
                workflow: None,
                kind: InstallWriteKind::Skill,
                path: format!("/tmp/repo/skills/{expected}/SKILL.md"),
                status: InstallWriteStatus::Created,
                host,
            }],
        );
        let doc = serde_json::to_value(&response).expect("serialize install fixture");
        let writes = doc["writes"].as_array().expect("writes array");
        assert_eq!(writes[0]["host"].as_str(), Some(expected));
    }

    // Antigravity does not expand the legacy host schema; it serializes with agent only.
    assert_eq!(rgctl::cli::install_output::host_compat("antigravity"), None);
    let response = build_install_response(
        "/tmp/repo",
        "local",
        vec!["antigravity".to_string()],
        false,
        false,
        false,
        vec![InstallWrite {
            agent: "antigravity".into(),
            workflow: None,
            kind: InstallWriteKind::Skill,
            path: "/tmp/repo/.agent/skills/rgctl/SKILL.md".into(),
            status: InstallWriteStatus::Created,
            host: rgctl::cli::install_output::host_compat("antigravity"),
        }],
    );
    let doc = serde_json::to_value(&response).expect("serialize install fixture");
    let writes = doc["writes"].as_array().expect("writes array");
    assert_eq!(writes[0]["agent"].as_str(), Some("antigravity"));
    assert!(writes[0].get("host").is_none());
}
