"""The Model panel through real keys (#422): saving, later launches, flags, top-up and refusals.

Fake Pi on PATH, temporary TEST homes and roots, no providers.
"""
import json
import os
from pathlib import Path
import tempfile

from think_history_terminal import Terminal

HOST = '''#!/usr/bin/env python3
import json, pathlib, sys
request = json.load(sys.stdin)
model = sys.argv[sys.argv.index('--model') + 1]
thinking = sys.argv[sys.argv.index('--thinking') + 1]
with pathlib.Path('TEST-calls.jsonl').open('a') as log:
    log.write(json.dumps(dict(model=model, thinking=thinking)) + '\\n')
if request.get('kind') == 'decision-review':
    print('{"decisions":[]}')
else:
    print(json.dumps(dict(parts=dict(who='I', outcome='keep TEST setup words',
        why='the setup matters', done_when='the saved model drafts'),
        open=['why'], questions=[dict(id='reason', target='why', text='Which TEST reason?')])))
'''
F1, F2, ESC, DOWN = b'\x1bOP', b'\x1bOQ', b'\x1b', b'\x1b[B'


class Launch:
    def __init__(self, root, home, args, size=(100, 30), operator='human'):
        self.root = root
        bin_dir = root / 'TEST-bin'
        self.terminal = Terminal(root, *size, ['--operator', operator, '--seed-session-root', root / 'TEST-sessions',
                                               '--record-root', root / 'TEST-records', *args],
                                 home=home, extra={'PATH': f"{bin_dir}{os.pathsep}{os.environ['PATH']}"})

    def __enter__(self):
        return self.terminal

    def __exit__(self, *_):
        self.terminal.close()


def calls(root):
    path = root / 'TEST-calls.jsonl'
    return [json.loads(line) for line in path.read_text().splitlines()] if path.exists() else []


def header(terminal):
    return terminal.screen.text().splitlines()[1]


def pick(terminal, label):
    """Open F1 and press the digit of the row naming `label`."""
    terminal.send(F1, label)
    terminal.send(terminal.screen.pick(label), 'Model ')


def journey(no_color):
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-model-') as directory:
        root = Path(directory)
        home = root / 'TEST-home'
        home.mkdir()
        (root / 'TEST-bin').mkdir()
        host = root / 'TEST-bin' / 'pi'
        host.write_text(HOST)
        host.chmod(0o700)
        saved = home / '.config/tinkery/model.json'
        colour = ['--no-color'] if no_color else []

        # No file: practice, unchanged. m opens the panel; esc writes nothing.
        with Launch(root, home, ['--home', *colour]) as terminal:
            terminal.wait(lambda screen: '1  think' in screen)
            assert 'practice' in header(terminal), header(terminal)
            terminal.send(b'm', 'Model     none')
            terminal.send(b'\r', 'enter done')
            terminal.send(b'test/saved', 'test/saved')
            terminal.send(b'\r', '↑↓ choose')
            terminal.send(DOWN + DOWN + b'\r', '5 each launch')
            terminal.send(ESC, '1  think')
            assert not saved.exists(), 'esc wrote the setup'
            terminal.send(b'm', 'Model     none')
            terminal.send(b'\r' + b'test/saved' + b'\r', '↑↓ choose')
            terminal.send(DOWN + b'\r', 'medium')
            terminal.send(DOWN + b'\r', '5 each launch')
            terminal.send(b's', 'drafts with it.')
            assert json.loads(saved.read_text()) == dict(model='test/saved', thinking='medium', calls=5)
            assert 'practice' in header(terminal), 'saving changed this launch'
            terminal.send(ESC, '1  think')
            terminal.quit()
        assert calls(root) == []

        # A later plain launch drafts with the saved setup; launching starts nothing, F2 sends.
        with Launch(root, home, colour) as terminal:
            terminal.wait(lambda screen: "What's on your mind?" in screen)
            assert '5 calls left' in header(terminal), header(terminal)
            assert calls(root) == []
            terminal.send(b'TEST saved words', 'TEST saved words')
            terminal.send(F2, 'Which TEST reason?')
            assert calls(root) and all(c == dict(model='test/saved', thinking='medium') for c in calls(root)), calls(root)
            terminal.wait(lambda _: f'{5 - len(calls(root))} calls left' in header(terminal))

        # Flags win for one launch and write nothing.
        before = saved.read_text()
        start = len(calls(root))
        with Launch(root, home, ['--shape-pi', '--model', 'test/flag', '--shape-budget', '1', *colour]) as terminal:
            terminal.wait(lambda _: '1 call left' in header(terminal))
            terminal.send(b'TEST flag words', 'TEST flag words')
            terminal.send(F2, 'Which TEST reason?')
            assert [c['model'] for c in calls(root)[start:]] == ['test/flag'], calls(root)
            # The thinking flag was not given, so the saved level applies.
            assert calls(root)[-1]['thinking'] == 'medium'
        assert saved.read_text() == before

        # Out of calls: words stay, nothing starts; F1's row adds calls for this launch only.
        start = len(calls(root))
        with Launch(root, home, ['--shape-pi', '--model', 'test/flag', '--shape-budget', '0', *colour]) as terminal:
            terminal.wait(lambda _: '0 calls left' in header(terminal))
            terminal.send(b'TEST topped up words', 'TEST topped up words')
            os.write(terminal.master, F2)
            terminal.wait(lambda screen: 'Add calls to keep going.' in ' '.join(screen.split()))
            assert len(calls(root)) == start
            assert 'TEST topped up words' in terminal.screen.text()
            pick(terminal, 'Model and calls')
            terminal.send(DOWN * 3 + b'\r', '5 left · enter adds 5')
            terminal.send(ESC, 'TEST topped up words')
            assert '5 calls left' in header(terminal), header(terminal)
            terminal.send(F2, 'Which TEST reason?')
            assert len(calls(root)) > start
        assert saved.read_text() == before, 'adding calls saved them'

        # An agent never inherits the saved calls, and can't save a setup or add calls.
        with Launch(root, home, colour, operator='agent') as terminal:
            terminal.wait(lambda screen: "What's on your mind?" in screen)
            assert 'practice' in header(terminal), header(terminal)
        with Launch(root, home, ['--shape-pi', '--shape-budget', '0', *colour], operator='agent') as terminal:
            terminal.wait(lambda _: '0 calls left' in header(terminal))
            terminal.send(b'TEST agent words', 'TEST agent words')
            pick(terminal, 'Model and calls')
            terminal.send(b's', "can't save a setup or add calls")
            terminal.send(DOWN * 3 + b'\r', '0 left · enter adds 5')
            assert '0 calls left' in header(terminal)
            terminal.send(ESC, 'TEST agent words')
        assert saved.read_text() == before, 'an agent changed the setup'

        # An invalid file is refused with its path, and the launch stays practice.
        saved.write_text('not json')
        with Launch(root, home, ['--home', *colour]) as terminal:
            terminal.wait(lambda screen: '1  think' in screen)
            assert 'practice' in header(terminal)
            shown = terminal.send(b'm', 'is not a model setup')
            assert 'using practice' in ' '.join(shown.split()), shown
            terminal.send(ESC, '1  think')
            terminal.quit()
        print(f'Model panel no_color={no_color}: {len(calls(root))} fake Pi starts')


if __name__ == '__main__':
    for no_color in [False, True]:
        journey(no_color)
