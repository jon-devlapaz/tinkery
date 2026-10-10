"""Manual/local only: real temporary pinned install + generated tone, never a microphone."""
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

ROOT = Path(__file__).resolve().parents[4]
source = (ROOT / 'voice/setup.mjs').read_text() + '\n' + (ROOT / 'voice/helper.mjs').read_text()
node = os.environ.get('TINKERY_NODE', 'node')
version = subprocess.check_output([node, '--version'], text=True, timeout=3).strip()
install = Path(tempfile.mkdtemp(prefix='tinkery-TEST-owned-native-'))
env = {**os.environ, 'TINKERY_VOICE_ROOT': str(install), 'TINKERY_VOICE_FAKE_PCM': ''}
for key in ['NODE_OPTIONS', 'NODE_PATH', 'TRANSCRIBE_LIBRARY', 'TINKERY_VOICE_MODEL']:
    env.pop(key, None)
start = time.monotonic()
script = 'import {installVoice,voicePaths} from ' + json.dumps((ROOT / 'voice/setup.mjs').as_uri()) + '; await installVoice(voicePaths());'
result = subprocess.run([node, '--input-type=module', '--eval', script], env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=120)
assert result.returncode == 0, result.stderr.decode()
seconds = time.monotonic() - start
bytes_installed = sum(p.stat().st_size for p in install.rglob('*') if p.is_file())
packages = {}
for manifest in install.glob('node_modules/**/package.json'):
    data = json.loads(manifest.read_text())
    if data.get('name'):
        packages[data['name']] = {'version': data.get('version'), 'bytes': sum(p.stat().st_size for p in manifest.parent.rglob('*') if p.is_file()), 'scripts': data.get('scripts', {})}
with tempfile.TemporaryDirectory(prefix='tinkery-TEST-generated-owned-') as directory:
    path = Path(directory) / 'generated-tone.wav'
    with wave.open(str(path), 'wb') as output:
        output.setparams((1, 2, 16000, 16000, 'NONE', 'not compressed'))
        output.writeframes(b''.join(struct.pack('<h', round(math.sin(i * 2 * math.pi * 440 / 16000) * 3000)) for i in range(16000)))
    process = subprocess.Popen([node, '--input-type=module', '--eval', source], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                               stderr=subprocess.DEVNULL, env={**env, 'TINKERY_VOICE_SMOKE_WAV': str(path)})
    names, buffer, partials, terminal = [], b'', 0, None
    def command(name):
        process.stdin.write(json.dumps(dict(command=name, take=1)).encode() + b'\n'); process.stdin.flush()
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
                line, buffer = buffer.split(b'\n', 1); event = json.loads(line); names.append(event['event'])
                if event['event'] == 'ready':
                    assert event['version'] == 1; command('start')
                if event['event'] == 'partial':
                    partials += 1
                    if partials == 2: command('stop')
                if event['event'] in ['final', 'error']: terminal = event
        assert terminal is not None and terminal['event'] == 'final', terminal or names
        print(json.dumps(dict(source='generated 1s 440Hz tone; no speech/microphone recording', node=version,
                              embedded_source_sha256=hashlib.sha256(source.encode()).hexdigest(), events=names,
                              final_empty=not terminal['text'], provider_calls=0, helper_recorder_imported=False, recorder_constructed=False,
                              installation_native_imports=['transcribe-cpp', '@picovoice/pvrecorder-node'],
                              install_root=str(install), install_seconds=round(seconds, 3), installed_bytes=bytes_installed,
                              model_bytes=731357568, packages=packages)))
    finally:
        process.stdin.close()
        try: process.wait(timeout=2)
        except subprocess.TimeoutExpired: process.kill(); process.wait()
    assert process.returncode == 0
