//! User-guide §16 + VHS tape workflow — all Tier-1 `rgctl-tests/ecommerce-*` projects.
//!
//! Each run starts a dedicated daemon under a temp `--daemon-home`, routes discover/gql/
//! blast/metrics/check through it, and stops the daemon when the session drops.

use super::rgctl_harness::{assert_ok, reserve_port, DaemonGuard, rgctl};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

pub struct SliceSpec {
    pub file: &'static str,
    pub line: &'static str,
    pub variable: &'static str,
    pub function: &'static str,
}

pub struct UserGuideProject {
    pub id: &'static str,
    pub dir_name: &'static str,
    pub languages: &'static str,
    pub exclude: &'static str,
    /// Tape step: primary blast-radius target.
    pub blast_primary: &'static str,
    /// GQL `CALLS` callee name (`WHERE b.name = …`).
    pub calls_callee: &'static str,
    /// CoolStore / hybrid CPG blast target.
    pub blast_coolstore: &'static str,
    /// `inspect <fn> cfg`
    pub inspect_function: &'static str,
    /// `cpg mutations --type …`
    pub cpg_mutation_type: &'static str,
    /// `export --query …`
    pub export_query: &'static str,
    pub slice: Option<SliceSpec>,
    /// Minimum body lines after the mutations header (0 = success only).
    pub cpg_mutations_min_body_lines: usize,
}

pub const PROJECTS: &[UserGuideProject] = &[
    UserGuideProject {
        id: "java",
        dir_name: "ecommerce-java",
        languages: "java",
        exclude: "target,data",
        blast_primary: "CartService::clearCart",
        calls_callee: "clearCart",
        blast_coolstore: "ShoppingCartService::priceShoppingCart",
        inspect_function: "checkout",
        cpg_mutation_type: "ShoppingCart",
        export_query: "name:clearCart",
        slice: Some(SliceSpec {
            file: "src/main/java/com/example/ecommerce/service/CartService.java",
            line: "53",
            variable: "item",
            function: "addItem",
        }),
        cpg_mutations_min_body_lines: 1,
    },
    UserGuideProject {
        id: "rust",
        dir_name: "ecommerce-rust",
        languages: "rust",
        exclude: "target",
        blast_primary: "src/services/order.rs::checkout",
        calls_callee: "checkout",
        blast_coolstore: "price_shopping_cart",
        inspect_function: "checkout",
        cpg_mutation_type: "ShoppingCart",
        export_query: "name:checkout",
        slice: Some(SliceSpec {
            file: "src/services/order.rs",
            line: "16",
            variable: "total",
            function: "checkout",
        }),
        cpg_mutations_min_body_lines: 1,
    },
    UserGuideProject {
        id: "python",
        dir_name: "ecommerce-python",
        languages: "python",
        exclude: ".venv,__pycache__",
        blast_primary: "app/services/order.py::checkout",
        calls_callee: "checkout",
        blast_coolstore: "price_shopping_cart",
        inspect_function: "checkout",
        cpg_mutation_type: "ShoppingCart",
        export_query: "name:checkout",
        slice: None,
        cpg_mutations_min_body_lines: 1,
    },
    UserGuideProject {
        id: "go",
        dir_name: "ecommerce-go",
        languages: "go",
        exclude: "vendor",
        blast_primary: "internal/service/order.go::Checkout",
        calls_callee: "Checkout",
        blast_coolstore: "PriceShoppingCart",
        inspect_function: "Checkout",
        cpg_mutation_type: "ShoppingCart",
        export_query: "name:Checkout",
        slice: None,
        cpg_mutations_min_body_lines: 1,
    },
    UserGuideProject {
        id: "csharp",
        dir_name: "ecommerce-csharp",
        languages: "csharp",
        exclude: "bin,obj,data",
        blast_primary: "ClearCartAsync",
        calls_callee: "GetUserCartAsync",
        blast_coolstore: "PriceShoppingCart",
        inspect_function: "CheckoutAsync",
        cpg_mutation_type: "ShoppingCart",
        export_query: "name:ClearCartAsync",
        slice: None,
        cpg_mutations_min_body_lines: 1,
    },
    UserGuideProject {
        id: "typescript",
        dir_name: "ecommerce-typescript",
        languages: "typescript",
        exclude: "node_modules,dist",
        blast_primary: "clearCart",
        calls_callee: "clearCart",
        blast_coolstore: "priceShoppingCart",
        inspect_function: "checkout",
        cpg_mutation_type: "ShoppingCart",
        export_query: "name:clearCart",
        slice: None,
        cpg_mutations_min_body_lines: 0,
    },
    UserGuideProject {
        id: "javascript",
        dir_name: "ecommerce-javascript",
        languages: "javascript",
        exclude: "node_modules",
        blast_primary: "clearCart",
        calls_callee: "clearCart",
        blast_coolstore: "priceShoppingCart",
        inspect_function: "checkout",
        cpg_mutation_type: "ShoppingCart",
        export_query: "name:clearCart",
        slice: None,
        cpg_mutations_min_body_lines: 0,
    },
    UserGuideProject {
        id: "c",
        dir_name: "ecommerce-c",
        languages: "c",
        exclude: "build,cmake-build-debug,.rgctl",
        blast_primary: "src/coolstore/services/shopping_cart_service.c::price_shopping_cart",
        calls_callee: "price_shopping_cart",
        blast_coolstore: "src/coolstore/services/shopping_cart_service.c::price_shopping_cart",
        inspect_function: "price_shopping_cart",
        cpg_mutation_type: "ShoppingCart",
        export_query: "name:price_shopping_cart",
        slice: None,
        cpg_mutations_min_body_lines: 0,
    },
    UserGuideProject {
        id: "cpp",
        dir_name: "ecommerce-cpp",
        languages: "cpp",
        exclude: "build,cmake-build-debug,.rgctl",
        blast_primary: "src/coolstore/services/shopping_cart_service.cpp::priceShoppingCart",
        calls_callee: "priceShoppingCart",
        blast_coolstore: "src/coolstore/services/shopping_cart_service.cpp::priceShoppingCart",
        inspect_function: "priceShoppingCart",
        cpg_mutation_type: "ShoppingCart",
        export_query: "name:priceShoppingCart",
        slice: None,
        cpg_mutations_min_body_lines: 1,
    },
];

pub fn rgctl_tests_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("rgctl-tests")
}

pub fn policy_file() -> PathBuf {
    rgctl_tests_root().join("rgctl-policy.json")
}

pub fn project_dir(project: &UserGuideProject) -> PathBuf {
    rgctl_tests_root().join(project.dir_name)
}

/// Isolated daemon workspace for one workflow run; stopped on drop.
pub struct WorkflowSession {
    _home_dir: tempfile::TempDir,
    daemon_home: PathBuf,
    source_repo: PathBuf,
    artifact_root: PathBuf,
    guard: DaemonGuard,
}

impl WorkflowSession {
    pub fn start(project: &UserGuideProject) -> Self {
        require_jq();
        let source_repo = project_dir(project);
        assert!(
            source_repo.is_dir(),
            "missing project dir {}",
            source_repo.display()
        );

        let home_dir = tempfile::tempdir().expect("tempdir for daemon home");
        let daemon_home = home_dir.path().to_path_buf();
        let port = reserve_port();
        let guard = DaemonGuard::new(daemon_home.clone());
        assert_ok(&guard.start_on_port(port), "daemon start");

        let disc = Command::new(rgctl())
            .current_dir(&source_repo)
            .arg("--daemon-home")
            .arg(&daemon_home)
            .args([
                "-f",
                "json",
                "discover",
                ".",
                "-l",
                project.languages,
                "-e",
                project.exclude,
                "--with-cfg",
                "--export-migration-hints",
            ])
            .output()
            .expect("discover");
        assert_ok(&disc, "discover");
        let disc_doc: Value =
            serde_json::from_slice(&disc.stdout).expect("discover JSON must parse");
        let cache = disc_doc
            .get("cache")
            .and_then(|v| v.as_str())
            .expect("discover JSON missing cache path");
        let artifact_root = PathBuf::from(cache);
        let migration = rgctl_graph::paths::artifact_path(&artifact_root, "migration_plan.json");
        assert!(
            migration.is_file(),
            "[{}] missing migration plan at {}",
            project.id,
            migration.display()
        );

        Self {
            _home_dir: home_dir,
            daemon_home,
            source_repo,
            artifact_root,
            guard,
        }
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(rgctl())
            .current_dir(&self.source_repo)
            .arg("--daemon-home")
            .arg(&self.daemon_home)
            .arg("-r")
            .arg(&self.artifact_root)
            .args(args)
            .output()
            .unwrap_or_else(|e| panic!("spawn rgctl {args:?}: {e}"))
    }

    fn run_json(&self, args: &[&str]) -> Output {
        let mut full = vec!["-f", "json"];
        full.extend_from_slice(args);
        self.run(&full)
    }

    fn assert_ok(&self, project: &UserGuideProject, step: &str, output: &Output) {
        assert_ok(output, &format!("[{}] {step}", project.id));
    }

    fn parse_json(&self, project: &UserGuideProject, step: &str, output: &Output) -> Value {
        self.assert_ok(project, step, output);
        serde_json::from_slice(&output.stdout).unwrap_or_else(|e| {
            panic!(
                "[{}] {step}: invalid JSON: {e}\nstdout: {}",
                project.id,
                String::from_utf8_lossy(&output.stdout)
            )
        })
    }

    fn jq_on_json(
        &self,
        project: &UserGuideProject,
        step: &str,
        json_stdout: &[u8],
        filter: &str,
    ) -> Value {
        let mut child = Command::new("jq")
            .args(["-c", filter])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap_or_else(|e| panic!("[{}] {step}: spawn jq: {e}", project.id));
        {
            use std::io::Write;
            child
                .stdin
                .take()
                .expect("jq stdin")
                .write_all(json_stdout)
                .expect("write jq stdin");
        }
        let out = child.wait_with_output().expect("jq wait");
        assert!(
            out.status.success(),
            "[{}] {step}: jq failed: {}\nfilter: {filter}",
            project.id,
            String::from_utf8_lossy(&out.stderr)
        );
        let trimmed = std::str::from_utf8(&out.stdout).expect("jq utf8").trim();
        serde_json::from_str(trimmed).unwrap_or_else(|e| {
            panic!(
                "[{}] {step}: jq output not JSON: {e}\n{}",
                project.id,
                String::from_utf8_lossy(&out.stdout)
            )
        })
    }

    fn run_json_jq(
        &self,
        project: &UserGuideProject,
        step: &str,
        args: &[&str],
        filter: &str,
    ) -> Value {
        let out = self.run_json(args);
        self.jq_on_json(project, step, &out.stdout, filter)
    }
}

impl Drop for WorkflowSession {
    fn drop(&mut self) {
        self.guard.stop();
        self.guard.assert_not_running();
    }
}

fn require_jq() {
    assert!(
        Command::new("jq")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false),
        "jq not found on PATH (required for user-guide workflow tests)"
    );
}

pub fn run_full_workflow(project: &UserGuideProject) {
    let session = WorkflowSession::start(project);

    // 2. gql all_functions | jq '.count'
    let gql_all = session.run_json(&["gql", "--macro-name", "all_functions", "unused"]);
    session.assert_ok(project, "gql all_functions", &gql_all);
    assert!(
        gql_all.stdout.len() > 8192,
        "[{}] gql all_functions stdout should exceed daemon truncation bound (8192), got {}",
        project.id,
        gql_all.stdout.len()
    );
    let count = session.run_json_jq(
        project,
        "gql all_functions jq count",
        &["gql", "--macro-name", "all_functions", "unused"],
        ".count",
    );
    assert!(
        count.as_u64().is_some_and(|n| n > 0),
        "[{}] gql all_functions count: {count}",
        project.id
    );

    // 3. gql CALLS → callee
    let calls_q = format!(
        "MATCH (a:Function)-[:CALLS]->(b:Function) WHERE b.name = '{}' RETURN a,b",
        project.calls_callee
    );
    let calls = session.parse_json(
        project,
        "gql CALLS",
        &session.run_json(&["gql", &calls_q]),
    );
    assert!(
        calls.get("count").and_then(|v| v.as_u64()).is_some_and(|n| n > 0),
        "[{}] gql CALLS to {}: {calls}",
        project.id,
        project.calls_callee
    );

    // 4. gql all_communities | jq projection
    let communities_jq = session.run_json_jq(
        project,
        "gql all_communities jq",
        &["gql", "--macro-name", "all_communities", "unused"],
        "[.rows[:3][][] | {id: .community_id, label, n: .member_count}]",
    );
    let rows = communities_jq.as_array().expect("communities jq array");
    assert!(!rows.is_empty(), "[{}] communities jq empty", project.id);
    for row in rows {
        let map = row.as_object().expect("community row object");
        assert!(map.contains_key("id"), "[{}] community row missing id", project.id);
        assert!(
            map.contains_key("label"),
            "[{}] community row missing label",
            project.id
        );
        assert!(
            map.contains_key("n"),
            "[{}] community row missing member_count",
            project.id
        );
    }

    // 5. communities list
    let list = session.run(&["communities", "list"]);
    session.assert_ok(project, "communities list", &list);
    assert!(
        !list.stdout.is_empty(),
        "[{}] communities list produced empty stdout",
        project.id
    );

    // 6. blast-radius text + JSON/jq
    let blast_text = session.run(&["blast-radius", project.blast_primary]);
    session.assert_ok(project, "blast-radius text", &blast_text);

    let blast_jq = session.run_json_jq(
        project,
        "blast-radius jq",
        &["blast-radius", project.blast_primary],
        "{score: .metrics.score, callers: .topology.direct_callers}",
    );
    let blast_obj = blast_jq.as_object().expect("blast jq object");
    assert!(
        blast_obj.contains_key("score"),
        "[{}] blast-radius jq missing score",
        project.id
    );
    assert!(
        blast_obj.get("callers").is_some_and(|v| v.is_array()),
        "[{}] blast-radius jq missing callers array",
        project.id
    );

    // 7. CoolStore / hybrid CPG
    let cpg_st = session.run(&["cpg", "status"]);
    session.assert_ok(project, "cpg status", &cpg_st);

    let cpg_mut = session.run(&[
        "cpg",
        "mutations",
        "--type",
        project.cpg_mutation_type,
        "--exclude-ctors",
    ]);
    session.assert_ok(project, "cpg mutations", &cpg_mut);
    let body_lines = String::from_utf8_lossy(&cpg_mut.stdout)
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('[') && !l.contains("Mutations of"))
        .count();
    assert!(
        body_lines >= project.cpg_mutations_min_body_lines,
        "[{}] cpg mutations expected >={} body lines, got {body_lines}\n{}",
        project.id,
        project.cpg_mutations_min_body_lines,
        String::from_utf8_lossy(&cpg_mut.stdout)
    );

    let blast_cool = session.run(&["blast-radius", project.blast_coolstore]);
    session.assert_ok(project, "blast-radius coolstore", &blast_cool);

    // 8. metrics --pagerank | jq
    let pagerank = session.run_json_jq(
        project,
        "metrics pagerank jq",
        &["metrics", "--pagerank"],
        ".pagerank.top[:3]",
    );
    let top = pagerank.as_array().expect("pagerank.top array");
    assert_eq!(top.len(), 3, "[{}] pagerank.top[:3] length", project.id);

    // 9. inspect cfg | jq
    let inspect_jq = session.run_json_jq(
        project,
        "inspect cfg jq",
        &["inspect", project.inspect_function, "cfg"],
        "{layer, blocks: (.nodes|length), edges: (.edges|length)}",
    );
    let inspect_obj = inspect_jq.as_object().expect("inspect jq");
    assert_eq!(
        inspect_obj.get("layer").and_then(|v| v.as_str()),
        Some("cfg"),
        "[{}] inspect layer",
        project.id
    );
    assert!(
        inspect_obj
            .get("blocks")
            .and_then(|v| v.as_u64())
            .is_some_and(|n| n > 0),
        "[{}] inspect blocks",
        project.id
    );

    // 10. slice (java + rust only — verified params)
    if let Some(slice) = &project.slice {
        let slice_out = session.run_json(&[
            "slice",
            slice.file,
            "--line",
            slice.line,
            "--variable",
            slice.variable,
            "--function",
            slice.function,
        ]);
        let slice_doc = session.parse_json(project, "slice", &slice_out);
        assert_eq!(
            slice_doc.get("schema_version").and_then(|v| v.as_u64()),
            Some(1)
        );
        assert!(slice_doc.get("nodes").is_some(), "[{}] slice nodes", project.id);
    }

    // 11. semantic index + queries (local; uses artifact root via -r)
    let sem_idx = session.run(&[
        "semantic",
        "index",
        "--embedder",
        "vocab",
        "--dimensions",
        "256",
    ]);
    session.assert_ok(project, "semantic index", &sem_idx);

    let sem1 = session.run_json_jq(
        project,
        "semantic query checkout jq",
        &[
            "semantic",
            "query",
            "shopping cart checkout",
            "--limit",
            "5",
        ],
        ".hits[:3] | map({name, score})",
    );
    let hits1 = sem1.as_array().expect("semantic hits array");
    assert!(!hits1.is_empty(), "[{}] semantic query hits", project.id);

    let sem2 = session.run_json_jq(
        project,
        "semantic query community jq",
        &[
            "semantic",
            "query",
            "shopping cart",
            "--scope",
            "community",
            "--limit",
            "3",
        ],
        ".hits | map({name, ranking, score})",
    );
    let hits2 = sem2.as_array().expect("semantic community hits");
    assert!(!hits2.is_empty(), "[{}] semantic community hits", project.id);
    assert!(
        hits2.iter().all(|h| {
            h.get("ranking")
                .and_then(|v| v.as_str())
                .is_some_and(|r| r == "community")
        }),
        "[{}] semantic community ranking",
        project.id
    );

    // 12. export mermaid
    let export_path =
        std::env::temp_dir().join(format!("rgctl-ug-{}-clearCart.mmd", project.id));
    let _ = fs::remove_file(&export_path);
    let export = session.run(&[
        "export",
        "--export-format",
        "mermaid",
        "--export-output",
        export_path.to_str().unwrap(),
        "--query",
        project.export_query,
    ]);
    session.assert_ok(project, "export mermaid", &export);
    let mmd = fs::read_to_string(&export_path)
        .unwrap_or_else(|e| panic!("[{}] read export: {e}", project.id));
    assert!(
        mmd.contains("graph") || mmd.contains("flowchart"),
        "[{}] mermaid export missing graph header",
        project.id
    );

    // 13. check policy | jq
    let policy = policy_file();
    assert!(policy.is_file(), "missing {}", policy.display());
    let check_jq = session.run_json_jq(
        project,
        "check policy jq",
        &["check", "--policy-file", policy.to_str().unwrap()],
        "{schema_version, violations: (.violations|length)}",
    );
    let check_obj = check_jq.as_object().expect("check jq");
    assert_eq!(
        check_obj.get("schema_version").and_then(|v| v.as_u64()),
        Some(1),
        "[{}] check schema_version",
        project.id
    );
    assert_eq!(
        check_obj.get("violations").and_then(|v| v.as_u64()),
        Some(0),
        "[{}] check violations",
        project.id
    );
}
