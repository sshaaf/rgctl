import type { Metadata } from "next";
import Link from "next/link";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { TerminalBlock } from "@/components/terminal";
import { GITHUB_REPO } from "@/lib/utils";

export const metadata: Metadata = {
  title: "Agents",
};

const steps = [
  {
    n: "1",
    title: "User prompt",
    body: "Natural language — “what breaks if I change checkout?”",
  },
  {
    n: "2",
    title: "Tool call",
    body: "Agent runs rgctl -f json … instead of dumping files into context.",
  },
  {
    n: "3",
    title: "Graph facts",
    body: "schema_version JSON — blast, GQL rows, semantic hits, CPG mutations.",
  },
  {
    n: "4",
    title: "Reason + edit",
    body: "LLM plans the change from facts, then verifies with check when needed.",
  },
];

export default function AgentsPage() {
  return (
    <div className="mx-auto max-w-3xl px-4 py-14 sm:px-6">
      <Badge className="mb-4">LLM workflows</Badge>
      <h1 className="text-3xl tracking-tight text-[var(--ink)] sm:text-4xl">
        Built for coding agents
      </h1>
      <p className="mt-3 text-[var(--body)]">
        Point your agent at{" "}
        <code className="font-mono text-[var(--body-strong)]">AGENTS.md</code>{" "}
        and the JSON recipes. Discover once; query forever.
      </p>

      <ol className="mt-10 grid gap-4 sm:grid-cols-2">
        {steps.map((s) => (
          <li
            key={s.n}
            className="rounded-[4px] border border-[var(--hairline)] p-4"
          >
            <p className="font-mono text-[11px] text-[var(--mute)]">
              Step {s.n}
            </p>
            <h2 className="mt-1 text-base text-[var(--ink)]">{s.title}</h2>
            <p className="mt-1 text-sm text-[var(--body)]">{s.body}</p>
          </li>
        ))}
      </ol>

      <section className="mt-12 space-y-3">
        <h2 className="text-lg text-[var(--ink)]">Minimal agent loop</h2>
        <TerminalBlock
          lines={[
            'export REPO=/path/to/repo',
            "rgctl -r \"$REPO\" discover .",
            "rgctl -r \"$REPO\" -f json gql --macro-name all_functions unused | jq '.count'",
            'rgctl -r "$REPO" -f json blast-radius "ShoppingCartService" --depth 3 \\',
            "  | jq '{score: .metrics.score, callers: .metrics.direct_callers_count}'",
          ]}
        />
      </section>

      <section className="mt-10 flex flex-wrap gap-3">
        <Button asChild>
          <a
            href={`${GITHUB_REPO}/blob/main/AGENTS.md`}
            target="_blank"
            rel="noreferrer"
          >
            Open AGENTS.md
          </a>
        </Button>
        <Button variant="ghost" asChild>
          <a
            href={`${GITHUB_REPO}/blob/main/docs/agent-recipes.md`}
            target="_blank"
            rel="noreferrer"
          >
            Recipes
          </a>
        </Button>
        <Button variant="ghost" asChild>
          <Link href="/demo/">Interactive demos</Link>
        </Button>
      </section>
    </div>
  );
}
