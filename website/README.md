# rgctl website

Static marketing + docs hub (Next.js App Router, `output: "export"`).

## Stack

- Next.js (static export)
- Tailwind CSS v4
- shadcn-style primitives (Button, Tabs, Badge)
- Warp-inspired tokens from `DESIGN.md`

## Demo media

VHS / dashboard recordings live in `docs/videos/` and are copied into
`public/demos/` on `pnpm dev` / `pnpm build` (`scripts/copy-demos.mjs`).

```bash
# regenerate sources
./docs/videos/record-user-guide-cli.sh
./docs/videos/burn-user-guide-captions.sh
./docs/videos/record-feature-demo.sh
```

## Develop

```bash
cd website
pnpm install
cp .env.example .env.local   # optional: set NEXT_PUBLIC_GA_MEASUREMENT_ID
pnpm dev
```

Open http://localhost:3000

## Analytics

GA4 loads only when `NEXT_PUBLIC_GA_MEASUREMENT_ID` is set (e.g. `G-XXXXXXXXXX`).

- Local: put the ID in `website/.env.local` (gitignored)
- CI / Pages: add repo secret `NEXT_PUBLIC_GA_MEASUREMENT_ID` (Actions → Secrets)

With the secret unset, the site builds with analytics disabled.

## Build

```bash
pnpm build   # writes ./out
```

For GitHub Pages project site locally:

```bash
NEXT_PUBLIC_BASE_PATH=/rgctl pnpm build
```

## Deploy

GitHub Actions workflow: `.github/workflows/website.yml`

Runs on pushes to **`main`** or **`docs`** (when `website/` or the workflow file changes), and via **workflow_dispatch** on either branch.

1. Repo **Settings → Pages → Build and deployment → Source: GitHub Actions**
2. Environment **github-pages** deployment branches: `main`, `docs`
3. Site: `https://shaaf.dev/rgctl/` (GitHub Pages + custom domain)

## Routes

| Path | Purpose |
|------|---------|
| `/` | Landing |
| `/install/` | Install + first hour |
| `/docs/` | Docs hub (links to repo markdown) |
| `/agents/` | Agent loop |
| `/demo/` | Interactive demo scenarios |
| `/community/` | Stars, discussions, contribute |
