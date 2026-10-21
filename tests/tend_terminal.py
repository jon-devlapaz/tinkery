"""Tend journeys with synthetic sessions and explicit TEST links; no model calls."""
import base64, fcntl, json, os, re, select, struct, tempfile, termios, time
import test_launch as subprocess
from datetime import datetime, timedelta, timezone
from pathlib import Path
from vt_screen import Screen

REPO = Path(__file__).resolve().parents[1]
GOAL = ('I can read a calm dark theme while preserving every confirmed goal line and keeping a clear next step with manual control.\n'
        'Why: I work at night.\nDone when: Nothing strands me.\n'
        'Must: Keep manual control.\nMust not: Run agents.')


GARDEN_KEYS = '↑↓←→ move   enter open   l link   esc back'
LINK_KEYS = '↑↓←→ pick   enter link   esc cancel'
UNLINK_KEYS = '↑↓←→ pick   enter unlink   esc cancel'
LABELS = ['Calm theme', 'Clear next step', 'Manual control', 'Keep originals',
          'Quiet garden', 'Readable goals', 'Local records']


def journey(width, height, kind='garden'):
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-tend-') as directory:
        root = Path(directory).resolve()
        sessions = root / 'TEST-sessions'
        now = datetime.now(timezone.utc)
        count = (30 if kind == 'whole-graph' else {160: 8, 320: 16}.get(width, 7)) + 1
        labels = LABELS + [f'Garden goal {i+1:02}' for i in range(len(LABELS), count)]
        states = [('saved', 'active', None), ('drafted', 'active', 'status: draft'),
                  ('ready', 'active', 'status: confirmed for intake'),
                  ('ended', 'completed', 'status: confirmed for intake')]
        states += [(f'goal-{i:02}', 'active', None) for i in range(4, count)]
        if kind == 'linked-rings':
            states = states[:2] + states[3:4]
            labels = labels[:2] + labels[3:4]
            count = len(states)
        for i, (name, status, seed) in enumerate(states):
            path = sessions / name
            path.mkdir(parents=True)
            (path / 'ledger.json').write_text(json.dumps(dict(session_id=name, status=status, goal=labels[i]+'\n'+GOAL,
                updated_at=(now-timedelta(hours=i)).isoformat())))
            if seed: (path / 'seed-contract.md').write_text(seed+'\n# Synthetic fixture only\n')
        ended_goal = labels.pop(next(i for i, (_, status, _) in enumerate(states) if status == 'completed'))
        count -= 1
        if kind == 'unreadable':
            broken = sessions / 'unreadable'
            broken.mkdir()
            (broken / 'ledger.json').write_text('not JSON')
        before = {str(p): p.read_bytes() for p in sessions.rglob('*') if p.is_file()}
        host = root / 'never-run-pi'
        host.write_text('#!/bin/sh\necho unexpected > provider-was-run\nexit 86\n')
        host.chmod(0o755)
        def launch():
            master, slave = os.openpty()
            fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', height, width, 0, 0))
            process = subprocess.Popen([str(REPO/'target/debug/tinkery'), '--home', '--shape-pi', '--record-root', str(root / 'TEST-records'),
                '--model', 'test/model', '--seed-session-root', str(sessions),
                '--link-root', str(root / 'TEST-links'),
                '--pi-command', str(host)], stdin=slave, stdout=slave, stderr=slave, cwd=root,
                env=subprocess.environment({**os.environ, 'TERM': 'xterm-256color', 'TINKERY_NODE': '/tinkery-TEST-no-node'}))
            return process, master, slave, Screen(width, height), bytearray()

        process, master, slave, screen, raw = launch()

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
            assert ended_goal not in screen.text(), screen.text()
            assert screen.text().splitlines()[height-4].strip().startswith('1 ended not shown'), screen.text()
            visible = [label for label in labels if any(label in line for line in garden_lines())]
            assert any('\u2801' <= char <= '\u28ff' for line in garden_lines() for char in line), screen.text()
            assert 'page ' not in screen.text(), screen.text()
            if width == 100 and kind == 'garden':
                print('TEST Tend 100×30 screen:\n'+screen.text())
            line = screen.text().splitlines()[height-3]
            assert 'goal saved · ' in line, line
            assert 'Why:' not in screen.text() and 'Done when:' not in screen.text()
            assert 'I can read' not in screen.text(), screen.text()
            ordinary()
            if kind == 'held-lock':
                link_root = root / 'TEST-links'
                link_root.mkdir()
                link_file = link_root / 'needs.json'
                saved = b'{"version":1,"needs":[]}'
                link_file.write_bytes(saved)
                with (link_root / 'needs.lock').open('w') as lock:
                    fcntl.flock(lock, fcntl.LOCK_EX)
                    send(b'l', LINK_KEYS)
                    send(b'j', 'Calm theme needs Clear next step')
                    start = time.monotonic()
                    send(b'\r', "Links couldn't be saved, so nothing changed. F1 then h for details.")
                    assert time.monotonic()-start < 3
                    send(b'\x1bOP', 'Right now')
                    send(b'h', 'another Tinkery is saving links')
                    send(b'\x1b', 'Right now')
                    send(b'\x1b', GARDEN_KEYS)
                    for back in [b'\x1b', b'\x1b[21~']:
                        send(b'l', LINK_KEYS)
                        send(b'j', 'Calm theme needs Clear next step')
                        start = time.monotonic()
                        send(b'\r'+back, '1  think')
                        assert time.monotonic()-start < 3
                        assert link_file.read_bytes() == saved
                        send(b'2', GARDEN_KEYS)
                wait(lambda: selected(0))
                move(b'j', 1)
                move(b'k', 0)
                assert link_file.read_bytes() == saved, 'Refusal must never queue a later save'
            elif kind in ('link-keyboard', 'link-mouse', 'linked-rings'):
                send(b'l', LINK_KEYS)
                assert 'Calm theme needs …' in screen.text(), screen.text()
                if kind == 'link-mouse':
                    column, row = position(labels[1])
                    os.write(master, click(column+len(labels[1])//2, row))
                else:
                    send(b'j', 'Calm theme needs Clear next step')
                    assert '› Calm theme' in screen.text() and '→ Clear next step' in screen.text(), screen.text()
                    ordinary()
                    os.write(master, b'\r')
                wait(lambda: selected(0) and 'needs Clear next step · not ready' in screen.text().splitlines()[height-3])
                link_file = root / 'TEST-links' / 'needs.json'
                expected = dict(version=1, needs=[dict(goal='saved', needs='drafted')])
                assert json.loads(link_file.read_text()) == expected
                if kind == 'linked-rings':
                    # Finish selection-following before dragging, then separate the two rings.
                    move(b'j', 1)
                    a, b = [position(label) for label in labels]
                    x, y = b[0]+len(labels[1])//2, b[1]
                    source_x = a[0]+len(labels[0])//2
                    direction = 1 if source_x < width//2 else -1
                    target_x, target_y = source_x+direction*(width//3), a[1]
                    os.write(master, (f'\x1b[<0;{x+1};{y+1}M'
                                      f'\x1b[<32;{target_x+1};{target_y+1}M'
                                      f'\x1b[<0;{target_x+1};{target_y+1}m').encode())
                    wait(lambda: selected(1)
                         and abs(position(labels[1])[0]+len(labels[1])//2-target_x) <= 2
                         and abs(position(labels[1])[1]-target_y) <= 1)
                    move(b'k', 0)
                    a, b = [position(label) for label in labels]
                    x = round((a[0]+len(labels[0])/2 + b[0]+len(labels[1])/2)/2)
                    y = round((a[1]+b[1])/2)+3
                    assert all(any('\u2801' <= screen.cells[row][col] <= '\u28ff'
                                   for row in range(y-1, y+2))
                               for col in range(x-2, x+3)), screen.text()
                    if width == 100:
                        print('TEST linked rings 100×30 screen:\n'+screen.text())
                send(b'\x1b[21~', '1  think')
                os.write(master, b'\x1b[21~')
                wait(lambda: process.poll() is not None)
                assert process.returncode == 0
                os.close(master); os.close(slave)
                process, master, slave, screen, raw = launch()
                wait(lambda: '1  think' in screen.text())
                send(b'2', GARDEN_KEYS)
                wait(lambda: selected(0) and 'needs Clear next step · not ready' in screen.text().splitlines()[height-3])
                ordinary()
                send(b'l', LINK_KEYS)
                if kind == 'link-mouse':
                    column, row = position(labels[1])
                    os.write(master, click(column+len(labels[1])//2, row))
                else:
                    send(b'j', UNLINK_KEYS)
                    send(b'\r', GARDEN_KEYS)
                wait(lambda: selected(0) and 'needs Clear next step' not in screen.text())
                assert json.loads(link_file.read_text()) == dict(version=1, needs=[])
            elif kind == 'whole-graph':
                for index in range(1, count): move(b'j', index)
                assert 'page ' not in screen.text(), screen.text()
                assert selected(count-1), screen.text()
            elif kind == 'drag-scroll':
                for key in [b'+', b'-', b'=', b'-', b'H', b'L', b'J', b'K']:
                    view_before = garden_lines()
                    os.write(master, key)
                    try:
                        wait(lambda: garden_lines() != view_before)
                    except AssertionError as error:
                        raise AssertionError(f'view key {key!r}: {error}') from error
                    assert GARDEN_KEYS in screen.text() and not (root/'TEST-links').exists()
                column, row = position(labels[0])
                x, y = column+len(labels[0])//2, row
                view_before = garden_lines()
                os.write(master, (f'\x1b[<0;{x+1};{y+1}M'
                                  f'\x1b[<32;{x+7};{y+2}M'
                                  f'\x1b[<0;{x+7};{y+2}m').encode())
                wait(lambda: garden_lines() != view_before)
                moved = position(labels[0])
                assert abs(moved[0]-column-6) <= 2 and abs(moved[1]-row-1) <= 1, screen.text()
                view_before = garden_lines()
                os.write(master, f'\x1b[<64;{width//2};{height//2}M'.encode())
                wait(lambda: garden_lines() != view_before)
                view_before = garden_lines()
                os.write(master, f'\x1b[<65;{width//2};{height//2}M'.encode())
                wait(lambda: garden_lines() != view_before)
                # A corner is empty: drag it to pan without moving a goal or opening detail.
                view_before = garden_lines()
                os.write(master, b'\x1b[<0;4;5M\x1b[<32;9;6M\x1b[<0;9;6m')
                wait(lambda: garden_lines() != view_before)
                assert GARDEN_KEYS in screen.text(), screen.text()
                move(b'j', 1)
            elif kind == 'unreadable':
                note = "1 ended not shown · 1 saved record couldn't be read; this view may be missing sessions."
                assert screen.text().splitlines()[height-4].strip() == note[:width-4], screen.text()
            else:
                move(b'\x1b[C', 1)
                move(b'\x1b[D', 0)
                move(b'j', 1)
                move(b'k', 0)
                move(b'\x1b[B', 1)
                move(b'\x1b[A', 0)
                column, row = position(labels[1])
                column += len(labels[1])//2
                os.write(master, click(column, row))
                wait(lambda: selected(1))
                assert screen.text().splitlines()[-1].strip() == GARDEN_KEYS, screen.text()
                time.sleep(.55)
                os.write(master, click(column, row) * 2)
                wait(lambda: 's shape   r history   esc back   F10 menu' in screen.text())
                assert labels[1] in screen.text(), screen.text()
                send(b'\x1b', GARDEN_KEYS)
                wait(lambda: selected(1))
                move(b'k', 0)
            for index, next_line in enumerate(['Next: shape it into a seed in Shape.',
                    'Next: confirm the seed draft in Shape.', 'Next: check the seed in Shape.']
                    if kind == 'garden' else []):
                label = ['goal saved', 'seed drafted', 'seed needs checking'][index]
                wait(lambda: label in screen.text().splitlines()[height-3])
                send(b'\r', 'esc back   F10 menu')
                assert 'Saved.' not in screen.text()
                assert labels[index] in screen.text(), screen.text()
                for line in GOAL.splitlines(): assert line in ' '.join(screen.text().split()), (line, screen.text())
                ordinary()
                assert next_line in screen.text(), screen.text()
                os.write(master, b'c'); time.sleep(.1)
                assert not copies(), 'Retired continuation prompt was copied'
                send(b'\x1bOP', 'Right now')
                ordinary()
                send(b'h', 'How it works')
                assert 'Session:' in screen.text() and str(root) in screen.text().replace('\n','').replace(' ',''), screen.text()
                send(b'\x1b', 'Right now')
                send(b'\x1b', 'esc back   F10 menu')
                send(b'\x1b', GARDEN_KEYS)
                wait(lambda: selected(index))
                if index < 2: move(b'j', index+1)
            send(b'\x1b', '1  think')
            send(b'2', GARDEN_KEYS)
            send(b'\x1b[21~', '1  think')
            assert process.poll() is None
            os.write(master, b'\x1b[21~')
            wait(lambda: process.poll() is not None)
            assert process.returncode == 0
            assert not (root/'provider-was-run').exists()
            if kind not in ('link-keyboard', 'link-mouse', 'linked-rings', 'held-lock'):
                assert not (root/'TEST-links').exists(), 'Navigation must not create a links store'
            assert {str(p): p.read_bytes() for p in sessions.rglob('*') if p.is_file()} == before
            name = 'garden/arrows/jk/mouse/status/detail/repeat copy/help/back/menu' if kind == 'garden' else 'garden/'+kind
            print(json.dumps(dict(width=width, height=height, journey=name,
                provider_calls=0, sessions='synthetic fixtures in temporary root', operator_confirmation=False)))
        finally:
            if process.poll() is None: process.kill(); process.wait()
            os.close(master); os.close(slave)


for size in [(80,24), (100,30), (160,40), (320,40)]: journey(*size)
journey(80, 24, 'whole-graph')
for size in [(80,24), (100,30), (160,40), (320,40)]: journey(*size, 'drag-scroll')
journey(80, 24, 'unreadable')
journey(100, 30, 'held-lock')
for size in [(80,24), (100,30), (160,40)]:
    journey(*size, 'link-keyboard')
    journey(*size, 'link-mouse')
    journey(*size, 'linked-rings')
