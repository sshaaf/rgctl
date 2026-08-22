import type { Metadata } from "next";
import Link from "next/link";
import { Badge } from "@/components/ui/badge";

export const metadata: Metadata = {
  title: "Docs",
};

const guides = [
  {
    title: "All guides",
    blurb: "CoolStore walkthroughs for discover, GQL, blast-radius, CPG, semantic, MCP, install --skill, and more.",
    href: "/docs/guides/",
  },
  {
    title: "Discovering and indexing",
    blurb: "Build the knowledge graph from source.",
    href: "/docs/guides/discovering-and-indexing/",
  },
  {
    title: "Graph query language",
    blurb: "Cypher-like MATCH over functions, types, and docs.",
    href: "/docs/guides/graph-query-language/",
  },
  {
    title: "Blast radius",
    blurb: "Upstream impact before you edit a symbol.",
    href: "/docs/guides/blast-radius-analysis/",
  },
  {
    title: "Hybrid CPG",
    blurb: "CALL + CFG/PDG: mutations, flows, slices.",
    href: "/docs/guides/hybrid-cpg/",
  },
  {
    title: "Agent skill",
    blurb: "rg-build install --skill for Claude Code and Cursor.",
    href: "/docs/guides/agent-skill/",
  },
  {
    title: "MCP server",
    blurb: "serve --mode mcp: stdio status tool and auto full pipeline.",
    href: "/docs/guides/mcp-server/",
  },
];

const primary = [
  {
    title: "AGENTS.md",
    blurb: "Index once, query with -f json — agent contract.",
    href: "/docs/AGENTS/",
  },
  {
    title: "Agent recipes",
    blurb: "Copy-paste multi-step workflows.",
    href: "/docs/agent-recipes/",
  },
  {
    title: "JSON API",
    blurb: "schema_version and field catalogs for scripts.",
    href: "/docs/json-api/",
  },
  {
    title: "User Guide",
    blurb: "Install, ecommerce-java walkthrough, CLI commands.",
    href: "/docs/user-guide/",
  },
  {
    title: "Introduction",
    blurb: "Concepts — graph, reachability, capability map.",
    href: "/docs/Introduction/",
  },
];

const secondary = [
  { title: "Languages", href: "/docs/languages/", blurb: "Tier 1 plugins and discover depth." },
  { title: "FAQ", href: "/docs/faq/", blurb: "Flags, embedders, exit codes." },
  { title: "Glossary", href: "/docs/glossary/", blurb: "Blast, CPG, communities, …" },
  { title: "HTTP API", href: "/docs/http-api/", blurb: "serve /api/query." },
  {
    title: "Migration how-to",
    href: "/docs/building-migration-plan/",
    blurb: "CLI-oriented migration phases.",
  },
];

export default function DocsPage() {
  return (
    <div className="mx-auto max-w-6xl px-4 py-14 sm:px-6">
      <Badge className="mb-4">Documentation</Badge>
      <h1 className="text-3xl tracking-tight text-[var(--ink)] sm:text-4xl">
        Docs hub
      </h1>
      <p className="mt-3 max-w-2xl text-[var(--body)]">
        Served from the repository <code className="text-sm">docs/</code> tree
        (agent-first). Prefer CLI <code className="text-sm">-f json</code> over
        the optional browser UI.
      </p>

      <h2 className="mt-10 text-lg font-medium text-[var(--ink)]">
        Guides
      </h2>
      <p className="mt-2 max-w-2xl text-sm text-[var(--body)]">
        Feature how-tos with a shared CoolStore example. Full list on the{" "}
        <Link href="/docs/guides/" className="underline">
          guides index
        </Link>
        .
      </p>
      <div className="mt-4 grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
        {guides.map((c) => (
          <Link
            key={c.title}
            href={c.href}
            className="group flex flex-col rounded-[4px] border border-[var(--hairline)] bg-[var(--canvas-soft)]/50 p-5 transition-colors hover:border-[var(--mute)]"
          >
            <h3 className="text-base font-medium text-[var(--ink)]">{c.title}</h3>
            <p className="mt-2 text-sm text-[var(--body)]">{c.blurb}</p>
          </Link>
        ))}
      </div>

      <h2 className="mt-12 text-lg font-medium text-[var(--ink)]">Primary</h2>
      <div className="mt-4 grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
        {primary.map((c) => (
          <Link
            key={c.title}
            href={c.href}
            className="group flex flex-col rounded-[4px] border border-[var(--hairline)] bg-[var(--canvas-soft)]/50 p-5 transition-colors hover:border-[var(--mute)]"
          >
            <h3 className="text-base font-medium text-[var(--ink)]">{c.title}</h3>
            <p className="mt-2 text-sm text-[var(--body)]">{c.blurb}</p>
          </Link>
        ))}
      </div>

      <h2 className="mt-12 text-lg font-medium text-[var(--ink)]">Secondary</h2>
      <div className="mt-4 grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
        {secondary.map((c) => (
          <Link
            key={c.title}
            href={c.href}
            className="rounded-[4px] border border-[var(--hairline)] p-4 text-sm hover:border-[var(--mute)]"
          >
            <div className="font-medium text-[var(--ink)]">{c.title}</div>
            <p className="mt-1 text-[var(--body)]">{c.blurb}</p>
          </Link>
        ))}
      </div>

      <p className="mt-10 text-sm text-[var(--mute)]">
        Optional UI:{" "}
        <Link href="/docs/dashboard-user-guide/" className="underline">
          Dashboard user guide
        </Link>
        . Contributors:{" "}
        <Link href="/docs/design/README/" className="underline">
          design/
        </Link>
        .
      </p>
    </div>
  );
}
