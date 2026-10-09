//! Help IA: command map labels, discover flag headings, security parent.

use std::process::Command;

fn rgctl() -> Command {
    Command::new(env!("CARGO_BIN_EXE_rgctl"))
}

fn stdout(cmd: &mut Command) -> String {
    let out = cmd.output().expect("run rgctl");
    assert!(
        out.status.success(),
        "rgctl failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn top_level_help_has_grouped_command_sections() {
    let help = stdout(rgctl().arg("--help"));
    for heading in [
        "Lifecycle:",
        "Query:",
        "Analysis:",
        "Security:",
        "Policy:",
        "Meta:",
    ] {
        assert!(
            help.contains(heading),
            "expected grouped heading {heading} in --help:\n{help}"
        );
    }
    // Lifecycle section should appear before Security in the Commands list.
    let lifecycle = help.find("Lifecycle:").expect("Lifecycle:");
    let security = help.find("Security:").expect("Security:");
    assert!(lifecycle < security, "Lifecycle should appear before Security");
    let discover = help.find("discover").expect("discover");
    let vuln = help.find("  vuln").expect("vuln");
    assert!(discover < vuln, "discover should be listed before vuln");
}

#[test]
fn discover_help_groups_pipeline_flags() {
    let help = stdout(rgctl().args(["discover", "--help"]));
    for heading in [
        "Paths & filters",
        "Pipeline features",
        "Migration",
        "Session / limits",
    ] {
        assert!(
            help.contains(heading),
            "expected discover help heading {heading}:\n{help}"
        );
    }
    assert!(help.contains("--with-cfg"));
}

#[test]
fn security_parent_lists_vuln_deps_taint() {
    let help = stdout(rgctl().args(["security", "--help"]));
    assert!(help.contains("vuln"));
    assert!(help.contains("deps"));
    assert!(help.contains("taint"));
    let vuln_help = stdout(rgctl().args(["security", "vuln", "--help"]));
    let top_vuln = stdout(rgctl().args(["vuln", "--help"]));
    assert!(
        vuln_help.contains("analyze") && top_vuln.contains("analyze"),
        "security vuln and top-level vuln should both expose analyze"
    );
}

#[test]
fn gql_help_marks_experimental() {
    let help = stdout(rgctl().args(["gql", "--help"]));
    let lower = help.to_ascii_lowercase();
    assert!(
        lower.contains("experimental") || lower.contains("prefer"),
        "gql help should mark experimental / prefer structured verbs:\n{help}"
    );
}
