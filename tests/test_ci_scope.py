"""Conservative CI routing and final-gate contracts; temporary Git roots, no network."""
import os
from pathlib import Path
import runpy
import test_launch as subprocess
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
        scope = runpy.run_path(str(ROOT / "scripts/ci-scope.py"))
        self.classify, self.setup_changed = scope["classify"], scope["setup_changed"]
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
         env=subprocess.environment())

    def commit(self):
        self.git("add", "--all")
        self.git("commit", "-qm", "Synthetic fixture")
        return self.git("rev-parse", "HEAD").strip()

    def test_known_prose_modification_is_lightweight(self):
        self.document.write_text("# Architecture\n\nUpdated prose.\n")
        self.assertTrue(self.classify(self.base, self.commit(), self.root))

    def rebase_with(self, names):
        for name in names:
            file = self.root / name
            file.parent.mkdir(parents=True, exist_ok=True)
            file.write_text("# Page\n\nExisting prose.\n")
        return self.commit()

    def test_constitution_design_and_research_prose_is_lightweight(self):
        names = ["docs/yohaku.md", "docs/design/menu.md",
                 "docs/research/shared/job-records-proposal.md"]
        base = self.rebase_with(names)
        for name in names:
            with self.subTest(path=name):
                (self.root / name).write_text("# Page\n\nUpdated prose.\n")
                self.assertTrue(self.classify(base, self.commit(), self.root))
                self.git("reset", "--hard", base)

    def test_prose_outside_the_allowlist_requires_full_checks(self):
        names = ["docs/terminal-setup.md", "docs/design/nested/page.md",
                 "docs/research/page.md", "docs/design/menu.txt"]
        base = self.rebase_with(names)
        for name in names:
            with self.subTest(path=name):
                (self.root / name).write_text("# Page\n\nUpdated prose.\n")
                self.assertFalse(self.classify(base, self.commit(), self.root))
                self.git("reset", "--hard", base)

    def test_diagram_and_sample_output_fences_are_inert(self):
        for example in ["```mermaid\nflowchart TD\n    A --> B\n```", "```text\n    Saved.\n```"]:
            with self.subTest(example=example):
                self.document.write_text("# Architecture\n\n" + example + "\n")
                self.assertTrue(self.classify(self.base, self.commit(), self.root))
                self.git("reset", "--hard", self.base)

    def test_command_fences_and_unclosed_inert_fences_require_full_checks(self):
        for example in ["```mermaid\nflowchart TD\n```\n\n```bash\ncargo test\n```",
                        "```mermaid\nflowchart TD\n    A --> B\n", "```text\nok\n````",
                        "```mermaidx\nA\n```", "~~~mermaid\nA\n~~~"]:
            with self.subTest(example=example):
                self.document.write_text("# Architecture\n\n" + example + "\n")
                self.assertFalse(self.classify(self.base, self.commit(), self.root))
                self.git("reset", "--hard", self.base)

    def test_fresh_clone_runs_only_when_setup_may_change(self):
        setup = ["AGENTS.md", "CLAUDE.md", "Cargo.toml", "Cargo.lock", "rust-toolchain.toml",
                 "scripts/cloud-setup.sh", ".github/workflows/ci.yml", "voice/package.json"]
        other = ["src/main.rs", "README.md", "docs/design/architecture.md", "tests/tend_terminal.py"]
        base = self.rebase_with(setup + other)
        for name in setup + other:
            with self.subTest(path=name):
                (self.root / name).write_text("Synthetic change\n")
                self.assertEqual(self.setup_changed(base, self.commit(), self.root), name in setup)
                self.git("reset", "--hard", base)
        (self.root / "src/main.rs").unlink()
        self.assertFalse(self.setup_changed(base, self.commit(), self.root))
        self.git("reset", "--hard", base)
        (self.root / "scripts/cloud-setup.sh").rename(self.root / "setup.sh")
        self.assertTrue(self.setup_changed(base, self.commit(), self.root))

    def test_uncertain_comparison_assumes_setup_changed(self):
        for base in ["", "0" * 40, "main", "--help"]:
            with self.subTest(base=base):
                self.assertTrue(self.setup_changed(base, self.base, self.root))
        self.assertTrue(self.setup_changed(self.base, self.base, self.root))
        with self.assertRaises(subprocess.CalledProcessError):
            self.setup_changed("f" * 40, self.base, self.root)

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
         env=subprocess.environment())
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout, "docs_only=false\nsetup=true\n")
        self.assertIn("full verification", result.stderr)

    def test_cli_git_failure_does_not_publish_a_route(self):
        result = subprocess.run(
            ["python3", str(ROOT / "scripts/ci-scope.py"), "--base", "f" * 40,
             "--head", self.base], capture_output=True, text=True, timeout=10,
         env=subprocess.environment())
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
                        SETUP="true", GITHUB_STEP_SUMMARY=os.devnull)

    def result(self, **changes):
        return subprocess.run(["bash", "-c", self.gate], env=subprocess.environment(dict(self.env, **changes)),
                              capture_output=True, text=True, timeout=10)

    def test_workflow_always_publishes_one_gate_without_path_filters(self):
        self.assertNotIn("paths:", self.workflow)
        self.assertNotIn("paths-ignore:", self.workflow)
        self.assertIn("  ready:\n    name: CI ready\n    if: always()", self.workflow)
        self.assertIn("    needs: [scope, docs, check, cold-clone]", self.workflow)
        self.assertIn("fetch-depth: 0", self.workflow)
        self.assertIn("python3 tests/test_ci_scope.py", self.workflow)
        self.assertIn("python3 tests/test_documentation.py", self.workflow)
        docs_job = self.workflow.split("\n  docs:\n", 1)[1].split("\n  check:\n", 1)[0]
        for test in ["test_documentation.py", "test_planning_docs.py", "test_job_record_docs.py"]:
            self.assertIn(f"python3 tests/{test}", docs_job)
        self.assertIn("setup: ${{ steps.classify.outputs.setup }}", self.workflow)
        self.assertIn("needs.scope.outputs.setup != 'false'", self.workflow)
        self.assertIn("cancel-in-progress: ${{ github.event_name == 'pull_request' }}", self.workflow)
        self.assertEqual(self.workflow.count("needs.scope.outputs.docs_only != 'true'"), 2)
        self.assertIn("github.event.pull_request.base.sha || github.event.before", self.workflow)

    def test_pull_requests_use_ubuntu_and_the_pr_tier_while_main_runs_everything(self):
        check = self.workflow.split("\n  check:\n", 1)[1].split("\n  cold-clone:\n", 1)[0]
        self.assertIn("""fromJSON(github.event_name == 'pull_request' && '["ubuntu-latest"]' """
                      """|| '["ubuntu-latest", "macos-latest"]')""", check)
        self.assertIn("TINKERY_TEST_TIER: ${{ github.event_name == 'pull_request' && 'pr' || 'full' }}", check)
        self.assertNotIn("TINKERY_TEST_TIER", self.workflow.split("\n  docs:\n", 1)[0])

    def test_valid_docs_and_full_routes_pass(self):
        for changes in [{}, {"EVENT": "push"},
                        {"DOCS_ONLY": "false", "DOCS_RESULT": "skipped",
                         "CHECK_RESULT": "success", "COLD_RESULT": "success"},
                        {"DOCS_ONLY": "false", "DOCS_RESULT": "skipped",
                         "CHECK_RESULT": "success", "SETUP": "false"},
                        {"DOCS_ONLY": "false", "EVENT": "push", "DOCS_RESULT": "skipped",
                         "CHECK_RESULT": "success"},
                        {"DOCS_ONLY": "false", "EVENT": "push", "DOCS_RESULT": "skipped",
                         "CHECK_RESULT": "success", "SETUP": "false"}]:
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
        full = dict(DOCS_ONLY="false", DOCS_RESULT="skipped", CHECK_RESULT="success")
        for changes in [dict(full, SETUP="false", COLD_RESULT="success"),
                        dict(full, SETUP="false", COLD_RESULT="failure"),
                        dict(full, SETUP="", COLD_RESULT="skipped"),
                        dict(full, SETUP="unknown", COLD_RESULT="success"),
                        dict(full, EVENT="push", SETUP="", COLD_RESULT="skipped")]:
            with self.subTest(changes=changes):
                self.assertNotEqual(self.result(**changes).returncode, 0)
        for changes in [{"DOCS_ONLY": ""}, {"DOCS_ONLY": "unknown"},
                        {"EVENT": "unknown"}, {"CHECK_RESULT": "success"},
                        {"COLD_RESULT": "success"}]:
            with self.subTest(changes=changes):
                self.assertNotEqual(self.result(**changes).returncode, 0)


if __name__ == "__main__":
    unittest.main()
