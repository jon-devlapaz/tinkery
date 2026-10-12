"""Original selection by keyboard and mouse: Tab, Shift-arrows, Ctrl-C, exact OSC 52 bytes.
Plain POSIX PTY, stub provider (counts invocations), temporary cwd. No audio, no live calls."""
import fcntl, json, os, select, signal, struct, subprocess, tempfile, termios, time
from pathlib import Path
from vt_screen import Screen
BINARY = Path(__file__).resolve().parents[1] / 'target/debug/tinkery'
HOST = '''#!/usr/bin/env python3
import json,sys
r=json.load(sys.stdin)
if r.get('kind') in ('meaning-preservation','question-continuity'):
 print(json.dumps(dict(missing=[],false_choice=False) if r['kind']=='meaning-preservation' else dict(repeated=[])));sys.exit(0)
if r.get('kind')=='decision-review':
 print(json.dumps(dict(decisions=[])));sys.exit(0)
with open('requests.jsonl','a') as f: f.write(json.dumps(r)+'\\n')
anchors=[dict(source=s['id'],quote=s['text'],occurrence=0) for s in r['sources']]
print(json.dumps(dict(parts=dict(who='I',outcome='keep selection exact',why='Copies must match',done_when='bytes match'),open=['outcome'],supports=anchors,misfits=[],questions=[dict(id='q1',text='Which words matter?')],alternatives=[])))
'''
TEXT = ('Typed evidence should copy exactly. ' * 24).strip()
HEAD = TEXT[:8]


def journey(width, height):
    with tempfile.TemporaryDirectory(prefix='tinkery-selection-pty-') as directory:
        cwd = Path(directory)
        host = cwd / 'pi'; host.write_text(HOST); host.chmod(0o755)
        skill = cwd / 'SKILL.md'; skill.write_text('# Seed Me\n### Shape the working draft\nUnderstand intent.\n### Size gate\n')
        master, slave = os.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', height, width, 0, 0))
        screen = Screen(width, height)
        process = subprocess.Popen(
            [str(BINARY), '--shape-pi', '--record-root', str(cwd / 'TEST-records'), '--model', 'test/model', '--seed-me', str(skill), '--pi-command', str(host)],
            stdin=slave, stdout=slave, stderr=slave, cwd=cwd, env={**os.environ, 'TERM': 'xterm-256color'})

        def pump(seconds=0.05):
            deadline = time.monotonic() + seconds
            while time.monotonic() < deadline:
                if select.select([master], [], [], 0.01)[0]:
                    screen.feed(os.read(master, 65536))

        def await_text(text, timeout=5):
            deadline = time.monotonic() + timeout
            while time.monotonic() < deadline:
                pump(0.02)
                if text in screen.text() and not screen.synchronized:
                    return
                if process.poll() is not None:
                    break
            raise AssertionError(f'Missing {text!r} at {width}x{height}:\n{screen.text()}')

        def requests():
            path = cwd / 'requests.jsonl'
            return path.read_text().splitlines() if path.exists() else []

        def copy(expected):
            before = len(screen.clipboards)
            os.write(master, b'\x03')
            deadline = time.monotonic() + 3
            while len(screen.clipboards) == before and time.monotonic() < deadline:
                pump(0.02)
            assert len(screen.clipboards) == before + 1, f'No OSC 52 copy at {width}x{height}'
            assert screen.clipboards[-1] == expected, (screen.clipboards[-1], expected)

        def no_copy():
            before = len(screen.clipboards)
            os.write(master, b'\x03'); pump(0.3)
            assert len(screen.clipboards) == before, 'stale selection was copied'
            assert process.poll() is None, 'Ctrl-C left think'

        def locate(text):
            for y, row in enumerate(screen.cells):
                line = ''.join(row)
                if text in line:
                    return line.index(text), y
            raise AssertionError(f'{text!r} not on screen:\n{screen.text()}')

        try:
            await_text("What's on your mind?")
            os.write(master, TEXT.encode()); await_text('copy exactly.')
            os.write(master, b'\x1bOQ'); await_text('Which words matter?')
            assert len(requests()) == 1
            sent = len(requests())
            # No mouse: Tab into the original, Shift-Right x8, Ctrl-C.
            os.write(master, b'\t'); pump(0.2)
            os.write(master, b'\x1b[1;2C' * 8); pump(0.3)
            copy(HEAD)
            # Tab returns to writing; nothing stale is copied; typing is ordinary text.
            os.write(master, b'\t'); pump(0.2); no_copy()
            os.write(master, b'qs?'); await_text('qs?')
            no_copy()
            # Shift-Tab is also a way in, resuming where the caret was; Esc drops the selection.
            os.write(master, b'\x1b[Z'); pump(0.2)
            os.write(master, b'\x1b[1;2C' * 3); pump(0.2)
            copy(TEXT[8:11])
            os.write(master, b'\x1b'); pump(0.2); no_copy()
            # A selection survives a resize as the same logical bytes.
            os.write(master, b'\x1b[1;2C' * 8); pump(0.2)
            for w, h in ((width + 6, height + 2), (width, height)):
                fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', h, w, 0, 0))
                os.kill(process.pid, signal.SIGWINCH); pump(0.3)
                screen.width, screen.height = w, h
                screen.cells = [[' '] * w for _ in range(h)]
                os.write(master, b'\x0c'); pump(0.4)
                copy(TEXT[11:19])
            # Mouse drag across the first words yields the same logical bytes.
            x, y = locate('Typed evidence')
            os.write(master, f'\x1b[<0;{x + 1};{y + 1}M\x1b[<32;{x + 9};{y + 1}M\x1b[<0;{x + 9};{y + 1}m'.encode()); pump(0.3)
            copy(HEAD)
            # Scrolling the original moves the screen, not the logical selection.
            os.write(master, f'\x1b[<65;{x + 1};{y + 1}M'.encode() * 4); pump(0.3)
            copy(HEAD)
            # Writing: Tab back, Ctrl-A selects only the writing box, typing replaces it.
            os.write(master, b'\t'); pump(0.2); no_copy()
            os.write(master, b'\x01'); pump(0.2)
            copy('qs?')
            os.write(master, b'ab'); await_text('ab')
            os.write(master, b'\x1b[1;2D'); pump(0.2)
            copy('b')
            # Bracketed paste replaces the selection and is sanitized like any paste.
            os.write(master, b'\x1b[200~X\r\x1b[31my\x1b[201~'); pump(0.3)
            os.write(master, b'\x01'); pump(0.2)
            copy('aX\n[31my')
            # Mouse drag across writing selects the same logical bytes.
            os.write(master, b'\x1b[F'); pump(0.1)
            x, y = locate('aX')
            os.write(master, f'\x1b[<0;{x + 1};{y + 1}M\x1b[<32;{x + 2};{y + 1}M\x1b[<0;{x + 2};{y + 1}m'.encode()); pump(0.3)
            copy('a')
            # Backspace removes exactly the selection; the rest of the bytes remain.
            os.write(master, b'\x7f'); pump(0.2)
            os.write(master, b'\x01'); pump(0.2)
            copy('X\n[31my')
            assert len(requests()) == sent, 'navigation/copy invoked the provider'
            assert {p.name for p in cwd.iterdir()} == {'pi', 'SKILL.md', 'requests.jsonl'}, 'Unexpected persistence'
            print(json.dumps(dict(width=width, height=height, journey='tab/shift-arrows/ctrl-c/mouse/resize', provider_calls=sent)))
        finally:
            if process.poll() is None:
                process.kill(); process.wait()
            os.close(master); os.close(slave)


for size in [(80, 24), (100, 30), (160, 40), (320, 40)]:
    journey(*size)
