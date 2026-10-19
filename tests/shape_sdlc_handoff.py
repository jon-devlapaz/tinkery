"""Shape's real writer → pinned SDLC intake/verify, with TEST-only confirmations.

No product model calls: one stub reply. All installs, Git commits, synthetic
attestations and the reversible consumer mutation live under a short /tmp TEST root.
Locked checks protect the agreement, not the strength of the agreed checks.
"""
import fcntl
import importlib.util
import json
import os
from pathlib import Path
import select
import shutil
import struct
import subprocess
import sys
import tempfile
import termios
import time

from vt_screen import Screen

REPO = Path(__file__).resolve().parents[1]
SKILL = Path(os.environ['SEED_ME_TEST_SKILL']).resolve()
# Draft consumer contract; move to its merge SHA only on the operator's coordinated landing.
SDLC_REF = '49c1b86a566d6b900ac14c370c3c4a2b9fcd300f'
spec = importlib.util.spec_from_file_location('seed_me_session', SKILL.parent / 'scripts/session.py')
helper = importlib.util.module_from_spec(spec)
spec.loader.exec_module(helper)
CONFIRMATION = 'TEST confirmation of r1 only; not operator approval or permission to build.'
# Original copied verbatim from /tmp/tinkery-TEST-stacker-KZcCiq/sessions/
# 37bc32a6-e13d-4558-ab44-6d00fabbd54a/seed-contract.md; never edit that evidence.
ORIGINAL = (REPO / 'tests/fixtures/stacker-r1-seed-contract.md').read_text()
GOAL = ORIGINAL.split('- goal [user] ', 1)[1].split(' (source:', 1)[0]


def command(argv, cwd, env, expected=0):
    result = subprocess.run([str(a) for a in argv], cwd=cwd, env=env,
                            capture_output=True, text=True, timeout=120)
    if expected is not None:
        assert result.returncode == expected, (argv, result.stdout, result.stderr)
    return result


def shape(root, env):
    """Draft through F2 and confirm through F3, never by writing a seed fixture."""
    sessions = root / 'TEST-sessions'
    session = helper.create(sessions)
    old = helper.load(session)
    state = helper.editable(old)
    state.update(goal=GOAL, origin='goal', current_question=None, nodes=[{
        'id': 'goal', 'kind': 'decision', 'prerequisites': [], 'evidence': [],
        'owner': 'TEST fixture', 'gate': 'TEST only', 'status': 'settled',
        'answer': GOAL, 'authority': 'user', 'authority_source': 'TEST fixture, not operator approval'}])
    for identifier, answer in [('stacker-fairness', 'skill'), ('stacker-checkpoint', 'aim for the top')]:
        state['nodes'].append({'id': identifier, 'kind': 'decision', 'prerequisites': [], 'evidence': [],
            'owner': 'TEST fixture', 'gate': 'TEST only', 'status': 'settled', 'answer': answer,
            'authority': 'user', 'authority_source': 'TEST replay of recorded Stacker decision, not new approval'})
    helper.publish(session, state, old['version'], 'TEST fixture only')
    reply = json.loads((REPO / 'tests/fixtures/stacker-r1-reply.json').read_text())
    (root / 'TEST-reply.json').write_text(json.dumps(reply))
    host = root / 'TEST-pi'
    host.write_text(f'#!{sys.executable}\n'
                    'import json, os, pathlib, sys\n'
                    'request = json.load(sys.stdin)\n'
                    'assert "ledger" in request\n'
                    'root = pathlib.Path(os.environ["TINKERY_TEST_DIR"])\n'
                    'with (root / "TEST-calls").open("a") as calls: calls.write("stub\\n")\n'
                    'print((root / "TEST-reply.json").read_text())\n')
    host.chmod(0o755)
    master, slave = os.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 30, 100, 0, 0))
    screen = Screen(100, 30)
    args = [REPO / 'target/debug/tinkery', '--home', '--shape-pi', '--model', 'test/model',
            '--shape-budget', '1', '--seed-me', SKILL, '--seed-session-root', sessions,
            '--record-root', root / 'TEST-records', '--link-root', root / 'TEST-links',
            '--pi-command', host]
    process = subprocess.Popen([str(a) for a in args], cwd=root, env=env,
                               stdin=slave, stdout=slave, stderr=slave)

    def wait(text):
        deadline = time.monotonic() + 25
        while time.monotonic() < deadline:
            if select.select([master], [], [], .02)[0]:
                screen.feed(os.read(master, 65536))
            if ' '.join(text.split()) in ' '.join(screen.text().split()) and not screen.synchronized:
                return
            if process.poll() is not None:
                break
        raise AssertionError(f'{text!r} not shown:\n{screen.text()}')

    def send(keys, text):
        os.write(master, keys)
        wait(text)

    try:
        wait('3  shape')
        send(b'3', 'enter open')
        send(b'\r', 'F2 look into the next question')
        # Seed Me's pre-mortem comes first, without a model call.
        send(b'\x1bOQ', 'Imagine this shipped and turned out wrong. What is the most likely reason?')
        send(b'TEST The sounds were missing.', 'TEST The sounds were missing.')
        send(b'\x1bOQ', 'Seed draft r1')
        wait('F3 confirm seed r1')
        send(CONFIRMATION.encode(), CONFIRMATION)
        # F3 confirms only a draft read to the end (#250).
        send(b'\x1bOR', 'Read to the end first: PgDn scrolls. Nothing was confirmed.')
        os.write(master, b'\x1b[6~' * 40)
        wait('Next: type your own words, then F3 confirms r1.')
        send(b'\x1bOR', 'Seed ready: you confirmed r1, and it still matches.')
        assert helper.status(session)['handoff']['verified'] is True
        assert json.loads((session / 'seed-confirmation.json').read_text())['source'] == CONFIRMATION
        assert helper.load(session)['goal'] == GOAL
        assert (root / 'TEST-calls').read_text().splitlines() == ['stub']
        print('Shape TEST confirmation: 100x30 PTY, real seed writer, 1 stub call, 0 product model calls')
        return session
    finally:
        if process.poll() is None:
            process.kill()
        process.wait()
        os.close(master)
        os.close(slave)


def handoff(root, env, session):
    source = root / 'TEST-tink-sdlc'
    command(['git', 'init', '-q', source], root, env)
    command(['git', 'fetch', '--depth', '1', 'https://github.com/jon-devlapaz/tink-sdlc.git',
             SDLC_REF], source, env)
    command(['git', 'checkout', '--detach', 'FETCH_HEAD'], source, env)
    head = command(['git', 'rev-parse', 'HEAD'], source, env).stdout.strip()
    assert head == SDLC_REF, (head, SDLC_REF)
    print(f'Pinned tink-sdlc: {head}', flush=True)
    project = root / 'TEST-project'
    project.mkdir()
    command([sys.executable, source / 'scripts/init.py', project], root, env)
    command(['git', 'init', '-q', '-b', 'main'], project, env)
    check = project / 'TEST-check'
    check.write_text('#!/bin/sh\necho "TEST existing output"\ntest -f TEST-accepted\n')
    check.chmod(0o755)
    command(['git', 'add', '-A'], project, env)
    command(['git', 'commit', '-qm', 'TEST isolated baseline'], project, env)
    runtime = project / '_system/scripts/sdlc.py'

    def sdlc(*args, expected=0):
        return command([sys.executable, runtime, *args], project, env, expected)

    # Replay the recorded Stacker reply first: all three records, no invented executable checks.
    original_files = {name: (session / name).read_bytes() for name in (
        'seed-contract.md', 'ledger.json', 'seed-confirmation.json', 'seed-checks.json')}
    sdlc('new', 'stacker-r1')
    sdlc('stage', 'stacker-r1', '1', '--here', '--seed-contract', str(session / 'seed-contract.md'),
         '--seed-me-helper', SKILL.parent / 'scripts/session.py')
    stacker_run = project / 'runs/stacker-r1'
    assert (stacker_run / 'seed-contract.md').read_bytes() == original_files['seed-contract.md']
    stacker_checks = json.loads((stacker_run / 'seed-checks.json').read_text())['items']
    assert [(i['kind'], i['locked']) for i in stacker_checks] == [
        ('judge', True), ('provisional', True), ('provisional', True)]
    records = json.loads(original_files['seed-checks.json'])
    assert [item['description'] for item in stacker_checks] == [record['statement'] for record in records]
    assert [{key: item[key] for key in record}
            for item, record in zip(stacker_checks, records)] == records
    assert {name: (session / name).read_bytes() for name in original_files} == original_files
    print('Real Stacker r1: judge + two provisional records imported and locked; no checks promoted')

    # Separate synthetic command records exercise verify mechanics, not Stacker quality.
    session = helper.create(root / 'TEST-command-sessions')
    old = helper.load(session)
    state = helper.editable(old)
    state.update(goal='TEST command enforcement', origin='goal', current_question=None, nodes=[{
        'id': 'goal', 'kind': 'decision', 'prerequisites': [], 'evidence': [],
        'owner': 'TEST fixture', 'gate': 'TEST only', 'status': 'settled',
        'answer': 'TEST command enforcement', 'authority': 'user', 'authority_source': 'TEST fixture only'}])
    helper.publish(session, state, old['version'], 'TEST command enforcement fixture')
    helper.set_checks(session, json.loads((REPO / 'tests/fixtures/shape-checks.json').read_text()))
    body = (REPO / 'tests/fixtures/shape-seed-reply.txt').read_text().replace(
        '## Affected users and systems', helper.checks_block(helper.read_checks(session)) + '## Affected users and systems', 1)
    body = body.replace('## Knowledge map', helper.decisions_block(helper.load(session)) + '\n## Knowledge map', 1)
    body = body.replace('## Suggested first slice', '## Evidence and uncertainty\nTEST command fixture; no investigation claimed.\n\n## Suggested first slice', 1)
    (session / 'seed-contract.md').write_text('status: draft\n# TEST commands\nrevision: r1\n\n## You are confirming\n' + body)
    helper.confirm_seed(session, 'r1', CONFIRMATION)
    sdlc('new', 'test-handoff')
    run = project / 'runs/test-handoff'
    worker = {'argv': [sys.executable, '-c', 'print("TEST worker check passes")'], 'timeout_seconds': 5}
    (project / '_system/verification.json').write_text(json.dumps({'require_tink': False, 'checks': [worker]}))
    (run / 'checklist.json').write_text(json.dumps({'schema': 1, 'items': [{
        'id': 'worker', 'description': 'TEST worker-written check', 'verify': 'TEST success', 'check': worker}]}))
    seed = session / 'seed-contract.md'
    confirmed = {name: (session / name).read_bytes()
                 for name in ('seed-contract.md', 'ledger.json', 'seed-confirmation.json', 'seed-checks.json')}
    # Stage 1 is the real intake. --here prevents worktrees; it prints a prompt, not an agent launch.
    sdlc('stage', 'test-handoff', '1', '--here', '--seed-contract', str(seed),
         '--seed-me-helper', SKILL.parent / 'scripts/session.py')
    assert (run / 'seed-contract.md').read_bytes() == confirmed['seed-contract.md']
    definition_path = run / 'seed-checks.json'
    definitions = definition_path.read_bytes()
    items = json.loads(definitions)['items']
    assert [(i['id'], i['kind'], i['locked']) for i in items] == [
        ('seed-1', 'cmd', True), ('seed-2', 'provisional', True), ('seed-3', 'judge', True)]
    assert (items[0]['cmd'], items[0]['expect'], items[0]['cwd']) == (
        './TEST-check', 'TEST existing output', '.')

    def approve():
        sdlc('decide', 'test-handoff', '3', 'approved', '--reviewer', 'TEST synthetic reviewer',
             '--source', 'TEST fixture only', '--reason', 'TEST definition, not operator approval')

    def attest(item):
        sdlc('mark', 'test-handoff', item, 'passed', '--reviewer', 'TEST synthetic reviewer',
             '--source', 'TEST fixture only', '--evidence', 'TEST synthetic observation, not operator attestation')

    approve()
    for mutation in ('edit', 'drop'):
        changed = json.loads(definitions)
        if mutation == 'edit':
            changed['items'][0]['cmd'] = 'true'
        else:
            changed['items'] = []
        try:
            definition_path.write_text(json.dumps(changed))
            assert 'Stage 3: stale' in sdlc('status', 'test-handoff').stdout
            assert sdlc('verify', 'test-handoff', expected=None).returncode != 0
            assert sdlc('decide', 'test-handoff', '3', 'approved', '--reviewer', 'TEST fixture',
                        '--source', 'TEST only', '--reason', 'TEST', expected=None).returncode != 0
        finally:
            definition_path.write_bytes(definitions)
    print('Locked seed edit/drop: stale gate; verify and reapproval refused')

    # No test fixture attestation may be mistaken for a real person's judgment.
    assert sdlc('mark', 'test-handoff', 'seed-3', 'passed', '--evidence', 'TEST worker guess',
                expected=None).returncode != 0
    accepted = project / 'TEST-accepted'
    accepted.touch()
    # Command success is insufficient for prose expect: or you judge:.
    missing_attestations = sdlc('verify', 'test-handoff', expected=None)
    assert missing_attestations.returncode != 0, missing_attestations.stdout
    assert 'seed-1-expect' in missing_attestations.stderr and 'seed-3' in missing_attestations.stderr
    attest('seed-1-expect')
    missing_judgment = sdlc('verify', 'test-handoff', expected=None)
    assert missing_judgment.returncode != 0 and 'seed-3' in missing_judgment.stderr
    attest('seed-3')
    accepted.unlink()
    receipt = run / '04-test/output/verification.json'

    def failing_seed_blocks():
        result = sdlc('verify', 'test-handoff', expected=None)
        assert result.returncode != 0, 'verify accepted a failing confirmed seed check'
        assert 'Seed check failed: seed-1' in result.stderr, result.stderr
        proof = json.loads(receipt.read_text())
        assert proof['result'] == 'failed', proof
        assert proof['seed_checks']['seed-1'] == 'failed', proof
        log = (run / '04-test/output/test-log.md').read_text()
        assert log.count('TEST worker check passes\n') == 2, log
        assert '# checklist item worker' in log and '# seed check seed-1' in log, log
        print('Settled seed FAIL blocks verify; every worker-written check PASSES')

    failing_seed_blocks()
    # Revert probe: drop imported seed enforcement in this TEST install only.
    # The exact same behavioral assertion must go red, then pass after restoration.
    original = runtime.read_bytes()
    needle = b'def seed_check_items(path):\n'
    assert original.count(needle) == 1
    try:
        runtime.write_bytes(original.replace(needle, needle + b'    return []  # TEST mutation: drop seed checks\n'))
        try:
            failing_seed_blocks()
        except AssertionError as error:
            assert str(error) == 'verify accepted a failing confirmed seed check', str(error)
            print(f'Revert probe RED: {error}')
        else:
            raise AssertionError('Dropping seed enforcement did not turn the regression red')
    finally:
        runtime.write_bytes(original)
    assert runtime.read_bytes() == original
    failing_seed_blocks()
    print('Revert probe RESTORED: failing seed blocks again')
    accepted.touch()
    result = sdlc('verify', 'test-handoff')
    assert 'Seed checks (operator-confirmed)' in result.stdout and 'Worker-written checks:' in result.stdout
    proof = json.loads(receipt.read_text())
    assert proof['result'] == 'passed', proof
    assert proof['checklist_items'] == {'worker': 'proven'}, proof
    assert proof['seed_checks'] == {'seed-1': 'proven', 'seed-1-expect': 'attested',
                                    'seed-2': 'pending', 'seed-3': 'attested'}, proof
    assert 'Verification: current' in sdlc('status', 'test-handoff').stdout
    assert {name: (session / name).read_bytes() for name in confirmed} == confirmed
    print('Settled seed PASS completes verify; provisional remains pending; manual judgments are TEST attestations')


def main():
    with tempfile.TemporaryDirectory(prefix='t194-TEST-', dir='/tmp') as directory:
        root = Path(directory).resolve()
        home, tools, tmp = (root / name for name in ('TEST-home', 'TEST-bin', 'tmp'))
        for folder in (home, tools, tmp):
            folder.mkdir()
        # Allow only known tools: no operator tink/tink-route, provider or launcher can leak in.
        for name in ('git', 'bash', 'python3'):
            (tools / name).symlink_to(shutil.which(name))
        env = {'PATH': f'{tools}:/usr/bin:/bin', 'HOME': str(home), 'TMPDIR': str(tmp),
               'TERM': 'xterm-256color', 'TINKERY_NODE': '/tinkery-TEST-no-node',
               'TINKERY_TEST_DIR': str(root), 'GIT_CONFIG_GLOBAL': '/dev/null', 'GIT_CONFIG_NOSYSTEM': '1',
               'GIT_TERMINAL_PROMPT': '0', 'GIT_AUTHOR_NAME': 'TEST fixture',
               'GIT_AUTHOR_EMAIL': 'test@example.invalid', 'GIT_COMMITTER_NAME': 'TEST fixture',
               'GIT_COMMITTER_EMAIL': 'test@example.invalid', 'PYTHONDONTWRITEBYTECODE': '1'}
        handoff(root, env, shape(root, env))


if __name__ == '__main__':
    main()
