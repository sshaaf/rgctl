import type { Metadata } from "next";
import Link from "next/link";
import { Badge } from "@/components/ui/badge";
import { DemoMedia } from "@/components/demo-media";
import { DemoPlayground } from "@/components/demo-playground";
import { GITHUB_REPO } from "@/lib/utils";

export const metadata: Metadata = {
  title: "Demo",
};

export default function DemoPage() {
  return (
    <div className="mx-auto max-w-6xl px-4 py-14 sm:px-6">
      <Badge className="mb-4">Interactive + recorded</Badge>
      <h1 className="text-3xl tracking-tight text-[var(--ink)] sm:text-4xl">
        See rgctl in action
      </h1>
      <p className="mt-3 max-w-2xl text-[var(--body)]">
        Interactive agent scenarios first — prompt → tool call → schema-aligned
        JSON → reasoning. Recorded CLI and dashboard walkthroughs below. Prefer
        the ecommerce-java fixture when you run commands locally.
      </p>

      <h2 className="mt-10 text-2xl tracking-tight text-[var(--ink)]">
        Agent skill scenarios
      </h2>
      <p className="mt-2 max-w-2xl text-sm text-[var(--body)]">
        Commands match the live CLI.
      </p>
      <div className="mt-8">
        <DemoPlayground />
      </div>

      <h2 className="mt-16 text-2xl tracking-tight text-[var(--ink)]">
        Recorded walkthroughs
      </h2>
      <p className="mt-2 max-w-2xl text-sm text-[var(--body)]">
        VHS terminal recording from{" "}
        <a
          href={`${GITHUB_REPO}/blob/main/docs/videos/user-guide-cli.tape`}
          className="text-[var(--ink)] underline"
          target="_blank"
          rel="noreferrer"
        >
          user-guide-cli.tape
        </a>
        , plus dashboard.
      </p>

      <div className="mt-8 space-y-14">
        <DemoMedia
          kind="cli"
          preferGif
          className="mx-auto max-w-4xl"
          caption="CLI (VHS) — discover → GQL → communities → blast → CPG → semantic."
        />
        <DemoMedia
          kind="dashboard"
          className="mx-auto max-w-4xl"
          caption="Dashboard — Graph, Search, CFG, Blast, Migration, and more."
        />
      </div>

      <p className="mt-4 text-sm text-[var(--mute)]">
        rgctl is focused on CLI usecases, however a tech preview dashboard is also available when the discover command runs with --with-dashboard{" "}.
      </p>
    </div>
  );
}
