import type { Metadata } from "next";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { GITHUB_DISCUSSIONS, GITHUB_REPO, GITHUB_RELEASES } from "@/lib/utils";

export const metadata: Metadata = {
  title: "Community",
};

const actions = [
  {
    title: "Star & watch",
    body: "Stars help others discover the project. Watching releases keeps you on CLI updates.",
    href: GITHUB_REPO,
    cta: "Open repository",
  },
  {
    title: "Discussions",
    body: "Ask questions, share agent recipes, and propose docs improvements.",
    href: GITHUB_DISCUSSIONS,
    cta: "Join discussions",
  },
  {
    title: "Issues & PRs",
    body: "Bug reports, language coverage, and docs PRs are the highest-leverage contributions.",
    href: `${GITHUB_REPO}/issues`,
    cta: "Browse issues",
  },
  {
    title: "Releases",
    body: "Grab binaries and read release notes. Docs match the CLI — verify with rgctl --version.",
    href: GITHUB_RELEASES,
    cta: "Latest release",
  },
];

export default function CommunityPage() {
  return (
    <div className="mx-auto max-w-3xl px-4 py-14 sm:px-6">
      <Badge className="mb-4">Open source</Badge>
      <h1 className="text-3xl tracking-tight text-[var(--ink)] sm:text-4xl">
        Grow rgctl with us
      </h1>
      <p className="mt-3 text-[var(--body)]">
        No product tiers — just an MIT-licensed tool. Adoption means stars,
        forks, working agent integrations, and clear docs.
      </p>

      <div className="mt-10 space-y-4">
        {actions.map((a) => (
          <div
            key={a.title}
            className="rounded-[4px] border border-[var(--hairline)] p-5"
          >
            <h2 className="text-base font-medium text-[var(--ink)]">
              {a.title}
            </h2>
            <p className="mt-1 text-sm text-[var(--body)]">{a.body}</p>
            <Button variant="ghost" size="sm" className="mt-3" asChild>
              <a href={a.href} target="_blank" rel="noreferrer">
                {a.cta}
              </a>
            </Button>
          </div>
        ))}
      </div>

      <p className="mt-10 text-sm text-[var(--mute)]">
        Contributing guide:{" "}
        <a
          href={`${GITHUB_REPO}/blob/main/CONTRIBUTING.md`}
          className="text-[var(--body-strong)] underline"
          target="_blank"
          rel="noreferrer"
        >
          CONTRIBUTING.md
        </a>
      </p>
    </div>
  );
}
