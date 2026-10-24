"""Bootstrap safety with fake commands and TEST homes; no packages or network calls."""
import os
from pathlib import Path
import shutil
import test_launch as subprocess
import tempfile
import unittest

REPO = Path(__file__).resolve().parents[1]


class CloudSetupTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='tinkery-TEST-cloud-')
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        (self.root / 'scripts').mkdir()
        shutil.copy2(REPO / 'scripts/cloud-setup.sh', self.root / 'scripts/cloud-setup.sh')
        binary = self.root / 'bin'
        binary.mkdir()
        self.env = dict(os.environ, HOME=str(self.root / 'home'),
                        CARGO_HOME=str(self.root / 'cargo'), RUSTUP_HOME=str(self.root / 'rustup'),
                        PATH=f"{binary}:{os.environ['PATH']}", TRACE=str(self.root / 'trace'),
                        FAKE_OS='Linux')
        commands = {'uname': 'echo "$FAKE_OS"'}
        for command in ('rustup', 'cargo', 'rustc', 'node', 'curl', 'python3', 'cc', 'make', 'ar', 'tar', 'xz', 'sha256sum'):
            commands[command] = f'echo {command} >> "$TRACE"'
        for command in ('git', 'wget', 'apt-get', 'sudo'):
            commands[command] = f'echo {command} >> "$TRACE"; exit 97'
        # Setup pipes the checksum line into sha256sum; a fake that exits unread races the writer
        # into SIGPIPE (exit 141) under pipefail, so it drains its input like the real one.
        commands['sha256sum'] = 'cat >/dev/null; echo sha256sum >> "$TRACE"'
        for name, body in commands.items():
            path = binary / name
            path.write_text('#!/bin/bash\n' + body + '\n')
            path.chmod(0o755)

    def run_setup(self):
        return subprocess.run(['bash', str(self.root / 'scripts/cloud-setup.sh')],
                              env=subprocess.environment(self.env, home=(self.env)['HOME']), text=True, capture_output=True, timeout=10)

    def test_non_linux_exits_before_any_side_effect(self):
        self.env['FAKE_OS'] = 'Darwin'
        result = self.run_setup()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('Linux only', result.stderr)
        self.assertFalse((self.root / 'trace').exists())
        self.assertFalse((self.root / '.cloud-tools').exists())

    def test_setup_is_repeatable_without_any_fixture_or_git_access(self):
        sentinel = self.root / 'TEST-unrelated-checkout'
        sentinel.mkdir()
        (sentinel / 'keep').write_text('TEST unchanged')
        for _ in range(2):
            result = self.run_setup()
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn('Setup complete', result.stdout)
            self.assertIn('export TINKERY_NODE=/tinkery-TEST-no-node', result.stdout)
        self.assertEqual((sentinel / 'keep').read_text(), 'TEST unchanged')
        self.assertNotIn('git', (self.root / 'trace').read_text().splitlines())

    def fake(self, name, body):
        path = self.root / 'bin' / name
        path.write_text('#!/bin/bash\n' + body + '\n')
        path.chmod(0o755)

    def test_node_older_than_22_gets_a_local_node_and_newer_is_kept(self):
        self.fake('uname', '[[ $1 == -m ]] && echo x86_64 || echo "$FAKE_OS"')
        self.fake('curl', 'echo "curl $*" >> "$TRACE"; out=${@: -1}; '
                  'case $out in *SHASUMS256.txt) echo "TEST  node-v24.1.0-linux-x64.tar.xz" > "$out";; '
                  '*) : > "$out";; esac')
        for version, installs in (('v20.11.0', True), ('v22.0.0', False)):
            self.fake('node', f'echo node >> "$TRACE"; echo {version}')
            (self.root / 'trace').unlink(missing_ok=True)
            result = self.run_setup()
            self.assertEqual(result.returncode, 0, result.stderr)
            fetched = 'nodejs.org' in (self.root / 'trace').read_text()
            self.assertEqual(fetched, installs, version)


if __name__ == '__main__':
    unittest.main()
