"""The workspace wrapper preserves failures and refuses writes to default application folders."""
import json
from pathlib import Path
import shutil
import sys
import tempfile
import unittest

import test_launch as subprocess

ROOT = Path(__file__).resolve().parents[1]


class RustTestHomes(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory(prefix='tinkery-TEST-rust-wrapper-')
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name).resolve()
        for name in ('scripts', 'tests', 'bin', 'HOME'):
            (self.root / name).mkdir()
        for name in ('scripts/rust-tests.py', 'tests/test_launch.py'):
            shutil.copyfile(ROOT / name, self.root / name)
        cargo = self.root / 'bin/cargo'
        cargo.write_text(f'''#!{sys.executable}
import json, os, sys
from pathlib import Path
home = Path(os.environ['HOME'])
with (Path.cwd() / 'TEST-call.json').open('x') as trace:
    json.dump(dict(argv=sys.argv[1:], home=str(home), cwd=str(Path.cwd()),
                   node=os.environ['TINKERY_NODE'], home_exists=home.is_dir()), trace)
print('TEST workspace output')
if application := os.environ.get('TEST_APPLICATION'):
    (home / '.local/share' / application).mkdir(parents=True)
raise SystemExit(int(os.environ.get('TEST_CARGO_EXIT', '0')))
''')
        cargo.chmod(0o755)
        self.env = subprocess.environment(home=self.root / 'HOME')
        self.env.update(PATH=str(self.root / 'bin'), TINKERY_NODE='/tinkery-TEST-no-node')

    def run_wrapper(self):
        (self.root / 'TEST-call.json').unlink(missing_ok=True)
        return subprocess.run([sys.executable, 'scripts/rust-tests.py'], cwd=self.root,
                              env=self.env, capture_output=True, text=True, timeout=10)

    def test_workspace_runs_once_in_its_own_home_and_keeps_output(self):
        result = self.run_wrapper()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn('TEST workspace output', result.stdout)
        call = json.loads((self.root / 'TEST-call.json').read_text())
        self.assertEqual(call['argv'], ['+stable', 'test', '--workspace', '--locked'])
        self.assertEqual(call['cwd'], str(self.root))
        self.assertTrue(call['home_exists'])
        self.assertNotEqual(call['home'], self.env['HOME'])
        self.assertIn('tinkery-TEST-rust-workspace-', call['home'])
        self.assertEqual(call['node'], '/tinkery-TEST-no-node')
        self.assertFalse(Path(call['home']).exists(), 'wrapper left its temporary home behind')

    def test_workspace_failure_stays_nonzero(self):
        self.env['TEST_CARGO_EXIT'] = '7'
        result = self.run_wrapper()
        self.assertEqual(result.returncode, 7, result.stdout + result.stderr)
        self.assertIn('TEST workspace output', result.stdout)

    def test_default_application_storage_refuses(self):
        for application in ('tinkery', 'seed-me'):
            with self.subTest(application=application):
                self.env['TEST_APPLICATION'] = application
                result = self.run_wrapper()
                self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
                self.assertIn('Rust tests wrote to default application data:', result.stderr)
                self.assertIn('/.local/share/' + application, result.stderr)


if __name__ == '__main__':
    unittest.main()
