"""Exercise the unmodified PR executable. macOS only. No model requests."""
import hashlib
import json
import os
from pathlib import Path
import sys
import tempfile

CHECKOUT = Path(sys.argv[1]).resolve()
sys.path.insert(0, str(CHECKOUT/'tests'))
import test_launch
from terminal_process import TerminalProcess
import tend_connection_terminal as flow

OUT = Path(__file__).resolve().parent
F3 = b'\x1bOR'
WORDS = '  TEST thought for PR 869: keep capture in Think.\nReopen this before choosing the next step. café  \n'
results = []


def files(root):
    return {str(p.relative_to(root)): p.read_bytes() for p in root.rglob('*') if p.is_file()}


def screen(term, name):
    value = term.screen.text()
    (OUT/f'{name}.txt').write_text(value)
    return value


def trial(size, inject):
    label = f'{size[0]}x{size[1]}-' + ('sync-failure' if inject else 'normal')
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-869-') as directory:
        root = Path(directory).resolve()
        repo = flow.fixture(root)
        (root/'home').mkdir()
        # A tripwire, not a model. Any attempted provider launch makes this run fail.
        provider = root/'provider-tripwire'
        provider.write_text('#!'+sys.executable+'\nfrom pathlib import Path\nPath('+repr(str(root/'provider-attempt'))+').write_text("unexpected launch")\nraise SystemExit(97)\n')
        provider.chmod(0o700)
        original_sessions = files(root/'sessions')
        original_records = files(root/'records')
        flag = root/'fail-sync'
        if inject: flag.touch()
        fault_log = root/'sync-faults'
        env = test_launch.environment({**os.environ, 'TERM':'xterm-256color', 'NO_COLOR':'1',
            'TINKERY_NODE':'/tinkery-TEST-no-node'}, home=root/'home')
        if inject:
            env.update(DYLD_INSERT_LIBRARIES=str(OUT/'fail-directory-sync.dylib'),
                TINKERY_TEST_SYNC_DIRECTORY=str(root/'links'), TINKERY_TEST_SYNC_FLAG=str(flag),
                TINKERY_TEST_SYNC_LOG=str(fault_log))
        args = [str(CHECKOUT/'target/debug/tinkery'), '--operator','agent','--home',
            '--seed-session-root',str(root/'sessions'), '--record-root',str(root/'records'),
            '--link-root',str(root/'links'), '--shape-pi','--model','test/no-model',
            '--shape-budget','0','--pi-command',str(provider)]
        def launch():
            term = TerminalProcess(args, cwd=repo, env=env, width=size[0], height=size[1])
            flow.wait(term, '1  think')
            return term
        def protected():
            assert files(root/'sessions') == original_sessions, 'Goal sessions changed'
            assert files(root/'records') == original_records, 'Goal confirmations changed'
            assert not (root/'provider-attempt').exists(), 'Provider was launched'
            assert not flow.requests(root), 'Provider received requests'
        term = launch()
        try:
            flow.send(term, b'1', 'F3 Save thought and open Tend')
            os.write(term.master, b'\x1b[200~'+WORDS.encode()+b'\x1b[201~')
            flow.wait(term, 'Reopen this before')
            before = screen(term, label+'-capture')
            assert 'no model call' in before
            if inject:
                flow.send(term, F3, 'Thought persistence is uncertain')
                error = screen(term, label+'-new-error')
                assert 'Reopen this before' in error and 'F3 Save thought' in error
                assert 'Saved thoughts' not in error and 'Thoughts: 1' not in error
                paths = list((root/'links').glob('thought-*.json'))
                assert len(paths) == 1
                stored = paths[0].read_bytes()
                assert json.loads(stored)['words'].encode() == WORDS.encode()
                assert len(fault_log.read_text().splitlines()) == 1, fault_log.read_bytes()
                # Existing exact duplicate must also refuse confirmed success on sync failure.
                flow.send(term, F3, 'Thought persistence is uncertain')
                # Drain until the second injected syscall is observed, not a stale screen.
                import time
                from terminal_process import wait_output
                for _ in wait_output(term.master, term.process, term.screen.feed, time.monotonic()+3):
                    if len(fault_log.read_text().splitlines()) == 2: break
                assert len(fault_log.read_text().splitlines()) == 2, fault_log.read_bytes()
                screen(term, label+'-duplicate-error')
                assert paths[0].read_bytes() == stored
                protected()
                flag.unlink()
            flow.send(term, F3, 'Thoughts: 1')
            screen(term, label+'-garden')
            flow.send(term, b'v', 'Saved thoughts')
            reader = screen(term, label+'-reader')
            assert 'Reopen this before' in reader and 'TEST thought for PR 869' in reader
            paths = list((root/'links').glob('thought-*.json'))
            assert len(paths) == 1
            stored = paths[0].read_bytes()
            assert json.loads(stored)['words'].encode() == WORDS.encode()
            protected()
        finally:
            term.close()
        # A new process reopens the saved original and copies it back without running it.
        term = launch()
        try:
            flow.send(term, b'2', 'Thoughts: 1')
            flow.send(term, b'v', 'Saved thoughts')
            reopened = screen(term, label+'-restart-reader')
            assert 'Reopen this before' in reopened and 'TEST thought for PR 869' in reopened
            flow.send(term, b'g', 'F3 Save thought and open Tend')
            screen(term, label+'-copy-back')
            flow.send(term, F3, 'Thoughts: 1')
            assert len(list((root/'links').glob('thought-*.json'))) == 1
            assert paths[0].read_bytes() == stored
            protected()
            results.append(dict(case=label, result='passed', exact_utf8_bytes=len(WORDS.encode()),
                thought_files=1, provider_attempts=0, goals_unchanged=True,
                injected_directory_sync_failures=2 if inject else 0,
                restart_reopen=True, unchanged_copy_duplicate=True))
            (OUT/'results.json').write_text(json.dumps(results, indent=2)+'\n')
            print(json.dumps(results[-1]), flush=True)
        finally:
            term.close()

for size in [(80,24),(100,30)]:
    trial(size, False)
    trial(size, True)
print('All four isolated trials passed. No provider launched.')
