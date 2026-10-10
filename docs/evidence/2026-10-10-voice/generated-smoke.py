"""Manual/local only: generated tone through installed ASR; never a recorder or Pi."""
from pathlib import Path
import hashlib
import json
import math
import os
import select
import struct
import subprocess
import tempfile
import time
import wave

ROOT = Path(__file__).resolve().parents[3]
source = (ROOT / 'voice/helper.mjs').read_text()
node = os.environ.get('TINKERY_NODE', 'node')
version = subprocess.check_output([node, '--version'], text=True, timeout=3).strip()
with tempfile.TemporaryDirectory(prefix='tinkery-TEST-generated-voice-') as directory:
    path = Path(directory) / 'generated-tone.wav'
    with wave.open(str(path), 'wb') as output:
        output.setparams((1, 2, 16000, 16000, 'NONE', 'not compressed'))
        output.writeframes(b''.join(struct.pack('<h', round(math.sin(i * 2 * math.pi * 440 / 16000) * 3000)) for i in range(16000)))
    process = subprocess.Popen([node, '--input-type=module', '--eval', source], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                               stderr=subprocess.DEVNULL, env={**os.environ, 'TINKERY_VOICE_FAKE_PCM': '',
                               'TINKERY_VOICE_SMOKE_WAV': str(path), 'TINKERY_PI_NODE_MAJOR': version[1:].split('.')[0]})
    names, buffer, partials, terminal = [], b'', 0, None
    def command(name):
        process.stdin.write(json.dumps(dict(command=name, take=1)).encode() + b'\n')
        process.stdin.flush()
    try:
        deadline = time.monotonic() + 30
        while time.monotonic() < deadline and terminal is None:
            if not select.select([process.stdout], [], [], .1)[0]:
                continue
            data = os.read(process.stdout.fileno(), 65536)
            if not data:
                break
            buffer += data
            while b'\n' in buffer:
                line, buffer = buffer.split(b'\n', 1)
                event = json.loads(line)
                names.append(event['event'])
                if event['event'] == 'ready':
                    assert event['version'] == 1
                    command('start')
                if event['event'] == 'partial':
                    partials += 1
                    if partials == 2:
                        command('stop')
                if event['event'] in ['final', 'error']:
                    terminal = event
        assert terminal is not None and terminal['event'] == 'final', terminal or names
        print(json.dumps(dict(source='generated 1s 440Hz tone; no speech/microphone recording', node=version,
                              helper_sha256=hashlib.sha256(source.encode()).hexdigest(), events=names,
                              final_empty=not terminal['text'], pi_calls=0, recorder_imported=False)))
    finally:
        process.stdin.close()
        try:
            process.wait(timeout=2)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()
    assert process.returncode == 0
