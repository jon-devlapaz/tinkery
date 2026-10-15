"""Conservative CI routing and final-gate contracts; temporary Git roots, no network."""
import os
from pathlib import Path
import runpy
import subprocess
import tempfile
import textwrap
import unittest

ROOT = Path(__file__).resolve().parents[1]
FULL_PATHS = ["src/main.rs", "AGENTS.md", "README.md", "Cargo.lock",
              ".github/workflows/ci.yml", "scripts/cloud-setup.sh",
              "docs/new-page.md", "docs/design/architecture.md\nextra"]


class ScopeTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="tinkery-TEST-ci-scope-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.classify = runpy.run_path(str(ROOT / "scripts/ci-scope.py"))["classify"]
        self.git("init", "-q")
        self.document = self.root / "docs/design/architecture.md"
        self.document.parent.mkdir(parents=True)
        self.document.write_text("# Architecture\n\nExisting prose.\n")
        for name in FULL_PATHS:
            file = self.root / name
            file.parent.mkdir(parents=True, exist_ok=True)
            file.write_text("Existing fixture\n")
        self.commit()
        self.base = self.git("rev-parse", "HEAD").strip()

    def git(self, *args):
        return subprocess.check_output(
            ["git", "-c", "user.name=CI fixture", "-c", "user.email=ci@example.invalid",
             "-c", "commit.gpgsign=false", *args], cwd=self.root, text=True,
            stderr=subprocess.PIPE,
        )

    def commit(self):
        self.git("add", "--all")
        self.git("commit", "-qm", "Synthetic fixture")
        return self.git("rev-parse", "HEAD").strip()

    def test_known_prose_modification_is_lightweight(self):
        self.document.write_text("# Architecture\n\nUpdated prose.\n")
        self.assertTrue(self.classify(self.base, self.commit(), self.root))

    def test_mixed_unknown_and_setup_paths_require_full_checks(self):
        for name in FULL_PATHS:
            with self.subTest(path=name):
                file = self.root / name
                file.parent.mkdir(parents=True, exist_ok=True)
                file.write_text("Synthetic change\n")
                self.document.write_text("# Architecture\n\nUpdated prose.\n")
                self.assertFalse(self.classify(self.base, self.commit(), self.root))
                self.git("reset", "--hard", self.base)

    def test_executable_examples_require_full_checks(self):
        for example in ["```bash\necho hello\n```", "~~~sh\necho hello\n~~~",
                        "    echo hello", "\techo hello",
                        "Run `cargo test --locked`.", "Run `python3 scripts/full-check.py`."]:
            with self.subTest(example=example):
                self.document.write_text("# Architecture\n\n" + example + "\n")
                self.assertFalse(self.classify(self.base, self.commit(), self.root))
                self.git("reset", "--hard", self.base)

    def test_removing_an_executable_example_still_requires_full_checks(self):
        self.document.write_text("```bash\ncargo test\n```\n")
        base = self.commit()
        self.document.write_text("# Architecture\n\nOnly prose now.\n")
        self.assertFalse(self.classify(base, self.commit(), self.root))

    def test_executable_file_mode_requires_full_checks(self):
        self.document.chmod(0o755)
        self.assertFalse(self.classify(self.base, self.commit(), self.root))

    def test_cli_reports_a_single_fail_closed_output(self):
        result = subprocess.run(
            ["python3", str(ROOT / "scripts/ci-scope.py"), "--base", "", "--head", "0" * 40],
            capture_output=True, text=True, timeout=10,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout, "docs_only=false\n")
        self.assertIn("full verification", result.stderr)

    def test_cli_git_failure_does_not_publish_a_route(self):
        result = subprocess.run(
            ["python3", str(ROOT / "scripts/ci-scope.py"), "--base", "f" * 40,
             "--head", self.base], capture_output=True, text=True, timeout=10,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(result.stdout, "")
        self.assertIn("Cannot classify committed changes", result.stderr)

    def test_deleted_renamed_and_symlinked_documents_require_full_checks(self):
        self.document.unlink()
        self.assertFalse(self.classify(self.base, self.commit(), self.root))
        self.git("reset", "--hard", self.base)
        self.document.rename(self.document.with_name("renamed.md"))
        self.assertFalse(self.classify(self.base, self.commit(), self.root))
        self.git("reset", "--hard", self.base)
        self.document.unlink()
        self.document.symlink_to("../../README.md")
        self.assertFalse(self.classify(self.base, self.commit(), self.root))

    def test_empty_missing_or_uncertain_comparison_never_selects_docs(self):
        for base in [self.base, "", "0" * 40, "main", "--help"]:
            with self.subTest(base=base):
                self.assertFalse(self.classify(base, self.base, self.root))
        with self.assertRaises(subprocess.CalledProcessError):
            self.classify("f" * 40, self.base, self.root)

    def test_comparison_uses_committed_content_not_worktree_edits(self):
        self.document.write_text("# Architecture\n\nUpdated prose.\n")
        head = self.commit()
        self.document.write_text("```bash\ncargo test\n```\n")
        self.assertTrue(self.classify(self.base, head, self.root))


class WorkflowGateTests(unittest.TestCase):
    def setUp(self):
        self.workflow = (ROOT / ".github/workflows/ci.yml").read_text()
        self.gate = textwrap.dedent(self.workflow.split("      - name: Validate required results\n", 1)[1]
                                   .split("        run: |\n", 1)[1])
        self.env = dict(os.environ, SCOPE_RESULT="success", DOCS_ONLY="true",
                        EVENT="pull_request", DOCS_RESULT="success",
                        CHECK_RESULT="skipped", COLD_RESULT="skipped",
                        GITHUB_STEP_SUMMARY=os.devnull)

    def result(self, **changes):
        return subprocess.run(["bash", "-c", self.gate], env=dict(self.env, **changes),
                              capture_output=True, text=True, timeout=10)

    def test_workflow_always_publishes_one_gate_without_path_filters(self):
        self.assertNotIn("paths:", self.workflow)
        self.assertNotIn("paths-ignore:", self.workflow)
        self.assertIn("  ready:\n    name: CI ready\n    if: always()", self.workflow)
        self.assertIn("    needs: [scope, docs, check, cold-clone]", self.workflow)
        self.assertIn("fetch-depth: 0", self.workflow)
        self.assertIn("python3 tests/test_ci_scope.py", self.workflow)
        self.assertIn("python3 tests/test_documentation.py", self.workflow)
        self.assertEqual(self.workflow.count("needs.scope.outputs.docs_only != 'true'"), 2)
        self.assertIn("github.event.pull_request.base.sha || github.event.before", self.workflow)

    def test_valid_docs_and_full_routes_pass(self):
        for changes in [{}, {"EVENT": "push"},
                        {"DOCS_ONLY": "false", "DOCS_RESULT": "skipped",
                         "CHECK_RESULT": "success", "COLD_RESULT": "success"},
                        {"DOCS_ONLY": "false", "EVENT": "push", "DOCS_RESULT": "skipped",
                         "CHECK_RESULT": "success"}]:
            with self.subTest(changes=changes):
                result = self.result(**changes)
                self.assertEqual(result.returncode, 0, result.stderr)

    def test_failures_cancellations_and_unexpected_skips_cannot_pass(self):
        for key in ["SCOPE_RESULT", "DOCS_RESULT"]:
            for status in ["failure", "cancelled", "skipped", ""]:
                with self.subTest(key=key, status=status):
                    self.assertNotEqual(self.result(**{key: status}).returncode, 0)
        for key in ["CHECK_RESULT", "COLD_RESULT"]:
            for status in ["failure", "cancelled", "skipped", ""]:
                with self.subTest(key=key, status=status):
                    changes = dict(DOCS_ONLY="false", DOCS_RESULT="skipped",
                                   CHECK_RESULT="success", COLD_RESULT="success")
                    changes[key] = status
                    self.assertNotEqual(self.result(**changes).returncode, 0)
        for changes in [{"DOCS_ONLY": ""}, {"DOCS_ONLY": "unknown"},
                        {"EVENT": "unknown"}, {"CHECK_RESULT": "success"},
                        {"COLD_RESULT": "success"}]:
            with self.subTest(changes=changes):
                self.assertNotEqual(self.result(**changes).returncode, 0)


if __name__ == "__main__":
    unittest.main()
