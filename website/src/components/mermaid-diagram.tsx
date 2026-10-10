"use client";

import { useEffect, useId, useState } from "react";
import { useTheme } from "@/components/theme-provider";

type Props = {
  chart: string;
};

/**
 * Client-only Mermaid renderer. Mermaid needs DOM APIs (getBBox), so we
 * dynamic-import inside useEffect and re-render when the site theme flips.
 */
export function MermaidDiagram({ chart }: Props) {
  const { theme } = useTheme();
  const reactId = useId().replace(/:/g, "");
  const [svg, setSvg] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    const source = chart.trim();
    if (!source) {
      setSvg(null);
      setError(null);
      return;
    }

    (async () => {
      try {
        const mermaid = (await import("mermaid")).default;
        mermaid.initialize({
          startOnLoad: false,
          securityLevel: "strict",
          theme: theme === "dark" ? "dark" : "neutral",
          fontFamily: "inherit",
        });
        const id = `mermaid-${reactId}-${Math.random().toString(36).slice(2, 9)}`;
        const { svg: rendered } = await mermaid.render(id, source);
        if (!cancelled) {
          setSvg(rendered);
          setError(null);
        }
      } catch (e) {
        if (!cancelled) {
          setSvg(null);
          setError(e instanceof Error ? e.message : "Failed to render diagram");
        }
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [chart, theme, reactId]);

  if (error) {
    return (
      <pre className="overflow-x-auto rounded-lg border border-[var(--hairline)] bg-[var(--canvas-soft)] p-3 text-sm text-[var(--mute)]">
        <code>{`%% mermaid render error: ${error}\n${chart.trim()}`}</code>
      </pre>
    );
  }

  if (!svg) {
    return (
      <div
        className="my-4 flex min-h-24 items-center justify-center rounded-lg border border-[var(--hairline)] bg-[var(--canvas-soft)] text-sm text-[var(--mute)]"
        aria-busy="true"
      >
        Rendering diagram…
      </div>
    );
  }

  return (
    <div
      className="my-4 overflow-x-auto rounded-lg border border-[var(--hairline)] bg-[var(--canvas-soft)] p-4 [&_svg]:mx-auto [&_svg]:max-w-full"
      dangerouslySetInnerHTML={{ __html: svg }}
    />
  );
}
