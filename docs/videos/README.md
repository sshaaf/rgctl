# Terminal + dashboard demos

Aligned with the [User Guide](../user-guide.md) (named communities, CoolStore `/services/*` + `cpg mutations --type ShoppingCart`, Kantra `--with-kantra`, semantic `--scope community`). Pattern for both:

1. Record a **clean** video (no on-screen caption chrome)
2. Keep `*-no-captions.mp4` for comparison
3. Burn timed **SRT** captions with ffmpeg → deliverable `.mp4`

---

## CLI (VHS)

| File | Purpose |
|------|---------|
| [`user-guide-cli.tape`](user-guide-cli.tape) | VHS script |
| [`record-user-guide-cli.sh`](record-user-guide-cli.sh) | Record → `user-guide-cli-no-captions.{gif,mp4}` |
| [`user-guide-cli.srt`](user-guide-cli.srt) | Subtitle cues |
| [`burn-user-guide-captions.sh`](burn-user-guide-captions.sh) | Burn → `user-guide-cli.mp4` |

```bash
cargo build --release
./docs/videos/record-user-guide-cli.sh
./docs/videos/burn-user-guide-captions.sh
```

---

## Markdown context graph (VHS)

| File | Purpose |
|------|---------|
| [`markdown-context-cli.tape`](markdown-context-cli.tape) | `discover -l markdown,java` + GQL on `tests/fixtures/markdown-context` |
| [`record-markdown-context-cli.sh`](record-markdown-context-cli.sh) | Record → `markdown-context-cli-no-captions.{gif,mp4}` |

```bash
cargo build --bin rgctl
./docs/videos/record-markdown-context-cli.sh
```

---

## Dashboard (Playwright)

| File | Purpose |
|------|---------|
| [`../dashboard/scripts/record-feature-demo.mjs`](../dashboard/scripts/record-feature-demo.mjs) | Tab montage (ecommerce-java defaults) |
| [`record-feature-demo.sh`](record-feature-demo.sh) | Discover + serve + record + burn |
| [`rgctl-feature-demo.srt`](rgctl-feature-demo.srt) | Written by the recorder |
| [`burn-feature-demo-captions.sh`](burn-feature-demo-captions.sh) | Burn → `rgctl-feature-demo.mp4` |

```bash
cargo build --release
./docs/videos/record-feature-demo.sh
# or step-by-step:
#   rgctl -r rgctl-tests/ecommerce-java serve --port 8080
#   DASHBOARD_URL=http://127.0.0.1:8080/ node dashboard/scripts/record-feature-demo.mjs
#   ./docs/videos/burn-feature-demo-captions.sh
```

Defaults: one beat per main tab (Dataflow shows mutations + PDG + dominator). Hold `DEMO_HOLD_SEC` (default 6.5). Override symbols with `CAPTURE_FN_*` / `CAPTURE_SEMANTIC_QUERY` / `MUTATIONS_TYPE`.

Captions need ffmpeg with `subtitles` (`brew install ffmpeg-full`).

---

## CoolStore microservice decomposition (VHS)

Blog walkthrough for [Decomposing a Monolith into Microservices](https://shaaf.dev/post/decomposing-a-monolith-into-microservices-with-call-graph-analysis/) on `example/coolstore-weblogic`.

| File | Purpose |
|------|---------|
| [`microservices-decomposition-cli.tape`](microservices-decomposition-cli.tape) | VHS script |
| [`record-microservices-decomposition-cli.sh`](record-microservices-decomposition-cli.sh) | Record → `microservices-decomposition-cli-no-captions.{gif,mp4}` |
| [`microservices-decomposition-cli.srt`](microservices-decomposition-cli.srt) | Subtitle cues |
| [`burn-microservices-decomposition-cli.sh`](burn-microservices-decomposition-cli.sh) | Burn → `microservices-decomposition-cli.mp4` |

```bash
cargo build --release
./docs/videos/record-microservices-decomposition-cli.sh
./docs/videos/burn-microservices-decomposition-cli.sh
```
