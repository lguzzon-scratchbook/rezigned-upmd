#!/usr/bin/env python3
"""Fail if docs language lists drift from REGISTRY in languages/mod.rs.

Usage:
    python3 scripts/check_languages.py

Checks README.md ("Built-in runners cover ..."), DEMO.md ("also supports ..."
plus fenced code-block tags, which demo Bash/sh/Python/JavaScript), and
.github/pages/index.html ("Language runners" feature) each cover exactly the
registry entries in crates/upmd-runner/src/languages/mod.rs.
Exit 0 = pass, 1 = drift. Stdlib only.
"""

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MOD_RS = ROOT / "crates/upmd-runner/src/languages/mod.rs"
README = ROOT / "README.md"
DEMO = ROOT / "DEMO.md"
INDEX = ROOT / ".github/pages/index.html"


def parse_registry(text):
    """Return (canonical names, alias -> canonical map), all lowercase."""
    canonical = []
    alias_to_canonical = {}
    for m in re.finditer(r"(\w+)\s*\{\s*aliases:\s*&\[([^\]]*)\]", text):
        name = m.group(1).lower()
        canonical.append(name)
        for alias in re.findall(r'"([^"]+)"', m.group(2)):
            alias_to_canonical[alias.lower()] = name
    # Display-only synonym used by README, not a code-block alias.
    alias_to_canonical.setdefault("posix shell", "shell")
    return canonical, alias_to_canonical


def split_names(fragment, alias_to_canonical):
    """Split a 'A, B, and C' list fragment into canonical names."""
    names = set()
    for chunk in fragment.split(","):
        chunk = chunk.strip().rstrip(".")
        if chunk.lower().startswith("and "):
            chunk = chunk[4:]
        chunk = chunk.strip().lower()
        if not chunk:
            continue
        names.add(alias_to_canonical.get(chunk, chunk))
    return names


def names_after_marker(text, pattern, alias_to_canonical, label):
    m = re.search(pattern, text, re.S)
    if not m:
        print(f"FAIL: {label}: list marker not found")
        sys.exit(1)
    return split_names(m.group(1), alias_to_canonical)


def main():
    registry, aliases = parse_registry(MOD_RS.read_text())
    expected = set(registry)
    if len(expected) != len(registry):
        print("FAIL: duplicate registry entry names")
        return 1

    readme = names_after_marker(
        README.read_text(), r"Built-in runners cover (.*?)\.", aliases, "README.md"
    )
    index = names_after_marker(
        INDEX.read_text(), r"Language runners</h3><p>(.*?)</p>", aliases, "index.html"
    )
    demo_listed = names_after_marker(
        DEMO.read_text(), r"also supports (.*?) when", aliases, "DEMO.md"
    )
    demo_fenced = {
        aliases[tag.lower()]
        for tag in re.findall(r"^```(\w+)", DEMO.read_text(), re.M)
        if tag.lower() in aliases
    }
    demo = demo_listed | demo_fenced

    failed = False
    for label, found in (("README.md", readme), ("index.html", index), ("DEMO.md", demo)):
        missing = sorted(expected - found)
        extra = sorted(found - expected)
        if missing or extra:
            failed = True
            print(f"FAIL: {label} drifts from REGISTRY")
            if missing:
                print(f"  missing: {', '.join(missing)}")
            if extra:
                print(f"  extra: {', '.join(extra)}")
    if readme != index or readme != demo:
        failed = True
        print("FAIL: README/DEMO/index.html lists disagree with each other")
    if failed:
        print("Update the docs lists or REGISTRY so all three match.")
        return 1
    print(f"✓ docs language lists match REGISTRY ({len(expected)} languages)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
