"""Long Think boundaries remain discoverable before review; fake Pi and TEST roots only."""
import fcntl
import json
import os
import select
import signal
import struct
import subprocess
import tempfile
import termios
import time
from pathlib import Path

from vt_screen import Screen

REPO = Path(__file__).resolve().parents[1]
BINARY = REPO / 'target/debug/tinkery'
SKILL = Path(os.environ['SEED_ME_TEST_SKILL']).resolve()
HELPER = SKILL.parent / 'scripts/session.py'
EXCLUSIONS = [
    'publish the café newsletter before its author checks every quotation, date, name, and link against the original notes',
    'replace the author’s wording with a guess about their intent, even when another sentence would appear shorter or more polished',
    'send a subscriber’s private address to another service, include hidden tracking, or schedule delivery without explicit consent from the editor',
]
PARTS = dict(
    who='I', outcome='prepare a café newsletter for its editor',
    why='readers need accurate local news',
    done_when='the editor can inspect every boundary before saving',
    must=['preserve the original notes'], must_not=EXCLUSIONS,
)
GOAL = '\n'.join([
    'I prepare a café newsletter for its editor.',
    'Why: readers need accurate local news.',
    'Done when: the editor can inspect every boundary before saving.',
    'Must: preserve the original notes.',
    *['Must not: ' + text + '.' for text in EXCLUSIONS],
])
DUMP = '  Prepare the café newsletter. Preserve these boundaries:\n' + '\n'.join(EXCLUSIONS) + '  '
HOST = '''#!/usr/bin/env python3
import json,sys
from pathlib import Path
r=json.load(sys.stdin)
with Path('calls.jsonl').open('a') as log: log.write(json.dumps(r)+'\\n')
if r.get('kind')=='decision-review': print('{"decisions":[]}');sys.exit(0)
if r.get('kind')=='meaning-preservation': print('{"missing":[],"false_choice":false}');sys.exit(0)
if r.get('kind')=='question-continuity': print('{"repeated":[]}');sys.exit(0)
parts=json.loads(Path('parts.json').read_text())
print(json.dumps(dict(parts=parts,questions=[],decisions=[],supports=[dict(source=s['id'],quote=s['text'],occurrence=0) for s in r['sources']],misfits=[])))
'''
PGDN, PGUP = b'\x1b[6~', b'\x1b[5~'
HINT = 'PgUp/PgDn scroll reading'


def helper(*args):
    return json.loads(subprocess.check_output(['python3', str(HELPER), *map(str, args)], text=True))


def journey(width, height, no_color):
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-reading-scroll-') as directory:
        cwd = Path(directory)
        sessions, records = cwd / 'TEST-sessions', cwd / 'TEST-records'
        host = cwd / 'pi'
        host.write_text(HOST)
        host.chmod(0o755)
        (cwd / 'parts.json').write_text(json.dumps(PARTS))
        master, slave = os.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', height, width, 0, 0))
        original = termios.tcgetattr(slave)
        screen = Screen(width, height)
        raw = bytearray()
        environment = {**os.environ, 'TERM': 'xterm-256color', 'HOME': str(cwd / 'TEST-home')}
        environment.pop('NO_COLOR', None)
        if no_color:
            environment['NO_COLOR'] = '1'
        process = subprocess.Popen(
            [str(BINARY), '--shape-pi', '--model', 'test/model', '--seed-me', str(SKILL),
             '--pi-command', str(host), '--seed-session-root', str(sessions), '--record-root', str(records)],
            stdin=slave, stdout=slave, stderr=slave, cwd=cwd, env=environment,
        )

        def feed():
            chunk = os.read(master, 65536)
            raw.extend(chunk)
            screen.feed(chunk)

        def wait(predicate, timeout=20):
            deadline = time.monotonic() + timeout
            while time.monotonic() < deadline:
                if select.select([master], [], [], .02)[0]:
                    feed()
                if not screen.synchronized and predicate(screen.text()):
                    return
                if process.poll() is not None:
                    break
            raise AssertionError(screen.text())

        def send(keys, text):
            os.write(master, keys)
            wait(lambda current: text in current)

        def step(keys):
            # Each page is observed before the next key, including a clamped final page.
            os.write(master, keys)
            deadline = time.monotonic() + .12
            while time.monotonic() < deadline or screen.synchronized:
                if select.select([master], [], [], .02)[0]:
                    feed()

        def reading():
            start = screen.width * 48 // 100 + 2
            rows = [''.join(row[start:screen.width - 2]).strip()
                    for row in screen.cells[4:screen.height - 14]]
            return ' '.join(' '.join(row for row in rows if 'PgUp/PgDn' not in row).split())

        def all_reading():
            step(PGUP * 40)
            seen = reading().split()
            last = reading()
            for _ in range(40):
                step(PGDN)
                current = reading()
                if current == last:
                    break
                words = current.split()
                overlap = next((n for n in range(min(len(seen), len(words)), 0, -1)
                                if seen[-n:] == words[:n]), 0)
                seen.extend(words[overlap:])
                last = current
            else:
                raise AssertionError('Reading never reached its final page')
            observed = ' '.join(seen)
            for exclusion in EXCLUSIONS:
                assert 'Must not: ' + exclusion + '.' in observed, (exclusion, observed)
            assert HINT in screen.text(), screen.text()

        def calls():
            return (cwd / 'calls.jsonl').read_bytes()

        def copied(keys):
            count = len(screen.clipboards)
            os.write(master, keys)
            wait(lambda _: len(screen.clipboards) > count)
            return screen.clipboards[-1]

        try:
            wait(lambda current: "What's on your mind?" in current)
            assert HINT not in screen.text()
            send(b'\x1b[200~' + DUMP.encode() + b'\x1b[201~', 'editor')
            send(b'\x1bOQ', "Nothing I'm unsure about.")
            assert EXCLUSIONS[-1] not in reading(), 'Fixture no longer reproduces initial clipping'
            # The original bug: no visible route tells the operator how to reach clipped boundaries.
            assert HINT in screen.text(), screen.text()
            initial_calls = calls()
            assert len(initial_calls.splitlines()) == 2, initial_calls
            assert not sessions.exists() and not records.exists()
            all_reading()
            send(PGUP * 40, 'I think you mean…')
            assert 'I prepare a café' in reading()

            # Help explains this existing route; opening and leaving it keeps the reading.
            send(b'\x1bOP', 'Right now')
            send(b'\x1bOP', 'Every key')
            help_seen = screen.text()
            for _ in range(20):
                if 'scroll the reading' in help_seen:
                    break
                step(PGDN)
                help_seen += '\n' + screen.text()
            assert 'scroll the reading' in help_seen, help_seen
            send(b'\x1b', 'I think you mean…')

            # Paging works from either focus and leaves unsent text intact.
            send(b'Keep this note', 'Keep this note')
            all_reading()
            assert copied(b'\x01\x03') == 'Keep this note'
            send(b'\t', HINT)
            all_reading()
            send(b'\t', HINT)
            assert copied(b'\x01\x03') == 'Keep this note'
            step(b'\x7f')
            assert calls() == initial_calls

            # Reflow in the same process must keep every exclusion reachable.
            new_width, new_height = (100, 30) if width == 80 else (80, 24)
            fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', new_height, new_width, 0, 0))
            screen = Screen(new_width, new_height)
            send(b'\x0c', HINT)
            all_reading()
            provisional = copied(b'\x1b[20~')
            assert GOAL in provisional and DUMP in provisional
            assert not sessions.exists() and not records.exists()

            send(b'\x1b[21~', 'Back to menu and lose this draft? y / n')
            step(b'n')
            assert process.poll() is None and 'lose this draft?' not in screen.text()
            send(b'\x1bOR', 'type confirm')
            step(b'\r')
            assert not sessions.exists() and not records.exists()
            step(PGDN * 40)
            send(b'confirm\r', 'Saved.')
            session = next(sessions.iterdir())
            ledger = helper('read', session)
            origin = next(node for node in ledger['nodes'] if node['id'] == ledger['origin'])
            assert ledger['goal'] == origin['answer'] == GOAL
            record_file = next(records.iterdir())
            record = json.loads(record_file.read_text())
            assert record['goal'] == GOAL and record['turns'][0]['text'] == DUMP
            saved = (session / 'ledger.json').read_bytes(), record_file.read_bytes()
            handoff = copied(b'c')
            assert str(session) in handoff and 'Do not initialize a new session or replace the confirmed goal.' in handoff
            assert calls() == initial_calls
            assert saved == ((session / 'ledger.json').read_bytes(), record_file.read_bytes())

            send(b'\x1b[21~', '1  think')
            assert helper('status', session)['viewer'] == 'started earlier, not running now'
            os.write(master, b'\x1b[21~')
            wait(lambda _: process.poll() is not None, timeout=10)
            assert process.returncode == 0
            while select.select([master], [], [], .02)[0]:
                feed()
            after = termios.tcgetattr(slave)
            original[3] &= ~getattr(termios, 'PENDIN', 0)
            after[3] &= ~getattr(termios, 'PENDIN', 0)
            assert after == original
            assert calls() == initial_calls
            print(json.dumps(dict(width=width, height=height, no_color=no_color,
                                  resized_to=[new_width, new_height], provider='stub', provider_calls=2,
                                  exclusions=len(EXCLUSIONS), exact_goal=True,
                                  test_confirmation='AUTOMATED TEST, NOT OPERATOR',
                                  journey='discover/page/help/input/resize/review/save/copy/exit')))
        finally:
            if process.poll() is None:
                process.kill()
                process.wait()
            # Stop only a viewer whose command names this synthetic TEST session.
            if sessions.exists():
                for session in sessions.iterdir():
                    viewer = session / 'viewer.json'
                    if viewer.exists():
                        pid = json.loads(viewer.read_text()).get('pid')
                        if pid:
                            command = subprocess.run(['ps', '-p', str(pid), '-o', 'command='], capture_output=True, text=True)
                            if str(SKILL.parent / 'scripts/viewer.py') in command.stdout and str(session.resolve()) in command.stdout:
                                try:
                                    os.kill(pid, signal.SIGTERM)
                                except ProcessLookupError:
                                    pass
            os.close(master)
            os.close(slave)


for size in [(80, 24), (100, 30)]:
    for no_color in [False, True]:
        journey(*size, no_color)
