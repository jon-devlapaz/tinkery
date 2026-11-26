"""scripts/install.py installs a launcher that runs under any HOME, and the check names it as the fix."""
import importlib.util
from pathlib import Path
import sys
import tempfile
import unittest

import test_launch as subprocess

REPO = Path(__file__).resolve().parents[1]
INSTALL = REPO / 'scripts/install.py'
CHECK = REPO / 'scripts/first-task-check.py'
# The launcher the operator wrote by hand before install.py existed: it finds the binary through HOME.
HAND_WRITTEN = '#!/bin/sh\nexec "$HOME/.local/share/tinkery/bin/tinkery" --home "$@"\n'


def module(path, name):
    spec = importlib.util.spec_from_file_location(name, path)
    loaded = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(loaded)
    return loaded


class InstallTests(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory(prefix='tinkery-TEST-install-')
        self.addCleanup(directory.cleanup)
        self.root = Path(directory.name).resolve()
        self.home = self.root / 'HOME'
        self.home.mkdir()
        self.env = subprocess.environment(home=self.home)
        self.env['PATH'] = f"{self.home / '.local/bin'}:{self.env['PATH']}"
        self.launcher = self.home / '.local/bin/tinkery'
        self.share = self.home / '.local/share/tinkery/bin'

    def build(self, name, snapshot_exit=0):
        """A TEST stand-in for a built binary; `--snapshot` exits with `snapshot_exit`."""
        binary = self.root / name
        binary.write_text(f'#!/bin/sh\necho "TEST {name} $*"\nexit {snapshot_exit}\n')
        binary.chmod(0o755)
        return binary

    def install(self, binary):
        return subprocess.run([sys.executable, str(INSTALL), '--binary', str(binary)], env=self.env,
                              capture_output=True, text=True, timeout=60)

    def check(self):
        result = subprocess.run([sys.executable, str(CHECK), '--tinkery', str(self.launcher)], env=self.env,
                                capture_output=True, text=True, timeout=60)
        return result.stdout

    def test_launcher_runs_under_another_home(self):
        result = self.install(self.build('TEST-build-1'))
        self.assertEqual(result.returncode, 0, result.stderr)
        text = self.launcher.read_text()
        self.assertIn(str(self.share / 'tinkery'), text)
        self.assertNotIn('$HOME', text)
        other = self.root / 'TEST-other-home'
        other.mkdir()
        ran = subprocess.run([str(self.launcher), '--snapshot'], env=subprocess.environment(home=other),
                             capture_output=True, text=True, timeout=30)
        self.assertEqual(ran.stdout, 'TEST TEST-build-1 --home --snapshot\n')
        self.assertIn('ready: Tinkery runs', self.check())

    def test_check_names_install_for_a_hand_written_launcher(self):
        self.share.mkdir(parents=True)
        (self.share / 'tinkery').write_bytes(self.build('TEST-old-build').read_bytes())
        (self.share / 'tinkery').chmod(0o755)
        self.launcher.parent.mkdir(parents=True)
        self.launcher.write_text(HAND_WRITTEN)
        self.launcher.chmod(0o755)
        out = self.check()
        self.assertIn('not yet: Tinkery runs', out)
        self.assertIn(f'Next: {self.launcher} is a launcher that scripts/install.py did not write', out)
        self.assertIn(f'run python3 {INSTALL} to replace it', out)
        result = self.install(self.build('TEST-build-2'))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual((self.share / 'launcher.previous').read_text(), HAND_WRITTEN)
        self.assertEqual((self.share / 'tinkery.previous').read_bytes(),
                         (self.root / 'TEST-old-build').read_bytes())
        self.assertIn('ready: Tinkery runs', self.check())

    def test_rerun_keeps_the_build_it_replaces(self):
        self.assertEqual(self.install(self.build('TEST-build-1')).returncode, 0)
        self.assertEqual(self.install(self.build('TEST-build-1')).returncode, 0)
        self.assertFalse((self.share / 'tinkery.previous').exists())
        result = self.install(self.build('TEST-build-2'))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn('TEST-build-2', (self.share / 'tinkery').read_text())
        self.assertIn('TEST-build-1', (self.share / 'tinkery.previous').read_text())
        self.assertFalse((self.share / 'launcher.previous').exists())

    def test_broken_build_is_refused_and_nothing_is_written(self):
        result = self.install(self.build('TEST-broken', snapshot_exit=3))
        self.assertEqual(result.returncode, 1)
        self.assertIn('install refused', result.stderr)
        self.assertFalse((self.home / '.local').exists())

    def test_check_knows_the_installer_marker(self):
        self.assertEqual(module(INSTALL, 'tinkery_install').MARKER,
                         module(CHECK, 'tinkery_first_task_check').LAUNCHER_MARKER)


if __name__ == '__main__':
    unittest.main()
