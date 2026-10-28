"""Confirmed Think history survives restart; reads use temporary TEST roots and stub Pi only."""
import fcntl
import json
import os
import select
import signal
import struct
import test_launch as subprocess
import tempfile
import termios
import time
from pathlib import Path

from vt_screen import Screen

REPO = Path(__file__).resolve().parents[1]
BINARY = REPO / 'target/debug/tinkery'
SIZES = [(80, 24), (100, 30), (160, 40), (320, 40)]
MENU = '1  think'
GARDEN = '↑↓←→ move   enter open   l link   esc back'
DETAIL = 'esc back   F10 menu'
HOST = '''#!/usr/bin/env python3
import json, pathlib, sys
r = json.load(sys.stdin)
with pathlib.Path('calls.jsonl').open('a') as f: f.write(json.dumps(r)+'\\n')
if r.get('kind') == 'decision-review':
    print(json.dumps(dict(decisions=[dict(part=d['part'], text=d['text'],
        answers=d['answers'], question=d.get('question', 'May refusals show private text?'),
        suggestion=dict(answer='Omit private text.', reason='Keep it private.'), covers=[i])
        for i, d in enumerate(r.get('decisions', []))])))
else:
    marker = r['sources'][0]['text'].splitlines()[0]
    print(json.dumps(dict(parts=dict(who='I', outcome='finish '+marker,
        why='the work is stuck', done_when='tests pass for '+marker),
        decisions=[dict(part='must_not', text='Private text', answers=['omit', 'include'],
            question='May refusals show private text?')] if len(r['sources']) == 1 else [],
        questions=[])))
'''


class Terminal:
    def __init__(self, root, width, height, args, home=None, extra=None):
        self.master, self.slave = os.openpty()
        fcntl.ioctl(self.slave, termios.TIOCSWINSZ, struct.pack('HHHH', height, width, 0, 0))
        self.screen = Screen(width, height)
        self.process = subprocess.Popen([str(BINARY), '--link-root', str(root / 'TEST-links'), *map(str, args)],
            stdin=self.slave, stdout=self.slave, stderr=self.slave, cwd=root,
            env=subprocess.environment({**os.environ, 'TERM': 'xterm-256color', 'TINKERY_NODE': '/tinkery-TEST-no-node',
                                        **(extra or {})}, home=home))

    def wait(self, predicate, timeout=20):
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            if select.select([self.master], [], [], .02)[0]:
                try:
                    self.screen.feed(os.read(self.master, 65536))
                except OSError:
                    break
            if not self.screen.synchronized and predicate(self.screen.text()):
                return self.screen.text()
            if self.process.poll() is not None:
                break
        if not self.screen.synchronized and predicate(self.screen.text()):
            return self.screen.text()
        raise AssertionError(self.screen.text())

    def send(self, keys, text):
        os.write(self.master, keys)
        return self.wait(lambda screen: text in screen)

    def changed(self, keys):
        before = self.screen.text()
        os.write(self.master, keys)
        return self.wait(lambda screen: screen != before)

    def quit(self):
        if MENU not in self.screen.text():
            self.send(b'\x1b[21~', MENU)
        os.write(self.master, b'q')
        self.wait(lambda _: self.process.poll() is not None)
        assert self.process.returncode == 0

    def close(self):
        if self.process.poll() is None:
            self.process.kill()
            self.process.wait()
        os.close(self.master)
        os.close(self.slave)


def files(root):
    return {str(path.relative_to(root)): path.read_bytes()
            for path in root.rglob('*') if path.is_file()}


def confirm_goals(root, width, height, no_color):
    sessions, records = root / 'TEST-sessions', root / 'TEST-records'
    host = root / 'pi'
    host.write_text(HOST)
    host.chmod(0o755)
    terminal = Terminal(root, width, height, ['--home', '--shape-pi', '--shape-budget', '64', '--model', 'test/model',
        '--pi-command', host, '--seed-session-root', sessions,
        '--record-root', records, *(['--no-color'] if no_color else [])])
    saved = {}
    try:
        terminal.wait(lambda screen: MENU in screen)
        for marker in ['ALPHA café', 'BRAVO naïve']:
            original = marker + '\n\n' + '\n'.join(
                f'{marker} original {number:02}: keep every word.' for number in range(1, 37))
            answer = f'omit\n{marker}: gardez les mots exacts.'
            terminal.send(b'1', "What's on your mind?")
            terminal.send(b'\x1b[200~' + original.encode() + b'\x1b[201~', 'original 36:')
            terminal.send(b'\x1bOQ', 'May refusals show private text?')
            terminal.send(b'\x1b[200~' + answer.encode() + b'\x1b[201~', 'gardez les mots exacts.')
            terminal.send(b'\x1bOQ', "Ready for your review")
            terminal.send(b'\x1bOP', 'Review the goal')
            terminal.send(terminal.screen.pick('Review the goal'), 'You are confirming')
            terminal.send(b'confirm\r', 'Saved.')
            matches = [json.loads(path.read_text()) for path in records.glob('*.json')
                       if marker in path.read_text()]
            assert len(matches) == 1, matches
            record = matches[0]
            assert record['turns'][0]['text'] == original
            assert [turn['answer'] for turn in record['turns'] if turn['kind'] == 'answered'] == [answer]
            assert [turn['answer'] for turn in record['turns'] if turn['kind'] == 'suggested'] == ['Omit private text.']
            session = Path(record['seed_me_session_path'])
            assert session.is_absolute() and session.parent == sessions.resolve(), record
            ledger = json.loads((session / 'ledger.json').read_text())
            assert ledger['goal'] == record['goal']
            saved[marker] = record
            terminal.send(b'\x1b', MENU)
        terminal.quit()
    finally:
        terminal.close()
    return saved


def read_history(terminal, marker, record):
    terminal.send(b'\r', DETAIL)
    assert marker in terminal.screen.text(), terminal.screen.text()
    terminal.send(b'r', 'Think history')
    terminal.wait(lambda screen: 'Original words 1' in screen)
    first = terminal.screen.text()
    assert str(Path(record['seed_me_session_path']).parent) not in first, first
    frames = [first]
    frames.append(terminal.changed(b'\x1b[B'))
    assert terminal.changed(b'\x1b[A') == first
    frames.append(terminal.changed(b'\x1b[6~'))
    assert terminal.changed(b'\x1b[5~') == first
    frames.append(terminal.changed(b'\x1b[F'))
    assert 'Confirmed goal' in frames[-1], frames[-1]
    assert terminal.changed(b'\x1b[H') == first
    # One-line steps also prove no line disappears between pages or at the bottom.
    for _ in range(100):
        before = terminal.screen.text()
        os.write(terminal.master, b'\x1b[B')
        try:
            frames.append(terminal.wait(lambda screen: screen != before, timeout=.5))
        except AssertionError:
            break
    else:
        raise AssertionError('History never reached its end')
    visible = '\n'.join(frames)
    other = 'BRAVO' if marker.startswith('ALPHA') else 'ALPHA'
    assert other not in visible, visible
    assert 'test/model' not in visible and 'structure-spans-history' not in visible, visible
    expected = [record['turns'][0]['text'], record['goal']]
    expected += [turn['answer'] for turn in record['turns'] if turn['kind'] == 'answered']
    for text in expected:
        for line in text.splitlines():
            assert line in visible, (line, visible)
    assert 'Answer 2' in visible
    assert "Readings · Tinkery's guesses" in visible
    assert 'Reading 1 · shown ' in visible and 'Reading 2 · shown ' in visible
    assert 'Tinkery suggested (not an answer)' in visible
    assert 'Omit private text.' in visible and 'Why: Keep it private.' in visible
    assert 'May refusals show private text?' in visible
    terminal.send(b'\x1bOP', 'Right now')
    terminal.send(b'h', 'How it works')
    assert 'Model: test/model' in terminal.screen.text(), terminal.screen.text()
    terminal.send(b'\x1b', 'Right now')
    terminal.send(b'\x1b', 'Think history')
    terminal.send(b'\x1b', DETAIL)
    assert marker in terminal.screen.text(), terminal.screen.text()
    assert other not in terminal.screen.text(), terminal.screen.text()
    terminal.send(b'\x1b', GARDEN)


def journey(width, height, no_color):
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-think-history-') as directory:
        root = Path(directory).resolve()
        saved = confirm_goals(root, width, height, no_color)
        before = files(root)
        calls = len((root / 'calls.jsonl').read_text().splitlines())
        assert calls == 8, calls
        terminal = Terminal(root, width, height, ['--home', '--seed-session-root', root / 'TEST-sessions',
            '--record-root', root / 'TEST-records', *(['--no-color'] if no_color else [])])
        try:
            terminal.wait(lambda screen: MENU in screen)
            terminal.send(b'2', GARDEN)
            seen = set()
            for index in range(2):
                selected = terminal.screen.text().splitlines()[height-3]
                matches = [marker for marker in saved if marker in selected]
                assert len(matches) == 1, terminal.screen.text()
                marker = matches[0]
                assert marker not in seen
                seen.add(marker)
                read_history(terminal, marker, saved[marker])
                if index == 0:
                    terminal.changed(b'j')
            terminal.send(b'\r', DETAIL)
            terminal.send(b'r', 'Think history')
            terminal.quit()
            assert files(root) == before, 'Reading history, help or navigation changed source files'
            print(json.dumps(dict(width=width, height=height, no_color=no_color,
                journey='confirm two/restart/history isolation/Unicode/scroll/help/back/menu/read-only',
                confirmed_goals=2, stub_calls_during_confirmation=calls, history_provider_calls=0)))
            return saved['ALPHA café']
        finally:
            terminal.close()


def unavailable(kind, template):
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-history-unavailable-') as directory:
        root = Path(directory).resolve()
        session = root / 'TEST-sessions' / 'saved-goal'
        session.mkdir(parents=True)
        (session / 'ledger.json').write_text(json.dumps(dict(schema_version=1,
            session_id=template['seed_me_session_id'], status='active', goal=template['goal'],
            updated_at='2026-10-15T12:00:00+00:00')))
        records = root / 'TEST-records'
        if kind != 'missing':
            records.mkdir()
            record = {**template, 'seed_me_session': session.name, 'seed_me_session_path': str(session)}
            if kind == 'legacy':
                record['schema'] = 1
                del record['seed_me_session_path']
            (records / 'history.json').write_text('not JSON' if kind == 'corrupt' else json.dumps(record))
            if kind == 'duplicate':
                (records / 'another.json').write_text(json.dumps(record))
        before = files(root)
        terminal = Terminal(root, 100, 30, ['--home', '--seed-session-root', session.parent,
            '--record-root', records, '--no-color'])
        try:
            terminal.wait(lambda screen: MENU in screen)
            terminal.send(b'2', GARDEN)
            terminal.send(b'\r', DETAIL)
            terminal.send(b'r', 'Think history unavailable')
            cause = {'missing': 'No Think history was found for this saved goal.',
                     'legacy': 'This older Think history cannot be matched safely.',
                     'corrupt': 'Think history could not be read.',
                     'duplicate': 'More than one Think record matches this saved session.'}[kind]
            assert cause in terminal.screen.text(), terminal.screen.text()
            assert 'original 01:' not in terminal.screen.text(), terminal.screen.text()
            assert 'esc back' in terminal.screen.text(), terminal.screen.text()
            terminal.send(b'\x1b', DETAIL)
            assert 'ALPHA café' in terminal.screen.text(), terminal.screen.text()
            terminal.quit()
            assert files(root) == before, 'Unavailable history changed source files'
            print(json.dumps(dict(width=100, height=30, no_color=True,
                journey=f'{kind}/unavailable/back/read-only', provider_calls=0)))
        finally:
            terminal.close()


if __name__ == '__main__':
    # Long-history wrapping and resize are covered in src/tend/history/tests.rs.
    for size, no_color in [((100, 30), False), ((80, 24), False), ((80, 24), True)]:
        template = journey(*size, no_color)
    for kind in ['missing', 'legacy', 'corrupt', 'duplicate']:
        unavailable(kind, template)
