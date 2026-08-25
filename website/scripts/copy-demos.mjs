import { cpSync, existsSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = join(here, "../..");
const destDir = join(here, "../public/demos");

mkdirSync(destDir, { recursive: true });

/** Source-of-truth recordings under docs/videos/ → website/public/demos/ */
const files = [
  ["docs/videos/user-guide-cli-no-captions.gif", "user-guide-cli.gif"],
  ["docs/videos/user-guide-cli.mp4", "user-guide-cli.mp4"],
  ["docs/videos/rgctl-feature-demo.mp4", "feature-demo.mp4"],
];

let copied = 0;
for (const [rel, name] of files) {
  const from = join(repoRoot, rel);
  if (!existsSync(from)) {
    console.warn(`[copy-demos] skip missing: ${rel}`);
    continue;
  }
  cpSync(from, join(destDir, name));
  copied += 1;
  console.log(`[copy-demos] ${rel} → public/demos/${name}`);
}

if (copied === 0) {
  console.warn(
    "[copy-demos] no demo media found — run ./docs/videos/record-user-guide-cli.sh first",
  );
}
