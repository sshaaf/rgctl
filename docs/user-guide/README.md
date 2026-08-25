# User-guide scenarios

Each runnable command in [user-guide.md](../user-guide.md) should have a JSON scenario here.
CI runs `scripts/user-guide-scenarios.py --check` against `rgctl-tests/ecommerce-java`.

## Scenario schema

```json
{
  "id": "04-discover-json",
  "args": ["-f", "json", "discover", ".", "-l", "java", "-e", "target"],
  "is_discover": true,
  "require_json_keys": ["schema_version", "command", "metrics"],
  "marker": "04-discover-json",
  "jq_filter": null,
  "sync_output": true
}
```

| Field | Meaning |
|-------|---------|
| `id` | Unique id (filename stem) |
| `args` | argv after binary (run with `-r` ecommerce-java) |
| `is_discover` | If true, tears down `.rgctl` then runs |
| `needs_discover` | Default true for non-discover scenarios |
| `require_json_keys` | Top-level JSON keys that must exist when stdout is JSON |
| `stdout_contains` | Substrings that must appear in stdout or stderr |
| `marker` | Optional `<!-- ug-scenario:ID -->` id in user-guide.md |
| `jq_filter` | Optional jq expression applied before sync/check |
| `sync_output` | Default true when marker present; set false to skip sample rewrite |

## Run

```bash
# from repo root; prefers target/release/rgctl
python3 scripts/user-guide-scenarios.py
python3 scripts/user-guide-scenarios.py --check          # CI: assert marker samples
python3 scripts/user-guide-scenarios.py --sync           # rewrite marker samples
python3 scripts/user-guide-scenarios.py --check-markers
```
