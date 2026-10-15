#!/usr/bin/env python3
"""Select the prose-only CI route from committed Git objects; uncertainty stays full."""
import argparse
from pathlib import Path
import re
import subprocess
import sys

PROSE_PATHS = re.compile(r"docs/yohaku\.md|docs/design/[^/]+\.md|docs/research/[^/]+/[^/]+\.md")
# Diagrams and sample output are not commands; every other fence keeps full checks.
INERT_FENCES = re.compile(r"^(`{3,})(?:mermaid|text)[ \t]*\n.*?^\1[ \t]*$", re.M | re.S)
SETUP_PATHS = re.compile(r"AGENTS\.md|CLAUDE\.md|Cargo\.(?:toml|lock)|rust-toolchain(?:\.toml)?|"
                         r"scripts/.*|\.github/.*|voice/package(?:-lock)?\.json")
EXAMPLES = re.compile(
    r"^\s*(?:`{3,}|~{3,})|^(?: {4,}|\t)[^\n]*\S|"
    r"`(?:cargo|python3?|bash|sh|node|npm|git|\./\S+)\s", re.M,
)


def classify(base, head, root):
    """Return true only for modified allowlisted prose without executable examples."""
    if any(not re.fullmatch(r"[0-9a-fA-F]{40}", revision or "") or
           revision == "0" * 40 for revision in (base, head)):
        return False

    def git(*args):
        return subprocess.check_output(["git", *args], cwd=root, stderr=subprocess.PIPE)

    changes = git("diff", "--name-status", "-z", "--no-renames", base, head).decode().split("\0")
    if changes.pop() != "" or len(changes) % 2:
        raise ValueError("Malformed Git change list")
    if not changes:
        return False
    for status, name in zip(changes[::2], changes[1::2]):
        if status != "M" or not PROSE_PATHS.fullmatch(name):
            return False
        for revision in (base, head):
            if not git("ls-tree", revision, "--", name).startswith(b"100644 blob "):
                return False
            if EXAMPLES.search(INERT_FENCES.sub("", git("show", f"{revision}:{name}").decode())):
                return False
    return True


def setup_changed(base, head, root):
    """Return false only when no committed change can affect a fresh clone's setup."""
    if any(not re.fullmatch(r"[0-9a-fA-F]{40}", revision or "") or
           revision == "0" * 40 for revision in (base, head)):
        return True
    names = subprocess.check_output(["git", "diff", "--name-only", "-z", "--no-renames", base, head],
                                    cwd=root, stderr=subprocess.PIPE).decode().split("\0")
    if names.pop() != "":
        raise ValueError("Malformed Git change list")
    return not names or any(SETUP_PATHS.fullmatch(name) for name in names)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", required=True)
    parser.add_argument("--head", required=True)
    args = parser.parse_args()
    try:
        root = Path(__file__).resolve().parents[1]
        docs_only = classify(args.base, args.head, root)
        setup = setup_changed(args.base, args.head, root)
    except (subprocess.CalledProcessError, ValueError, UnicodeError) as error:
        parser.exit(1, f"Cannot classify committed changes; refusing to select a route: {error}\n")
    print(f"docs_only={str(docs_only).lower()}")
    print(f"setup={str(setup).lower()}")
    print("CI scope: " + ("known prose only" if docs_only else "full verification") +
          ("" if docs_only else "; fresh-clone setup " + ("changed" if setup else "unchanged")),
          file=sys.stderr)


if __name__ == "__main__":
    main()
