"""Shape reading journeys: real Rust ledger sessions in TEST roots, a Pi that must never run, no model calls.
Opening and reading write nothing; answering (F2/F3) is covered by shape_answer_terminal.py. Under an agent
launch declaration Shape only reads: F2, F3, F6, F7, F8, F11 and F12 are held and say why (#247)."""
import base64, fcntl, importlib.util, os, re, select, shlex, struct, tempfile, termios, time
import test_launch as subprocess
from pathlib import Path
from vt_screen import Screen

REPO = Path(__file__).resolve().parents[1]
import session_fixture as helper

GOAL = ('I can see where my seed stands without switching tools.\n'
        'Why: I lose the thread between tools.\nDone when: Shape shows the next question.\n'
        'Must: Keep my words.\nMust not: Run agents.')
DECIDED = 'In one file I can search.'
DETAIL_KEYS = 's shape   r history   esc back   F10 menu'
# P-051-inspired TEST reconstruction, not a transcript or repository evidence.
P051_QUESTION = 'Should Stacker be skill-based or simulate payout interference?'
P051_FINDINGS = [
    'DESIGN.md:106-108: Fairness is an explicit design value. The inspected rules do not settle '
    'whether Stacker should be skill-only or simulate the arcade cabinet withholding a win. '
    'The linked external rules have not been read; that gameplay choice remains yours.',
    'arcade_menu/games.json:1-20: The menu registers existing cabinets, but no Stacker cabinet. '
    'Adding a cabinet would extend this registration list and the menu integration checks. '
    'This identifies a future change surface, not a reason to choose either gameplay rule.',
    'renderer.py:1-30: The shared renderer provides paper styling and terminal drawing. '
    'Existing cabinets separate rendering from game rules. A new cabinet could reuse that '
    'surface; changing shared drawing could affect every cabinet and needs compatibility checks.',
    'audio.py:1-30: The shared audio interface supports cabinet sounds and a muted fallback. '
    'Successful audio calls do not prove audible playback. Repeated playback still needs human '
    'taste; no audio device was used for this TEST observation.',
    'tests/test_rules.py:1-30: Existing rule checks are separate from presentation checks. '
    'No Stacker-specific regression exists in this TEST scenario. A later implementation '
    'needs behavior-specific assertions, including an observed red or targeted reversion probe.',
]
SHAPE_KEYS = 'F5 so far   esc back   F10 menu'
HELD = "Delegated shaping isn't supported yet. Shape only reads; Esc goes back."
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
    if name == 'p051':
        nodes[-1].update(label='Stacker gameplay', question=P051_QUESTION,
                         evidence=P051_FINDINGS, recommendation='Skill-only, matching the fairness principle.')
    state.update(goal=goal, origin='goal', nodes=nodes, current_question='storage' if question else None)
    helper.publish(session, state, old['version'], 'TEST fixture')
    return session


def seed(session, confirm=False):
    helper.set_checks(session, [{'statement': 'Keep records local.', 'kind': 'judge', 'source': 'TEST fixture'}])
    body = ('status: draft\n# TEST seed\nrevision: r1\n## You are confirming\nKeep records local.\n\n'
            '## Now  (the agent keeps these current)\nTEST draft.\n\n## Goal\nKeep records local.\n\n'
            + helper.decisions_block(helper.load(session))
            + '\n## Constraints and exclusions\nTEST only.\n\n'
            + helper.checks_block(helper.read_checks(session))
            + '## Open questions\nNone.\n\n## Knowledge map\nTEST only.\n\n## Evidence\nTEST fixture.\n')
    (session / 'seed-contract.md').write_text(body, encoding='utf-8')
    if confirm:
        helper.confirm_seed(session, 'r1', 'TEST fixture only: confirm r1')


def files(root):
    return {str(p): p.read_bytes() for p in root.rglob('*') if p.is_file()}


def journey(width, height, case, no_color=False, operator='human'):
    held = operator == 'agent'
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-shape-') as directory:
        cwd = Path(directory).resolve()
        sessions = cwd / "TEST operator's sessions"
        if case == 'list':
            make(sessions, 'question')
            drafted_session = make(sessions, 'drafted', goal='Drafted goal\n' + GOAL, question=False)
            seed(drafted_session)
            seed(make(sessions, 'ready', goal='Ready goal\n' + GOAL, question=False), confirm=True)
        else:
            session = make(sessions, case, question=case in ('question', 'p051', 'ended', 'unreadable'))
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
        process = subprocess.Popen([str(REPO / 'target/debug/tinkery'), '--operator', operator, '--home',
            '--shape-pi',
            '--model', 'test/model', '--seed-session-root', str(sessions),
            '--record-root', str(cwd / 'TEST-records'), '--link-root', str(cwd / 'TEST-links'),
            '--pi-command', str(host)], stdin=slave, stdout=slave, stderr=slave, cwd=cwd, env=subprocess.environment(env))
        screen = Screen(width, height)
        output = bytearray()

        def copies():
            return [base64.b64decode(data).decode() for data in
                    re.findall(rb'\x1b\]52;[^;]*;([^\x07\x1b]*)(?:\x07|\x1b\\)', output)]

        def wait(predicate):
            deadline = time.monotonic() + 10
            while time.monotonic() < deadline:
                if select.select([master], [], [], .02)[0]:
                    data = os.read(master, 65536)
                    output.extend(data)
                    screen.feed(data)
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
                before_tend = files(sessions)
                send(b'2', 'enter open')
                if case in ('ready', 'stale'):
                    assert 'seed needs checking' in flat(), screen.text()
                    assert 'seed ready' not in flat().lower(), screen.text()
                send(b'\r', DETAIL_KEYS)
                if case in ('ready', 'stale'):
                    assert 'Next: check the seed in Shape.' in flat(), screen.text()
                    assert 'ready for intake' not in flat(), screen.text()
                    assert files(sessions) == before_tend, 'Tend changed confirmation evidence'
                    assert not (cwd / 'provider-was-run').exists()
                if case == 'ended':
                    helper.end(next(sessions.iterdir()), 'stopped', 'TEST fixture')
                if case == 'unreadable':
                    (next(sessions.iterdir()) / 'ledger.json').write_text('not JSON')
                before = files(sessions)
                expected = {
                    'question': 'Where records live Keep records local or synced?',
                    'p051': P051_QUESTION,
                    'drafted': 'Seed draft r1' if held else 'Confirming the seed is not approval to build.',
                    'ready': 'Seed ready: you confirmed r1, and it still matches.',
                    'stale': "Tinkery can't verify it: the seed changed after revision r1 was confirmed.",
                    'ended': 'This session has ended (stopped).',
                    'unreadable': 'Tinkery refused to read this session:',
                }[case]
                send(b's', expected)
                keys = {'question': 'F2 save + look   F11 defer   F12 delegate   esc back   F10 menu',
                        'p051': 'F2 save + look   F11 defer   F12 delegate   esc back   F10 menu',
                        'drafted': 'F3 confirm seed r1   F7 teach-back   esc back   F10 menu',
                        'ready': 'c copy intake   F6 end session   ' + SHAPE_KEYS,
                        'unreadable': 'esc back   F10 menu'}.get(case, SHAPE_KEYS)
                if held:
                    keys = {'ready': 'c copy intake   ' + SHAPE_KEYS}.get(case, SHAPE_KEYS)
                # Scrolling is offered only on overflow; arrows belong to an active editor.
                acts = case in ('question', 'p051', 'drafted') and not held
                scroll_keys = ('pgup/dn read   ' + keys
                               if acts else '↑↓ scroll   ' + keys)
                bar = screen.text().splitlines()[-1].strip()
                # A bar too wide for the terminal closes up rather than lose Esc or F10.
                assert bar in [k.replace('   ', gap) for k in (keys, scroll_keys) for gap in ('   ', '  ')], screen.text()
                # The key bar names the next action, so no Next line repeats it.
                assert flat().count('Next:') == (1 if held else 0 if acts or case == 'ready' else 1), screen.text()
                if held:
                    assert HELD in flat(), screen.text()
                    for key in (b'\x1bOQ', b'\x1bOR', b'\x1b[17~', b'\x1b[18~', b'\x1b[19~', b'\x1b[23~', b'\x1b[24~'):
                        os.write(master, key * 2)
                    send(b'x', HELD)
                    assert 'your answer' not in flat() and 'your words' not in flat(), screen.text()
                    assert files(sessions) == before, 'A held Shape wrote'
                if case == 'drafted':
                    assert 'status: draft' not in screen.text() and 'Seed draft r1' in screen.text()
                elif case != 'unreadable':
                    lines = GOAL.splitlines()
                    assert lines[0] in flat() and lines[1] not in flat(), screen.text()
                assert copies() == [], 'Opening Shape copied without a key'
                copy_before = files(cwd)
                os.write(master, b'c')
                if case == 'ready':
                    seed_path = str((session / 'seed-contract.md').resolve())
                    tinkery_path = str((REPO / 'target/debug/tinkery').resolve())
                    quote = lambda path: "'" + path.replace("'", "'\\''") + "'"
                    expected_command = ('python3 _system/scripts/sdlc.py stage <run> 1 --seed-contract ' +
                        quote(seed_path) + ' --tinkery ' + quote(tinkery_path))
                    wait(lambda: copies() == [expected_command] and 'Copied.' in flat())
                    assert shlex.split(copies()[0]) == ['python3', '_system/scripts/sdlc.py',
                        'stage', '<run>', '1', '--seed-contract', seed_path,
                        '--tinkery', tinkery_path]
                    send(b'c', 'Copied.')
                    wait(lambda: copies() == [expected_command, expected_command])
                else:
                    assert 'c copy intake' not in flat(), screen.text()
                if case != 'unreadable':
                    so_far()
                else:
                    send(b'\x1bOP', 'What went wrong')
                    send(b'\x1b', keys)
                assert len(copies()) == (2 if case == 'ready' else 0), copies()
                assert files(cwd) == copy_before, 'Copy/navigation wrote files'
                if case == 'ready':
                    send(b'\x1bOP', 'Replace <run> with your SDLC run.')
                    assert 'grants no approval to build' in flat(), screen.text()
                    send(b'\x1b', keys)
                if case == 'p051':
                    assert flat().count('What I found:') == 1, screen.text()
                    assert ' '.join(P051_FINDINGS[0].split()) in flat(), screen.text()
                    assert 'Also found:' not in flat(), screen.text()
                    for finding in P051_FINDINGS[1:]:
                        assert finding.split(':')[0] not in flat(), screen.text()
                    assert 'your answer' in flat() and 'F2 save + look' in flat(), screen.text()
                    send(b'Keep it skill-only.', 'Keep it skill-only.')
                    send(b'\x1b[15~', 'Findings for this question')
                    # Every complete observation is reachable, without saving or calling.
                    viewports = [flat()]
                    for _ in range(80):
                        os.write(master, b'\x1b[B')
                        time.sleep(.02)
                        while select.select([master], [], [], .02)[0]:
                            screen.feed(os.read(master, 65536))
                        viewports.append(flat())
                    for finding in P051_FINDINGS:
                        assert any(' '.join(finding.split()) in view for view in viewports), finding
                    send(b'\x1b[15~', 'Keep it skill-only.')
                    assert 'Also found:' not in flat(), screen.text()
                    assert P051_FINDINGS[-1].split(':')[0] not in flat(), screen.text()
                if case == 'question':
                    assert 'What I found: README.md:3 says local only' in flat(), screen.text()
                    assert 'Suggestion: Local, matching the README.' in flat(), screen.text()
                    assert 'Option' not in screen.text(), screen.text()
                if width == 100 and case == 'question' and not no_color:
                    print('TEST Shape 100×30 screen:\n' + screen.text())
                assert str(cwd) not in screen.text(), screen.text()
                # Return now waits for fresh discovery, not the cached detail's footer.
                if case == 'ended':
                    send(b'\x1b', 'No active goals. 1 ended not shown.')
                    assert GOAL.splitlines()[0] not in flat(), screen.text()
                elif case == 'unreadable':
                    send(b'\x1b', 'Saved session unavailable. F1 then h for details.')
                    assert GOAL.splitlines()[0] not in flat(), screen.text()
                else:
                    next_step = ('Next: confirm the seed draft in Shape.' if case == 'drafted' else
                                 'Next: check the seed in Shape.' if case in ('ready', 'stale') else
                                 'Next: shape it into a seed in Shape.')
                    send(b'\x1b', 'tinkery › tend')
                    wait(lambda: next_step in flat())
                    shows_goal()
            assert files(sessions) == before, 'Shape changed a session file'
            assert not (cwd / 'provider-was-run').exists()
            assert not (cwd / 'TEST-records').exists() and not (cwd / 'TEST-links').exists()
            print(f'Shape {case} journey passed at {width}x{height}' + (' NO_COLOR' if no_color else '')
                  + (' under --operator agent, writes held' if held else ''))
        finally:
            if process.poll() is None:
                process.kill()
                process.wait()
            os.close(master)
            os.close(slave)


if __name__ == '__main__':
    for size in [(100, 30), (80, 24)]:
        for case in ['question', 'p051', 'drafted', 'ready', 'stale', 'ended', 'unreadable', 'list']:
            journey(*size, case)
    journey(100, 30, 'question', no_color=True)
    journey(80, 24, 'p051', no_color=True)
    for case in ['question', 'drafted', 'ready']:
        journey(100, 30, case, operator='agent')
    journey(80, 24, 'question', no_color=True, operator='agent')
