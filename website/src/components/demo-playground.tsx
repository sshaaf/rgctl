"use client";

import { useMemo, useState } from "react";
import { demos, demoFlows } from "@/lib/demos";
import { Badge } from "@/components/ui/badge";
import { JsonPanel, TerminalBlock } from "@/components/terminal";
import { cn } from "@/lib/utils";

export function DemoPlayground() {
  const [flow, setFlow] = useState<string>(demoFlows[0]);
  const filtered = useMemo(
    () => demos.filter((d) => d.flow === flow),
    [flow],
  );
  const [activeId, setActiveId] = useState(filtered[0]?.id ?? demos[0].id);
  const active =
    demos.find((d) => d.id === activeId) ?? filtered[0] ?? demos[0];

  return (
    <div className="space-y-6">
      <div className="flex flex-wrap gap-2">
        {demoFlows.map((f) => (
          <button
            key={f}
            type="button"
            onClick={() => {
              setFlow(f);
              const next = demos.find((d) => d.flow === f);
              if (next) setActiveId(next.id);
            }}
            className={cn(
              "rounded-[3px] border px-3 py-1.5 text-xs font-medium transition-colors",
              flow === f
                ? "border-[var(--ink)] bg-[var(--canvas-soft)] text-[var(--ink)]"
                : "border-[var(--hairline)] text-[var(--body)] hover:text-[var(--ink)]",
            )}
          >
            {f}
          </button>
        ))}
      </div>

      <div className="grid gap-6 lg:grid-cols-[240px_1fr]">
        <aside className="space-y-1">
          {filtered.map((d) => (
            <button
              key={d.id}
              type="button"
              onClick={() => setActiveId(d.id)}
              className={cn(
                "block w-full rounded-[3px] px-3 py-2 text-left text-sm transition-colors",
                active.id === d.id
                  ? "bg-[var(--canvas-soft)] text-[var(--ink)]"
                  : "text-[var(--body)] hover:text-[var(--ink)]",
              )}
            >
              {d.title}
            </button>
          ))}
        </aside>

        <div className="space-y-4">
          <div className="space-y-2">
            <Badge>User prompt</Badge>
            <p className="text-lg text-[var(--ink)]">&ldquo;{active.prompt}&rdquo;</p>
            {active.note ? (
              <p className="text-sm text-[var(--mute)]">{active.note}</p>
            ) : null}
          </div>

          <div className="space-y-2">
            <Badge>Agent tool call</Badge>
            <TerminalBlock
              lines={active.commands.map((c) => `rgctl ${c}`)}
            />
          </div>

          <div className="grid gap-4 md:grid-cols-2">
            <JsonPanel title="stdout · -f json (illustrative)" json={active.output} />
            <div className="rounded-[4px] border border-[var(--hairline)] bg-[var(--canvas-soft)] p-4">
              <p className="mb-2 font-mono text-[11px] text-[var(--mute)]">
                LLM reasoning
              </p>
              <p className="text-sm leading-relaxed text-[var(--body-strong)]">
                {active.reasoning}
              </p>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
