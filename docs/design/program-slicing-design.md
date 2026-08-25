# Program Slicing — Engineering Design

**Backward and forward program slices** over a function’s PDG: the minimal set of statements that affect (or are affected by) a variable at a given source line.

![Program Slicing tab — source editor and slice controls (gbuilder)](../images/design/program-slicing/slice-editor.png)

*Figure 1: **Program Slicing** tab — function picker, line/variable/direction inputs, and CodeMirror source view with highlighted slice lines.*

---

## 1. Goals

| Goal | How |
|------|-----|
| Minimal relevant code | PDG traversal from slice criterion |
| CLI automation | `rgctl slice FILE --line N --variable V` |
| Dashboard exploration | WASM `compute_slice` on exported PDG bundles |
| Security cross-check | `--taint` mode shares taint engine paths |

Requires `discover --with-cfg` (and `--with-taint` for discover-time taint) for PDG archives and dashboard export.

---

## 2. Architecture overview

```mermaid
flowchart LR
  subgraph discover["discover --with-cfg"]
    CFG[CFG per function]
    PDG[PDG per function]
    ARC[cfg_pdg.archive.bin]
    CFG --> PDG --> ARC
  end

  subgraph export["Dashboard export"]
    SI[slice_index.json]
    SB[slice/*.json]
    ARC --> SI
    ARC --> SB
  end

  subgraph runtime["Query"]
    CLI[slice CLI — reads source + PDG]
    WASM[worker compute_slice]
    CLI --> PDG
    WASM --> SB
  end
```

---

## 3. Slice criterion

| Field | Role |
|-------|------|
| `file` | Source path (read from disk at CLI; bundled in dashboard) |
| `--line` | 1-based line number |
| `--variable` | Variable name at criterion |
| `--function` | Enclosing **method** name (disambiguation) |
| `--direction` | `backward` (default) or `forward` |

---

## 4. Rust implementation map

| Component | Path |
|-----------|------|
| CFG construction | `crates/rgctl-analysis/src/cfg.rs` |
| PDG + slicing | `crates/rgctl-analysis/src/pdg.rs`, `slicing.rs` |
| CLI | `src/cli/slice.rs` |
| Archive storage | `crates/rgctl-analysis/src/storage.rs` |
| Dashboard export | `crates/rgctl-dashboard/src/slice_export.rs` |

---

## 5. Dashboard implementation

| Piece | Path |
|-------|------|
| Tab | `dashboard/src/SliceView.tsx` |
| Editor | `dashboard/src/sliceEditor.tsx` (CodeMirror) |
| Worker | `computeSlice(functionId, line, variable, direction)` |
| Index | `slice_index.json` → `slice/{uuid}.json` |

---

## 6. CLI usage

```bash
rgctl discover . --cfg
rgctl slice src/Foo.java --line 42 --variable cart --function checkOut
rgctl -f json slice src/Foo.java --line 10 --variable req --direction forward
rgctl slice src/Foo.java --line 8 --variable input --taint
```

---

## 7. Testing

| Layer | Location |
|-------|----------|
| Analysis unit tests | `crates/rgctl-analysis/src/slicing.rs` |
| CLI subprocess | `tests/cli_output/all_commands_sanity.rs` |
| Dashboard harness | `tests/dashboard_harness.rs` (`slice_index.json`) |

Screenshots: `capture-design-screenshots.mjs` → `docs/images/design/program-slicing/`.

---

## 8. Related docs

- [PDG design](pdg-design.md) · [Taint design](taint-analysis-design.md)
- [Analysis architecture](../analysis-architecture.md)
