"""Executable documentation contracts; fake commands, no providers or operator data."""
import json
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
import unittest

REPO = Path(__file__).resolve().parents[1]


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
