//! Cross-feature consistency QE (OpenSpec `qe-sanity-gates` lane 5).
//!
//! After discover, require agreement across:
//! C1 CALLS ↔ blast-radius, C2/C3 analysis_results / macro_call_index when filled,
//! C4 semantic expand ⊆ CALLS, C5 CFG text ⊆ CALLS, plus centrality degrees.
//!
//! C2/C3: empty blast caches on flat/on-demand graphs are accepted (#28 won't-fix).
//! See `rgbuilder-tests/correctness/QE.md`.

use rgbuilder::analysis::{AnalysisResults, MacroCallIndex};
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::str;
use uuid::Uuid;

fn fixture_repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tiny_polyglot_repo")
}

fn rgbuilder_bin() -> PathBuf {
    for key in ["CARGO_BIN_EXE_rg_ctl", "CARGO_BIN_EXE_rg_ctl"] {
        if let Ok(p) = std::env::var(key) {
            return PathBuf::from(p);
        }
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let debug = root.join("target/debug/rg_ctl");
    if debug.is_file() {
        return debug;
    }
    root.join("target/release/rg_ctl")
}

fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let target = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_all(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

fn short_name(s: &str) -> String {
    s.rsplit([':', '/', '.']).next().unwrap_or(s).to_string()
}

struct Sandbox {
    _dir: tempfile::TempDir,
    repo: PathBuf,
}

impl Sandbox {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        copy_dir_all(&fixture_repo(), dir.path()).expect("copy fixture");
        Self {
            repo: dir.path().to_path_buf(),
            _dir: dir,
        }
    }

    fn run(&self, args: &[&str]) -> Output {
        let mut cmd = Command::new(rgbuilder_bin());
        cmd.arg("-r").arg(&self.repo);
        cmd.args(args);
        cmd.output().expect("spawn rg-build")
    }

    fn parse_json(&self, output: &Output) -> Value {
        let stdout = str::from_utf8(&output.stdout).expect("utf8");
        serde_json::from_str(stdout).unwrap_or_else(|e| {
            panic!(
                "JSON parse: {e}\nstdout:\n{stdout}\nstderr:\n{}",
                str::from_utf8(&output.stderr).unwrap_or("")
            )
        })
    }
}

fn assert_success(output: &Output, label: &str) {
    assert!(
        output.status.success(),
        "{label} failed (exit {:?}):\nstderr: {}\nstdout: {}",
        output.status.code(),
        str::from_utf8(&output.stderr).unwrap_or(""),
        str::from_utf8(&output.stdout).unwrap_or("")
    );
}

fn gql_callers(sandbox: &Sandbox, callee: &str) -> BTreeSet<String> {
    let q = format!("MATCH (a:Function)-[:CALLS]->(b:Function) WHERE b.name = '{callee}' RETURN a");
    let out = sandbox.run(&["-f", "json", "gql", &q]);
    assert_success(&out, &format!("gql callers of {callee}"));
    let doc = sandbox.parse_json(&out);
    doc["rows"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|row| {
            row.as_array()?
                .first()?
                .get("node")
                .and_then(|n| n.as_str())
                .map(short_name)
        })
        .collect()
}

fn gql_callees(sandbox: &Sandbox, caller: &str) -> BTreeSet<String> {
    let q = format!("MATCH (a:Function)-[:CALLS]->(b:Function) WHERE a.name = '{caller}' RETURN b");
    let out = sandbox.run(&["-f", "json", "gql", &q]);
    assert_success(&out, &format!("gql callees of {caller}"));
    let doc = sandbox.parse_json(&out);
    doc["rows"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|row| {
            row.as_array()?
                .first()?
                .get("node")
                .and_then(|n| n.as_str())
                .map(short_name)
        })
        .collect()
}

fn blast_direct_caller_names(blast: &Value) -> BTreeSet<String> {
    blast
        .pointer("/topology/direct_callers")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|c| {
                    c.get("fqn")
                        .or_else(|| c.get("name"))
                        .or_else(|| c.get("symbol"))
                        .and_then(|f| f.as_str())
                        .map(short_name)
                })
                .collect()
        })
        .unwrap_or_default()
}

fn blast_impact_zone_len(blast: &Value) -> usize {
    blast
        .pointer("/topology/impact_zone")
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .or_else(|| {
            blast
                .pointer("/metrics/impact_zone_size")
                .and_then(|v| v.as_u64())
                .map(|n| n as usize)
        })
        .unwrap_or(0)
}

fn parse_uuid(s: &str) -> Uuid {
    Uuid::parse_str(s).unwrap_or_else(|e| panic!("bad uuid {s}: {e}"))
}

fn check(failures: &mut Vec<String>, ok: bool, id: &str, detail: impl std::fmt::Display) {
    if !ok {
        failures.push(format!("[{id}] {detail}"));
    }
}

#[test]
fn cross_feature_consistency_after_discover() {
    let sandbox = Sandbox::new();
    let mut failures: Vec<String> = Vec::new();

    let discover = sandbox.run(&[
        "-f",
        "json",
        "discover",
        ".",
        "--languages",
        "java,rust",
        "--cfg",
    ]);
    assert_success(&discover, "discover --cfg");
    let discover_doc = sandbox.parse_json(&discover);
    let files_failed = discover_doc
        .pointer("/metrics/files_failed")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    check(
        &mut failures,
        files_failed == 0,
        "REL",
        format!("discover reported files_failed={files_failed} on fixture"),
    );
    let files_discovered = discover_doc
        .pointer("/metrics/files_discovered")
        .and_then(|v| v.as_u64());
    let files_processed = discover_doc
        .pointer("/metrics/files_processed")
        .and_then(|v| v.as_u64());
    if let (Some(d), Some(p)) = (files_discovered, files_processed) {
        check(
            &mut failures,
            d == p,
            "REL",
            format!("discover metrics mismatch: files_discovered={d} files_processed={p}"),
        );
    }

    let blast_pe = sandbox.run(&["-f", "json", "blast-radius", "publishEvent"]);
    assert_success(&blast_pe, "blast-radius publishEvent");
    let blast_pe_doc = sandbox.parse_json(&blast_pe);
    let pe_id = parse_uuid(
        blast_pe_doc
            .pointer("/target/id")
            .and_then(|v| v.as_str())
            .expect("blast target.id"),
    );

    let gql_callers_pe = gql_callers(&sandbox, "publishEvent");
    let blast_callers_pe = blast_direct_caller_names(&blast_pe_doc);
    check(
        &mut failures,
        gql_callers_pe == blast_callers_pe,
        "C1",
        format!("GQL callers {gql_callers_pe:?} vs blast {blast_callers_pe:?}"),
    );
    check(
        &mut failures,
        !gql_callers_pe.is_empty(),
        "C1",
        "publishEvent must have ≥1 CALLS caller on fixture",
    );

    let analysis_path = sandbox.repo.join(".rgbuilder/analysis_results.bin");
    check(
        &mut failures,
        analysis_path.is_file(),
        "C2",
        format!("missing {}", analysis_path.display()),
    );
    if analysis_path.is_file() {
        let analysis = AnalysisResults::load(&analysis_path).expect("load analysis_results");
        // Flat/on-demand graphs skip bulk blast fill at discover (#28 won't-fix).
        // C2 only fails when a non-empty cache disagrees with live blast-radius.
        match analysis.get_blast_radius(pe_id) {
            None => {
                eprintln!(
                    "[C2] no blast_radius column for publishEvent (expected when bulk fill skipped; #28)"
                );
            }
            Some(br)
                if br.direct_callers == 0
                    && br.impact_zone_size == 0
                    && !blast_callers_pe.is_empty() =>
            {
                eprintln!(
                    "[C2] analysis_results blast columns empty while live blast has callers \
                     (expected on-demand skip; #28 won't-fix)"
                );
            }
            Some(br) => {
                check(
                    &mut failures,
                    br.direct_callers as usize == blast_callers_pe.len(),
                    "C2",
                    format!(
                        "analysis_results.direct_callers={} vs blast len={}",
                        br.direct_callers,
                        blast_callers_pe.len()
                    ),
                );
                check(
                    &mut failures,
                    br.impact_zone_size as usize == blast_impact_zone_len(&blast_pe_doc),
                    "C2",
                    format!(
                        "analysis_results.impact_zone_size={} vs blast zone {}",
                        br.impact_zone_size,
                        blast_impact_zone_len(&blast_pe_doc)
                    ),
                );
            }
        }

        if let Some(cent) = analysis.get_centrality(pe_id) {
            let pe_callees = gql_callees(&sandbox, "publishEvent");
            check(
                &mut failures,
                cent.in_degree as usize == gql_callers_pe.len(),
                "DEG",
                format!(
                    "centrality in_degree={} vs CALLS callers {}",
                    cent.in_degree,
                    gql_callers_pe.len()
                ),
            );
            check(
                &mut failures,
                cent.out_degree as usize == pe_callees.len(),
                "DEG",
                format!(
                    "centrality out_degree={} vs CALLS callees {}",
                    cent.out_degree,
                    pe_callees.len()
                ),
            );
        } else {
            failures.push(format!("[DEG] no centrality row for publishEvent {pe_id}"));
        }
    }

    let macro_path = MacroCallIndex::default_path(&sandbox.repo);
    check(
        &mut failures,
        macro_path.is_file(),
        "C3",
        format!("missing {}", macro_path.display()),
    );
    if macro_path.is_file() {
        let macro_idx = MacroCallIndex::load(&macro_path)
            .expect("load macro index")
            .expect("macro index Some");
        // Missing/empty macro entry with live blast callers = intentional skip (#28).
        match macro_idx.get(pe_id) {
            None => {
                eprintln!(
                    "[C3] no macro_call_index entry for publishEvent (expected when bulk fill skipped; #28)"
                );
            }
            Some(entry) if entry.direct_caller_names.is_empty() && !blast_callers_pe.is_empty() => {
                eprintln!(
                    "[C3] macro_call_index empty while live blast has callers \
                     (expected on-demand skip; #28 won't-fix)"
                );
            }
            Some(entry) => {
                let macro_names: BTreeSet<String> = entry
                    .direct_caller_names
                    .iter()
                    .map(|s| short_name(s))
                    .collect();
                check(
                    &mut failures,
                    macro_names == blast_callers_pe,
                    "C3",
                    format!("macro names {macro_names:?} vs blast {blast_callers_pe:?}"),
                );
            }
        }
    }

    let inspect = sandbox.run(&["-f", "json", "inspect", "checkout", "cfg"]);
    assert_success(&inspect, "inspect checkout cfg");
    let cfg_doc = sandbox.parse_json(&inspect);
    let mut cfg_texts = Vec::new();
    if let Some(nodes) = cfg_doc["nodes"].as_array() {
        for n in nodes {
            if let Some(stmts) = n.get("statements").and_then(|s| s.as_array()) {
                for st in stmts {
                    if let Some(t) = st.get("text").and_then(|t| t.as_str()) {
                        cfg_texts.push(t.to_string());
                    } else if let Some(t) = st.as_str() {
                        cfg_texts.push(t.to_string());
                    }
                }
            }
            if let Some(t) = n.get("label").and_then(|t| t.as_str()) {
                cfg_texts.push(t.to_string());
            }
        }
    }
    let joined = cfg_texts.join("\n");
    let calls_callees = gql_callees(&sandbox, "checkout");
    for required in ["publishEvent", "process"] {
        let in_cfg = joined.contains(required);
        let in_calls = calls_callees.iter().any(|c| c == required);
        check(
            &mut failures,
            in_calls,
            "C5",
            format!("CALLS checkout→{required} missing; callees={calls_callees:?}"),
        );
        check(
            &mut failures,
            in_cfg,
            "C5",
            format!("CFG text missing '{required}'; stmts={cfg_texts:?}"),
        );
        check(
            &mut failures,
            !in_cfg || in_calls,
            "C5",
            format!("CFG mentions '{required}' but CALLS missing"),
        );
    }

    let index = sandbox.run(&[
        "-f",
        "json",
        "semantic",
        "index",
        "--dimensions",
        "256",
        "--embedder",
        "vocab",
    ]);
    assert_success(&index, "semantic index");
    let query = sandbox.run(&[
        "-f",
        "json",
        "semantic",
        "query",
        "checkout",
        "--limit",
        "5",
        "--expand",
        "neighbors",
        "--expand-depth",
        "1",
    ]);
    assert_success(&query, "semantic query expand neighbors");
    let qdoc = sandbox.parse_json(&query);
    let hits = qdoc["hits"].as_array().expect("hits");
    if let Some(checkout_hit) = hits.iter().find(|h| h["name"].as_str() == Some("checkout")) {
        let hit_id = checkout_hit["node_id"].as_str().unwrap_or("");
        let expansion = qdoc
            .get("expansion")
            .and_then(|e| e.get("neighbors"))
            .and_then(|v| v.as_array());
        match expansion {
            None => failures.push("[C4] missing expansion.neighbors".into()),
            Some(expansion) => {
                check(
                    &mut failures,
                    !expansion.is_empty(),
                    "C4",
                    "expected non-empty neighbors for checkout",
                );
                let mut allowed: BTreeSet<String> = gql_callees(&sandbox, "checkout");
                allowed.extend(gql_callers(&sandbox, "checkout"));
                for n in expansion {
                    let rel_anchor = n
                        .get("anchor_node_id")
                        .and_then(|a| a.as_str())
                        .unwrap_or("");
                    if !rel_anchor.is_empty() && rel_anchor != hit_id {
                        continue;
                    }
                    let name = n
                        .get("name")
                        .and_then(|x| x.as_str())
                        .map(short_name)
                        .unwrap_or_default();
                    if name.is_empty() || name == "checkout" {
                        continue;
                    }
                    check(
                        &mut failures,
                        allowed.contains(&name),
                        "C4",
                        format!("neighbor '{name}' not in CALLS neighborhood {allowed:?}"),
                    );
                }
            }
        }
    } else {
        failures.push("[C4] checkout not in semantic top-5 for query 'checkout'".into());
    }

    assert!(
        failures.is_empty(),
        "cross-feature QE required failures ({}):\n{}",
        failures.len(),
        failures.join("\n")
    );
}
