"""Shape reading journeys: real Seed Me helper sessions in TEST roots, a Pi that must never run, no model calls.
Opening and reading write nothing; answering (F2/F3) is covered by shape_answer_terminal.py."""
import fcntl, importlib.util, os, select, struct, subprocess, tempfile, termios, time
from pathlib import Path
from vt_screen import Screen

REPO = Path(__file__).resolve().parents[1]
SKILL = Path(os.environ['SEED_ME_TEST_SKILL']).resolve()
spec = importlib.util.spec_from_file_location('seed_me_session', SKILL.parent / 'scripts/session.py')
helper = importlib.util.module_from_spec(spec)
spec.loader.exec_module(helper)

GOAL = ('I can see where my seed stands without switching tools.\n'
        'Why: I lose the thread between tools.\nDone when: Shape shows the next question.\n'
        'Must: Keep my words.\nMust not: Run agents.')
DECIDED = 'In one file I can search.'
DETAIL_KEYS = 'c copy prompt   s shape   r history   esc back   F10 menu'
SHAPE_KEYS = 'F5 so far   esc back   F10 menu'
LIST_KEYS = '↑↓ move   enter open   esc back'


def node(identifier, **fields):
    return {'id': identifier, 'kind': 'decision', 'prerequisites': [], 'evidence': [],
            'owner': 'TEST operator', 'gate': 'TEST choice', **fields}


def make(root, name, goal=GOAL, question=True):
    """A human session with a confirmed goal and, optionally, one current question."""
    session = helper.create(root)
    old = helper.load(session)
    state = helper.editable(old)
    nodes = [node('goal', status='settled', answer=goal, authority='user',
                  authority_source='TEST fixture, not real approval'),
             node('notes', status='settled', label='TEST where notes go', answer=DECIDED,
                  authority='user', authority_source='TEST fixture, not real approval')]
    if question:
        nodes.append(node('storage', status='unresolved', label='Where records live',
                          question='Keep records local or synced?',
                          evidence=['README.md:3 says local only'],
                          recommendation='Local, matching the README.'))
    state.update(goal=goal, origin='goal', nodes=nodes, current_question='storage' if question else None)
    helper.publish(session, state, old['version'], 'TEST fixture')
    return session


def seed(session, confirm=False):
    body = ('status: draft\n# TEST seed\nrevision: r1\n## You are confirming\nKeep records local.\n\n'
            + helper.decisions_block(helper.load(session)))
    (session / 'seed-contract.md').write_text(body, encoding='utf-8')
    if confirm:
        helper.confirm_seed(session, 'r1', 'TEST fixture only: confirm r1')


def files(root):
    return {str(p): p.read_bytes() for p in root.rglob('*') if p.is_file()}


def journey(width, height, case, no_color=False):
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-shape-') as directory:
        cwd = Path(directory).resolve()
        sessions = cwd / 'TEST-sessions'
        if case == 'list':
            make(sessions, 'question')
            drafted_session = make(sessions, 'drafted', goal='Drafted goal\n' + GOAL, question=False)
            seed(drafted_session)
            seed(make(sessions, 'ready', goal='Ready goal\n' + GOAL, question=False), confirm=True)
        else:
            session = make(sessions, case, question=case in ('question', 'ended', 'unreadable'))
            if case == 'drafted':
                seed(session)
            if case in ('ready', 'stale'):
                seed(session, confirm=True)
            if case == 'stale':
                path = session / 'seed-contract.md'
                path.write_text(path.read_text().replace('Keep records local.', 'Keep records synced.'))
        host = cwd / 'never-run-pi'
        host.write_text('#!/bin/sh\necho unexpected > provider-was-run\nexit 86\n')
        host.chmod(0o755)
        master, slave = os.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', height, width, 0, 0))
        env = {**os.environ, 'TERM': 'xterm-256color', 'TINKERY_NODE': '/tinkery-TEST-no-node'}
        if no_color:
            env['NO_COLOR'] = '1'
        process = subprocess.Popen([str(REPO / 'target/debug/tinkery'), '--home', '--shape-pi',
            '--model', 'test/model', '--seed-me', str(SKILL), '--seed-session-root', str(sessions),
            '--record-root', str(cwd / 'TEST-records'), '--link-root', str(cwd / 'TEST-links'),
            '--pi-command', str(host)], stdin=slave, stdout=slave, stderr=slave, cwd=cwd, env=env)
        screen = Screen(width, height)

        def wait(predicate):
            deadline = time.monotonic() + 10
            while time.monotonic() < deadline:
                if select.select([master], [], [], .02)[0]:
                    screen.feed(os.read(master, 65536))
                if predicate() and not screen.synchronized:
                    return
                if process.poll() is not None:
                    break
            raise AssertionError(screen.text())

        def send(data, text):
            os.write(master, data)
            wait(lambda: ' '.join(text.split()) in flat())

        def flat():
            return ' '.join(screen.text().split())

        def so_far(first=''):
            """F5's page: the whole goal byte for byte, then the operator's decision; Esc closes it."""
            keys = screen.text().splitlines()[-1].strip()
            send(b'\x1b[15~', 'F5 close')
            rows = [row[2:].rstrip() for row in screen.text().splitlines()]
            start = rows.index('Your goal, as you confirmed it') + 1
            shown = '\n'.join(rows[start:rows.index('', start)])
            assert shown == (first + '\n' if first else '') + GOAL, screen.text()
            assert f'TEST where notes go: {DECIDED} (you)' in flat(), screen.text()
            assert 'esc back' not in screen.text(), screen.text()
            send(b'\x1b', keys)

        def shows_goal(first=''):
            for line in ([first] if first else []) + GOAL.splitlines():
                assert ' '.join(line.split()) in flat(), (line, screen.text())

        try:
            wait(lambda: '3  shape' in screen.text())
            if case == 'list':
                send(b'3', LIST_KEYS)
                assert 'Ready goal' not in screen.text(), screen.text()
                assert 'goal saved' in screen.text() and 'seed drafted' in screen.text(), screen.text()
                before = files(sessions)
                lines = screen.text().splitlines()
                drafted = 'j' if any('› I can see' in l for l in lines) else ''
                os.write(master, drafted.encode())
                send(b'\r', 'Confirming the seed is not approval to build.')
                assert 'status: draft' not in screen.text(), screen.text()
                so_far('Drafted goal')
                # Confirmed elsewhere while open: back at the list, Shape reads again and drops it.
                helper.confirm_seed(drafted_session, 'r1', 'TEST fixture only: confirm r1')
                before = files(sessions)
                send(b'\x1b', LIST_KEYS)
                wait(lambda: 'Drafted goal' not in screen.text() and 'goal saved' in screen.text())
                send(b'\x1b', '3  shape')
            else:
                send(b'2', 'enter open')
                send(b'\r', DETAIL_KEYS)
                if case == 'ended':
                    helper.end(next(sessions.iterdir()), 'stopped', 'TEST fixture', no_viewer='TEST fixture')
                if case == 'unreadable':
                    (next(sessions.iterdir()) / 'ledger.json').write_text('not JSON')
                before = files(sessions)
                expected = {
                    'question': 'Where records live Keep records local or synced?',
                    'drafted': 'Confirming the seed is not approval to build.',
                    'ready': 'Seed ready: you confirmed r1, and it still matches.',
                    'stale': "Seed Me can't verify it: the seed changed after revision r1 was confirmed.",
                    'ended': 'This session has ended (stopped).',
                    'unreadable': 'Seed Me refused to read this session:',
                }[case]
                send(b's', expected)
                keys = {'question': 'F2 save answer and look into next   F5 so far   esc back   F10 menu',
                        'drafted': 'F3 confirm seed r1   F5 so far   esc back   F10 menu',
                        'unreadable': 'esc back   F10 menu'}.get(case, SHAPE_KEYS)
                # Scrolling is offered only on overflow; arrows belong to an active editor.
                acts = case in ('question', 'drafted')
                scroll_keys = ('pgup/dn read   ' + keys.replace('F2 save answer and look into next', 'F2 save + look')
                               if acts else '↑↓ scroll   ' + keys)
                bar = screen.text().splitlines()[-1].strip()
                assert bar in (keys, scroll_keys), screen.text()
                # The key bar names the one next action on a question or a draft, so no Next line does.
                assert flat().count('Next:') == (0 if acts else 1), screen.text()
                if case == 'drafted':
                    assert 'status: draft' not in screen.text() and 'Seed draft r1' in screen.text()
                elif case != 'unreadable':
                    lines = GOAL.splitlines()
                    assert lines[0] in flat() and lines[1] not in flat(), screen.text()
                if case != 'unreadable':
                    so_far()
                if case == 'question':
                    assert 'What I found: README.md:3 says local only' in flat(), screen.text()
                    assert 'Suggestion: Local, matching the README.' in flat(), screen.text()
                    assert 'Option' not in screen.text(), screen.text()
                if width == 100 and case == 'question' and not no_color:
                    print('TEST Shape 100×30 screen:\n' + screen.text())
                assert str(cwd) not in screen.text(), screen.text()
                if case == 'unreadable':
                    send(b'\x1bOP', 'What went wrong')
                    send(b'\x1b', keys)
                # Return now waits for fresh discovery, not the cached detail's footer.
                if case == 'ended':
                    send(b'\x1b', 'No active goals. 1 ended not shown.')
                    assert GOAL.splitlines()[0] not in flat(), screen.text()
                elif case == 'unreadable':
                    send(b'\x1b', 'Saved session unavailable. F1 then h for details.')
                    assert GOAL.splitlines()[0] not in flat(), screen.text()
                else:
                    next_step = ('Next: confirm the seed draft with Seed Me.' if case == 'drafted' else
                                 'Next: the seed is ready for intake.' if case in ('ready', 'stale') else
                                 'Next: shape it into a seed with Seed Me.')
                    send(b'\x1b', 'tinkery › tend')
                    wait(lambda: next_step in flat())
                    shows_goal()
            assert files(sessions) == before, 'Shape changed a session file'
            assert not (cwd / 'provider-was-run').exists()
            assert not (cwd / 'TEST-records').exists() and not (cwd / 'TEST-links').exists()
            print(f'Shape {case} journey passed at {width}x{height}' + (' NO_COLOR' if no_color else ''))
        finally:
            if process.poll() is None:
                process.kill()
                process.wait()
            os.close(master)
            os.close(slave)


for size in [(100, 30), (80, 24)]:
    for case in ['question', 'drafted', 'ready', 'stale', 'ended', 'unreadable', 'list']:
        journey(*size, case)
journey(100, 30, 'question', no_color=True)
