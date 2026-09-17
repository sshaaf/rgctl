# Discover workflow

**When:** First use, rebuild after large changes, or incremental `--files` update.

| Intent | Command |
|--------|---------|
| Index repo | `cd "$REPO" && rgctl discover .` or `rgctl -r "$REPO" discover` |
| Full pipeline | `discover . --full` |
| Incremental | `discover --files path1,path2` (requires existing `.rgctl/`) |

**Fast path:** If `.rgctl/` exists and the user did not ask to rebuild, do **not** re-run discover.

Common flags: `--with-cfg` (CFG/PDG archive), `--with-ast-skeleton`, `--with-dfg-loops` (loop-carried PDG tags). Migration plan output is the **migrate** workflow; Konveyor rules are the **kantra** workflow — do not conflate them with a plain index.

Artifacts live at `{repo}/.rgctl/`. Check CFG readiness with `rgctl -f json cpg status` before slice/PDG workflows.
