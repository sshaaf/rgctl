"use client";

import { useState } from "react";
import { Copy, Check } from "lucide-react";
import { cn } from "@/lib/utils";

export function TerminalBlock({
  prompt = "$",
  lines,
  className,
}: {
  prompt?: string;
  lines: string[];
  className?: string;
}) {
  const [copied, setCopied] = useState(false);
  const text = lines.join("\n");

  async function onCopy() {
    await navigator.clipboard.writeText(text);
    setCopied(true);
    window.setTimeout(() => setCopied(false), 1500);
  }

  return (
    <div
      className={cn(
        "overflow-hidden rounded-lg border border-[var(--hairline)] bg-[var(--canvas-soft)]",
        className,
      )}
    >
      <div className="flex items-center justify-between border-b border-[var(--hairline)] bg-[var(--surface)]/70 px-3 py-2">
        <div className="flex gap-1.5" aria-hidden>
          <span className="h-2.5 w-2.5 rounded-full bg-[var(--mute)]/40" />
          <span className="h-2.5 w-2.5 rounded-full bg-[var(--mute)]/40" />
          <span className="h-2.5 w-2.5 rounded-full bg-[var(--mute)]/40" />
        </div>
        <button
          type="button"
          onClick={onCopy}
          className="inline-flex items-center gap-1 font-[family-name:var(--font-mono)] text-[11px] text-[var(--mute)] hover:text-[var(--ink)]"
        >
          {copied ? (
            <Check className="h-3 w-3" />
          ) : (
            <Copy className="h-3 w-3" />
          )}
          {copied ? "Copied" : "Copy"}
        </button>
      </div>
      <pre className="overflow-x-auto p-4 font-[family-name:var(--font-mono)] text-[13px] leading-relaxed text-[var(--body-strong)]">
        {lines.map((line, i) => (
          <div key={i} className="whitespace-pre-wrap">
            {line.startsWith("$") || line.startsWith("rgctl") ? (
              <>
                <span className="text-[var(--mute)]">{prompt} </span>
                <span className="text-[var(--ink)]">
                  {line.replace(/^\$\s?/, "")}
                </span>
              </>
            ) : (
              <span className="text-[var(--body)]">{line}</span>
            )}
          </div>
        ))}
      </pre>
    </div>
  );
}

export function JsonPanel({
  title,
  json,
  className,
}: {
  title: string;
  json: string;
  className?: string;
}) {
  return (
    <div
      className={cn(
        "overflow-hidden rounded-lg border border-[var(--hairline)] bg-[var(--canvas-soft)]",
        className,
      )}
    >
      <div className="border-b border-[var(--hairline)] px-3 py-2 font-[family-name:var(--font-mono)] text-[11px] text-[var(--mute)]">
        {title}
      </div>
      <pre className="overflow-x-auto p-4 font-[family-name:var(--font-mono)] text-[12px] leading-relaxed text-[var(--body-strong)]">
        {json}
      </pre>
    </div>
  );
}
