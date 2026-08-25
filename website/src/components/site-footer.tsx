import Link from "next/link";
import {
  footerAgents,
  footerContribute,
  footerLearn,
} from "@/lib/nav";
import { GITHUB_REPO } from "@/lib/utils";

function FooterCol({
  title,
  items,
}: {
  title: string;
  items: readonly { href: string; label: string; external?: boolean }[];
}) {
  return (
    <div>
      <h3 className="mb-3 text-xs font-medium uppercase tracking-wide text-[var(--mute)]">
        {title}
      </h3>
      <ul className="space-y-2">
        {items.map((item) => (
          <li key={item.href}>
            {"external" in item && item.external ? (
              <a
                href={item.href}
                className="text-sm text-[var(--body)] hover:text-[var(--ink)]"
                target="_blank"
                rel="noreferrer"
              >
                {item.label}
              </a>
            ) : (
              <Link
                href={item.href}
                className="text-sm text-[var(--body)] hover:text-[var(--ink)]"
              >
                {item.label}
              </Link>
            )}
          </li>
        ))}
      </ul>
    </div>
  );
}

export function SiteFooter() {
  return (
    <footer className="mt-auto border-t border-[var(--hairline)] bg-[var(--canvas)]">
      <div className="mx-auto grid max-w-6xl gap-10 px-4 py-12 sm:grid-cols-2 sm:px-6 lg:grid-cols-4">
        <div className="space-y-3">
          <p className="font-[family-name:var(--font-serif)] text-lg font-semibold text-[var(--ink)]">
            rgctl
          </p>
          <p className="max-w-xs text-sm text-[var(--body)]">
            Open-source code knowledge graph for LLM agents — accurate answers,
            fewer tokens.
          </p>
          <p className="font-mono text-[11px] text-[var(--mute)]">MIT License</p>
        </div>
        <FooterCol title="Learn" items={footerLearn} />
        <FooterCol title="Agents" items={footerAgents} />
        <FooterCol title="Contribute" items={footerContribute} />
      </div>
      <div className="border-t border-[var(--hairline)]">
        <div className="mx-auto flex max-w-6xl flex-wrap items-center justify-between gap-3 px-4 py-4 sm:px-6">
          <p className="text-xs text-[var(--mute)]">
            Built for adoption — star, fork, and open issues on GitHub.
          </p>
          <a
            href={GITHUB_REPO}
            className="text-xs text-[var(--body-strong)] hover:text-[var(--ink)]"
            target="_blank"
            rel="noreferrer"
          >
            github.com/sshaaf/rgctl
          </a>
        </div>
      </div>
    </footer>
  );
}
