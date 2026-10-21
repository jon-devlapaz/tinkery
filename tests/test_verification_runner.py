"""Project verification policy and exact-block extraction; no tools, provider or audio calls."""
from pathlib import Path
import json
import runpy
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]


class FullCheckTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='tinkery-TEST-full-check-contract-')
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        api = runpy.run_path(str(ROOT / 'scripts/full-check.py'), run_name='test_extractor')
        self.main = api['main']
        self.block = api['block']
        self.block.__globals__['ROOT'] = self.root

    def source(self, source):
        (self.root / 'AGENTS.md').write_text(source)

    def test_exact_bytes_without_substituting_commands(self):
        commands = 'export TINKERY_NODE=/tinkery-TEST-no-node\necho "exact  spacing"\n'
        self.source('before\n<!-- full-check:start -->\n```bash\n' + commands +
                    '```\n<!-- full-check:end -->\nafter\n')
        self.assertEqual(self.block(), commands)

    def test_missing_duplicate_empty_or_multiple_blocks_refuse(self):
        for source in ['', '<!-- full-check:start -->',
                       '<!-- full-check:start --><!-- full-check:start --><!-- full-check:end -->',
                       '<!-- full-check:start -->\n```bash\n\n```\n<!-- full-check:end -->',
                       '<!-- full-check:start -->\n```bash\necho one\n```\n```bash\necho two\n```\n<!-- full-check:end -->']:
            with self.subTest(source=source):
                self.source(source)
                with self.assertRaises(ValueError):
                    self.block()

    def test_temporary_policy_invokes_exact_runner_with_isolated_home(self):
        policy = self.root / 'verification.json'
        policy.write_text(json.dumps({
            'require_tink': False,
            'checks': [{'argv': ['python3', 'scripts/full-check.py'], 'timeout_seconds': 1800}],
        }))
        config = json.loads(policy.read_text())
        self.assertIs(config['require_tink'], False)
        check = config['checks'][0]
        self.assertEqual(check['argv'], ['python3', 'scripts/full-check.py'])
        self.assertGreater(check['timeout_seconds'], 0)
        commands = 'echo "exact commands"\n'
        self.source('<!-- full-check:start -->\n```bash\n' + commands +
                    '```\n<!-- full-check:end -->\n')
        calls = []

        def run(argv, **kwargs):
            calls.append(argv)
            if argv[0] == 'bash':
                self.assertEqual(argv[:3], ['bash', '-euo', 'pipefail'])
                self.assertEqual(Path(argv[3]).read_text(), commands)
                self.assertEqual(kwargs['cwd'], self.root)
                home = Path(kwargs['env']['HOME'])
                self.assertTrue(home.is_dir())
                self.assertIn('tinkery-TEST-', str(home))
                self.assertEqual(kwargs['env']['TINKERY_NODE'], '/tinkery-TEST-no-node')
                self.assertTrue(kwargs['check'])

        with patch('subprocess.run', side_effect=run):
            self.main()
        self.assertEqual(len(calls), 1)
        self.assertEqual(calls[0][0], 'bash')


if __name__ == '__main__':
    unittest.main()
