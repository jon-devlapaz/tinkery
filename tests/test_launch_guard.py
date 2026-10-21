"""Launch safety and a fail-closed source gate; never starts Tinkery or any subprocess."""
import ast
import os
from pathlib import Path
import pwd
import runpy
import sys
import tempfile
import unittest
from unittest import mock

import test_launch

ROOT = Path(__file__).resolve().parent
PROCESS_APIS = {'Popen', 'run', 'check_output'}
RAW_MODULES = {'subprocess', '_posixsubprocess', 'multiprocessing', 'pty'}


def bypasses(source):
    """All test process creation goes through the guard, even for dynamically built argv."""
    tree = ast.parse(source)
    problems = []
    guarded_modules, guarded_functions = set(), set()
    os_modules, os_functions = {'os'}, set()
    for node in ast.walk(tree):
        if isinstance(node, ast.Import):
            for alias in node.names:
                module = alias.name.split('.')[0]
                if module in RAW_MODULES:
                    problems.append((node.lineno, f'raw process module {alias.name}'))
                if alias.name == 'test_launch':
                    guarded_modules.add(alias.asname or alias.name)
                if alias.name == 'os':
                    os_modules.add(alias.asname or alias.name)
        elif isinstance(node, ast.ImportFrom):
            if (node.module or '').split('.')[0] in RAW_MODULES:
                problems.append((node.lineno, f'raw process import {node.module}'))
            if node.module == 'test_launch':
                guarded_functions.update(alias.asname or alias.name for alias in node.names
                                         if alias.name in PROCESS_APIS)
            if node.module == 'os':
                os_functions.update(alias.asname or alias.name for alias in node.names
                                    if alias.name in {'system', 'popen'}
                                    or alias.name.startswith(('exec', 'spawn', 'posix_spawn')))
    for node in ast.walk(tree):
        if not isinstance(node, ast.Call):
            continue
        function = node.func
        guarded = isinstance(function, ast.Name) and function.id in guarded_functions
        if isinstance(function, ast.Attribute) and isinstance(function.value, ast.Name):
            module, name = function.value.id, function.attr
            guarded |= module in guarded_modules and name in PROCESS_APIS
            if module in os_modules and (name in {'system', 'popen'}
                                        or name.startswith(('exec', 'spawn', 'posix_spawn'))):
                problems.append((node.lineno, 'unguarded OS process launch'))
            if name.startswith('create_subprocess_'):
                problems.append((node.lineno, 'unguarded asynchronous process launch'))
        if isinstance(function, ast.Name) and function.id in os_functions:
            problems.append((node.lineno, 'unguarded imported OS process launch'))
        if guarded and not any(keyword.arg == 'env' for keyword in node.keywords):
            problems.append((node.lineno, 'guarded launches must select HOME explicitly'))
        if (isinstance(function, ast.Name) and function.id == '__import__'
                or isinstance(function, ast.Attribute) and function.attr == 'import_module'):
            if node.args and isinstance(node.args[0], ast.Constant) and node.args[0].value in RAW_MODULES:
                problems.append((node.lineno, 'dynamic raw process import'))
    return problems


class LaunchGuardTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix='tinkery-TEST-guard-')
        self.addCleanup(self.temporary.cleanup)
        self.home = Path(self.temporary.name).resolve()

    def test_every_python_test_uses_the_shared_process_boundary(self):
        problems = []
        for path in sorted(ROOT.glob('*.py')):
            if path.name == 'test_launch.py':
                continue
            problems.extend(f'{path.name}:{line}: {why}'
                            for line, why in bypasses(path.read_text()))
        self.assertEqual(problems, [], 'Route launches through test_launch with an explicit TEST HOME')

    def test_gate_rejects_direct_alias_import_dynamic_argv_and_os_bypasses(self):
        for source in [
            'import subprocess\nsubprocess.Popen([binary])',
            'import subprocess as sp\nsp.run(args)',
            'from subprocess import Popen as spawn\nspawn(argv)',
            'import os as system\nsystem.execv(binary, argv)',
            'from os import posix_spawn as launch\nlaunch(binary, argv, env)',
            '__import__("subprocess").run(argv)',
            'import importlib\nimportlib.import_module("subprocess").run(argv)',
            'import test_launch as sp\nsp.Popen(argv)',
        ]:
            with self.subTest(source=source):
                self.assertTrue(bypasses(source))
        self.assertEqual(bypasses('import test_launch as sp\nsp.Popen(argv, env=env)'), [])

    def test_real_home_is_from_password_database_not_the_home_environment(self):
        real_home = pwd.getpwuid(os.getuid()).pw_dir
        with mock.patch.dict(os.environ, HOME=str(self.home)):
            for api in PROCESS_APIS:
                with self.subTest(api=api), mock.patch(f'test_launch.subprocess.{api}') as start:
                    with self.assertRaisesRegex(test_launch.UnsafeTestHome, 'password-database'):
                        getattr(test_launch, api)(['TEST-never-start'], env={'HOME': real_home})
                    start.assert_not_called()

    def test_alias_to_real_home_and_missing_relative_or_unmarked_homes_refuse(self):
        alias = self.home / 'TEST-real-home-alias'
        alias.symlink_to(pwd.getpwuid(os.getuid()).pw_dir, target_is_directory=True)
        for env in [None, {}, {'HOME': ''}, {'HOME': 'TEST-relative'},
                    {'HOME': str(self.home / 'TEST-missing')}, {'HOME': str(alias)},
                    {'HOME': str(self.home.parent)}]:
            with self.subTest(env=env), mock.patch('test_launch.subprocess.Popen') as start:
                with self.assertRaises(test_launch.UnsafeTestHome):
                    test_launch.Popen(['TEST-never-start'], env=env)
                start.assert_not_called()

    def test_safe_explicit_environment_reaches_each_api_without_mutating_the_caller(self):
        env = {'HOME': str(self.home), 'TEST_SENTINEL': 'unchanged'}
        for api in PROCESS_APIS:
            with self.subTest(api=api), mock.patch(f'test_launch.subprocess.{api}') as start:
                getattr(test_launch, api)(['TEST-never-start'], env=env)
                self.assertEqual(start.call_args.kwargs['env'], env)
                self.assertIsNot(start.call_args.kwargs['env'], env)
        self.assertEqual(env, {'HOME': str(self.home), 'TEST_SENTINEL': 'unchanged'})

    def test_rust_cli_adapter_uses_the_same_guard_before_starting_a_binary(self):
        with mock.patch.dict(os.environ, HOME=pwd.getpwuid(os.getuid()).pw_dir), \
             mock.patch.object(sys, 'argv', ['test_launch.py', 'TEST-never-start']), \
             mock.patch('subprocess.run') as start:
            with self.assertRaisesRegex(ValueError, 'password-database'):
                runpy.run_path(str(ROOT / 'test_launch.py'), run_name='__main__')
            start.assert_not_called()
        rust = (ROOT / 'seed_cli.rs').read_text()
        self.assertIn('/tests/test_launch.py', rust)
        self.assertIn('.env("HOME", home.path())', rust)
        self.assertNotIn('Command::new(env!("CARGO_BIN_EXE_tinkery"))', rust)

    def test_factory_selects_an_existing_test_home_and_never_changes_process_home(self):
        before = os.environ.get('HOME')
        env = test_launch.environment()
        self.assertTrue(Path(env['HOME']).is_dir())
        self.assertIn('TEST', env['HOME'])
        self.assertNotEqual(Path(env['HOME']).resolve(), Path(pwd.getpwuid(os.getuid()).pw_dir).resolve())
        self.assertEqual(os.environ.get('HOME'), before)
        selected = test_launch.environment({'TEST_SENTINEL': 'kept'}, home=self.home / 'TEST-child')
        self.assertTrue(Path(selected['HOME']).is_dir())
        self.assertEqual(selected['TEST_SENTINEL'], 'kept')
        repository_tmp = ROOT.parent / 'target/tmp'
        repository_tmp.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(prefix='tinkery-TEST-guard-', dir=repository_tmp) as directory:
            selected = test_launch.environment(home=Path(directory) / 'TEST-home')
            self.assertTrue(Path(selected['HOME']).is_dir())


if __name__ == '__main__':
    unittest.main()
