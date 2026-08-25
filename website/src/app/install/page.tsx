import type { Metadata } from "next";
import Link from "next/link";
import { TerminalBlock } from "@/components/terminal";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { GITHUB_RELEASES, GITHUB_REPO } from "@/lib/utils";

export const metadata: Metadata = {
  title: "Install",
};

export default function InstallPage() {
  return (
    <div className="mx-auto max-w-3xl px-4 py-14 sm:px-6">
      <Badge className="mb-4">Get started</Badge>
      <h1 className="font-[family-name:var(--font-serif)] text-3xl font-semibold tracking-tight text-[var(--ink)] sm:text-4xl">
        Install rgBuilder
      </h1>
      <p className="mt-3 text-[var(--body)]">
        Prefer a release binary for day-to-day use. Build from source when you
        need the latest main. Pull Git LFS only if you use the optional
        code-daemon semantic embedder.
      </p>

      <section className="mt-10 space-y-3">
        <h2 className="text-lg text-[var(--ink)]">Option A — GitHub Releases</h2>
        <p className="text-sm text-[var(--body)]">
          Download the latest asset for your platform from{" "}
          <a
            href={GITHUB_RELEASES}
            className="text-[var(--primary)] underline"
            target="_blank"
            rel="noreferrer"
          >
            Releases
          </a>
          , put <code className="font-mono text-[var(--body-strong)]">rgctl</code>{" "}
          on your <code className="font-mono">PATH</code>, then install the agent
          skill into the repo you will index:
        </p>
        <TerminalBlock
          lines={[
            "rgctl --version",
            "rgctl install --skill",
          ]}
        />
        <p className="text-sm text-[var(--mute)]">
          That writes{" "}
          <code className="font-mono">.claude/skills/rgbuilder/</code> and{" "}
          <code className="font-mono">.cursor/skills/rgbuilder/</code>. See the{" "}
          <Link href="/docs/guides/agent-skill/" className="underline">
            agent skill guide
          </Link>
          .
        </p>
        <Button variant="ghost" asChild>
          <a href={GITHUB_RELEASES} target="_blank" rel="noreferrer">
            Open releases
          </a>
        </Button>
      </section>

      <section className="mt-12 space-y-3">
        <h2 className="text-lg text-[var(--ink)]">Option B — Build from source</h2>
        <TerminalBlock
          lines={[
            "git clone https://github.com/sshaaf/rgBuilder.git",
            "cd rgBuilder",
            "# Optional: only if you use `semantic index --embedder code-daemon` (~206 MB)",
            "git lfs pull",
            "cargo build --release --bin rgctl",
            "./target/release/rgctl --version",
          ]}
        />
      </section>

      <section className="mt-12 space-y-3">
        <h2 className="text-lg text-[var(--ink)]">First hour</h2>
        <p className="text-sm text-[var(--body)]">
          Use the in-tree{" "}
          <code className="font-mono text-[var(--body-strong)]">
            rgbuilder-tests/ecommerce-java
          </code>{" "}
          fixture (canonical walkthrough in the User Guide).
        </p>
        <TerminalBlock
          lines={[
            "cd rgbuilder-tests/ecommerce-java",
            "rgctl discover .",
            "rgctl -f json gql --macro-name all_functions unused | jq '.count'",
            'rgctl -f json blast-radius "priceShoppingCart" --depth 2',
          ]}
        />
        <p className="text-sm text-[var(--mute)]">
          Dashboard and migration JSON are opt-in: add{" "}
          <code className="font-mono">--with-dashboard</code> /{" "}
          <code className="font-mono">--export-migration-hints</code>.
        </p>
      </section>

      <section className="mt-12 flex flex-wrap gap-3">
        <Button asChild>
          <Link href="/docs/guides/">Read the guides</Link>
        </Button>
        <Button variant="ghost" asChild>
          <Link href="/docs/">Docs hub</Link>
        </Button>
        <Button variant="ghost" asChild>
          <Link href="/demo/">Try demos</Link>
        </Button>
        <Button variant="ghost" asChild>
          <a href={`${GITHUB_REPO}/blob/main/docs/user-guide.md`} target="_blank" rel="noreferrer">
            User Guide on GitHub
          </a>
        </Button>
      </section>
    </div>
  );
}
