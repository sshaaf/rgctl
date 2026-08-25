import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

/** Prefix paths for GitHub Pages project sites. */
export function withBase(path: string): string {
  const base = process.env.NEXT_PUBLIC_BASE_PATH || "";
  if (!path.startsWith("/")) return `${base}/${path}`;
  return `${base}${path}`;
}

export const GITHUB_REPO = "https://github.com/sshaaf/rgctl";
export const GITHUB_RELEASES = `${GITHUB_REPO}/releases/latest`;
export const GITHUB_DISCUSSIONS = `${GITHUB_REPO}/discussions`;
