#!/usr/bin/env python3
"""Select the prose-only CI route from committed Git objects; uncertainty stays full."""
import argparse
from pathlib import Path
import re
import subprocess
import sys

PROSE_PATHS = frozenset({"docs/design/architecture.md"})
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
        if status != "M" or name not in PROSE_PATHS:
            return False
        for revision in (base, head):
            if not git("ls-tree", revision, "--", name).startswith(b"100644 blob "):
                return False
            if EXAMPLES.search(git("show", f"{revision}:{name}").decode()):
                return False
    return True


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", required=True)
    parser.add_argument("--head", required=True)
    args = parser.parse_args()
    try:
        docs_only = classify(args.base, args.head, Path(__file__).resolve().parents[1])
    except (subprocess.CalledProcessError, ValueError, UnicodeError) as error:
        parser.exit(1, f"Cannot classify committed changes; refusing to select a route: {error}\n")
    print(f"docs_only={str(docs_only).lower()}")
    print("CI scope: " + ("known prose only" if docs_only else "full verification"), file=sys.stderr)


if __name__ == "__main__":
    main()
