"""Replay the pinned helper's own seed suites into byte-for-byte Rust differential cases.

The fixture is never edited. Only its clock and viewer URL read are replaced; the
viewer files are outside this slice. All durable seed/ledger files remain literal.
"""
import contextlib
from datetime import datetime, timezone
import importlib
import io
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from harvest import helper, STAMP

sys.dont_write_bytecode = True
fixture = helper().resolve().parents[3]
sys.path.insert(0, str(fixture / 'tests'))
base = importlib.import_module('test_seed_me_strict_format')
session = importlib.import_module('test_seed_me_session').session
session.now = lambda: STAMP


class Clock(datetime):
    @classmethod
    def now(cls, tz=None):
        return cls(2026, 10, 19, tzinfo=timezone.utc)


session.datetime = Clock
# Status deliberately must not connect to even a local recorded viewer URL.
session.viewer_state = lambda directory: 'not started'
FILES = ('ledger.json', 'seed-contract.md', 'seed-checks.json', 'seed-confirmation.json', '.writer.lock')
cases = []
current_test = ''


def files(directory):
    return {name: (directory / name).read_bytes().hex() for name in FILES if (directory / name).is_file()}


def run(*args):
    args = [str(a) for a in args]
    command = args[0]
    directory = None
    op = None
    if command in ('status', 'end'):
        directory, op = Path(args[1]), command
    elif command == 'seed':
        if args[1] == 'checks':
            op, directory = 'checks-' + args[2], Path(args[3])
        else:
            op, directory = args[1], Path(args[2])
    before = files(directory) if directory else None
    records = Path(args[4]).read_text() if op == 'checks-set' else None
    out, err = io.StringIO(), io.StringIO()
    old = sys.argv
    sys.argv = ['session.py', *args]
    code = 0
    try:
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            try:
                session.main()
            except SystemExit as error:
                code = error.code
    finally:
        sys.argv = old
    result = subprocess.CompletedProcess(args, code, out.getvalue(), err.getvalue())
    if directory:
        # JSON status minus viewer keys is the new read-only command's contract.
        output = result.stdout
        if op == 'status' and not code:
            status = json.loads(output)
            for key in ('viewer', 'snapshot', 'snapshot_current'):
                del status[key]
            output = json.dumps(status, ensure_ascii=False)
        case = dict(name=current_test, op=op, directory=str(directory), args=args,
                    before=before, after=files(directory), code=code,
                    output=output, message=result.stderr.splitlines()[0] if code else '', records=records)
        cases.append(case)
    return result


base.run = run
modules = [base]
for name in ('checks', 'handoff_integrity', 'decisions_sync'):
    module = importlib.import_module('test_seed_me_' + name)
    module.run = run
    modules.append(module)


class Results(unittest.TextTestResult):
    def startTest(self, test):
        global current_test
        current_test = test.id()
        super().startTest(test)


suite = unittest.TestSuite(unittest.defaultTestLoader.loadTestsFromModule(m) for m in modules)
result = unittest.TextTestRunner(stream=sys.stderr, resultclass=Results).run(suite)
if not result.wasSuccessful():
    raise SystemExit('pinned seed suites failed; no partial oracle accepted')
with tempfile.TemporaryDirectory(prefix='TEST-seed-edges-') as root:
    directory = session.create(Path(root) / 'TEST-sessions')
    payload = Path(root) / 'TEST-checks.json'
    current_test = 'edge:literal-check-rendering'
    for value in (' 1. nested', '  + prefix', '١. unicode number', 'é', 'a\u001cb', '\u200b', '\U000e0001'):
        payload.write_text(json.dumps([dict(statement=value, kind='judge', source='TEST')]))
        assert run('seed', 'checks', 'set', directory, payload).returncode == 0
        assert run('seed', 'checks', 'render', directory).returncode == 0
    current_test = 'edge:python-repr-status'
    for first in ('quote\'\"', '\u001c', '\u200b', '\U000e0001', '\u0378', 'é\u0301', '\ue000'):
        (directory / 'seed-contract.md').write_text(first + '\n')
        assert run('status', directory).returncode == 0
for case in cases:
    print(json.dumps(case, ensure_ascii=False))
print(json.dumps(dict(summary=dict(upstream_tests=result.testsRun, cases=len(cases)))))
