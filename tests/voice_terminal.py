"""Node-independent PTY: scripted voice transport and shaping host, no mic/Pi calls."""
import ast
import fcntl
import json
import os
import select
import struct
import subprocess
import sys
import tempfile
import termios
import time
from pathlib import Path
from vt_screen import Screen

ROOT = Path(__file__).resolve().parents[1]
VOICE_KEY = '⌃⌥Z' if sys.platform == 'darwin' else '6'
NODE = '''#!/usr/bin/env python3
import json,os,sys,threading,time
from pathlib import Path
if sys.argv[1:]==['--version']:
 print('v26.0.0');sys.exit(0)
assert sys.argv[1]=='--input-type=module' and sys.argv[2]=='--eval'
assert 'TranscribeModel.load' in sys.argv[3]
lock=threading.Lock()
def emit(event):
 with lock: print(json.dumps(event),flush=True)
with open('node-pids.txt','a') as f:f.write(str(os.getpid())+'\\n')
emit(dict(event='ready',version=1))
Path('node-ready.txt').write_text('ready')
def final(take):
 time.sleep(.15)
 emit(dict(event='final',take=take,text='spoken words' if take==1 else 'second words' if take==2 else 'late final'))
try:
 for line in sys.stdin:
  c=json.loads(line);take=c['take']
  with open('voice-calls.jsonl','a') as f:f.write(json.dumps(c)+'\\n')
  if c['command']=='start':
   emit(dict(event='listening',take=take));emit(dict(event='partial',take=take,text='ghost words'))
  else:threading.Thread(target=final,args=(take,),daemon=True).start()
finally:
 with open('node-closed.txt','a') as f:f.write(str(os.getpid())+'\\n')
'''
# Reuse the existing shaping contract fixture without importing/running its journeys.
HOST = next(ast.literal_eval(n.value) for n in ast.parse((ROOT / 'tests/brain_dump_terminal.py').read_text()).body
            if isinstance(n, ast.Assign) and any(isinstance(t, ast.Name) and t.id == 'HOST' for t in n.targets))


def journey(width, height, no_color):
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-voice-pty-') as directory:
        cwd = Path(directory)
        for name, source in [('node', NODE), ('pi', HOST)]:
            p = cwd / name
            p.write_text(source)
            p.chmod(0o755)
        (cwd / 'python3').symlink_to(sys.executable)
        skill = cwd / 'SKILL.md'
        skill.write_text('# Seed Me\n### Shape the working draft\nUnderstand intent.\n### Size gate\n')
        master, slave = os.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', height, width, 0, 0))
        original = termios.tcgetattr(slave)
        screen = Screen(width, height)
        env = {**os.environ, 'TERM': 'xterm-256color', 'TINKERY_NODE': str(cwd / 'node'), 'PATH': str(cwd), 'SHELL': '/bin/false', 'TINKERY_VOICE_FAKE_PCM': '', 'TINKERY_VOICE_SMOKE_WAV': ''}
        if no_color:
            env['NO_COLOR'] = '1'
        else:
            env.pop('NO_COLOR', None)
        process = subprocess.Popen([str(ROOT / 'target/debug/tinkery'), '--shape-pi', '--model', 'test/model',
                                    '--seed-me', str(skill), '--pi-command', str(cwd / 'pi')],
                                   stdin=slave, stdout=slave, stderr=slave, cwd=cwd, env=env)

        def pump():
            if select.select([master], [], [], .02)[0]:
                screen.feed(os.read(master, 65536))

        def wait(predicate, label, timeout=5):
            deadline = time.monotonic() + timeout
            while time.monotonic() < deadline:
                pump()
                if predicate() and not screen.synchronized:
                    assert screen.cells[1][2:9] == list('tinkery'), screen.text()
                    return
                if process.poll() is not None:
                    break
            raise AssertionError(f'{label}:\n{screen.text()}')

        def text(needle):
            wait(lambda: needle in screen.text(), needle)

        def send(data, needle=None):
            os.write(master, data)
            if needle:
                text(needle)

        def rows(name):
            p = cwd / name
            return [json.loads(line) for line in p.read_text().splitlines()] if p.exists() else []

        try:
            text(VOICE_KEY + ' speak')
            wait(lambda: (cwd / 'node-ready.txt').exists(), 'fake helper ready')
            deadline = time.monotonic() + .1
            while time.monotonic() < deadline: pump()
            assert not rows('voice-calls.jsonl') and not rows('requests.jsonl')
            send(b'\x1bOP', 'Right now')
            send(b'\x1bOP', 'Every key')
            text('speak (local)')
            send(b'\x1bOP', "What's on your mind?")
            send(b'typed ', 'typed')
            send(b'\x1b[17~', 'ghost words')
            text(VOICE_KEY + ' stop   esc cancel')
            send(b'kept ', 'typed kept ghost words')
            assert not rows('requests.jsonl')
            send(b'\x1b[17~', 'typed kept spoken words')
            text(VOICE_KEY + ' speak')
            assert not rows('requests.jsonl')
            send(b'\x1bOQ', 'Which evidence gets confused?')
            assert rows('requests.jsonl')[0]['sources'][0]['text'] == 'typed kept spoken words'
            # CSI-u Ctrl+Alt+Z: never Ctrl+Z undo or a character insertion.
            send(b'\x1b[122;7u', 'ghost words')
            send(b'\x1b[20~')
            wait(lambda: bool(screen.clipboards), 'clipboard')
            assert 'ghost words' not in screen.clipboards[-1]
            assert 'typed kept spoken words' in screen.clipboards[-1]
            send(b'\x1bOQ', 'What would make a handover')
            requests = rows('requests.jsonl')
            assert len(requests) == 2
            assert requests[1]['sources'][1]['text'] == 'second words'
            assert requests[1]['sources'][1]['in_reply_to'] == 'meaning'
            send(b'untouched', 'untouched')
            send('Ω'.encode(), 'ghost words')
            send(b'\x1b', VOICE_KEY + ' speak')
            time.sleep(.3)
            for _ in range(5):
                pump()
            assert 'untouched' in screen.text() and 'late final' not in screen.text()
            assert len(rows('requests.jsonl')) == 2
            send(b'\x15')
            send(b'\x1bOR', 'type confirm to save')
            before = len(rows('voice-calls.jsonl'))
            send(b'\x1b[17~' + 'Ω'.encode())
            time.sleep(.1)
            assert len(rows('voice-calls.jsonl')) == before
            assert 'Ω' not in screen.text() and len(rows('requests.jsonl')) == 2
            send(b'\x1b', VOICE_KEY + ' speak')
            send(b'\x1b[17~', 'ghost words')
            send(b'\x1b[21~', 'Back to menu and lose this draft? y / n')
            wait(lambda: (cwd / 'node-closed.txt').exists(), 'helper stdin EOF')
            for pid in (cwd / 'node-pids.txt').read_text().splitlines():
                try:
                    os.kill(int(pid), 0)
                except ProcessLookupError:
                    pass
                else:
                    raise AssertionError(f'Voice process {pid} was not reaped')
            send(b'n')
            send(b'\x1b[17~')
            text(VOICE_KEY + ' speak')
            assert len(rows('requests.jsonl')) == 2
            send(b'\x1b[21~', 'Back to menu and lose this draft? y / n')
            send(b'y')
            process.wait(timeout=5)
            assert process.returncode == 0
            after = termios.tcgetattr(slave)
            original[3] &= ~getattr(termios, 'PENDIN', 0)
            after[3] &= ~getattr(termios, 'PENDIN', 0)
            assert after == original
            assert {p.name for p in cwd.iterdir()} == {'node', 'pi', 'python3', 'SKILL.md', 'voice-calls.jsonl', 'requests.jsonl', 'node-pids.txt', 'node-closed.txt', 'node-ready.txt'}
            print(json.dumps(dict(width=width, height=height, no_color=no_color, journey='current cursor/stop/F2/cancel/alias/review/EOF/reap', pi_calls=0, microphone=False, node_runtime=False)))
        finally:
            if process.poll() is None:
                process.kill()
                process.wait()
            os.close(master)
            os.close(slave)


for no_color in [False, True]:
    for size in [(80, 24), (100, 30), (160, 40), (320, 40)]:
        journey(*size, no_color)
