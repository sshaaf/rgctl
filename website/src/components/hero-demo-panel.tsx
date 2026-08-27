"use client";

import { useMemo, useState } from "react";
import {
  CapabilityGraph,
  type GraphSceneId,
} from "@/components/blast-radius-graph";
import { cn } from "@/lib/utils";

type Demo = {
  id: GraphSceneId;
  label: string;
  caption: string;
  copyText: string;
  lines: { kind: "cmd" | "out" | "json" | "gap"; text?: string }[];
};

const DEMOS: Demo[] = [
  {
    id: "blast-radius",
    label: "blast-radius",
    caption: "ecommerce-java · impact at depth 2",
    copyText:
      'cd rgctl-tests/ecommerce-java\nrgctl discover .\nrgctl -f json blast-radius "priceShoppingCart" --depth 2',
    lines: [
      { kind: "cmd", text: "discover ." },
      { kind: "out", text: "✓ graph built · reachability cached" },
      { kind: "gap" },
      {
        kind: "cmd",
        text: '-f json blast-radius "priceShoppingCart" --depth 2',
      },
      { kind: "out", text: "{" },
      { kind: "json", text: '  "root": "priceShoppingCart",' },
      {
        kind: "json",
        text: '  "impacted": ["CartController", "TaxCalculator",',
      },
      {
        kind: "json",
        text: '    "OrderRepository", "PaymentGateway"],',
      },
      { kind: "json", text: '  "depth": 2, "edges": 5' },
      { kind: "out", text: "}" },
    ],
  },
  {
    id: "gql",
    label: "gql",
    caption: "exact inventory · typed edges",
    copyText:
      "rgctl -f json gql --macro-name all_functions unused",
    lines: [
      {
        kind: "cmd",
        text: "gql --macro-name all_functions unused",
      },
      { kind: "out", text: "{" },
      { kind: "json", text: '  "schema_version": 1,' },
      { kind: "json", text: '  "rows": [{ "name": "priceShoppingCart", … }]' },
      { kind: "out", text: "}" },
    ],
  },
  {
    id: "semantic",
    label: "semantic",
    caption: "intent search over the graph",
    copyText:
      'rgctl -f json semantic query "checkout flow" --limit 5',
    lines: [
      {
        kind: "cmd",
        text: 'semantic query "checkout flow" --limit 5',
      },
      { kind: "out", text: "{" },
      {
        kind: "json",
        text: '  "hits": [{ "name": "priceShoppingCart", "score": 0.91 }]',
      },
      { kind: "out", text: "}" },
    ],
  },
  {
    id: "cpg",
    label: "cpg",
    caption: "CALL + CFG/PDG façade",
    copyText:
      "rgctl -f json cpg slice --function priceShoppingCart",
    lines: [
      {
        kind: "cmd",
        text: "cpg slice --function priceShoppingCart",
      },
      { kind: "out", text: "{" },
      {
        kind: "json",
        text: '  "nodes": 24, "data_deps": 11, "ctrl_deps": 8',
      },
      { kind: "out", text: "}" },
    ],
  },
  {
    id: "metrics",
    label: "metrics",
    caption: "PageRank · harmonic · hotspots",
    copyText: "rgctl -f json metrics --pagerank",
    lines: [
      { kind: "cmd", text: "metrics --pagerank" },
      { kind: "out", text: "{" },
      {
        kind: "json",
        text: '  "top": [{ "name": "CartController", "pr": 0.082 }]',
      },
      { kind: "out", text: "}" },
    ],
  },
  {
    id: "taint",
    label: "taint",
    caption: "source → sink · security paths",
    copyText:
      "cd your-repo\nrgctl discover . --with-taint --with-security",
    lines: [
      {
        kind: "cmd",
        text: "discover . --with-taint --with-security",
      },
      { kind: "out", text: "✓ taint index · CVE path tags ready" },
      { kind: "gap" },
      { kind: "cmd", text: "-f json cpg flows … --direction forward" },
      { kind: "out", text: '{ "paths": 3, "sinks": ["PaymentGateway"] }' },
    ],
  },
  {
    id: "communities",
    label: "communities",
    caption: "subsystem clusters",
    copyText:
      "rgctl -f json gql --macro-name all_communities unused",
    lines: [
      {
        kind: "cmd",
        text: "gql --macro-name all_communities unused",
      },
      { kind: "out", text: "{" },
      {
        kind: "json",
        text: '  "communities": [{ "id": "12", "size": 48, "label": "cart" }]',
      },
      { kind: "out", text: "}" },
    ],
  },
  {
    id: "migration",
    label: "migration",
    caption: "prioritized plan + CI check",
    copyText:
      "cd your-repo\nrgctl discover . --export-migration-hints\nrgctl -f json check --policy-file policy.json",
    lines: [
      { kind: "cmd", text: "discover . --export-migration-hints" },
      { kind: "out", text: "✓ wrote .rgctl/migration_plan.json" },
      { kind: "gap" },
      { kind: "cmd", text: "check --policy-file policy.json" },
      { kind: "out", text: "✓ no blast-radius policy violations" },
    ],
  },
];

export function HeroDemoPanel({ className }: { className?: string }) {
  const [active, setActive] = useState(DEMOS[0].id);
  const [copied, setCopied] = useState(false);
  const demo = useMemo(
    () => DEMOS.find((d) => d.id === active) ?? DEMOS[0],
    [active],
  );

  async function onCopy() {
    await navigator.clipboard.writeText(demo.copyText);
    setCopied(true);
    window.setTimeout(() => setCopied(false), 1500);
  }

  return (
    <div
      className={cn(
        "overflow-hidden rounded-2xl border border-[var(--hairline)] bg-[var(--surface)] shadow-[0_1px_0_rgba(0,0,0,0.04)]",
        className,
      )}
    >
      <div className="flex flex-wrap items-center justify-between gap-3 border-b border-[var(--hairline)] px-4 py-3 sm:px-5">
        <div className="flex flex-wrap gap-1.5">
          {DEMOS.map((d) => (
            <button
              key={d.id}
              type="button"
              onClick={() => setActive(d.id)}
              className={cn(
                "rounded-full border px-3 py-1 font-[family-name:var(--font-mono)] text-[12px] font-medium transition-colors",
                d.id === active
                  ? "border-[var(--primary)] bg-[var(--primary)] text-[var(--on-primary)]"
                  : "border-[var(--hairline)] bg-[var(--canvas-soft)] text-[var(--body)] hover:border-[var(--mute)] hover:text-[var(--ink)]",
              )}
            >
              {d.label}
            </button>
          ))}
        </div>
        <span className="hidden font-[family-name:var(--font-mono)] text-[12px] text-[var(--mute)] md:inline">
          {demo.caption}
        </span>
      </div>

      <div className="grid lg:grid-cols-2">
        <div className="border-b border-[var(--hairline)] p-5 sm:p-6 lg:border-b-0 lg:border-r">
          <div className="font-[family-name:var(--font-mono)] text-[13.5px] leading-[2.05]">
            {demo.lines.map((line, i) => {
              if (line.kind === "gap") {
                return <div key={i} className="h-2.5" />;
              }
              if (line.kind === "cmd") {
                return (
                  <div key={i}>
                    <span className="font-semibold text-[var(--primary)]">
                      rgctl ›{" "}
                    </span>
                    <span className="font-semibold text-[var(--ink)]">
                      {line.text}
                    </span>
                  </div>
                );
              }
              if (line.kind === "json") {
                return (
                  <div key={i} className="text-[var(--body)]">
                    {line.text?.split(/("(?:\\.|[^"])*")/g).map((part, j) =>
                      part.startsWith('"') ? (
                        <span key={j} className="text-[var(--primary)]">
                          {part}
                        </span>
                      ) : (
                        <span key={j}>{part}</span>
                      ),
                    )}
                  </div>
                );
              }
              return (
                <div key={i} className="text-[12.5px] text-[var(--mute)]">
                  {line.text}
                </div>
              );
            })}
          </div>
          <button
            type="button"
            onClick={onCopy}
            className="mt-5 rounded-full border border-[var(--ink)] bg-[var(--canvas-soft)] px-3 py-1.5 font-[family-name:var(--font-mono)] text-[11px] font-bold text-[var(--ink)] transition-colors hover:bg-[var(--primary)] hover:text-[var(--on-primary)] hover:border-[var(--primary)]"
          >
            {copied ? "Copied" : "Copy commands"}
          </button>
        </div>

        <div className="bg-[var(--canvas)]/40 p-4 sm:p-5">
          <CapabilityGraph key={demo.id} sceneId={demo.id} />
        </div>
      </div>
    </div>
  );
}
