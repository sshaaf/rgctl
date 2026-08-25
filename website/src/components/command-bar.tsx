"use client";

import { useState } from "react";
import Link from "next/link";
import { ArrowRight } from "lucide-react";

const examples = [
  {
    label: "Index once",
    cmd: "discover .",
    href: "/install/",
  },
  {
    label: "Impact",
    cmd: '-f json blast-radius "updateQuantity" --depth 2',
    href: "/demo/",
  },
  {
    label: "Intent search",
    cmd: '-f json semantic query "checkout flow"',
    href: "/agents/",
  },
  {
    label: "Hotspots",
    cmd: "-f json metrics --pagerank",
    href: "/demo/",
  },
];

export function CommandBar() {
  const [idx, setIdx] = useState(0);
  const current = examples[idx];

  return (
    <div className="rounded-lg border border-[var(--hairline)] bg-[var(--canvas-soft)] p-2">
      <div className="flex flex-wrap gap-1 border-b border-[var(--hairline)] pb-2">
        {examples.map((ex, i) => (
          <button
            key={ex.label}
            type="button"
            onClick={() => setIdx(i)}
            className={
              i === idx
                ? "rounded-md bg-[var(--surface)] px-2 py-1 text-xs text-[var(--ink)] shadow-sm ring-1 ring-[var(--hairline)]"
                : "rounded-md px-2 py-1 text-xs text-[var(--mute)] hover:text-[var(--ink)]"
            }
          >
            {ex.label}
          </button>
        ))}
      </div>
      <div className="flex items-center gap-3 px-2 py-3 font-[family-name:var(--font-mono)] text-[13px]">
        <span className="text-[var(--mute)]">$</span>
        <span className="flex-1 truncate text-[var(--ink)]">
          rgctl {current.cmd}
        </span>
        <Link
          href={current.href}
          className="inline-flex items-center gap-1 text-xs text-[var(--primary)] hover:underline"
        >
          Try <ArrowRight className="h-3 w-3" />
        </Link>
      </div>
    </div>
  );
}
