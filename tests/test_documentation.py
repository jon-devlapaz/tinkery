"""Executable documentation contracts; fake commands, no providers or operator data."""
import fnmatch
import hashlib
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
    return {path for path in listed.split("\0") if path and (REPO / path).is_file()}


def agent_docs(tracked, read):
    """Our agent-facing pages. Vendored upstream docs, upstream skills and test fixtures
    are not ours to steer agents with, nor is what an installed tink-sdlc scaffold lists
    in its receipt; CLAUDE.md only imports the entry point and names the skills folder."""
    receipt = "_system/scaffold.json"
    scaffold = set(json.loads(read(receipt))["files"]) if receipt in tracked else set()
    return sorted(path for path in tracked if path.endswith(".md") and path != "CLAUDE.md"
                  and path not in scaffold
                  and not path.startswith(("vendor/", ".agents/skills/", "tests/fixtures/")))


TRACKED = tracked_files()
TRACKED_DIRS = {str(parent) for path in TRACKED for parent in Path(path).parents} - {"."}
DOCS = agent_docs(TRACKED, lambda path: (REPO / path).read_text())
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

    def test_one_guide_serves_every_harness(self):
        # Claude Code doesn't load .agents/skills/ by itself, as Codex and Pi do.
        self.assertEqual("@AGENTS.md\n\nProject skills live in `.agents/skills/`. "
                         "When a task matches one, read its `SKILL.md`.\n",
                         (REPO / "CLAUDE.md").read_text(),
                         f"Agent guidance lives in {ENTRY}; CLAUDE.md only imports it")

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
    def test_an_installed_sdlc_scaffold_steers_only_its_own_docs(self):
        receipt = json.dumps({"files": ["_system/SDLC.md", "stages/01-plan/CONTEXT.md"]})
        tracked = {"AGENTS.md", "docs/yohaku.md", "_system/SDLC.md", "_system/scaffold.json",
                   "stages/01-plan/CONTEXT.md", "stages/notes.md"}
        self.assertEqual(["AGENTS.md", "docs/yohaku.md", "stages/notes.md"],
                         agent_docs(tracked, {"_system/scaffold.json": receipt}.get))
        # An untracked receipt excuses nothing.
        self.assertIn("_system/SDLC.md",
                      agent_docs(tracked - {"_system/scaffold.json"}, lambda path: receipt))

    def test_retro_loads_its_style_guide_without_a_harness_specific_tool(self):
        skill = ".agents/skills/retro/SKILL.md"
        first_step = re.search(r"^1\. (.+)$", (REPO / skill).read_text(), re.M).group(1)
        self.assertTrue(first_step.startswith("Read [writing-for-agents]"), first_step)
        links = list(relative_links(skill))
        self.assertIn(("../writing-for-agents/SKILL.md",
                       ".agents/skills/writing-for-agents/SKILL.md", ""), links)
        self.assertIn(".agents/skills/writing-for-agents/SKILL.md", TRACKED)

    def test_retro_moves_corrections_left_without_removing_decisions(self):
        skill = ".agents/skills/retro/SKILL.md"
        self.assertIn(("../../../docs/yohaku.md#for-anyone-changing-tinkery-human-or-agent",
                       "docs/yohaku.md", "for-anyone-changing-tinkery-human-or-agent"),
                      list(relative_links(skill)))
        self.assertIn("never a decision", prose(skill))
        self.assertIn("**Move each correction left.**", prose("docs/yohaku.md"))

    def test_backlog_forwarders_are_retired_without_losing_permissions(self):
        for area in ("think", "menu", "measure"):
            self.assertFalse((REPO / f"docs/research/{area}/backlog.md").exists())
        shared = prose("docs/research/shared/backlog.md")
        self.assertIn("Neither authorizes a backend migration.", shared)
        self.assertIn("https://github.com/jon-devlapaz/tinkery/issues/118", shared)
        architecture = list(relative_links("docs/design/architecture.md"))
        self.assertIn(("../research/shared/backlog.md#think",
                       "docs/research/shared/backlog.md", "think"), architecture)
        source = prose("docs/design/architecture.md")
        for issue in (119, 120):
            self.assertIn(f"https://github.com/jon-devlapaz/tinkery/issues/{issue}", source)

    def test_think_history_has_one_home_and_compatible_anchors(self):
        historical = (REPO / "findings/README.md").read_text()
        current = (REPO / "docs/design/think.md").read_text()
        sections = {
            "## Decision-review evidence (#34)":
                "21e9620a512fcdedd1611ddd327905ab8d6639b41407492d65a38f1b51d02b6a",
            "## Evidence-note regression check (#58)":
                "8f5cda3ee02995f16e4c1f701cc62a0693c6b1ea3444e0b2cc33e0e86908f1df",
            "## Goal-history and review regression check (#64)":
                "35963ff33ae28925daf24ad1bbe2c46cd430c4749da3a2eae999ef850ba12d29",
        }
        for heading, expected in sections.items():
            self.assertEqual(1, historical.count(heading))
            body = historical.split(heading, 1)[1].split("\n## ", 1)[0].strip()
            digest = hashlib.sha256(f"{heading}\n\n{body}".encode()).hexdigest()
            self.assertEqual(expected, digest, f"Preserve the historical evidence for {heading}")
            self.assertEqual(1, current.count(heading))
            pointer = current.split(heading, 1)[1].split("\n## ", 1)[0].strip()
            anchor = re.sub(r"[^\w\- ]", "", heading[3:].lower()).replace(" ", "-")
            self.assertEqual(f"See [historical evidence](../../findings/README.md#{anchor}).", pointer)
        self.assertIn("Constraint-polarity errors remain possible", current)
        self.assertIn("judgment, not proof that choices mean the same thing", current)

    def test_ci_can_extract_one_valid_verification_block(self):
        source = (REPO / "AGENTS.md").read_text()
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
            for flag in ("--seed-session-root", "--record-root", "--link-root"):
                self.assertIn(flag, args, f"Trial must override {flag}; defaults use operator data")
                path = Path(args[args.index(flag) + 1])
                self.assertTrue(path.is_absolute(), (flag, path))
                self.assertTrue(path.is_relative_to(trial), (flag, path))
                destinations.append(path)
            self.assertEqual(len(set(destinations)), len(destinations),
                             "Sessions, think records and links need separate roots")

            capture.unlink()
            (binary / "mktemp").write_text(f"#!{sys.executable}\nraise SystemExit(1)\n")
            failed = run_trial()
            self.assertNotEqual(failed.returncode, 0)
            self.assertFalse(capture.exists(), "A failed temporary root must prevent launch")


if __name__ == "__main__":
    unittest.main()
