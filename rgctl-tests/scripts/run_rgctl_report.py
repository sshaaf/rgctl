#!/usr/bin/env python3
"""Run rgctl feature matrix against Tier 1 e-commerce test apps and publish reports.

Outputs (default: rgctl-reports/):
  - REPORT.md / REPORT.html — cross-project summary report
  - languages/<id>.md / languages/<id>.html — comprehensive per-language reports
  - README.md — index linking all artifacts
  - all-results.json — full machine-readable results
  - <project>-summary.json — per-project summaries
  - <project>-metrics.json, <project>-blast.json, <project>-export.json — artifacts

Usage:
  ./scripts/run_rgctl_report.py
  RGCTL=/path/to/rgctl ./scripts/run_rgctl_report.py --update-readmes
"""

from __future__ import annotations

import argparse
import html
import json
import os
import re
import shutil
import subprocess
import sys
from dataclasses import dataclass, field
from datetime import date, datetime, timezone
from pathlib import Path
from typing import Any

# ---------------------------------------------------------------------------
# Project configuration (Tier 1 e-commerce apps)
# ---------------------------------------------------------------------------

@dataclass
class ProjectConfig:
    id: str
    dir_name: str
    language: str
    exclude: str
    blast_symbol: str
    notes: str
    slice_file: str = ""
    slice_line: int = 16
    slice_variable: str = "total"
    slice_function: str = "checkout"


PROJECTS: list[ProjectConfig] = [
    ProjectConfig(
        id="rust",
        dir_name="ecommerce-rust",
        language="Rust",
        exclude="target",
        blast_symbol="src/services/order.rs::checkout",
        slice_file="src/services/order.rs",
        notes="Best deep-analysis reference (CFG/PDG/inspect/taint).",
    ),
    ProjectConfig(
        id="python",
        dir_name="ecommerce-python",
        language="Python",
        exclude=".venv,__pycache__",
        blast_symbol="app/services/order.py::checkout",
        slice_file="app/services/order.py",
        slice_line=35,
        notes="Second full CFG/PDG language; rich class nodes.",
    ),
    ProjectConfig(
        id="go",
        dir_name="ecommerce-go",
        language="Go",
        exclude="vendor",
        blast_symbol="internal/service/order.go::Checkout",
        slice_file="internal/service/order.go",
        slice_function="Checkout",
        notes="Partial Go indexing possible; verify file coverage in discover metrics.",
    ),
    ProjectConfig(
        id="java",
        dir_name="ecommerce-java",
        language="Java",
        exclude="target,data",
        blast_symbol="src/main/java/com/example/ecommerce/service/OrderService.java::checkout",
        slice_file="src/main/java/com/example/ecommerce/service/OrderService.java",
        notes="Strongest CALLS graph and community modularity in this suite.",
    ),
    ProjectConfig(
        id="csharp",
        dir_name="ecommerce-csharp",
        language="C#",
        exclude="bin,obj,data",
        blast_symbol="src/Ecommerce/Services/OrderService.cs::CheckoutAsync",
        slice_file="src/Ecommerce/Services/OrderService.cs",
        slice_function="CheckoutAsync",
        notes="ASP.NET Core mirror of Java; Tier 1 CFG/taint/calls.",
    ),
    ProjectConfig(
        id="typescript",
        dir_name="ecommerce-typescript",
        language="TypeScript",
        exclude="node_modules,dist",
        blast_symbol="src/services/orderService.ts::checkout",
        slice_file="src/services/orderService.ts",
        notes="High AST node count; compare with JavaScript sibling.",
    ),
    ProjectConfig(
        id="javascript",
        dir_name="ecommerce-javascript",
        language="JavaScript",
        exclude="node_modules",
        blast_symbol="src/services/orderService.js::checkout",
        slice_file="src/services/orderService.js",
        notes="Mirror of TypeScript graph without types.",
    ),
    ProjectConfig(
        id="c",
        dir_name="ecommerce-c",
        language="C",
        exclude="build,cmake-build-debug,.rgctl",
        blast_symbol="src/coolstore/services/shopping_cart_service.c::price_shopping_cart",
        slice_file="src/coolstore/services/shopping_cart_service.c",
        slice_function="price_shopping_cart",
        slice_variable="cart",
        notes="C fixture with CoolStore /services cart pricing mutations.",
    ),
    ProjectConfig(
        id="cpp",
        dir_name="ecommerce-cpp",
        language="C++",
        exclude="build,cmake-build-debug,.rgctl",
        blast_symbol="src/coolstore/services/shopping_cart_service.cpp::priceShoppingCart",
        slice_file="src/coolstore/services/shopping_cart_service.cpp",
        slice_function="priceShoppingCart",
        slice_variable="cart",
        notes="C++ fixture with CoolStore /services cart pricing mutations.",
    ),
]

FEATURE_ROWS: list[tuple[str, str]] = [
    ("discover (`--cfg`)", "discover"),
    ("Dashboard", "dashboard"),
    ("GQL queries", "gql"),
    ("Metrics (communities + PageRank)", "metrics"),
    ("Blast radius", "blast_radius"),
    ("Export (JSON subgraph)", "export"),
    ("CI check (`--policy-file`)", "check"),
    ("Program slice", "slice"),
    ("Taint analysis", "taint"),
    ("Inspect CFG", "inspect_cfg"),
    ("Inspect PDG", "inspect_pdg"),
    ("Inspect dominators", "inspect_dom"),
    ("Serve daemon", "serve"),
]

STATUS_ICON = {
    "ok": "✓",
    "partial": "◐",
    "unsupported": "—",
    "n/a": "—",
    "not_run": "—",
    "fail": "✗",
}


# ---------------------------------------------------------------------------
# rgctl runner
# ---------------------------------------------------------------------------

def resolve_rgctl(explicit: str | None) -> Path:
    if explicit:
        path = Path(explicit).expanduser().resolve()
        if not path.is_file():
            sys.exit(f"rgctl binary not found: {path}")
        return path

    env = os.environ.get("RGCTL")
    if env:
        path = Path(env).expanduser().resolve()
        if path.is_file():
            return path

    which = shutil.which("rgctl")
    if which:
        return Path(which).resolve()

    # Embedded under rgctl/rgctl-tests/scripts → workspace root is parents[2]
    root = Path(__file__).resolve().parents[2]
    candidates = [
        root / "target" / "release" / "rgctl",
        root / "target" / "debug" / "rgctl",
        root / "rgctl" / "target" / "release" / "rgctl",
        root / "rgctl" / "target" / "debug" / "rgctl",
        root / "rgctl" / "target" / "release" / "rgctl",
        root / "rgctl" / "target" / "debug" / "rgctl",
    ]
    for c in candidates:
        if c.is_file():
            return c.resolve()

    sys.exit(
        "Could not find rgctl. Set RGCTL=/path/to/rgctl or pass --rgctl."
    )


def run_cmd(
    rgctl: Path,
    args: list[str],
    cwd: Path,
    timeout: int = 600,
) -> tuple[int, str, str]:
    cmd = [str(rgctl), *args]
    try:
        proc = subprocess.run(
            cmd,
            cwd=cwd,
            capture_output=True,
            text=True,
            timeout=timeout,
        )
        return proc.returncode, proc.stdout, proc.stderr
    except subprocess.TimeoutExpired as exc:
        return 124, exc.stdout or "", exc.stderr or "timeout"


def parse_json_out(stdout: str) -> Any | None:
    stdout = stdout.strip()
    if not stdout:
        return None
    for line in reversed(stdout.splitlines()):
        line = line.strip()
        if line.startswith("{"):
            try:
                return json.loads(line)
            except json.JSONDecodeError:
                continue
    try:
        return json.loads(stdout)
    except json.JSONDecodeError:
        return None


def gql_count(rgctl: Path, cwd: Path, label: str) -> int | None:
    code, out, _ = run_cmd(rgctl, ["-f", "json", "gql", f"MATCH (n:{label}) RETURN n"], cwd)
    if code != 0:
        code, out, _ = run_cmd(rgctl, ["gql", f"MATCH (n:{label}) RETURN n"], cwd)
        if code != 0:
            return None
    data = parse_json_out(out)
    if isinstance(data, list):
        return len(data)
    if isinstance(data, dict):
        for key in ("results", "nodes", "data"):
            if isinstance(data.get(key), list):
                return len(data[key])
    lines = [ln for ln in out.splitlines() if ln.strip() and not ln.startswith("Error")]
    return len(lines) if lines else 0


def gql_function_names(rgctl: Path, cwd: Path) -> list[str]:
    """Unique function symbol names from text-mode GQL (one name per line)."""
    code, out, _ = run_cmd(rgctl, ["gql", "MATCH (n:Function) RETURN n"], cwd)
    if code != 0:
        return []
    names: list[str] = []
    seen: set[str] = set()
    for ln in out.splitlines():
        s = ln.strip()
        if s and not s.startswith("Error") and s not in seen:
            seen.add(s)
            names.append(s)
    return names


def scan_blast_top_scores(
    rgctl: Path,
    cwd: Path,
    top_n: int = 10,
) -> dict[str, Any]:
    """Run blast-radius on every indexed function name; return top scores > 0."""
    names = gql_function_names(rgctl, cwd)
    hits: list[dict[str, Any]] = []
    for name in names:
        code, out, _ = run_cmd(rgctl, ["-f", "json", "blast-radius", name], cwd)
        if code != 0:
            continue
        data = parse_json_out(out)
        if not isinstance(data, dict):
            continue
        m = data.get("metrics", {})
        score = m.get("score") or 0
        if score > 0:
            t = data.get("target", {})
            hits.append(
                {
                    "symbol": name,
                    "score": score,
                    "direct_callers": m.get("direct_callers_count"),
                    "impact_zone_size": m.get("impact_zone_size"),
                    "canonical_fqn": t.get("canonical_fqn"),
                    "file_path": t.get("file_path"),
                }
            )
    hits.sort(key=lambda x: -x["score"])
    top = hits[:top_n]
    return {
        "functions_scanned": len(names),
        "count_score_gt_zero": len(hits),
        "top_n": top_n,
        "top": top,
        "max_score": hits[0]["score"] if hits else 0.0,
        "max_symbol": hits[0]["symbol"] if hits else None,
    }


def classify_deep(err: str, code: int, out: str) -> str:
    if code == 0 and out.strip():
        return "ok"
    el = err.lower()
    if "unsupported" in el or "unsupportedlanguage" in el.replace(" ", ""):
        return "unsupported"
    if "not found" in el or "no pdg" in el or "no cfg" in el or "ambiguous" in el:
        return "unsupported"
    if "no pdg node" in el:
        return "partial"
    return "fail"


def cache_size_mb(cache_dir: Path) -> float:
    if not cache_dir.is_dir():
        return 0.0
    total = sum(f.stat().st_size for f in cache_dir.rglob("*") if f.is_file())
    return round(total / 1024 / 1024, 2)


def run_project(
    rgctl: Path,
    repo_root: Path,
    out_dir: Path,
    policy: Path,
    project: ProjectConfig,
    clean_cache: bool,
    blast_scan: bool = True,
    blast_top_n: int = 10,
) -> dict[str, Any]:
    cwd = repo_root / project.dir_name
    if not cwd.is_dir():
        return {
            "project": project.id,
            "language": project.language,
            "path": project.dir_name,
            "features": {"discover": {"status": "fail", "error": f"missing directory {cwd}"}},
            "cache_mb": 0,
        }

    cache = cwd / ".rgctl"
    if clean_cache and cache.exists():
        shutil.rmtree(cache)

    result: dict[str, Any] = {
        "project": project.id,
        "language": project.language,
        "path": project.dir_name,
        "features": {},
    }

    # discover
    code, out, err = run_cmd(
        rgctl,
        ["-f", "json", "discover", ".", "--cfg", "-e", project.exclude],
        cwd,
    )
    disc = parse_json_out(out)
    dm = disc.get("metrics", {}) if isinstance(disc, dict) else {}
    result["features"]["discover"] = {
        "status": "ok" if code == 0 and disc else "fail",
        "files_indexed": dm.get("files_indexed"),
        "files_discovered": dm.get("files_discovered"),
        "files_skipped": dm.get("files_skipped"),
        "nodes_generated": dm.get("nodes_generated"),
        "edges_generated": dm.get("edges_generated"),
        "duration_ms": dm.get("duration_ms"),
        "cfg_enabled": True,
        "exclude": project.exclude,
        "error": err.strip()[-500:] if code != 0 else None,
    }

    if not cache.is_dir():
        result["cache_mb"] = 0
        return result

    result["features"]["dashboard"] = {
        "status": "ok" if (cache / "dashboard").is_dir() else "fail",
        "path": ".rgctl/dashboard",
    }

    # gql
    calls_code, calls_out, _ = run_cmd(
        rgctl,
        ["gql", "MATCH (a:Function)-[:CALLS]->(b:Function) RETURN a,b LIMIT 500"],
        cwd,
    )
    calls_sample = len([ln for ln in calls_out.splitlines() if ln.strip() and "Error" not in ln])
    result["features"]["gql"] = {
        "status": "ok",
        "functions": gql_count(rgctl, cwd, "Function"),
        "classes": gql_count(rgctl, cwd, "Class"),
        "files": gql_count(rgctl, cwd, "File"),
        "calls_edges_sampled": calls_sample if calls_code == 0 else None,
        "checkout_blast_symbol": project.blast_symbol,
    }

    # metrics
    code, out, err = run_cmd(
        rgctl,
        ["-f", "json", "metrics", "--communities", "--pagerank"],
        cwd,
    )
    metrics = parse_json_out(out)
    comm = metrics.get("communities", {}) if isinstance(metrics, dict) else {}
    pr = metrics.get("pagerank", {}) if isinstance(metrics, dict) else {}
    result["features"]["metrics"] = {
        "status": "ok" if code == 0 else "fail",
        "communities": comm.get("count"),
        "modularity": comm.get("modularity"),
        "pagerank_converged": pr.get("converged"),
        "pagerank_iterations": pr.get("iterations"),
        "error": err.strip()[-300:] if code != 0 else None,
    }
    (out_dir / f"{project.id}-metrics.json").write_text(out)

    # blast-radius
    code, out, err = run_cmd(
        rgctl,
        ["-f", "json", "blast-radius", project.blast_symbol],
        cwd,
    )
    blast = parse_json_out(out)
    bm = blast.get("metrics", {}) if isinstance(blast, dict) else {}
    blast_status = "ok" if code == 0 and blast else "fail"
    if blast_status == "fail" and project.id == "go":
        blast_status = "n/a"
    result["features"]["blast_radius"] = {
        "status": blast_status,
        "symbol": project.blast_symbol,
        "score": bm.get("score"),
        "impact_zone_size": bm.get("impact_zone_size"),
        "direct_callers": bm.get("direct_callers_count"),
        "target_file": (blast.get("target") or {}).get("file_path") if isinstance(blast, dict) else None,
        "error": err.strip()[-300:] if code != 0 else None,
    }
    (out_dir / f"{project.id}-blast.json").write_text(out)

    # blast-radius: top scores across all indexed function names
    if blast_scan:
        print(f"  blast scan: {project.id} …", flush=True)
        blast_top = scan_blast_top_scores(rgctl, cwd, top_n=blast_top_n)
        result["blast_top_scores"] = blast_top
        (out_dir / f"{project.id}-blast-top.json").write_text(
            json.dumps(blast_top, indent=2)
        )
        if blast_top["count_score_gt_zero"]:
            print(
                f"  blast scan: {blast_top['count_score_gt_zero']} symbols score>0 "
                f"(max {blast_top['max_symbol']}={blast_top['max_score']:.2f})",
                flush=True,
            )
        else:
            print("  blast scan: no symbols with score > 0", flush=True)
    else:
        result["blast_top_scores"] = {"skipped": True}

    # export
    export_path = out_dir / f"{project.id}-export.json"
    code, out, err = run_cmd(
        rgctl,
        [
            "export",
            "--export-format",
            "json",
            "--export-output",
            str(export_path),
            "--query",
            "MATCH (n:Function) RETURN n LIMIT 100",
        ],
        cwd,
    )
    result["features"]["export"] = {
        "status": "ok" if export_path.is_file() and export_path.stat().st_size > 0 else "fail",
        "bytes": export_path.stat().st_size if export_path.is_file() else 0,
        "query": "MATCH (n:Function) RETURN n LIMIT 100",
        "error": err.strip()[-200:] if code != 0 else None,
    }

    # check
    code, out, err = run_cmd(
        rgctl,
        ["-f", "json", "check", "--policy-file", str(policy)],
        cwd,
    )
    chk = parse_json_out(out)
    result["features"]["check"] = {
        "status": "ok" if code == 0 else "fail",
        "passed": chk.get("passed") if isinstance(chk, dict) else None,
        "error": err.strip()[-200:] if code != 0 else None,
    }
    (out_dir / f"{project.id}-check.json").write_text(out)

    # slice / taint / inspect
    slice_path = project.slice_file
    if slice_path and not (cwd / slice_path).is_file():
        alt = next(
            (p.relative_to(cwd) for p in cwd.rglob("*order*") if p.suffix in {".rs", ".py", ".go", ".java", ".ts", ".js"} and p.is_file()),
            None,
        )
        if alt:
            slice_path = str(alt)

    if slice_path and (cwd / slice_path).is_file():
        slice_args = [
            "slice",
            slice_path,
            "--line",
            str(project.slice_line),
            "--variable",
            project.slice_variable,
            "--function",
            project.slice_function,
        ]
        code, out, err = run_cmd(rgctl, slice_args, cwd)
        st = classify_deep(err, code, out)
        if st == "unsupported" and project.language in {"Rust", "Python", "Java"}:
            st = "partial"
        result["features"]["slice"] = {
            "status": st,
            "file": slice_path,
            "line": project.slice_line,
            "variable": project.slice_variable,
            "function": project.slice_function,
            "output_lines": len(out.splitlines()),
            "error": err.strip()[-250:] if st not in {"ok"} else None,
        }

        code, out, err = run_cmd(rgctl, slice_args + ["--taint"], cwd)
        st = classify_deep(err, code, out)
        result["features"]["taint"] = {
            "status": st if st != "partial" else "ok",
            "flows": out.strip()[:200] if code == 0 else None,
            "error": err.strip()[-250:] if code != 0 else None,
        }
    else:
        result["features"]["slice"] = {"status": "unsupported", "note": "slice source file not found"}
        result["features"]["taint"] = {"status": "unsupported", "note": "slice source file not found"}

    inspect_sym = project.blast_symbol.split("::")[-1]
    for mode, key in [("cfg", "inspect_cfg"), ("pdg", "inspect_pdg"), ("dom", "inspect_dom")]:
        code, out, err = run_cmd(rgctl, ["inspect", inspect_sym, mode], cwd)
        if code != 0:
            code, out, err = run_cmd(rgctl, ["inspect", project.blast_symbol, mode], cwd)
        st = classify_deep(err, code, out)
        preview = out.strip().splitlines()[0][:160] if out.strip() else None
        result["features"][key] = {
            "status": st,
            "preview": preview,
            "error": err.strip()[-200:] if st not in {"ok", "partial"} else None,
        }

    result["features"]["serve"] = {
        "status": "not_run",
        "note": "Use `rgctl serve` after discover for warm repeated queries (Unix socket).",
    }

    result["cache_mb"] = cache_size_mb(cache)
    result["notes"] = project.notes
    (out_dir / f"{project.id}-summary.json").write_text(json.dumps(result, indent=2))
    return result


# ---------------------------------------------------------------------------
# Report generation
# ---------------------------------------------------------------------------

def icon(status: str | None) -> str:
    return STATUS_ICON.get(status or "", status or "—")


def active_projects(results: dict[str, Any]) -> list[ProjectConfig]:
    return [p for p in PROJECTS if p.id in results]


HTML_STYLE = """
    :root { font-family: system-ui, sans-serif; color: #1a1a1a; background: #fafafa; }
    body { max-width: 1100px; margin: 2rem auto; padding: 0 1rem; line-height: 1.5; }
    h1 { font-size: 1.75rem; }
    h2 { margin-top: 2rem; border-bottom: 1px solid #ddd; padding-bottom: 0.25rem; }
    h3 { margin-top: 1.25rem; }
    .meta { color: #555; font-size: 0.95rem; }
    table.report { border-collapse: collapse; width: 100%; margin: 1rem 0; font-size: 0.9rem; }
    table.report th, table.report td { border: 1px solid #ddd; padding: 0.45rem 0.6rem; text-align: left; }
    table.report th { background: #eee; }
    table.report td.num, table.report th.num { text-align: right; }
    .st { font-weight: 600; }
    .st-ok { color: #0a7; }
    .st-fail { color: #c33; }
    .st-partial { color: #c80; }
    .st-unsupported, .st-n/a, .st-not_run { color: #888; }
    code { background: #eee; padding: 0.1rem 0.35rem; border-radius: 3px; }
    pre { background: #222; color: #eee; padding: 1rem; overflow-x: auto; border-radius: 6px; }
    a { color: #0366d6; }
    """


def html_esc(value: Any) -> str:
    return html.escape("" if value is None else str(value))


def html_table(
    headers: list[str],
    rows: list[list[str]],
    numeric_cols: set[int] | None = None,
) -> str:
    numeric_cols = numeric_cols or set()
    parts = ['<table class="report">', "<thead><tr>"]
    for h in headers:
        parts.append(f"<th>{html_esc(h)}</th>")
    parts.append("</tr></thead><tbody>")
    for row in rows:
        parts.append("<tr>")
        for i, cell in enumerate(row):
            cls = ' class="num"' if i in numeric_cols else ""
            parts.append(f"<td{cls}>{cell}</td>")
        parts.append("</tr>")
    parts.append("</tbody></table>")
    return "\n".join(parts)


def status_html(status: str | None) -> str:
    st = status or "na"
    return (
        f'<span class="st st-{html_esc(st)}">{html_esc(icon(status))}</span>'
    )


def feature_detail(key: str, feat: dict[str, Any]) -> str:
    if not feat:
        return "—"
    if key == "discover" and feat.get("status") == "ok":
        return (
            f"{feat.get('files_indexed')} indexed / {feat.get('files_discovered', '?')} discovered, "
            f"{feat.get('nodes_generated')} nodes, {feat.get('edges_generated')} edges"
        )
    if key == "gql" and feat.get("status") == "ok":
        return (
            f"{feat.get('functions')} functions, {feat.get('classes')} classes, "
            f"CALLS sample {feat.get('calls_edges_sampled', '—')}"
        )
    if key == "metrics" and feat.get("status") == "ok":
        mod = feat.get("modularity")
        mod_s = f"{mod:.3f}" if isinstance(mod, (int, float)) else "—"
        return f"{feat.get('communities')} communities, modularity {mod_s}"
    if key == "blast_radius" and feat.get("status") == "ok":
        return f"score {feat.get('score')}, {feat.get('direct_callers')} callers"
    if key == "export" and feat.get("status") == "ok":
        return f"{feat.get('bytes', 0):,} bytes"
    if key == "check" and feat.get("status") == "ok":
        return f"passed={feat.get('passed')}"
    if key == "slice":
        if feat.get("file"):
            return f"{feat.get('file')}:{feat.get('line')} `{feat.get('variable')}`"
        return feat.get("note") or feat.get("error") or "—"
    if key == "taint" and feat.get("flows"):
        return feat.get("flows", "")[:80]
    if key.startswith("inspect_") and feat.get("preview"):
        return feat["preview"][:100]
    return feat.get("error") or feat.get("note") or "—"


def language_executive_summary(project: ProjectConfig, result: dict[str, Any]) -> str:
    parts: list[str] = []
    d = result.get("features", {}).get("discover", {})
    if d.get("status") == "ok":
        parts.append(
            f"**{project.language}** app `{project.dir_name}` indexed **{d.get('files_indexed')}** files "
            f"({d.get('nodes_generated')} nodes, **{d.get('edges_generated')}** edges, "
            f"{d.get('duration_ms')} ms)."
        )
        if d.get("files_discovered") and d.get("files_indexed", 0) < d.get("files_discovered", 0):
            parts.append(
                f"Partial coverage: **{d['files_indexed']}/{d['files_discovered']}** files indexed."
            )
    else:
        parts.append(f"**{project.language}** discover failed.")

    m = result.get("features", {}).get("metrics", {})
    if isinstance(m.get("modularity"), (int, float)) and m["modularity"] > 0:
        parts.append(f"Graph modularity **{m['modularity']:.3f}** ({m.get('communities')} communities).")

    bt = result.get("blast_top_scores", {})
    if bt.get("count_score_gt_zero", 0) > 0:
        parts.append(
            f"**{bt['count_score_gt_zero']}** functions with blast score > 0 "
            f"(peak `{bt.get('max_symbol')}` = **{bt.get('max_score', 0):.2f}**)."
        )
    else:
        b = result.get("features", {}).get("blast_radius", {})
        if b.get("status") == "ok":
            parts.append(
                f"Checkout blast target `{b.get('symbol')}` scores **{b.get('score', 0)}** "
                "(typical for leaf handlers)."
            )

    deep = []
    for key, label in [
        ("slice", "slice"),
        ("taint", "taint"),
        ("inspect_cfg", "CFG"),
        ("inspect_pdg", "PDG"),
    ]:
        st = result.get("features", {}).get(key, {}).get("status")
        if st in {"ok", "partial"}:
            deep.append(label)
    if deep:
        parts.append(f"Deep analysis available: {', '.join(deep)}.")
    elif project.notes:
        parts.append(project.notes.rstrip("."))

    return " ".join(parts)


def executive_summary(results: dict[str, Any]) -> str:
    projects = active_projects(results)
    lines = []
    ok = sum(
        1 for p in projects
        if results[p.id].get("features", {}).get("discover", {}).get("status") == "ok"
    )
    if ok == len(projects):
        lines.append(
            f"All **{len(projects)}** Tier 1 reference stores indexed successfully."
        )
    else:
        lines.append(f"**{ok}/{len(projects)}** projects indexed successfully.")

    best_edges = max(
        (results[p.id]["features"]["discover"].get("edges_generated") or 0, p.language)
        for p in projects
        if results[p.id].get("features", {}).get("discover", {}).get("status") == "ok"
    )
    if best_edges[0]:
        lines.append(
            f"**{best_edges[1]}** produced the largest call graph in this run (**{best_edges[0]}** edges)."
        )

    java_m = results.get("java", {}).get("features", {}).get("metrics", {}).get("modularity")
    if java_m:
        lines.append(f"**Java** modularity **{java_m:.3f}** — clearest community structure in this suite.")

    go_d = results.get("go", {}).get("features", {}).get("discover", {})
    if go_d.get("files_discovered") and go_d.get("files_indexed"):
        if go_d["files_indexed"] < go_d["files_discovered"]:
            lines.append(
                f"**Go** partial coverage: **{go_d['files_indexed']}/{go_d['files_discovered']}** files indexed."
            )

    lines.append(
        "Deep analysis (CFG/PDG slice, taint) is strongest on **Rust** and **Python**; "
        "**Java** gets inspect overlays with `--cfg`. "
        "See [`languages/`](languages/) for per-language reports."
    )

    any_hits = False
    for p in projects:
        bt = results[p.id].get("blast_top_scores", {})
        if bt.get("count_score_gt_zero", 0) > 0:
            any_hits = True
            lines.append(
                f"**{p.language}** has **{bt['count_score_gt_zero']}** functions with blast score > 0 "
                f"(peak: `{bt.get('max_symbol')}` = **{bt.get('max_score', 0):.2f}**)."
            )
    if not any_hits:
        lines.append(
            "No indexed function returned blast score > 0 in the top-score scan "
            "(checkout targets are often leaf nodes with score 0)."
        )

    return " ".join(lines)


def render_blast_top_summary_markdown(results: dict[str, Any]) -> list[str]:
    """Cross-project summary table for full function blast scan."""
    lines = [
        "",
        "## Blast-radius scan (summary)",
        "",
        "Full function scan across all languages. "
        "[Per-language top symbols →](languages/) see each language report § Top symbols.",
        "",
        "| Project | Scanned | Score > 0 | Max score | Top symbol | Report |",
        "|---------|--------:|----------:|----------:|------------|--------|",
    ]
    for p in active_projects(results):
        bt = results[p.id].get("blast_top_scores", {})
        if bt.get("skipped"):
            lines.append(
                f"| {p.language} | — | — | — | skipped | "
                f"[{p.id}.md](languages/{p.id}.md) |"
            )
            continue
        max_s = bt.get("max_score") or 0
        max_sym = bt.get("max_symbol") or "—"
        lines.append(
            f"| {p.language} | {bt.get('functions_scanned', '—')} | "
            f"{bt.get('count_score_gt_zero', 0)} | "
            f"{max_s:.2f} | `{max_sym}` | "
            f"[{p.id}.md](languages/{p.id}.md) |"
        )
    lines.append("")
    return lines


def render_blast_top_symbols_for_project(result: dict[str, Any]) -> list[str]:
    """Top symbol table for a single language report."""
    bt = result.get("blast_top_scores", {})
    top = bt.get("top") or []
    lines = [
        "",
        "## Top symbols",
        "",
        "Every function from `MATCH (n:Function) RETURN n` scored with `blast-radius`. "
        "Checkout handlers are often leaf nodes (score 0); hubs and repository methods rank higher.",
        "",
    ]
    if bt.get("skipped"):
        lines += ["_Scan skipped for this run._", ""]
        return lines
    lines += [
        f"**Scanned:** {bt.get('functions_scanned', '—')} functions · "
        f"**Score > 0:** {bt.get('count_score_gt_zero', 0)} · "
        f"**Max:** `{bt.get('max_symbol') or '—'}` = {bt.get('max_score') or 0:.2f}",
        "",
    ]
    if not top:
        lines += ["_No functions with score > 0._", ""]
        return lines
    lines += [
        "| Rank | Symbol | Score | Callers | Impact | FQN |",
        "|-----:|--------|------:|--------:|-------:|-----|",
    ]
    for i, row in enumerate(top, 1):
        lines.append(
            f"| {i} | `{row.get('symbol')}` | {row.get('score', 0):.2f} | "
            f"{row.get('direct_callers', '—')} | {row.get('impact_zone_size', '—')} | "
            f"`{row.get('canonical_fqn') or '—'}` |"
        )
    lines += [
        "",
        f"Raw JSON: [`../{result.get('project', 'project')}-blast-top.json`](../{result.get('project')}-blast-top.json)",
        "",
    ]
    return lines


def render_language_markdown(
    project: ProjectConfig,
    result: dict[str, Any],
    rgctl_path: Path,
    run_date: str,
) -> str:
    f = result.get("features", {})
    d = f.get("discover", {})
    g = f.get("gql", {})
    b = f.get("blast_radius", {})
    m = f.get("metrics", {})
    mod = m.get("modularity")
    mod_s = round(mod, 3) if isinstance(mod, (int, float)) else "—"

    lines = [
        f"# rgctl Report — {project.language}",
        "",
        f"**App:** `{project.dir_name}` · **Run date:** {run_date}  ",
        f"**rgctl:** `{rgctl_path}`  ",
        f"**Summary report:** [REPORT.md](../REPORT.md) · [HTML](../REPORT.html)",
        "",
        "## Overview",
        "",
        language_executive_summary(project, result),
        "",
        project.notes,
        "",
        "## Feature coverage",
        "",
        "| Feature | Status | Details |",
        "|---------|:------:|---------|",
    ]
    for label, key in FEATURE_ROWS:
        feat = f.get(key, {})
        lines.append(f"| {label} | {icon(feat.get('status'))} | {feature_detail(key, feat)} |")

    lines += [
        "",
        "## Indexing (`discover`)",
        "",
        "| Metric | Value |",
        "|--------|------:|",
        f"| Status | {icon(d.get('status'))} |",
        f"| Files discovered | {d.get('files_discovered', '—')} |",
        f"| Files indexed | {d.get('files_indexed', '—')} |",
        f"| Files skipped | {d.get('files_skipped', '—')} |",
        f"| Nodes | {d.get('nodes_generated', '—')} |",
        f"| Edges | {d.get('edges_generated', '—')} |",
        f"| Duration | {d.get('duration_ms', '—')} ms |",
        f"| Cache | {result.get('cache_mb', '—')} MB |",
        f"| CFG enabled | {d.get('cfg_enabled', True)} |",
        f"| Exclude (`-e`) | `{project.exclude}` |",
        "",
        "```bash",
        f"cd {project.dir_name}",
        f"rgctl -f json discover . --cfg -e {project.exclude}",
        "```",
        "",
        "## Graph query (`gql`)",
        "",
        "| Metric | Value |",
        "|--------|------:|",
        f"| Functions | {g.get('functions', '—')} |",
        f"| Classes | {g.get('classes', '—')} |",
        f"| Files | {g.get('files', '—')} |",
        f"| CALLS edges (sample) | {g.get('calls_edges_sampled', '—')} |",
        "",
        "```bash",
        "rgctl gql 'MATCH (n:Function) RETURN n'",
        "rgctl gql 'MATCH (n:Function) WHERE n.name LIKE \"*checkout*\" RETURN n'",
        "rgctl gql 'MATCH (a:Function)-[:CALLS]->(b:Function) RETURN a,b LIMIT 20'",
        "```",
        "",
        "## Blast radius — checkout target",
        "",
        "| Field | Value |",
        "|-------|-------|",
        f"| Symbol | `{b.get('symbol', project.blast_symbol)}` |",
        f"| Status | {icon(b.get('status'))} |",
        f"| Score | {b.get('score', '—')} |",
        f"| Direct callers | {b.get('direct_callers', '—')} |",
        f"| Impact zone | {b.get('impact_zone_size', '—')} |",
        f"| Target file | `{b.get('target_file') or '—'}` |",
        "",
        "```bash",
        f"rgctl -f json blast-radius '{project.blast_symbol}'",
        "```",
    ]
    if b.get("error"):
        lines += ["", f"_Error:_ `{b['error'][:200]}`", ""]

    lines += render_blast_top_symbols_for_project(result)

    lines += [
        "",
        "## Metrics (`metrics --communities --pagerank`)",
        "",
        "| Metric | Value |",
        "|--------|------:|",
        f"| Communities | {m.get('communities', '—')} |",
        f"| Modularity | {mod_s} |",
        f"| PageRank converged | {m.get('pagerank_converged', '—')} |",
        f"| PageRank iterations | {m.get('pagerank_iterations', '—')} |",
        "",
        "```bash",
        "rgctl -f json metrics --communities --pagerank",
        "```",
        "",
        "## Export (`export`)",
        "",
    ]
    e = f.get("export", {})
    lines += [
        f"- Status: {icon(e.get('status'))} · **{e.get('bytes', 0):,}** bytes",
        f"- Query: `{e.get('query', '—')}`",
        "",
        "## CI policy (`check --policy-file`)",
        "",
    ]
    c = f.get("check", {})
    lines += [
        f"- Status: {icon(c.get('status'))} · passed={c.get('passed', '—')}",
        f"- Policy: [`../../rgctl-policy.json`](../../rgctl-policy.json)",
        "",
        "## Deep analysis",
        "",
        "### Program slice",
        "",
    ]
    sl = f.get("slice", {})
    lines += [
        f"| Field | Value |",
        f"|-------|-------|",
        f"| Status | {icon(sl.get('status'))} |",
        f"| File | `{sl.get('file', '—')}` |",
        f"| Line | {sl.get('line', '—')} |",
        f"| Variable | `{sl.get('variable', '—')}` |",
        f"| Function | `{sl.get('function', '—')}` |",
        f"| Output lines | {sl.get('output_lines', '—')} |",
        "",
    ]
    if sl.get("error"):
        lines.append(f"_Note:_ {sl['error'][:300]}")
        lines.append("")

    ta = f.get("taint", {})
    lines += [
        "### Taint analysis",
        "",
        f"Status: {icon(ta.get('status'))}",
        "",
    ]
    if ta.get("flows"):
        lines += [f"Preview: `{ta['flows'][:200]}`", ""]
    if ta.get("error"):
        lines += [f"_Error:_ {ta['error'][:200]}", ""]

    for title, key in [
        ("Inspect CFG", "inspect_cfg"),
        ("Inspect PDG", "inspect_pdg"),
        ("Inspect dominators", "inspect_dom"),
    ]:
        ins = f.get(key, {})
        lines += [f"### {title}", "", f"Status: {icon(ins.get('status'))}", ""]
        if ins.get("preview"):
            lines += [f"Preview: `{ins['preview']}`", ""]
        if ins.get("error"):
            lines += [f"_Error:_ {ins['error'][:200]}", ""]

    srv = f.get("serve", {})
    lines += [
        "## Serve daemon",
        "",
        f"Status: {icon(srv.get('status'))} — {srv.get('note', 'Not exercised in batch run.')}",
        "",
        "## Raw artifacts",
        "",
        f"| File | Description |",
        f"|------|-------------|",
        f"| [`../{project.id}-summary.json`](../{project.id}-summary.json) | Feature matrix |",
        f"| [`../{project.id}-metrics.json`](../{project.id}-metrics.json) | Raw metrics |",
        f"| [`../{project.id}-blast.json`](../{project.id}-blast.json) | Checkout blast-radius |",
        f"| [`../{project.id}-blast-top.json`](../{project.id}-blast-top.json) | Top symbol scan |",
        f"| [`../{project.id}-export.json`](../{project.id}-export.json) | Function subgraph export |",
        f"| [`../{project.id}-check.json`](../{project.id}-check.json) | Policy check output |",
        "",
        "## Reproduce",
        "",
        "```bash",
        f"cd {project.dir_name}",
        f"rgctl -f json discover . --cfg -e {project.exclude}",
        f"rgctl -f json blast-radius '{project.blast_symbol}'",
        "rgctl -f json metrics --communities --pagerank",
        "rgctl -f json check --policy-file ../rgctl-policy.json",
        "```",
        "",
    ]
    return "\n".join(lines)


def render_language_html(
    project: ProjectConfig,
    result: dict[str, Any],
    rgctl_path: Path,
    run_at: str,
    run_date: str,
) -> str:
    f = result.get("features", {})
    d = f.get("discover", {})
    b = f.get("blast_radius", {})
    m = f.get("metrics", {})
    mod = m.get("modularity")
    mod_s = round(mod, 3) if isinstance(mod, (int, float)) else "—"
    bt = result.get("blast_top_scores", {})
    top = bt.get("top") or []

    feature_rows = [
        [html_esc(label), status_html(f.get(key, {}).get("status")), html_esc(feature_detail(key, f.get(key, {})))]
        for label, key in FEATURE_ROWS
    ]

    top_table = ""
    if bt.get("skipped"):
        top_table = "<p><em>Scan skipped for this run.</em></p>"
    elif not top:
        top_table = "<p><em>No functions with score &gt; 0.</em></p>"
    else:
        top_table = html_table(
            ["Rank", "Symbol", "Score", "Callers", "Impact", "FQN"],
            [
                [
                    html_esc(i),
                    f"<code>{html_esc(row.get('symbol'))}</code>",
                    html_esc(f"{row.get('score', 0):.2f}"),
                    html_esc(row.get("direct_callers")),
                    html_esc(row.get("impact_zone_size")),
                    f"<code>{html_esc(row.get('canonical_fqn') or '—')}</code>",
                ]
                for i, row in enumerate(top, 1)
            ],
            numeric_cols={0, 2, 3, 4},
        )

    parts = [
        "<!DOCTYPE html>",
        '<html lang="en">',
        "<head>",
        '<meta charset="utf-8">',
        f"<title>rgctl Report — {html_esc(project.language)}</title>",
        f"<style>{HTML_STYLE}</style>",
        "</head>",
        "<body>",
        f"<h1>rgctl Report — {html_esc(project.language)}</h1>",
        f'<p class="meta"><strong>App:</strong> <code>{html_esc(project.dir_name)}</code><br>',
        f"<strong>Run date:</strong> {html_esc(run_date)}<br>",
        f"<strong>rgctl:</strong> <code>{html_esc(rgctl_path)}</code><br>",
        f'<a href="../REPORT.md">Summary report</a> · '
        f'<a href="{html_esc(project.id)}.md">Markdown</a> · '
        f'<a href="../{html_esc(project.id)}-summary.json">JSON</a></p>',
        f"<p>{html_esc(language_executive_summary(project, result))}</p>",
        f"<p>{html_esc(project.notes)}</p>",
        "<h2>Feature coverage</h2>",
        html_table(["Feature", "Status", "Details"], feature_rows),
        "<h2>Indexing</h2>",
        html_table(
            ["Metric", "Value"],
            [
                ["Status", status_html(d.get("status"))],
                ["Files discovered", html_esc(d.get("files_discovered"))],
                ["Files indexed", html_esc(d.get("files_indexed"))],
                ["Nodes", html_esc(d.get("nodes_generated"))],
                ["Edges", html_esc(d.get("edges_generated"))],
                ["Duration", html_esc(f"{d.get('duration_ms')} ms")],
                ["Cache", html_esc(f"{result.get('cache_mb')} MB")],
                ["Exclude", f"<code>{html_esc(project.exclude)}</code>"],
            ],
            numeric_cols={1},
        ),
        "<h2>Blast radius — checkout</h2>",
        html_table(
            ["Field", "Value"],
            [
                ["Symbol", f"<code>{html_esc(b.get('symbol'))}</code>"],
                ["Score", html_esc(b.get("score"))],
                ["Direct callers", html_esc(b.get("direct_callers"))],
                ["Impact zone", html_esc(b.get("impact_zone_size"))],
                ["Status", status_html(b.get("status"))],
            ],
            numeric_cols={1},
        ),
        "<h2>Top symbols</h2>",
        f"<p>Scanned {html_esc(bt.get('functions_scanned', '—'))} functions; "
        f"{html_esc(bt.get('count_score_gt_zero', 0))} with score &gt; 0.</p>",
        top_table,
        "<h2>Metrics</h2>",
        html_table(
            ["Metric", "Value"],
            [
                ["Communities", html_esc(m.get("communities"))],
                ["Modularity", html_esc(mod_s)],
                ["PageRank converged", html_esc(m.get("pagerank_converged"))],
                ["PageRank iterations", html_esc(m.get("pagerank_iterations"))],
            ],
        ),
        "<h2>Deep analysis</h2>",
        html_table(
            ["Analysis", "Status", "Details"],
            [
                ["Slice", status_html(f.get("slice", {}).get("status")), html_esc(feature_detail("slice", f.get("slice", {})))],
                ["Taint", status_html(f.get("taint", {}).get("status")), html_esc(feature_detail("taint", f.get("taint", {})))],
                ["Inspect CFG", status_html(f.get("inspect_cfg", {}).get("status")), html_esc(feature_detail("inspect_cfg", f.get("inspect_cfg", {})))],
                ["Inspect PDG", status_html(f.get("inspect_pdg", {}).get("status")), html_esc(feature_detail("inspect_pdg", f.get("inspect_pdg", {})))],
                ["Inspect dominators", status_html(f.get("inspect_dom", {}).get("status")), html_esc(feature_detail("inspect_dom", f.get("inspect_dom", {})))],
            ],
        ),
        "<h2>Reproduce</h2>",
        "<pre>"
        f"cd {html_esc(project.dir_name)}\n"
        f"rgctl -f json discover . --cfg -e {html_esc(project.exclude)}\n"
        f"rgctl -f json blast-radius '{html_esc(project.blast_symbol)}'\n"
        "rgctl -f json metrics --communities --pagerank\n"
        "rgctl -f json check --policy-file ../rgctl-policy.json"
        "</pre>",
        "</body></html>",
    ]
    return "\n".join(parts)


def render_blast_top_markdown(results: dict[str, Any]) -> list[str]:
    return render_blast_top_summary_markdown(results)


def render_summary_markdown(
    results: dict[str, Any],
    rgctl_path: Path,
    run_date: str,
) -> str:
    projects = active_projects(results)
    lines = [
        "# rgctl Summary Report — Tier 1 E-commerce Test Apps",
        "",
        f"**Run date:** {run_date}  ",
        f"**rgctl:** `{rgctl_path}`  ",
        "**Scope:** cross-project comparison. Per-language deep dives: [`languages/`](languages/).",
        "",
        "Raw machine output: [`all-results.json`](all-results.json) · "
        "[HTML summary](REPORT.html)",
        "",
        "## Executive summary",
        "",
        executive_summary(results),
        "",
        "## Language reports",
        "",
        "Comprehensive per-language reports (indexing, GQL, blast, top symbols, metrics, deep analysis).",
        "",
        "| Language | App | Files | Edges | Max blast | Report |",
        "|----------|-----|------:|------:|----------:|--------|",
    ]
    for p in projects:
        d = results[p.id]["features"]["discover"]
        bt = results[p.id].get("blast_top_scores", {})
        max_b = f"{(bt.get('max_score') or 0):.2f}" if not bt.get("skipped") else "—"
        lines.append(
            f"| {p.language} | `{p.dir_name}` | {d.get('files_indexed', '—')} | "
            f"{d.get('edges_generated', '—')} | {max_b} | "
            f"[MD](languages/{p.id}.md) · [HTML](languages/{p.id}.html) |"
        )

    lines += [
        "",
        "## Summary matrix",
        "",
        "| Feature | " + " | ".join(p.language for p in projects) + " |",
        "|---------|" + "|".join(":----:" for _ in projects) + "|",
    ]
    for label, key in FEATURE_ROWS:
        row = [label] + [
            icon(results[p.id]["features"].get(key, {}).get("status")) for p in projects
        ]
        lines.append("| " + " | ".join(row) + " |")

    lines += [
        "",
        "## Indexing (`discover`)",
        "",
        "| Project | Files indexed | Nodes | Edges | Duration | Cache |",
        "|---------|--------------:|------:|------:|---------:|------:|",
    ]
    for p in projects:
        d = results[p.id]["features"]["discover"]
        if d.get("status") != "ok":
            lines.append(f"| {p.language} | — | — | — | fail | — |")
            continue
        lines.append(
            f"| {p.language} | {d.get('files_indexed')} | {d.get('nodes_generated')} | "
            f"{d.get('edges_generated')} | {d.get('duration_ms')} ms | {results[p.id]['cache_mb']} MB |"
        )

    lines += [
        "",
        "## GQL (`gql`)",
        "",
        "| Project | Functions | Classes | Files | CALLS sample |",
        "|---------|----------:|--------:|------:|-------------:|",
    ]
    for p in projects:
        g = results[p.id]["features"].get("gql", {})
        lines.append(
            f"| {p.language} | {g.get('functions', '—')} | {g.get('classes', '—')} | "
            f"{g.get('files', '—')} | {g.get('calls_edges_sampled', '—')} |"
        )

    lines += [
        "",
        "## Blast radius — checkout target",
        "",
        "| Project | Symbol | Score | Direct callers | Impact zone | Status |",
        "|---------|--------|------:|---------------:|------------:|:------:|",
    ]
    for p in projects:
        b = results[p.id]["features"].get("blast_radius", {})
        if b.get("status") == "ok":
            lines.append(
                f"| {p.language} | `{b.get('symbol')}` | {b.get('score')} | "
                f"{b.get('direct_callers')} | {b.get('impact_zone_size')} | {icon('ok')} |"
            )
        else:
            note = b.get("error") or b.get("status", "—")
            lines.append(
                f"| {p.language} | `{b.get('symbol', '—')}` | — | — | — | "
                f"{icon(b.get('status'))} {str(note)[:40]} |"
            )

    lines += render_blast_top_markdown(results)

    lines += [
        "",
        "## Metrics (`metrics --communities --pagerank`)",
        "",
        "| Project | Communities | Modularity | PageRank converged | Iterations |",
        "|---------|------------:|-----------:|:------------------:|-----------:|",
    ]
    for p in projects:
        m = results[p.id]["features"].get("metrics", {})
        mod = m.get("modularity")
        mod_s = round(mod, 3) if isinstance(mod, (int, float)) else "—"
        lines.append(
            f"| {p.language} | {m.get('communities', '—')} | {mod_s} | "
            f"{m.get('pagerank_converged', '—')} | {m.get('pagerank_iterations', '—')} |"
        )

    lines += [
        "",
        "## Export · CI · Deep analysis",
        "",
        "| Project | Export | Check | Slice | Taint | CFG | PDG |",
        "|---------|:------:|:-----:|:-----:|:-----:|:---:|:---:|",
    ]
    for p in projects:
        f = results[p.id]["features"]
        e = f.get("export", {})
        lines.append(
            f"| {p.language} | {icon(e.get('status'))} ({e.get('bytes', 0):,} B) | "
            f"{icon(f.get('check', {}).get('status'))} | "
            f"{icon(f.get('slice', {}).get('status'))} | {icon(f.get('taint', {}).get('status'))} | "
            f"{icon(f.get('inspect_cfg', {}).get('status'))} | "
            f"{icon(f.get('inspect_pdg', {}).get('status'))} |"
        )

    lines += [
        "",
        "## Reproduce",
        "",
        "```bash",
        "./scripts/run_rgctl_report.py",
        "RGCTL=/path/to/rgctl ./scripts/run_rgctl_report.py --update-readmes",
        "```",
        "",
    ]
    return "\n".join(lines)


def render_summary_html(
    results: dict[str, Any],
    rgctl_path: Path,
    run_at: str,
    run_date: str,
) -> str:
    projects = active_projects(results)

    feature_rows = []
    for label, key in FEATURE_ROWS:
        cells = [html_esc(label)] + [
            status_html(results[p.id]["features"].get(key, {}).get("status"))
            for p in projects
        ]
        feature_rows.append(cells)

    lang_rows = []
    for p in projects:
        d = results[p.id]["features"]["discover"]
        bt = results[p.id].get("blast_top_scores", {})
        max_b = f"{(bt.get('max_score') or 0):.2f}" if not bt.get("skipped") else "—"
        lang_rows.append([
            html_esc(p.language),
            f"<code>{html_esc(p.dir_name)}</code>",
            html_esc(d.get("files_indexed")),
            html_esc(d.get("edges_generated")),
            html_esc(max_b),
            f'<a href="languages/{html_esc(p.id)}.html">{html_esc(p.id)}</a>',
        ])

    index_rows = []
    for p in projects:
        d = results[p.id]["features"]["discover"]
        if d.get("status") != "ok":
            index_rows.append([html_esc(p.language), "—", "—", "—", "—", "—"])
        else:
            index_rows.append([
                html_esc(p.language),
                html_esc(d.get("files_indexed")),
                html_esc(d.get("nodes_generated")),
                html_esc(d.get("edges_generated")),
                html_esc(f"{d.get('duration_ms')} ms"),
                html_esc(f"{results[p.id]['cache_mb']} MB"),
            ])

    blast_rows = []
    for p in projects:
        bt = results[p.id].get("blast_top_scores", {})
        blast_rows.append([
            html_esc(p.language),
            html_esc(bt.get("functions_scanned", "—")),
            html_esc(bt.get("count_score_gt_zero", 0)),
            html_esc(f"{(bt.get('max_score') or 0):.2f}"),
            f"<code>{html_esc(bt.get('max_symbol') or '—')}</code>",
            f'<a href="languages/{html_esc(p.id)}.html">report</a>',
        ])

    body_parts = [
        "<!DOCTYPE html>",
        '<html lang="en">',
        "<head>",
        '<meta charset="utf-8">',
        "<title>rgctl Summary Report</title>",
        f"<style>{HTML_STYLE}</style>",
        "</head>",
        "<body>",
        "<h1>rgctl Summary Report — Tier 1 E-commerce Test Apps</h1>",
        f'<p class="meta"><strong>Run date:</strong> {html_esc(run_date)}<br>',
        f"<strong>rgctl:</strong> <code>{html_esc(rgctl_path)}</code><br>",
        f'<a href="REPORT.md">Markdown</a> · <a href="all-results.json">JSON</a> · '
        f'<a href="languages/">Language reports</a></p>',
        f"<p>{html_esc(executive_summary(results))}</p>",
        "<h2>Language reports</h2>",
        html_table(
            ["Language", "App", "Files", "Edges", "Max blast", "Report"],
            lang_rows,
            numeric_cols={2, 3, 4},
        ),
        "<h2>Summary matrix</h2>",
        html_table(
            ["Feature"] + [p.language for p in projects],
            feature_rows,
        ),
        "<h2>Indexing</h2>",
        html_table(
            ["Project", "Files", "Nodes", "Edges", "Duration", "Cache"],
            index_rows,
            numeric_cols={1, 2, 3},
        ),
        "<h2>Blast-radius scan</h2>",
        html_table(
            ["Project", "Scanned", "Score &gt; 0", "Max score", "Top symbol", "Details"],
            blast_rows,
            numeric_cols={1, 2, 3},
        ),
        "<h2>Reproduce</h2>",
        "<pre>./scripts/run_rgctl_report.py\n"
        "RGCTL=/path/to/rgctl ./scripts/run_rgctl_report.py --update-readmes</pre>",
        "</body></html>",
    ]
    return "\n".join(body_parts)


# Backward-compatible aliases
render_markdown = render_summary_markdown
render_html = render_summary_html


def render_reports_index(run_date: str, results: dict[str, Any] | None = None) -> str:
    lang_rows = ""
    if results:
        for p in active_projects(results):
            lang_rows += (
                f"| {p.language} | [`languages/{p.id}.md`](languages/{p.id}.md) · "
                f"[HTML](languages/{p.id}.html) |\n"
            )
    else:
        lang_rows = "| _(run report to populate)_ | — |\n"

    return f"""# rgctl reports

Published output from [`scripts/run_rgctl_report.py`](../scripts/run_rgctl_report.py).

## Summary

| Artifact | Description |
|----------|-------------|
| [REPORT.md](REPORT.md) | Cross-project summary (last run: {run_date}) |
| [REPORT.html](REPORT.html) | HTML summary |
| [all-results.json](all-results.json) | Machine-readable results for all projects |

## Language reports (`languages/`)

Comprehensive per-language analysis (indexing, GQL, blast, top symbols, metrics, deep analysis).

| Language | Report |
|----------|--------|
{lang_rows}
## Raw artifacts

| Pattern | Description |
|---------|-------------|
| `<project>-summary.json` | Per-project feature matrix |
| `<project>-metrics.json` | Raw `metrics` command output |
| `<project>-blast.json` | Raw `blast-radius` JSON (checkout target) |
| `<project>-blast-top.json` | Top blast scores from full function scan |
| `<project>-export.json` | Exported function subgraph |

Regenerate:

```bash
./scripts/run_rgctl_report.py --update-readmes
```
"""


def update_parent_readme(repo_root: Path, results: dict[str, Any], run_at: str) -> None:
    readme = repo_root / "README.md"
    text = readme.read_text() if readme.is_file() else ""
    marker = "## rgctl analysis results"
    if marker in text:
        text = text.split(marker)[0].rstrip() + "\n"

    block = [
        "",
        marker,
        "",
        f"Summary: **[rgctl-reports/REPORT.md](rgctl-reports/REPORT.md)** · "
        f"[HTML](rgctl-reports/REPORT.html) (run {run_at})",
        "",
        "**Language reports:** "
        + " · ".join(
            f"[{p.language}](rgctl-reports/languages/{p.id}.md)"
            for p in active_projects(results)
        ),
        "",
        "### Feature coverage (✓ ok · ◐ partial · — unsupported/n/a)",
        "",
        "| Feature | Rust | Py | Go | Java | TS | JS |",
        "|---------|:----:|:--:|:--:|:----:|:--:|:--:|",
    ]
    for label, key in FEATURE_ROWS:
        row = [label] + [
            icon(results.get(p.id, {}).get("features", {}).get(key, {}).get("status"))
            for p in PROJECTS
        ]
        block.append("| " + " | ".join(row) + " |")
    block += [
        "",
        "### Index size",
        "",
        "| Project | Files | Nodes | Edges | Discover ms |",
        "|---------|------:|------:|------:|------------:|",
    ]
    for p in PROJECTS:
        d = results.get(p.id, {}).get("features", {}).get("discover", {})
        if d.get("status") == "ok":
            block.append(
                f"| {p.language} | {d.get('files_indexed')} | {d.get('nodes_generated')} | "
                f"{d.get('edges_generated')} | {d.get('duration_ms')} |"
            )
        else:
            block.append(f"| {p.language} | — | — | — | fail |")
    block += [
        "",
        "### Blast radius (max score per project)",
        "",
        "Full function scan (`--blast-top N`); checkout leaf symbols often score 0.",
        "",
        "| Project | Scanned | Score > 0 | Max score | Top symbol |",
        "|---------|--------:|----------:|----------:|------------|",
    ]
    for p in PROJECTS:
        bt = results.get(p.id, {}).get("blast_top_scores", {})
        if not bt or bt.get("skipped"):
            block.append(f"| {p.language} | — | — | — | (scan skipped) |")
        else:
            block.append(
                f"| {p.language} | {bt.get('functions_scanned', '—')} | "
                f"{bt.get('count_score_gt_zero', 0)} | "
                f"{(bt.get('max_score') or 0):.2f} | `{bt.get('max_symbol') or '—'}` |"
            )
    block += [
        "",
        "Per-project details: [`rgctl-reports/languages/`](rgctl-reports/languages/) · "
        "each `ecommerce-*/README.md` § **rgctl**.",
        "",
        "Regenerate: `./scripts/run_rgctl_report.py`",
        "",
    ]
    readme.write_text(text + "\n".join(block))


def update_project_readmes(repo_root: Path, results: dict[str, Any], run_at: str) -> None:
    for p in active_projects(results):
        readme = repo_root / p.dir_name / "README.md"
        if not readme.is_file():
            readme.write_text(f"# {p.dir_name}\n\nE-commerce reference app.\n")
        text = readme.read_text()
        if "## rgctl" in text:
            text = text.split("## rgctl")[0].rstrip() + "\n"

        r = results.get(p.id, {})
        f = r.get("features", {})
        d = f.get("discover", {})
        bt = r.get("blast_top_scores", {})
        top_rows = bt.get("top") or []
        top_md = ""
        if bt.get("skipped"):
            top_md = "\n### Top symbols\n\n_Scan skipped for this run._\n"
        elif top_rows:
            top_md = "\n### Top symbols\n\n| Symbol | Score | Callers | Impact |\n|--------|------:|--------:|-------:|\n"
            for row in top_rows[:5]:
                top_md += (
                    f"| `{row.get('symbol')}` | {row.get('score', 0):.2f} | "
                    f"{row.get('direct_callers', '—')} | {row.get('impact_zone_size', '—')} |\n"
                )
        else:
            top_md = "\n### Top symbols\n\n_No functions with score > 0._\n"

        section = f"""
## rgctl

See [summary report](../rgctl-reports/REPORT.md) · [language report](../rgctl-reports/languages/{p.id}.md) · [HTML](../rgctl-reports/languages/{p.id}.html) ({run_at}).

```bash
rgctl -f json discover . --cfg -e {p.exclude}
rgctl -f json blast-radius '{p.blast_symbol}'
rgctl -f json metrics --communities --pagerank
rgctl -f json check --policy-file ../rgctl-policy.json
```

| Metric | Value |
|--------|------:|
| Files indexed | {d.get('files_indexed', '—')} |
| Nodes | {d.get('nodes_generated', '—')} |
| Edges | {d.get('edges_generated', '—')} |
| Discover ms | {d.get('duration_ms', '—')} |
| Cache MB | {r.get('cache_mb', '—')} |

| Feature | Status |
|---------|:------:|
| discover | {icon(f.get('discover', {}).get('status'))} |
| blast-radius | {icon(f.get('blast_radius', {}).get('status'))} |
| metrics | {icon(f.get('metrics', {}).get('status'))} |
| export | {icon(f.get('export', {}).get('status'))} |
| check | {icon(f.get('check', {}).get('status'))} |
| slice / taint | {icon(f.get('slice', {}).get('status'))} / {icon(f.get('taint', {}).get('status'))} |
{top_md}
{p.notes}

Raw: [`../rgctl-reports/{p.id}-summary.json`](../rgctl-reports/{p.id}-summary.json)
"""
        readme.write_text(text.rstrip() + "\n" + section)


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--rgctl",
        help="Path to rgctl binary (default: RGCTL env, PATH, or ../rgctl/target/debug/rgctl)",
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=None,
        help="Report output directory (default: <repo>/rgctl-reports)",
    )
    parser.add_argument(
        "--repo-root",
        type=Path,
        default=None,
        help="rgctl-tests repo root (default: parent of scripts/)",
    )
    parser.add_argument(
        "--no-clean",
        action="store_true",
        help="Do not remove existing .rgctl caches before discover",
    )
    parser.add_argument(
        "--update-readmes",
        action="store_true",
        help="Refresh README.md summary tables in repo root and each ecommerce-* project",
    )
    parser.add_argument(
        "--projects",
        nargs="*",
        choices=[p.id for p in PROJECTS],
        help="Run subset of projects (default: all)",
    )
    parser.add_argument(
        "--blast-top",
        type=int,
        default=10,
        metavar="N",
        help="Number of top blast scores to keep per project (default: 10)",
    )
    parser.add_argument(
        "--skip-blast-scan",
        action="store_true",
        help="Skip full function blast-radius scan (faster; omits top scores section)",
    )
    args = parser.parse_args()

    repo_root = (args.repo_root or Path(__file__).resolve().parents[1]).resolve()
    out_dir = (args.output_dir or repo_root / "rgctl-reports").resolve()
    policy = repo_root / "rgctl-policy.json"
    rgctl = resolve_rgctl(args.rgctl)

    if not policy.is_file():
        sys.exit(f"Missing policy file: {policy}")

    out_dir.mkdir(parents=True, exist_ok=True)
    projects = [p for p in PROJECTS if not args.projects or p.id in args.projects]

    run_at = datetime.now(timezone.utc).strftime("%Y-%m-%d %H:%M UTC")
    run_date = date.today().isoformat()

    meta = {
        "run_at": run_at,
        "run_date": run_date,
        "rgctl": str(rgctl),
        "repo_root": str(repo_root),
        "projects": [p.id for p in projects],
    }
    (out_dir / "run-meta.json").write_text(json.dumps(meta, indent=2))

    print(f"rgctl: {rgctl}")
    print(f"output:   {out_dir}")
    print(f"projects: {', '.join(p.id for p in projects)}")
    print()

    results: dict[str, Any] = {}
    failures = 0
    for project in projects:
        print(f"=== {project.id} ({project.language}) ===", flush=True)
        result = run_project(
            rgctl,
            repo_root,
            out_dir,
            policy,
            project,
            clean_cache=not args.no_clean,
            blast_scan=not args.skip_blast_scan,
            blast_top_n=args.blast_top,
        )
        results[project.id] = result
        st = result["features"].get("discover", {}).get("status")
        if st != "ok":
            failures += 1
            print(f"  discover: FAIL", flush=True)
        else:
            d = result["features"]["discover"]
            print(
                f"  discover: ok — files={d.get('files_indexed')} nodes={d.get('nodes_generated')} "
                f"edges={d.get('edges_generated')} ({d.get('duration_ms')} ms)",
                flush=True,
            )

    (out_dir / "all-results.json").write_text(json.dumps(results, indent=2))

    lang_dir = out_dir / "languages"
    lang_dir.mkdir(parents=True, exist_ok=True)
    for project in projects:
        result = results[project.id]
        lang_md = render_language_markdown(project, result, rgctl, run_date)
        (lang_dir / f"{project.id}.md").write_text(lang_md)
        lang_html = render_language_html(project, result, rgctl, run_at, run_date)
        (lang_dir / f"{project.id}.html").write_text(lang_html)

    md = render_summary_markdown(results, rgctl, run_date)
    (out_dir / "REPORT.md").write_text(md)

    html_doc = render_summary_html(results, rgctl, run_at, run_date)
    (out_dir / "REPORT.html").write_text(html_doc)

    (out_dir / "README.md").write_text(render_reports_index(run_date, results))

    if args.update_readmes:
        update_parent_readme(repo_root, results, run_date)
        update_project_readmes(repo_root, results, run_date)
        print("Updated README.md files", flush=True)

    print()
    print(f"Wrote {out_dir / 'REPORT.md'} (summary)")
    print(f"Wrote {out_dir / 'REPORT.html'} (summary)")
    print(f"Wrote {lang_dir}/ (language reports)")
    print(f"Wrote {out_dir / 'all-results.json'}")

    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
