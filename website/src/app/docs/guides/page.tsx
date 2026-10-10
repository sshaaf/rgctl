import type { Metadata } from "next";
import Link from "next/link";
import { Badge } from "@/components/ui/badge";

export const metadata: Metadata = {
  title: "Guides",
};

/** `added` is YYYY-MM-DD (guide first landed). Index sorts newest first, then title. */
const guides = [
  {
    title: "Clone detection",
    feature: "clones",
    blurb: "Exact (Type-1), bloom, and sub-function fragment clones.",
    href: "/docs/guides/clone-detection/",
    added: "2026-10-10",
  },
  {
    title: "Pull request review",
    feature: "review paths, review check",
    blurb: "Structural before/after call-path inspection and PR gates.",
    href: "/docs/guides/pr-review/",
    added: "2026-10-10",
  },
  {
    title: "Watch mode and incremental update",
    feature: "update, serve --watch",
    blurb: "Keep the graph fresh without full rediscover.",
    href: "/docs/guides/watch-mode/",
    added: "2026-10-09",
  },
  {
    title: "Structured graph queries",
    feature: "find / callers / relations / …",
    blurb: "Agent-facing graph exploration verbs.",
    href: "/docs/guides/structured-query/",
    added: "2026-10-02",
  },
  {
    title: "Markdown context graph",
    feature: "discover -l markdown · export",
    blurb: "Index docs, Obsidian/OKF export, fixture feature tour.",
    href: "/docs/guides/markdown-context-graph/",
    added: "2026-08-24",
  },
  {
    title: "Agent pack",
    feature: "install --skill",
    blurb: "Single skill install --skill; NL → CLI via references/workflows.md.",
    href: "/docs/guides/agent-skill/",
    added: "2026-08-21",
  },
  {
    title: "Blast radius analysis",
    feature: "blast-radius",
    blurb: "Measure upstream impact before changing a function.",
    href: "/docs/guides/blast-radius-analysis/",
    added: "2026-08-21",
  },
  {
    title: "CI policy checks",
    feature: "check, pr-check",
    blurb: "PR temporal gates and local policy checks.",
    href: "/docs/guides/ci-policy-checks/",
    added: "2026-08-21",
  },
  {
    title: "Community detection",
    feature: "communities",
    blurb: "Identify and label functional clusters in your codebase.",
    href: "/docs/guides/community-detection/",
    added: "2026-08-21",
  },
  {
    title: "Discovering and indexing",
    feature: "discover",
    blurb: "Build the knowledge graph from source code.",
    href: "/docs/guides/discovering-and-indexing/",
    added: "2026-08-21",
  },
  {
    title: "Exporting graphs",
    feature: "export",
    blurb: "Serialize to JSON, GraphML, Graphviz, Mermaid, or Obsidian.",
    href: "/docs/guides/exporting-graphs/",
    added: "2026-08-21",
  },
  {
    title: "Graph metrics",
    feature: "metrics",
    blurb: "PageRank, betweenness, and community detection analytics.",
    href: "/docs/guides/graph-metrics/",
    added: "2026-08-21",
  },
  {
    title: "HTTP server and dashboard",
    feature: "serve",
    blurb: "Run an HTTP API and browser-based dashboard.",
    href: "/docs/guides/http-server-and-dashboard/",
    added: "2026-08-21",
  },
  {
    title: "Hybrid CPG",
    feature: "cpg",
    blurb: "Combined call-graph + per-function CFG/PDG analysis.",
    href: "/docs/guides/hybrid-cpg/",
    added: "2026-08-21",
  },
  {
    title: "Inspecting CFG, PDG, and dominance",
    feature: "inspect",
    blurb: "Examine control flow, data dependence, and dominator trees.",
    href: "/docs/guides/inspecting-cfg-pdg-dominance/",
    added: "2026-08-21",
  },
  {
    title: "Migration planning",
    feature: "discover --export-migration-hints",
    blurb: "Generate a dependency-aware migration roadmap.",
    href: "/docs/guides/migration-planning/",
    added: "2026-08-21",
  },
  {
    title: "Program slicing",
    feature: "slice",
    blurb: "Extract the minimal set of statements affecting a variable.",
    href: "/docs/guides/program-slicing/",
    added: "2026-08-21",
  },
  {
    title: "Semantic search",
    feature: "semantic",
    blurb: "Natural-language search over function symbols.",
    href: "/docs/guides/semantic-search/",
    added: "2026-08-21",
  },
].sort((a, b) => {
  const byDate = b.added.localeCompare(a.added);
  if (byDate !== 0) return byDate;
  return a.title.localeCompare(b.title);
});

export default function GuidesIndexPage() {
  return (
    <div className="mx-auto max-w-6xl px-4 py-14 sm:px-6">
      <Badge className="mb-4">Guides</Badge>
      <h1 className="text-3xl tracking-tight text-[var(--ink)] sm:text-4xl">
        Guides
      </h1>
      <p className="mt-3 max-w-2xl text-[var(--body)]">
        Step-by-step how-tos with a shared CoolStore example (
        <code className="text-sm">example/coolstore</code>).
      </p>

      <div className="mt-10 grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
        {guides.map((g) => (
          <Link
            key={g.href}
            href={g.href}
            className="group flex flex-col rounded-[4px] border border-[var(--hairline)] bg-[var(--canvas-soft)]/50 p-5 transition-colors hover:border-[var(--mute)]"
          >
            <h2 className="text-base font-medium text-[var(--ink)]">{g.title}</h2>
            <p className="mt-1 font-mono text-[11px] text-[var(--mute)]">
              {g.feature}
            </p>
            <p className="mt-2 text-sm text-[var(--body)]">{g.blurb}</p>
          </Link>
        ))}
      </div>
    </div>
  );
}
