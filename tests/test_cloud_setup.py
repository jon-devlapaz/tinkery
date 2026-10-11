"""Bootstrap safety checks: isolated homes, fake commands, no package/network calls."""
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

REPO = Path(__file__).resolve().parents[1]
REMOTE = "https://github.com/jon-devlapaz/seed-me.git"


class CloudSetupTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="tinkery-TEST-cloud-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        (self.root / "scripts").mkdir()
        (self.root / ".github/workflows").mkdir(parents=True)
        shutil.copy2(REPO / "scripts/cloud-setup.sh", self.root / "scripts/cloud-setup.sh")
        shutil.copy2(REPO / ".github/workflows/ci.yml", self.root / ".github/workflows/ci.yml")
        binary = self.root / "bin"
        binary.mkdir()
        self.env = dict(os.environ, HOME=str(self.root / "home"),
                        CARGO_HOME=str(self.root / "cargo"), RUSTUP_HOME=str(self.root / "rustup"),
                        PATH=f"{binary}:{os.environ['PATH']}", TRACE=str(self.root / "trace"),
                        FAKE_OS="Linux", FAKE_ORIGIN=REMOTE, FAKE_STATUS="", FAKE_STATUS_EXIT="0")
        commands = {
            "uname": 'echo "$FAKE_OS"',
            "git": 'echo "git $*" >> "$TRACE"\ncase "$*" in\n'
                   '  *"remote get-url origin") echo "$FAKE_ORIGIN";;\n'
                   '  *"status --porcelain") printf "%s" "$FAKE_STATUS"; exit "$FAKE_STATUS_EXIT";;\n'
                   '  *) exit 97;;\nesac',
            "rustup": 'echo rustup >> "$TRACE"',
            "cargo": 'echo cargo >> "$TRACE"',
            "rustc": 'echo rustc >> "$TRACE"',
            "node": 'echo node >> "$TRACE"',
        }
        for command in ("curl", "wget", "apt-get", "sudo"):
            commands[command] = f'echo {command} >> "$TRACE"; exit 97'
        for name, body in commands.items():
            path = binary / name
            path.write_text("#!/bin/bash\n" + body + "\n")
            path.chmod(0o755)

    def fixture(self):
        path = self.root / ".ci-seed-me"
        (path / ".git").mkdir(parents=True)
        (path / "keep.txt").write_text("TEST operator-owned sentinel")
        return path

    def rejected(self, message):
        result = subprocess.run(["bash", str(self.root / "scripts/cloud-setup.sh")],
                                env=self.env, text=True, capture_output=True, timeout=10)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(message, result.stderr)
        self.assertFalse((self.root / ".cloud-tools").exists())
        trace = self.root / "trace"
        if trace.exists():
            self.assertTrue(all(line.startswith("git ") for line in trace.read_text().splitlines()))
        return result

    def test_non_linux_exits_before_any_side_effect(self):
        self.env["FAKE_OS"] = "Darwin"
        self.rejected("Linux only")
        self.assertFalse((self.root / "trace").exists())

    def test_existing_non_checkout_is_preserved(self):
        path = self.root / ".ci-seed-me"
        path.write_text("TEST keep me")
        self.rejected("not a Git checkout")
        self.assertEqual(path.read_text(), "TEST keep me")

    def test_unrelated_origin_is_preserved(self):
        path = self.fixture()
        self.env["FAKE_ORIGIN"] = "https://example.invalid/unrelated.git"
        self.rejected("unexpected origin")
        self.assertEqual((path / "keep.txt").read_text(), "TEST operator-owned sentinel")

    def test_dirty_checkout_is_preserved_before_tool_changes(self):
        path = self.fixture()
        self.env["FAKE_STATUS"] = " M keep.txt"
        self.rejected("is dirty")
        self.assertEqual((path / "keep.txt").read_text(), "TEST operator-owned sentinel")

    def test_status_failure_is_not_treated_as_clean(self):
        self.fixture()
        self.env["FAKE_STATUS_EXIT"] = "97"
        self.rejected("Cannot read .ci-seed-me status")

    def test_unpinned_ci_fixture_is_rejected(self):
        ci = self.root / ".github/workflows/ci.yml"
        source = ci.read_text()
        import re
        ci.write_text(re.sub(r"ref: [0-9a-f]{40}", "ref: main", source))
        self.rejected("full Seed Me commit SHA")


if __name__ == "__main__":
    unittest.main()
