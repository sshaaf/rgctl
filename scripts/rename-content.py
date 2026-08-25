#!/usr/bin/env python3
"""Bulk content rename rgctl → rgctl. Run from repo root."""
from __future__ import annotations

import os
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SKIP_DIRS = {".git", "target", "node_modules"}
EXTENSIONS = {
    ".rs", ".toml", ".md", ".json", ".yaml", ".yml", ".sh", ".py",
    ".tsx", ".ts", ".js", ".mjs", ".html", ".txt", ".tape", ".lock",
}

REPLACEMENTS = [
    (".rgctl", ".rgctl"),
    ("RGCTL_", "RGCTL_"),
    ("RGCTL_", "RGCTL_"),
    ("rgctl", "rgctl"),
    ("rgctl-", "rgctl-"),
    ("rgctl_", "rgctl_"),
    ("rgctl", "rgctl"),
]


def should_process(path: Path) -> bool:
    if path.suffix not in EXTENSIONS and path.name not in ("AGENTS.md", "languages.toml"):
        return False
    for part in path.parts:
        if part in SKIP_DIRS:
            return False
    return True


def main() -> int:
    changed = 0
    for dirpath, dirnames, filenames in os.walk(ROOT):
        dirnames[:] = [d for d in dirnames if d not in SKIP_DIRS]
        for name in filenames:
            path = Path(dirpath) / name
            if not should_process(path):
                continue
            try:
                text = path.read_text(encoding="utf-8")
            except (UnicodeDecodeError, OSError):
                continue
            original = text
            for old, new in REPLACEMENTS:
                text = text.replace(old, new)
            if text != original:
                path.write_text(text, encoding="utf-8")
                changed += 1
    print(f"Updated {changed} files")
    return 0


if __name__ == "__main__":
    sys.exit(main())
