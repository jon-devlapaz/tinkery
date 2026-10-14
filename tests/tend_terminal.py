"""Read-only tend journeys with synthetic fixture sessions; no model calls or operator confirmation."""
import base64, fcntl, json, os, re, select, struct, subprocess, tempfile, termios, time
from datetime import datetime, timedelta, timezone
from pathlib import Path
from vt_screen import Screen

REPO = Path(__file__).resolve().parents[1]
SKILL = Path(os.environ['SEED_ME_TEST_SKILL']).resolve()
GOAL = ('I can read a calm dark theme while preserving every confirmed goal line and keeping a clear next step with manual control.\n'
        'Why: I work at night.\nDone when: Nothing strands me.\n'
        'Must: Keep manual control.\nMust not: Run agents.')


GARDEN_KEYS = '↑↓←→ move   enter open   esc back'
LABELS = ['Calm theme', 'Clear next step', 'Manual control', 'Keep originals',
          'Quiet garden', 'Readable goals', 'Local records']


def journey(width, height, kind='garden'):
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-tend-') as directory:
        root = Path(directory).resolve()
        sessions = root / 'TEST-sessions'
        now = datetime.now(timezone.utc)
        count = 30 if kind == 'paging' else {160: 8, 320: 16}.get(width, 7)
        labels = LABELS + [f'Garden goal {i+1:02}' for i in range(len(LABELS), count)]
        states = [('saved', 'active', None), ('drafted', 'active', 'status: draft'),
                  ('ready', 'active', 'status: confirmed for intake'),
                  ('ended', 'completed', 'status: confirmed for intake')]
        states += [(f'goal-{i:02}', 'active', None) for i in range(4, count)]
        for i, (name, status, seed) in enumerate(states):
            path = sessions / name
            path.mkdir(parents=True)
            (path / 'ledger.json').write_text(json.dumps(dict(status=status, goal=labels[i]+'\n'+GOAL,
                updated_at=(now-timedelta(hours=i)).isoformat())))
            if seed: (path / 'seed-contract.md').write_text(seed+'\n# Synthetic fixture only\n')
        if kind == 'unreadable':
            broken = sessions / 'unreadable'
            broken.mkdir()
            (broken / 'ledger.json').write_text('not JSON')
        before = {str(p): p.read_bytes() for p in sessions.rglob('*') if p.is_file()}
        host = root / 'never-run-pi'
        host.write_text('#!/bin/sh\necho unexpected > provider-was-run\nexit 86\n')
        host.chmod(0o755)
        master, slave = os.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', height, width, 0, 0))
        screen = Screen(width, height)
        raw = bytearray()
        process = subprocess.Popen([str(REPO/'target/debug/tinkery'), '--home', '--shape-pi', '--record-root', str(root / 'TEST-records'),
            '--model', 'test/model', '--seed-me', str(SKILL), '--seed-session-root', str(sessions),
            '--pi-command', str(host)], stdin=slave, stdout=slave, stderr=slave, cwd=root,
            env={**os.environ, 'TERM': 'xterm-256color', 'TINKERY_NODE': '/tinkery-TEST-no-node'})

        def feed():
            data = os.read(master, 65536)
            raw.extend(data)
            screen.feed(data)

        def wait(predicate):
            deadline = time.monotonic()+5
            while time.monotonic() < deadline:
                if select.select([master], [], [], .02)[0]: feed()
                if predicate() and not screen.synchronized: return
                if process.poll() is not None: break
            raise AssertionError(screen.text())

        def send(data, text):
            os.write(master, data)
            wait(lambda: text in screen.text())

        def copies():
            return re.findall(rb'\x1b\]52;c;([^\x07\x1b]+)(?:\x07|\x1b\\)', bytes(raw))

        def ordinary():
            assert str(root) not in screen.text() and 'any harness' not in screen.text(), screen.text()

        def garden_lines():
            return screen.text().splitlines()[3:height-4]

        def position(label):
            found = [(line.index(label), row+3) for row, line in enumerate(garden_lines()) if label in line]
            assert len(found) == 1, (label, screen.text())
            return found[0]

        def selected(index):
            lines = screen.text().splitlines()
            return (lines[-1].strip() == GARDEN_KEYS
                    and sum(line.count('› ') for line in garden_lines()) == 1
                    and any('› '+labels[index] in line for line in garden_lines())
                    and lines[height-3].startswith('  '+labels[index]))

        def move(key, index):
            os.write(master, key)
            wait(lambda: selected(index))

        def click(column, row):
            return (f'\x1b[<0;{column+1};{row+1}M'
                    f'\x1b[<0;{column+1};{row+1}m').encode()

        try:
            wait(lambda: '1  think' in screen.text())
            send(b'2', GARDEN_KEYS)
            wait(lambda: selected(0))
            visible = labels if kind != 'paging' else [label for label in labels
                                                       if any(label in line for line in garden_lines())]
            positions = [position(label) for label in visible]
            assert positions == sorted(positions, key=lambda point: (point[1], point[0])), screen.text()
            for label, (column, row) in zip(visible, positions):
                for ring_row in screen.text().splitlines()[row+1:row+3]:
                    assert any('\u2801' <= char <= '\u28ff' for char in ring_row[column:column+len(label)]), (label, screen.text())
            line = screen.text().splitlines()[height-3]
            assert 'goal saved · ' in line, line
            assert 'Why:' not in screen.text() and 'Done when:' not in screen.text()
            assert 'I can read' not in screen.text(), screen.text()
            ordinary()
            if kind == 'paging':
                assert len(visible) < len(labels), screen.text()
                for index in range(1, count): move(b'j', index)
                line = screen.text().splitlines()[height-3]
                page = re.search(r'page (\d+) of (\d+)', line)
                assert page and int(page[1]) == int(page[2]) > 1, line
                assert not any(label in screen.text() for label in visible), screen.text()
            elif kind == 'unreadable':
                assert "1 saved record couldn't be read; this view may be missing sessions." in screen.text(), screen.text()
            else:
                move(b'\x1b[C', 1)
                move(b'\x1b[D', 0)
                move(b'j', 1)
                move(b'k', 0)
                below = next(i for i, (_, row) in enumerate(positions) if row > positions[0][1])
                move(b'\x1b[B', below)
                move(b'\x1b[A', 0)
                column, row = position(labels[1])
                os.write(master, click(column, row))
                wait(lambda: selected(1))
                assert screen.text().splitlines()[-1].strip() == GARDEN_KEYS, screen.text()
                time.sleep(.55)
                os.write(master, click(column, row) * 2)
                wait(lambda: 'c copy prompt   esc back   10 menu' in screen.text())
                assert labels[1] in screen.text(), screen.text()
                send(b'\x1b', GARDEN_KEYS)
                wait(lambda: selected(1))
                move(b'k', 0)
            for index, next_line in enumerate(['Next: shape it into a seed with Seed Me.',
                    'Next: confirm the seed draft with Seed Me.', 'Next: the seed is ready for intake.', None]
                    if kind == 'garden' else []):
                label = ['goal saved', 'seed drafted', 'seed ready', 'ended'][index]
                wait(lambda: label in screen.text().splitlines()[height-3])
                send(b'\r', 'esc back   10 menu')
                assert 'Saved.' not in screen.text()
                assert labels[index] in screen.text(), screen.text()
                for line in GOAL.splitlines(): assert line in ' '.join(screen.text().split()), (line, screen.text())
                ordinary()
                if next_line:
                    assert next_line in screen.text(), screen.text()
                    for _ in range(2):
                        count = len(copies())
                        os.write(master, b'c')
                        wait(lambda: len(copies()) == count+1)
                        text = base64.b64decode(copies()[-1]).decode()
                        target = sessions / ['saved','drafted','ready'][index]
                        assert f'"{target}"' in text and f'"{SKILL}"' in text
                        wait(lambda: 'Copied.' in screen.text())
                    assert copies()[-1] == copies()[-2]
                else:
                    assert 'Next:' not in screen.text() and 'c copy prompt' not in screen.text()
                    count = len(copies())
                    send(b'c\x1bOP', 'Right now')
                    assert len(copies()) == count and 'c copy prompt' not in screen.text()
                    send(b'\x1b', 'esc back   10 menu')
                send(b'\x1bOP', 'Right now')
                ordinary()
                send(b'h', 'How it works')
                assert 'Session:' in screen.text() and str(root) in screen.text().replace('\n','').replace(' ',''), screen.text()
                send(b'\x1b', 'Right now')
                send(b'\x1b', 'esc back   10 menu')
                send(b'\x1b', GARDEN_KEYS)
                wait(lambda: selected(index))
                if index < 3: move(b'j', index+1)
            send(b'\x1b', '1  think')
            send(b'2', GARDEN_KEYS)
            send(b'\x1b[21~', '1  think')
            assert process.poll() is None
            os.write(master, b'\x1b[21~')
            wait(lambda: process.poll() is not None)
            assert process.returncode == 0
            assert not (root/'provider-was-run').exists()
            assert {str(p): p.read_bytes() for p in sessions.rglob('*') if p.is_file()} == before
            name = 'garden/arrows/jk/mouse/status/detail/repeat copy/help/back/menu' if kind == 'garden' else 'garden/'+kind
            print(json.dumps(dict(width=width, height=height, journey=name,
                provider_calls=0, sessions='synthetic fixtures in temporary root', operator_confirmation=False)))
        finally:
            if process.poll() is None: process.kill(); process.wait()
            os.close(master); os.close(slave)


for size in [(80,24), (100,30), (160,40), (320,40)]: journey(*size)
journey(80, 24, 'paging')
journey(80, 24, 'unreadable')
