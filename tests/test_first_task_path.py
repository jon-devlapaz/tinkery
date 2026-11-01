"""The first-task journey controls its tools without touching host agent installations."""
import os
from pathlib import Path
import shutil
import tempfile
import unittest
from unittest import mock

import first_task_path
import test_launch


class FirstTaskToolsTests(unittest.TestCase):
    def test_only_selected_tools_and_the_later_stand_in_are_visible(self):
        with tempfile.TemporaryDirectory(prefix='tinkery-TEST-first-task-tools-') as directory:
            root = Path(directory)
            host = root / 'TEST-host-bin'
            host.mkdir()
            for name in ('pi', 'tink'):
                program = host / name
                program.write_text('#!/bin/sh\nexit 97\n')
                program.chmod(0o755)
            originals = {p.name: p.read_bytes() for p in host.iterdir()}
            tools = root / 'TEST tools with spaces'
            tools.mkdir()
            inherited = test_launch.environment(home=root / 'HOME')
            inherited.update(PATH=f"{host}:{os.environ['PATH']}", TINK_SDLC_PI=str(host / 'pi'))
            before = dict(inherited)
            env = first_task_path.isolated_tools(tools, inherited)
            self.assertEqual(inherited, before)
            self.assertEqual(env['HOME'], inherited['HOME'])
            self.assertEqual(env['PATH'], str(tools))
            self.assertEqual(env['TINK_SDLC_PI'], str(tools / 'pi'))
            self.assertIsNone(shutil.which('pi', path=env['PATH']))
            self.assertIsNone(shutil.which('tink', path=env['PATH']))
            self.assertEqual(first_task_path.shell('command -v pi || true; command -v tink || true',
                                                   env, root), '')
            for name in ('bash', 'git', 'mkdir', 'chmod', 'python3'):
                self.assertIsNotNone(shutil.which(name, path=env['PATH']))
            self.assertEqual(first_task_path.shell("python3 -c 'print(123)'", env, root), '123\n')
            pi = tools / 'pi'
            pi.write_text('#!/bin/sh\nprintf "TEST stand-in\\n"\n')
            pi.chmod(0o755)
            self.assertEqual(first_task_path.shell('pi', env, root), 'TEST stand-in\n')
            self.assertEqual({p.name: p.read_bytes() for p in host.iterdir()}, originals)

    def test_missing_guide_tool_is_an_explicit_error(self):
        with tempfile.TemporaryDirectory(prefix='tinkery-TEST-first-task-tools-') as directory:
            with mock.patch.object(first_task_path.shutil, 'which', return_value=None):
                with self.assertRaisesRegex(RuntimeError, 'requires bash on PATH'):
                    first_task_path.isolated_tools(Path(directory), {'PATH': '/TEST-no-tools'})


if __name__ == '__main__':
    unittest.main()
