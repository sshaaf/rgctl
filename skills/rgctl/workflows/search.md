# Search workflow

**When:** Natural-language or intent-based code location (requires `semantic index`).

```bash
rgctl -r "$REPO" semantic index                    # opt-in; default vocab. extras: --embedder code-daemon|hash
rgctl -r "$REPO" -f json semantic query "checkout flow" --limit 10
```

Fusion is on by default for semantic query; use GQL for exact graph patterns.

### NL function search

**User intent:** *"Where is the code that handles our checkout flow?"*

Report top `hits[]` (`name`, `score`, `file_path`).

### Community semantic

**User intent:** *"Which architectural subsystem owns checkout?"*

```bash
rgctl -r "$REPO" -f json semantic query "checkout" --scope community --limit 10
```

Hits are pooled **community** results (same `hits[]` contract).

### Concept search with 0 LIKE hits

If GQL LIKE returns 0 for a concept (e.g., "ingress", "gateway"):

1. Try `communities list` and grep labels
2. Try `semantic query "<concept>"`
3. Broaden LIKE to non-Function node types (Modules, Classes)

Concepts often live in package/directory paths or type names, not bare function names.
