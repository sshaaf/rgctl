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
    title: "Install the pack",
    body: "Meta skill rgctl, eight workflow skills, and slash commands via install --skill --with-commands.",
  },
  {
    n: "2",
    title: "Discover once",
    body: "rgctl discover . writes {repo}/.rgctl/ — later questions are graph lookups.",
  },
  {
    n: "3",
    title: "Route the question",
    body: "Chat /rgctl-impact or /rgctl-gql, or the matching workflow skill — agents spawn rgctl -f json.",
  },
  {
    n: "4",
    title: "Reason + edit",
    body: "schema_version JSON on stdout — blast, GQL, CPG, semantic — then verify with check / pr-check.",
  },
];

const workflows = [
  { chat: "/rgctl-discover", cli: "discover" },
  { chat: "/rgctl-impact", cli: "blast-radius" },
  { chat: "/rgctl-flow", cli: "slice · cpg" },
  { chat: "/rgctl-search", cli: "semantic query" },
  { chat: "/rgctl-gql", cli: "gql" },
  { chat: "/rgctl-migrate", cli: "migration_plan.json" },
  { chat: "/rgctl-kantra", cli: "discover --with-kantra" },
  { chat: "/rgctl-gate", cli: "check · pr-check" },
];

export default function AgentsPage() {
  return (
    <div className="mx-auto max-w-3xl px-4 py-14 sm:px-6">
      <Badge className="mb-4">LLM workflows</Badge>
      <h1 className="text-3xl tracking-tight text-[var(--ink)] sm:text-4xl">
        Built for coding agents
      </h1>
      <p className="mt-3 text-[var(--body)]">
        The agent pack is not one skill — it installs a{" "}
        <strong className="font-medium text-[var(--ink)]">router</strong>,{" "}
        <strong className="font-medium text-[var(--ink)]">
          eight workflow skills
        </strong>
        , and matching{" "}
        <strong className="font-medium text-[var(--ink)]">slash commands</strong>{" "}
        so agents pick the right <code className="font-mono">rgctl -f json</code>{" "}
        path instead of dumping files into context.
      </p>

      <section className="mt-10 space-y-3">
        <h2 className="text-lg text-[var(--ink)]">Install into your repo</h2>
        <TerminalBlock
          lines={[
            "cd /path/to/your-app",
            "rgctl install --skill --with-commands --tools cursor,claude,codex,antigravity,agents",
            "rgctl discover .",
          ]}
        />
        <p className="text-sm text-[var(--mute)]">
          Writes meta skill{" "}
          <code className="font-mono">rgctl</code>, workflows like{" "}
          <code className="font-mono">rgctl-gql</code>, and commands such as{" "}
          <code className="font-mono">.cursor/commands/rgctl-gql.md</code>. See{" "}
          <Link href="/docs/guides/agent-commands/" className="underline">
            agent commands
          </Link>
          .
        </p>
      </section>

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
        <h2 className="text-lg text-[var(--ink)]">Workflow ↔ CLI</h2>
        <p className="text-sm text-[var(--body)]">
          Slash commands steer the agent; the engine remains the terminal CLI.
          Claude uses colon style (<code className="font-mono">/rgctl:gql</code>
          ).
        </p>
        <div className="overflow-x-auto rounded-[4px] border border-[var(--hairline)]">
          <table className="w-full min-w-[28rem] text-left text-sm">
            <thead className="border-b border-[var(--hairline)] bg-[var(--surface)]">
              <tr>
                <th className="px-3 py-2 font-medium text-[var(--ink)]">
                  Chat (Cursor)
                </th>
                <th className="px-3 py-2 font-medium text-[var(--ink)]">
                  Primary CLI
                </th>
              </tr>
            </thead>
            <tbody>
              {workflows.map((w) => (
                <tr
                  key={w.chat}
                  className="border-b border-[var(--hairline)] last:border-0"
                >
                  <td className="px-3 py-2 font-mono text-[13px] text-[var(--body-strong)]">
                    {w.chat}
                  </td>
                  <td className="px-3 py-2 font-mono text-[13px] text-[var(--body)]">
                    {w.cli}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </section>

      <section className="mt-12 space-y-3">
        <h2 className="text-lg text-[var(--ink)]">Minimal agent loop</h2>
        <TerminalBlock
          lines={[
            'export REPO=/path/to/repo',
            'cd "$REPO" && rgctl discover .   # or: rgctl -r "$REPO" discover',
            "rgctl -r \"$REPO\" -f json gql --macro-name all_functions unused | jq '.count'",
            'rgctl -r "$REPO" -f json blast-radius "ShoppingCartService" --depth 3 \\',
            "  | jq '{score: .metrics.score, callers: .metrics.direct_callers_count}'",
          ]}
        />
      </section>

      <section className="mt-10 flex flex-wrap gap-3">
        <Button asChild>
          <Link href="/docs/guides/agent-commands/">Agent commands guide</Link>
        </Button>
        <Button variant="ghost" asChild>
          <Link href="/docs/guides/agent-skill/">Pack walkthrough</Link>
        </Button>
        <Button variant="ghost" asChild>
          <a
            href={`${GITHUB_REPO}/blob/main/docs/agents/USER_AGENTS_TEMPLATE.md`}
            target="_blank"
            rel="noreferrer"
          >
            USER_AGENTS_TEMPLATE
          </a>
        </Button>
        <Button variant="ghost" asChild>
          <Link href="/demo/">Interactive demos</Link>
        </Button>
      </section>
    </div>
  );
}
