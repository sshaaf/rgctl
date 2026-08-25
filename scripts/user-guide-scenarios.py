#!/usr/bin/env python3
"""Run docs/user-guide/scenarios/*.json against rgbuilder-tests/ecommerce-java.

Modes:
  (default)     run suite + soft marker presence
  --check-markers   only verify open/close markers exist
  --check           run suite and verify marker-bounded samples match scrubbed output
  --sync            run suite and rewrite marker-bounded samples from live output
  --strict-markers  fail if any scenario lacks markers in user-guide.md
"""

from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SCENARIO_DIR = ROOT / "docs" / "user-guide" / "scenarios"
FIXTURE = ROOT / "rgbuilder-tests" / "ecommerce-java"
GUIDE = ROOT / "docs" / "user-guide.md"

# Volatile / host-specific JSON keys (scrubbed for guide compare)
SCRUB_KEYS = {
    "duration_ms",
    "edges_generated",
    "nodes_generated",
    "id",
    "uuid",
    "elapsed_ms",
    "peak_rss_bytes",
    "wall_ms",
}


def find_bin() -> Path:
    env = os.environ.get("CARGO_BIN_EXE_rgctl")
    if env:
        return Path(env)
    which = shutil.which("rgctl")
    if which:
        return Path(which)
    for cand in (
        ROOT / "target" / "release" / "rgctl",
        ROOT / "target" / "debug" / "rgctl",
    ):
        if cand.is_file():
            return cand
    raise SystemExit("rgctl binary not found — cargo build --release -p rgbuilder")


def load_scenarios() -> list[dict]:
    files = sorted(SCENARIO_DIR.glob("*.json"))
    if not files:
        raise SystemExit(f"no scenarios in {SCENARIO_DIR}")
    out = []
    for f in files:
        data = json.loads(f.read_text())
        data["_path"] = str(f)
        out.append(data)
    return out


def run_one(bin: Path, repo: Path, args: list[str]) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [str(bin), "-r", str(repo), *args],
        cwd=str(repo),
        capture_output=True,
        text=True,
        check=False,
    )


def assert_json(stdout: str, keys: list[str], sid: str) -> None:
    try:
        doc = json.loads(stdout)
    except json.JSONDecodeError as e:
        raise AssertionError(f"{sid}: stdout not JSON: {e}\n{stdout[:500]}") from e
    for k in keys:
        if k not in doc:
            raise AssertionError(f"{sid}: missing JSON key {k!r} in {list(doc)[:20]}")


def ensure_discover(bin: Path, repo: Path) -> None:
    rb = repo / ".rgbuilder"
    if (rb / "graph.snapshot.bin").is_file():
        return
    print("==> discover (suite setup)")
    cp = run_one(bin, repo, ["discover", ".", "-l", "java", "-e", "target"])
    if cp.returncode != 0:
        raise SystemExit(f"discover failed:\n{cp.stderr}")


def scrub_paths(s: str) -> str:
    s = re.sub(r"[^\s\"]*?/rgbuilder-tests/ecommerce-java/[^\s\"]*?/([^/\s\"]+\.\w+)", r"…/\1", s)
    s = re.sub(r"/Users/[^\s\"]+", "…", s)
    s = re.sub(r"/home/[^\s\"]+", "…", s)
    s = re.sub(r"/tmp/[^\s\"]+", "/tmp/…", s)
    return s


def scrub_json_value(v):
    if isinstance(v, dict):
        out = {}
        for k, x in v.items():
            if k in SCRUB_KEYS or (k.endswith("_id") and k != "community_id"):
                out[k] = "…"
            elif k == "file_path" and isinstance(x, str):
                m = re.search(r"([^/]+\.\w+)$", x.replace("\\", "/"))
                out[k] = f"…/{m.group(1)}" if m else "…"
            else:
                out[k] = scrub_json_value(x)
        return out
    if isinstance(v, list):
        return [scrub_json_value(x) for x in v]
    if isinstance(v, str):
        return scrub_paths(v)
    return v


def apply_jq_filter(stdout: str, jq_filter: str | None) -> str:
    if not jq_filter:
        return stdout
    try:
        doc = json.loads(stdout)
    except json.JSONDecodeError:
        return stdout
    # Minimal subset of jq used in the guide — only object projections we need
    # Prefer real jq when available.
    jq = shutil.which("jq")
    if jq:
        cp = subprocess.run(
            [jq, "-c", jq_filter],
            input=stdout,
            capture_output=True,
            text=True,
            check=False,
        )
        if cp.returncode == 0:
            # pretty-print for guide
            try:
                return json.dumps(json.loads(cp.stdout), indent=2) + "\n"
            except json.JSONDecodeError:
                return cp.stdout
    # Fallback for `{score: .metrics.score, callers: .topology.direct_callers}`
    m = re.fullmatch(
        r"\{score:\s*\.metrics\.score,\s*callers:\s*\.topology\.direct_callers\}",
        jq_filter.strip(),
    )
    if m:
        out = {
            "score": doc.get("metrics", {}).get("score"),
            "callers": doc.get("topology", {}).get("direct_callers"),
        }
        return json.dumps(out, indent=2) + "\n"
    if jq_filter.strip() == ".count":
        return str(doc.get("count", "")) + "\n"
    raise SystemExit(f"unsupported jq_filter without jq binary: {jq_filter!r}")


def expected_sample(stdout: str, sc: dict) -> str:
    body = apply_jq_filter(stdout, sc.get("jq_filter"))
    if sc.get("jq_filter") == ".count":
        return "```text\n" + body.strip() + "\n```\n"
    if sc.get("require_json_keys") or body.lstrip().startswith(("{", "[")):
        try:
            doc = json.loads(body)
            doc = scrub_json_value(doc)
            return (
                "```json\n"
                + json.dumps(doc, indent=2, ensure_ascii=False)
                + "\n```\n"
            )
        except json.JSONDecodeError:
            pass
    text = scrub_paths(body).strip() + "\n"
    return "```text\n" + text + "```\n"


def marker_bounds(mid: str) -> tuple[str, str]:
    return f"<!-- ug-scenario:{mid} -->", f"<!-- /ug-scenario:{mid} -->"


def extract_marker_block(guide: str, mid: str) -> str | None:
    open_m, close_m = marker_bounds(mid)
    if open_m not in guide or close_m not in guide:
        return None
    start = guide.index(open_m) + len(open_m)
    end = guide.index(close_m)
    return guide[start:end]


def normalize_block(block: str) -> str:
    block = block.strip()
    # Drop optional "Example:" preface
    block = re.sub(r"^Example:\s*", "", block, flags=re.I).strip()
    return block + "\n"


def replace_marker_block(guide: str, mid: str, inner: str) -> str:
    open_m, close_m = marker_bounds(mid)
    start = guide.index(open_m) + len(open_m)
    end = guide.index(close_m)
    return guide[:start] + "\n" + inner.rstrip() + "\n" + guide[end:]


def check_markers(strict: bool) -> list[str]:
    guide = GUIDE.read_text()
    scenarios = load_scenarios()
    missing = []
    for sc in scenarios:
        if "marker" not in sc:
            continue
        mid = sc["marker"] or sc["id"]
        open_m, close_m = marker_bounds(mid)
        if open_m not in guide or close_m not in guide:
            missing.append(mid)
    if missing:
        print(
            "markers missing in user-guide.md (add fences when syncing outputs):\n  "
            + "\n  ".join(missing),
            file=sys.stderr,
        )
        if strict:
            raise SystemExit(1)
    else:
        print("all scenario markers present")
    return missing


def run_scenario(bin: Path, sc: dict) -> str:
    sid = sc["id"]
    args = list(sc["args"])
    # Resolve /tmp export paths under a temp dir when present
    args = [
        (
            a
            if not a.startswith("/tmp/")
            else str(Path(tempfile.gettempdir()) / Path(a).name)
        )
        for a in args
    ]
    if sc.get("is_discover"):
        shutil.rmtree(FIXTURE / ".rgbuilder", ignore_errors=True)
    elif sc.get("needs_discover", True):
        ensure_discover(bin, FIXTURE)
    print(f"==> {sid}")
    cp = run_one(bin, FIXTURE, args)
    if cp.returncode != 0:
        raise SystemExit(f"{sid} exit {cp.returncode}\n{cp.stderr}\n{cp.stdout[:400]}")
    if keys := sc.get("require_json_keys"):
        assert_json(cp.stdout, keys, sid)
    for needle in sc.get("stdout_contains") or []:
        if needle not in cp.stdout and needle not in cp.stderr:
            raise SystemExit(f"{sid}: stdout/stderr missing {needle!r}")
    print("    ok")
    return cp.stdout


def run_suite(bin: Path, *, sync: bool, check: bool) -> None:
    if not FIXTURE.is_dir():
        raise SystemExit(f"missing fixture {FIXTURE}")
    scenarios = load_scenarios()
    guide = GUIDE.read_text()
    guide_dirty = False
    mismatches = []

    for sc in scenarios:
        stdout = run_scenario(bin, sc)
        mid = sc.get("marker") or sc["id"]
        if "marker" not in sc:
            continue
        block = extract_marker_block(guide, mid)
        if block is None:
            continue
        if not sc.get("sync_output", True):
            continue
        sample = expected_sample(stdout, sc)
        # Preserve "Example:" preface if present
        preface = ""
        if block.lstrip().lower().startswith("example:"):
            preface = "Example:\n\n"
        new_inner = preface + sample
        if sync:
            guide = replace_marker_block(guide, mid, new_inner)
            guide_dirty = True
            print(f"    synced marker {mid}")
        elif check:
            # Compare fence bodies loosely: JSON structurally after scrub
            old = normalize_block(block)
            new = normalize_block(new_inner)
            if not blocks_match(old, new, sc):
                mismatches.append(mid)
                print(f"    MISMATCH marker {mid}", file=sys.stderr)

    if sync and guide_dirty:
        GUIDE.write_text(guide)
        print(f"wrote {GUIDE}")
    if check and mismatches:
        raise SystemExit(
            "marker output drift (re-run with --sync):\n  " + "\n  ".join(mismatches)
        )


def blocks_match(old: str, new: str, sc: dict) -> bool:
    def fence_body(s: str) -> str | None:
        m = re.search(r"```(?:json|text)?\n(.*?)```", s, re.S)
        return m.group(1) if m else None

    ob, nb = fence_body(old), fence_body(new)
    if ob is None or nb is None:
        return old.strip() == new.strip()
    # Text compare for count-style
    if sc.get("jq_filter") == ".count" or (ob.strip().isdigit() and nb.strip().isdigit()):
        return ob.strip() == nb.strip()
    try:
        oj = scrub_json_value(json.loads(ob))
        nj = scrub_json_value(json.loads(nb))
    except json.JSONDecodeError:
        return scrub_paths(ob).strip() == scrub_paths(nb).strip()
    return json_stable(oj) == json_stable(nj)


def json_stable(v):
    """Compare with ellipsis wildcards and float tolerance for scores."""
    if isinstance(v, dict):
        return {k: json_stable(x) for k, x in sorted(v.items())}
    if isinstance(v, list):
        return [json_stable(x) for x in v]
    if isinstance(v, float):
        return round(v, 1)
    if v == "…":
        return "…"
    return v


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--check-markers", action="store_true")
    ap.add_argument("--strict-markers", action="store_true")
    ap.add_argument("--check", action="store_true", help="fail on marker output drift")
    ap.add_argument("--sync", action="store_true", help="rewrite marker samples")
    args = ap.parse_args()
    if args.check_markers:
        check_markers(args.strict_markers)
        return
    bin = find_bin()
    print(f"binary: {bin}")
    run_suite(bin, sync=args.sync, check=args.check)
    check_markers(args.strict_markers)
    print("all scenarios passed")


if __name__ == "__main__":
    main()
