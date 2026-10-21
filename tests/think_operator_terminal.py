#!/usr/bin/env python3
"""Launch declarations are explicit, visible and never inferred from terminal input."""
import os
from pathlib import Path
import test_launch as subprocess
import tempfile
import unittest

BINARY = Path(__file__).resolve().parents[1] / "target/debug/tinkery"


class OperatorDeclaration(unittest.TestCase):
    def run_cli(self, *args):
        with tempfile.TemporaryDirectory(prefix="tinkery-TEST-operator-") as root:
            env = dict(os.environ, HOME=root, NO_COLOR="1")
            return subprocess.run(
                [str(BINARY), *args], env=subprocess.environment(env, home=(env)['HOME']), text=True,
                capture_output=True, timeout=10, check=False,
            )

    def test_header_discloses_each_launch_declaration(self):
        for args, expected in [
            ([], "not declared"),
            (["--operator", "human"], "declared: human"),
            (["--operator", "agent"], "declared: agent"),
        ]:
            with self.subTest(args=args):
                result = self.run_cli("--snapshot", *args)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertIn(expected + " (not verified)", result.stdout.splitlines()[1])
                self.assertNotIn("operator typed confirm", result.stdout)

    def test_invalid_missing_and_repeated_declarations_refuse(self):
        for args in [
            ["--operator"], ["--operator", "operator"],
            ["--operator", ""], ["--operator", "HUMAN"],
            ["--operator", "agent", "--operator", "human"],
        ]:
            with self.subTest(args=args):
                result = self.run_cli(*args)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("--operator", result.stderr)

    def test_help_names_the_declaration_not_identity(self):
        result = self.run_cli("--help")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("--operator human|agent", result.stdout)
        self.assertIn("not verified identity", result.stdout)


if __name__ == "__main__":
    unittest.main()
