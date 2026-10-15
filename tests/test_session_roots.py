from pathlib import Path
import hashlib
import json
import os
import re
import subprocess
import tempfile
import unittest

BINARY = Path(__file__).resolve().parents[1] / 'target/debug/tinkery'


class SessionRootTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix='tinkery-TEST-roots-')
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.cwd = self.root / 'TEST-cwd'
        self.cwd.mkdir()
        self.env = {'PATH': os.environ['PATH'], 'TERM': 'xterm-256color',
                    'TINKERY_NODE': '/tinkery-TEST-no-node', 'PYTHONDONTWRITEBYTECODE': '1',
                    'TINK_SUBSTRATE_DIR': str(self.root / 'TEST-no-substrate')}
        self.skill = self.root / 'TEST-SKILL.md'
        self.skill.write_text('# Seed Me\n### Shape the working draft\nUnderstand intent.\n### Size gate\n')
        self.host = self.root / 'TEST-never-pi'
        self.called = self.root / 'TEST-provider-called'
        self.host.write_text('#!/bin/sh\nprintf unexpected > "' + str(self.called) + '"\nexit 99\n')
        self.host.chmod(0o700)

    def environment(self, home):
        env = dict(self.env)
        if home is not None:
            env['HOME'] = home
        return env

    def run_cli(self, args, home):
        return subprocess.run([str(BINARY), *map(str, args)], cwd=self.cwd,
                              env=self.environment(home), capture_output=True, text=True, timeout=10)

    def ledger(self, root, name='TEST-session'):
        file = root / name / 'ledger.json'
        file.parent.mkdir(parents=True)
        file.write_text(json.dumps({'status': 'active', 'goal': 'TEST root sentinel'}))
        return file

    def has_count(self, output, count):
        return re.search(r'\btend\s+' + str(count) + r'\b', output) is not None

    def live_args(self):
        return ['--shape-pi', '--model', 'test/model', '--seed-me', self.skill,
                '--pi-command', self.host]

    def test_invalid_home_never_discovers_or_creates_implicit_relative_sessions(self):
        for home in (None, '', 'TEST-relative-home'):
            with self.subTest(home=home):
                implicit = self.cwd / (home or '') / '.local/share/seed-me/sessions'
                empty = self.run_cli(['--home', '--snapshot'], home)
                self.assertEqual(0, empty.returncode, empty.stderr)
                self.assertFalse(implicit.exists())
                sentinel = self.ledger(implicit)
                before = hashlib.sha256(sentinel.read_bytes()).hexdigest()
                result = self.run_cli(['--home', '--snapshot'], home)
                self.assertEqual(0, result.returncode, result.stderr)
                self.assertFalse(self.has_count(result.stdout, 1), result.stdout)
                self.assertEqual(before, hashlib.sha256(sentinel.read_bytes()).hexdigest())
                sentinel.unlink()
                sentinel.parent.rmdir()
                implicit.rmdir()

    def test_explicit_roots_work_in_practice_without_persistence(self):
        destination = self.root / 'TEST-selected'
        sentinel = self.ledger(destination)
        before = sentinel.read_bytes()
        for home in (None, '', 'TEST-relative-home'):
            for selected in (destination, Path('../TEST-selected')):
                with self.subTest(home=home, selected=selected):
                    result = self.run_cli(['--home', '--snapshot', '--seed-session-root', selected], home)
                    self.assertEqual(0, result.returncode, result.stderr)
                    self.assertTrue(self.has_count(result.stdout, 1), result.stdout)
                    self.assertEqual(before, sentinel.read_bytes())
        self.assertFalse(self.called.exists())

    def test_live_refuses_missing_session_root_before_host_or_helper_access(self):
        for home in (None, '', 'TEST-relative-home'):
            for entry in ([], ['--home']):
                with self.subTest(home=home, entry=entry):
                    records = self.root / 'TEST-records'
                    result = self.run_cli(entry + self.live_args() + ['--record-root', records], home)
                    self.assertNotEqual(0, result.returncode)
                    self.assertIn('pass --seed-session-root PATH', result.stderr)
                    self.assertFalse(records.exists())
                    self.assertFalse(self.called.exists())
                    self.assertFalse((self.cwd / (home or '') / '.local').exists())

    def test_session_root_does_not_bypass_separate_record_root_guard(self):
        for home in (None, '', 'TEST-relative-home'):
            result = self.run_cli(self.live_args() + ['--seed-session-root', self.root / 'TEST-sessions'], home)
            self.assertNotEqual(0, result.returncode)
            self.assertIn('pass --record-root PATH', result.stderr)
            self.assertFalse(self.called.exists())

    def test_live_snapshot_remains_usable_without_a_session_root(self):
        for home in (None, '', 'TEST-relative-home'):
            result = self.run_cli(['--home', '--snapshot'] + self.live_args() +
                                  ['--record-root', self.root / 'TEST-records'], home)
            self.assertEqual(0, result.returncode, result.stderr)
            self.assertFalse(self.has_count(result.stdout, 1))
            self.assertFalse(self.called.exists())
            self.assertFalse((self.root / 'TEST-records').exists())
            self.assertFalse((self.cwd / (home or '') / '.local').exists())

    def test_absolute_home_default_and_explicit_override_are_distinct(self):
        home = self.root / 'TEST-home'
        default = home / '.local/share/seed-me/sessions'
        self.ledger(default)
        selected = self.root / 'TEST-selected'
        self.ledger(selected, 'one')
        self.ledger(selected, 'two')
        result = self.run_cli(['--home', '--snapshot'], str(home))
        self.assertEqual(0, result.returncode, result.stderr)
        self.assertTrue(self.has_count(result.stdout, 1), result.stdout)
        result = self.run_cli(['--home', '--snapshot', '--seed-session-root', selected], str(home))
        self.assertEqual(0, result.returncode, result.stderr)
        self.assertTrue(self.has_count(result.stdout, 2), result.stdout)


if __name__ == '__main__':
    unittest.main()
