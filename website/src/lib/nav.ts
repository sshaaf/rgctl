export const primaryNav = [
  { href: "/docs/", label: "Docs" },
  { href: "/docs/guides/", label: "Guides" },
  { href: "/docs/languages/", label: "Languages" },
  { href: "/agents/", label: "Agents" },
  { href: "/demo/", label: "Demo" },
  { href: "/community/", label: "Community" },
] as const;

export const footerLearn = [
  { href: "/docs/", label: "Documentation" },
  { href: "/docs/guides/", label: "Guides" },
  { href: "/docs/languages/", label: "Languages" },
  { href: "/install/", label: "Install" },
  { href: "/docs/user-guide/", label: "User Guide" },
  { href: "/docs/faq/", label: "FAQ" },
] as const;

export const footerAgents = [
  { href: "/agents/", label: "Agent overview" },
  { href: "/docs/AGENTS/", label: "AGENTS.md" },
  { href: "/docs/agent-recipes/", label: "Recipes" },
  { href: "/docs/json-api/", label: "JSON API" },
] as const;

export const footerContribute = [
  {
    href: "https://github.com/sshaaf/rgctl/blob/main/CONTRIBUTING.md",
    label: "Contributing",
    external: true,
  },
  {
    href: "https://github.com/sshaaf/rgctl/issues",
    label: "Issues",
    external: true,
  },
  {
    href: "https://github.com/sshaaf/rgctl/discussions",
    label: "Discussions",
    external: true,
  },
  {
    href: "https://github.com/sshaaf/rgctl/releases",
    label: "Releases",
    external: true,
  },
] as const;
