"""Rust unit tests keep durable fixtures out of the default application folders."""
from pathlib import Path
import tempfile
import unittest

import test_launch as subprocess

ROOT = Path(__file__).resolve().parents[1]


class RustTestHomes(unittest.TestCase):
    def test_unit_tests_use_explicit_temporary_destinations(self):
        with tempfile.TemporaryDirectory(prefix='tinkery-TEST-rust-unit-home-') as directory:
            home = Path(directory)
            result = subprocess.run(
                ['cargo', '+stable', 'test', '--locked', '--lib'], cwd=ROOT,
                env=subprocess.environment(home=home), stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT, text=True, timeout=180,
            )
            self.assertEqual(result.returncode, 0, result.stdout)
            for application in ('tinkery', 'seed-me'):
                root = home / '.local/share' / application
                self.assertFalse(root.exists(), f'Rust tests wrote to default application data: {root}')


if __name__ == '__main__':
    unittest.main()
