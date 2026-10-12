"""Read-only shape journeys with synthetic fixture sessions; no model calls or operator confirmation."""
import base64, fcntl, json, os, re, select, struct, subprocess, tempfile, termios, time
from datetime import datetime, timedelta, timezone
from pathlib import Path
from vt_screen import Screen

REPO = Path(__file__).resolve().parents[1]
SKILL = Path(os.environ['SEED_ME_TEST_SKILL']).resolve()
GOAL = ('I can read a calm dark theme while preserving every confirmed goal line and keeping a clear next step with manual control.\n'
        'Why: I work at night.\nDone when: Nothing strands me.\n'
        'Must: Keep manual control.\nMust not: Run agents.')


def journey(width, height):
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-shape-') as directory:
        root = Path(directory).resolve()
        sessions = root / 'TEST-sessions'
        now = datetime.now(timezone.utc)
        for i, (name, status, seed) in enumerate([
            ('saved', 'active', None), ('drafted', 'active', 'status: draft'),
            ('ready', 'active', 'status: confirmed for intake'), ('ended', 'completed', 'status: confirmed for intake')
        ]):
            path = sessions / name
            path.mkdir(parents=True)
            (path / 'ledger.json').write_text(json.dumps(dict(status=status, goal=GOAL,
                updated_at=(now-timedelta(hours=i)).isoformat())))
            if seed: (path / 'seed-contract.md').write_text(seed+'\n# Synthetic fixture only\n')
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

        try:
            wait(lambda: '1  think' in screen.text())
            send(b'2', '↑↓ move   enter open   esc back')
            for index, label in enumerate(['goal saved', 'seed drafted', 'seed ready', 'ended']):
                line = screen.text().splitlines()[index+3]
                assert line.startswith('  I can read') and label in line and ' · ' in line, line
                if width <= 100: assert '…' in line, line
            assert 'Why:' not in screen.text() and 'Done when:' not in screen.text()
            ordinary()
            for index, next_line in enumerate(['Next: shape it into a seed with Seed Me.',
                    'Next: confirm the seed draft with Seed Me.', 'Next: the seed is ready for intake.', None]):
                send(b'\r', 'esc back   10 menu')
                assert 'Saved.' not in screen.text()
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
                send(b'\x1b', '↑↓ move   enter open   esc back')
                if index < 3: os.write(master, b'j')
            send(b'\x1b[21~', '1  think')
            assert process.poll() is None
            os.write(master, b'\x1b[21~')
            wait(lambda: process.poll() is not None)
            assert process.returncode == 0
            assert not (root/'provider-was-run').exists()
            assert {str(p): p.read_bytes() for p in sessions.rglob('*') if p.is_file()} == before
            print(json.dumps(dict(width=width, height=height, journey='list/status/detail/repeat copy/help/back/menu',
                provider_calls=0, sessions='synthetic fixtures in temporary root', operator_confirmation=False)))
        finally:
            if process.poll() is None: process.kill(); process.wait()
            os.close(master); os.close(slave)


for size in [(80,24), (100,30), (160,40), (320,40)]: journey(*size)
