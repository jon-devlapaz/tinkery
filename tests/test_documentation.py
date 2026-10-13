"""Executable documentation contracts; fake commands, no providers or operator data."""
import fnmatch
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
import unittest

REPO = Path(__file__).resolve().parents[1]
ENTRY = "AGENTS.md"


def tracked_files():
    listed = subprocess.run(["git", "ls-files", "-z"], cwd=REPO, check=True,
                            capture_output=True, text=True).stdout
    return {path for path in listed.split("\0") if path}


TRACKED = tracked_files()
TRACKED_DIRS = {str(parent) for path in TRACKED for parent in Path(path).parents} - {"."}
# Vendored upstream docs and test fixtures are not ours to steer agents with.
DOCS = sorted(path for path in TRACKED if path.endswith(".md")
              and not path.startswith(("vendor/", "tests/fixtures/")))
FENCE = re.compile(r"^(```|~~~).*?^\1", re.M | re.S)
LINK = re.compile(r"\]\(([^)\s]+)\)|^\[[^\]]+\]:\s*(\S+)", re.M)


def prose(path):
    """The page without fenced code, where links and paths are only examples."""
    return FENCE.sub("", (REPO / path).read_text())


def anchors(path):
    """GitHub's heading slugs, with -1, -2 for repeats, plus explicit ids."""
    seen, found = {}, set(re.findall(r'<a id="([^"]+)"', prose(path)))
    for heading in re.findall(r"^#{1,6}\s+(.+?)\s*#*$", prose(path), re.M):
        slug = re.sub(r"[^\w\- ]", "", heading.lower()).replace(" ", "-")
        found.add(f"{slug}-{seen[slug]}" if slug in seen else slug)
        seen[slug] = seen.get(slug, 0) + 1
    return found


def relative_links(path):
    """(as written, repository path, fragment) for each link that stays in the repository."""
    for match in LINK.finditer(prose(path)):
        url = match.group(1) or match.group(2)
        if re.match(r"[a-z][a-z0-9+.-]*:", url):
            continue
        target, _, fragment = url.partition("#")
        resolved = os.path.normpath(Path(path).parent / target) if target else path
        yield url, resolved, fragment


class DocumentationLintTests(unittest.TestCase):
    """Deterministic checks that every agent-facing page points at things that exist."""

    def test_links_reach_tracked_files_and_headings(self):
        problems = []
        for path in DOCS:
            for url, target, fragment in relative_links(path):
                if target not in TRACKED and target not in TRACKED_DIRS:
                    problems.append(f"{path}: {url} is not a tracked file")
                elif fragment and target.endswith(".md") and fragment not in anchors(target):
                    problems.append(f"{path}: {url} names no heading in {target}")
        self.assertEqual([], problems, "Fix the link, or the heading it names")

    def test_code_paths_name_tracked_files(self):
        roots = {path.split("/")[0] for path in TRACKED if "/" in path}
        problems = []
        for path in DOCS:
            for code in re.findall(r"`([^`\n]+)`", prose(path)):
                candidate = re.sub(r":\d.*$", "", code).rstrip("/")
                if (candidate.split("/")[0] in roots and "/" in candidate
                        and not re.search(r"[\s<>{}]", candidate)
                        and not any(fnmatch.fnmatch(known, candidate)
                                    for known in TRACKED | TRACKED_DIRS)):
                    problems.append(f"{path}: `{code}` names nothing in the repository")
        self.assertEqual([], problems, "Rename or drop paths the code no longer has")

    def test_documented_options_exist(self):
        problems = []
        for path in DOCS:
            for block in re.findall(r"^```(?:bash|sh)\n(.*?)^```", (REPO / path).read_text(),
                                    re.M | re.S):
                for line in block.replace("\\\n", " ").splitlines():
                    command = re.match(r"\s*cargo\s.*?\brun\b(.*?)\s--\s(.*)", line)
                    if not command:
                        continue
                    example = re.search(r"--example\s+(\S+)", command.group(1))
                    source = f"examples/{example.group(1)}.rs" if example else "src/main.rs"
                    accepted = (REPO / source).read_text()
                    for option in re.findall(r"(?<!\S)(--[a-z][a-z-]*)", command.group(2)):
                        if f'"{option}"' not in accepted:
                            problems.append(f"{path}: {source} accepts no {option}")
        self.assertEqual([], problems, "Documented commands must use options the program parses")

    def test_every_page_is_reachable_from_the_entry_point(self):
        reached, queue = {ENTRY}, [ENTRY]
        while queue:
            for _, target, _ in relative_links(queue.pop()):
                if target in DOCS and target not in reached:
                    reached.add(target)
                    queue.append(target)
        self.assertEqual([], sorted(set(DOCS) - reached),
                         f"Link each page from a page {ENTRY} already reaches")


class DocumentationTests(unittest.TestCase):
    def test_ci_can_extract_one_valid_verification_block(self):
        source = (REPO / "CLAUDE.md").read_text()
        for marker in ("<!-- full-check:start -->", "<!-- full-check:end -->"):
            self.assertEqual(source.count(marker), 1)
        section = source.split("<!-- full-check:start -->", 1)[1].split(
            "<!-- full-check:end -->", 1)[0]
        blocks = re.findall(r"```bash\n(.*?)\n```", section, re.S)
        self.assertEqual(len(blocks), 1)
        result = subprocess.run(["bash", "-n"], input=blocks[0], text=True,
                                capture_output=True, timeout=10)
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_live_trial_isolates_sessions_and_think_records(self):
        source = (REPO / "docs/terminal-setup.md").read_text()
        blocks = [block for block in re.findall(r"```(?:sh|bash)\n(.*?)\n```", source, re.S)
                  if "--shape-pi" in block]
        self.assertEqual(len(blocks), 1)
        with tempfile.TemporaryDirectory(prefix="tinkery-TEST-docs-") as directory:
            root = Path(directory)
            binary = root / "bin"
            binary.mkdir()
            trial = root / "TEST trial with spaces"
            trial.mkdir()
            capture = root / "args.json"
            commands = {
                "mktemp": 'import os\nprint(os.environ["TEST_TRIAL_ROOT"])\n',
                "cargo": 'import json, os, sys\nfrom pathlib import Path\n'
                         'Path(os.environ["TEST_CAPTURE"]).write_text(json.dumps(sys.argv[1:]))\n',
            }
            for name, body in commands.items():
                path = binary / name
                path.write_text(f"#!{sys.executable}\n{body}")
                path.chmod(0o755)

            def run_trial():
                return subprocess.run(
                    [shutil.which("bash"), "-c", blocks[0]],
                    cwd=root, text=True, capture_output=True, timeout=10,
                    env={"HOME": str(root / "home"), "PATH": str(binary),
                         "TEST_TRIAL_ROOT": str(trial), "TEST_CAPTURE": str(capture)},
                )

            result = run_trial()
            self.assertEqual(result.returncode, 0, result.stderr)
            args = json.loads(capture.read_text())
            self.assertIn("--shape-pi", args)
            destinations = []
            for flag in ("--seed-session-root", "--record-root"):
                self.assertIn(flag, args, f"Trial must override {flag}; defaults use operator data")
                path = Path(args[args.index(flag) + 1])
                self.assertTrue(path.is_absolute(), (flag, path))
                self.assertTrue(path.is_relative_to(trial), (flag, path))
                destinations.append(path)
            self.assertNotEqual(destinations[0], destinations[1],
                                "Sessions and think records need separate roots")

            capture.unlink()
            (binary / "mktemp").write_text(f"#!{sys.executable}\nraise SystemExit(1)\n")
            failed = run_trial()
            self.assertNotEqual(failed.returncode, 0)
            self.assertFalse(capture.exists(), "A failed temporary root must prevent launch")


if __name__ == "__main__":
    unittest.main()
