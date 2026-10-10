import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { readFileSync, mkdtempSync, mkdirSync, writeFileSync, readdirSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const source = readFileSync(new URL('./helper.mjs', import.meta.url), 'utf8');
function client(t, config = {}, env = {}) {
  const child = spawn(process.execPath, ['--input-type=module', '--eval', source], {
    env: { ...process.env, TINKERY_VOICE_FAKE_PCM: 'tone', TINKERY_VOICE_SMOKE_WAV: '',
      TINKERY_PI_NODE_MAJOR: process.versions.node.split('.')[0], TINKERY_VOICE_FAKE_CONFIG: JSON.stringify(config), ...env },
    stdio: ['pipe', 'pipe', 'pipe'],
  });
  let output = '', stderr = '', events = [], waiters = [];
  child.stdout.on('data', data => {
    output += data;
    for (;;) { const end = output.indexOf('\n'); if (end < 0) break;
      const event = JSON.parse(output.slice(0, end)); output = output.slice(end + 1); events.push(event);
      for (const waiter of [...waiters]) if (waiter.predicate(event)) { waiters = waiters.filter(w => w !== waiter); clearTimeout(waiter.timer); waiter.resolve(event); }
    }
  });
  child.stderr.on('data', data => { stderr += data; });
  const exit = new Promise(resolve => child.on('close', (code, signal) => resolve({ code, signal })));
  t.after(async () => { child.stdin.destroy(); if (child.exitCode === null) child.kill(); await exit; });
  const wait = (name, predicate = () => true) => {
    const existing = events.find(event => event.event === name && predicate(event)); if (existing) return Promise.resolve(existing);
    return new Promise((resolve, reject) => { const waiter = { predicate: event => event.event === name && predicate(event), resolve,
      timer: setTimeout(() => { waiters = waiters.filter(w => w !== waiter); reject(new Error(`Timed out waiting for ${name}: ${JSON.stringify(events)}, stderr ${stderr}`)); }, 4000) }; waiters.push(waiter); });
  };
  return { child, wait, exit, events, send: (command, take = 1) => child.stdin.write(JSON.stringify({ command, take }) + '\n'), stderr: () => stderr };
}
const delay = ms => new Promise(resolve => setTimeout(resolve, ms));
function wave(silence = false) {
  const n = 16000, b = Buffer.alloc(44 + n * 2);
  b.write('RIFF'); b.writeUInt32LE(b.length - 8, 4); b.write('WAVEfmt ', 8); b.writeUInt32LE(16, 16);
  b.writeUInt16LE(1, 20); b.writeUInt16LE(1, 22); b.writeUInt32LE(16000, 24); b.writeUInt32LE(32000, 28); b.writeUInt16LE(2, 32); b.writeUInt16LE(16, 34);
  b.write('data', 36); b.writeUInt32LE(n * 2, 40);
  for (let i = 0; i < n; i++) b.writeInt16LE(silence ? 0 : Math.round(Math.sin(i * Math.PI * 880 / 16000) * 3000), 44 + i * 2);
  return b;
}

test('eval ESM loads fake engine once and reuses it across takes', async t => {
  const c = client(t); assert.equal((await c.wait('ready')).version, 1);
  for (const take of [1, 2]) {
    c.send('start', take); await c.wait('listening', e => e.take === take); await c.wait('partial', e => e.take === take);
    c.send('stop', take); assert.equal((await c.wait('final', e => e.take === take)).text, 'spoken words');
  }
  assert.equal(c.events.filter(e => e.event === 'ready').length, 1); assert.equal(c.stderr(), '');
});
test('stop during asynchronous start never begins recording', async t => {
  const c = client(t, { startDelay: 100 }); await c.wait('ready'); c.send('start'); c.send('stop');
  assert.equal((await c.wait('final')).text, ''); assert.ok(!c.events.some(e => e.event === 'listening'));
});
test('cancel during start and finalization discards callbacks, newer take survives', async t => {
  const c = client(t, { startDelay: 100, finalDelay: 100 }); await c.wait('ready'); c.send('start'); c.send('cancel');
  c.send('start', 2); await c.wait('partial', e => e.take === 2); c.send('stop', 2); await delay(30); c.send('cancel', 2);
  c.send('start', 3); await c.wait('partial', e => e.take === 3); c.send('stop', 3); await c.wait('final', e => e.take === 3);
  assert.ok(!c.events.some(e => e.event === 'final' && e.take !== 3));
});
test('stdin EOF exits immediately during load, listen and finalization', { timeout: 5000 }, async t => {
  for (const stage of ['load', 'listen', 'final']) {
    const c = client(t, { loadDelay: stage === 'load' ? 20000 : 0, finalDelay: 20000, traceRecorder: true });
    if (stage !== 'load') { await c.wait('ready'); c.send('start'); await c.wait('partial'); if (stage === 'final') c.send('stop'); }
    const start = Date.now(); c.child.stdin.end(); assert.equal((await c.exit).code, 0); assert.ok(Date.now() - start < 1000);
    assert.ok(!c.events.some(e => e.event === 'final'));
    if (stage === 'load') assert.ok(!c.events.some(e => e.event === 'test-recorder'));
    else assert.ok(c.events.some(e => e.event === 'test-recorder' && e.operation === 'stop'));
  }
});
test('recorder starts only on command, then stops and releases once', async t => {
  const c = client(t, { traceRecorder: true }); await c.wait('ready'); await delay(30);
  assert.ok(!c.events.some(e => e.event === 'test-recorder'));
  c.send('start'); await c.wait('partial'); c.send('stop'); await c.wait('final');
  assert.deepEqual(c.events.filter(e => e.event === 'test-recorder').map(e => e.operation), ['start', 'stop', 'release']);
});
test('cancel during permission query never starts the recorder', async t => {
  const c = client(t, { traceRecorder: true, permissionDelay: 100 }); await c.wait('ready'); c.send('start'); c.send('cancel'); await delay(150);
  assert.ok(!c.events.some(e => e.event === 'test-recorder')); assert.ok(!c.events.some(e => e.event === 'listening'));
});
test('device selection and sample-rate validation use the real recorder glue', async t => {
  for (const [config, kind] of [[{ noDevices: true }, 'mic-unavailable'], [{ microphone: { type: 'device', name: 'missing', occurrence: 0 } }, 'mic-unavailable'], [{ sampleRate: 8000 }, 'capture']]) {
    const c = client(t, config); await c.wait('ready'); c.send('start'); assert.equal((await c.wait('error')).kind, kind);
  }
});
test('signals terminate capture without a final', async t => {
  const c = client(t); await c.wait('ready'); c.send('start'); await c.wait('partial'); c.child.kill('SIGTERM'); await c.exit;
  assert.ok(!c.events.some(e => e.event === 'final'));
});
test('near-zero whole take reports permission guidance, not empty final', async t => {
  const c = client(t, { text: 'thank you' }, { TINKERY_VOICE_FAKE_PCM: 'silence' }); await c.wait('ready'); c.send('start'); await c.wait('partial'); c.send('stop');
  const error = await c.wait('error'); assert.equal(error.kind, 'mic-permission'); assert.match(error.message, /near-zero.*Privacy/); assert.ok(!c.events.some(e => e.event === 'final'));
});
test('batch-only models still produce final without partials', async t => {
  const c = client(t, { batch: true }); await c.wait('ready'); c.send('start'); await c.wait('listening'); await delay(60); c.send('stop'); await c.wait('final');
  assert.ok(!c.events.some(e => e.event === 'partial'));
});
test('injected failure kinds are specific and never produce a final', async t => {
  for (const [stage, kind] of [['load', 'engine-load'], ['permission', 'mic-permission'], ['record', 'mic-unavailable'], ['record', 'mic-busy'], ['feed', 'transcription'], ['final', 'transcription']]) {
    const c = client(t, { fail: stage, kind, message: 'Action required.' });
    if (stage !== 'load') { await c.wait('ready'); c.send('start'); if (stage === 'final') { await c.wait('partial'); c.send('stop'); } }
    const error = await c.wait('error'); assert.equal(error.kind, kind); assert.equal(error.message, 'Action required.'); assert.ok(!c.events.some(e => e.event === 'final'));
  }
});
test('PCM backlog is bounded and cancellation suppresses queued work', async t => {
  const c = client(t, { feedDelay: 2000 }); await c.wait('ready'); c.send('start');
  const error = await c.wait('error'); assert.equal(error.kind, 'capture'); assert.match(error.message, /backlog/); assert.ok(!c.events.some(e => e.event === 'final'));
});
test('malformed, oversized and stale commands fail explicitly', async t => {
  const c = client(t); await c.wait('ready'); c.child.stdin.write('not json\n'); assert.equal((await c.wait('error')).kind, 'protocol');
  const d = client(t); await d.wait('ready'); d.child.stdin.write('x'.repeat(32769)); assert.equal((await d.wait('error')).kind, 'protocol');
  const e = client(t); await e.wait('ready'); e.send('start'); await e.wait('listening'); e.send('start'); assert.equal((await e.wait('error')).kind, 'protocol');
});
test('Node major mismatch precedes engine loading with the prescribed message', async t => {
  const c = client(t, {}, { TINKERY_PI_NODE_MAJOR: '999' }); const error = await c.wait('error');
  assert.equal(error.kind, 'engine-load'); assert.equal(error.message, 'Voice needs the Node version Pi uses (999). Set TINKERY_NODE to it.'); assert.ok(!c.events.some(e => e.event === 'ready'));
});
test('missing Pi Voice and missing model are distinguished without native imports', async t => {
  const home = mkdtempSync(join(tmpdir(), 'tinkery-TEST-voice-discovery-'));
  const c = client(t, {}, { HOME: home, TINKERY_VOICE_FAKE_PCM: '' }); assert.equal((await c.wait('error')).kind, 'no-pi-voice');
  const install = join(home, '.pi/agent/npm/node_modules/@earendil-works/pi-voice'); mkdirSync(install, { recursive: true }); writeFileSync(join(install, 'package.json'), '{}');
  writeFileSync(join(home, '.pi/agent/pi-voice.json'), JSON.stringify({ version: 1, backend: { type: 'transcribe-cpp' }, microphone: { type: 'system-default' }, transcriptionLanguage: 'en', model: { path: join(home, 'missing.gguf') } }));
  const d = client(t, {}, { HOME: home, TINKERY_VOICE_FAKE_PCM: '' }); assert.equal((await d.wait('error')).kind, 'no-model');
});
test('eval resolves import-only engine exports from an isolated Pi Voice install', async t => {
  const home = mkdtempSync(join(tmpdir(), 'tinkery-TEST-voice-modules-')); const modules = join(home, '.pi/agent/npm/node_modules');
  const voice = join(modules, '@earendil-works/pi-voice'); const engine = join(modules, 'transcribe-cpp'); const recorder = join(modules, '@picovoice/pvrecorder-node');
  for (const path of [voice, join(engine, 'dist'), recorder]) mkdirSync(path, { recursive: true });
  writeFileSync(join(voice, 'package.json'), '{}');
  writeFileSync(join(engine, 'package.json'), JSON.stringify({ type: 'module', exports: { '.': { import: './dist/index.js' } } }));
  writeFileSync(join(engine, 'dist/index.js'), 'export const setLogHandler=()=>{}; export const TranscribeModel={load:async()=>({capabilities:{languages:["en"]}})};');
  writeFileSync(join(recorder, 'package.json'), '{"main":"index.js"}'); writeFileSync(join(recorder, 'index.js'), 'exports.PvRecorder=class { constructor(){throw new Error("Recorder must not be constructed at load")} };');
  const model = join(home, 'model.gguf'); writeFileSync(model, 'isolated fake model');
  writeFileSync(join(home, '.pi/agent/pi-voice.json'), JSON.stringify({ version: 1, backend: { type: 'transcribe-cpp' }, microphone: { type: 'system-default' }, transcriptionLanguage: 'en', model: { path: model } }));
  const c = client(t, {}, { HOME: home, TINKERY_VOICE_FAKE_PCM: '' }); await c.wait('ready'); assert.ok(!c.events.some(e=>e.event==='listening')); assert.equal(c.stderr(), ''); c.child.stdin.end(); assert.equal((await c.exit).code, 0);
});
test('generated WAV fake capture writes neither audio nor transcript', async t => {
  const root = mkdtempSync(join(tmpdir(), 'tinkery-TEST-voice-wave-')); const path = join(root, 'generated.wav'); writeFileSync(path, wave());
  const before = readdirSync(root); const c = client(t, {}, { TINKERY_VOICE_FAKE_PCM: path }); await c.wait('ready'); c.send('start'); await c.wait('partial'); c.send('stop'); await c.wait('final');
  assert.deepEqual(readdirSync(root), before); assert.equal(c.stderr(), '');
  const bad = join(root, 'bad.wav'); writeFileSync(bad, 'not wave'); const d = client(t, {}, { TINKERY_VOICE_FAKE_PCM: bad }); assert.equal((await d.wait('error')).kind, 'capture');
});
test('asynchronous native feeds borrow immutable PCM buffers', async t => {
  const root = mkdtempSync(join(tmpdir(), 'tinkery-TEST-voice-immutable-')); const path = join(root, 'generated.wav');
  const pcm = wave(); for (let i = 8000; i < 16000; i++) pcm.writeInt16LE(7000, 44 + i * 2); writeFileSync(path, pcm);
  const c = client(t, { verifyBorrowed: true, feedDelay: 250 }, { TINKERY_VOICE_FAKE_PCM: path }); await c.wait('ready'); c.send('start'); await c.wait('partial'); c.send('stop'); await c.wait('final');
  assert.ok(!c.events.some(e => e.event === 'error'));
});
test('empty fake env does not hide the explicitly supplied smoke WAV', async t => {
  const home = mkdtempSync(join(tmpdir(), 'tinkery-TEST-voice-smoke-input-'));
  const path = join(home, 'generated.wav'); writeFileSync(path, wave());
  const c = client(t, {}, { HOME: home, TINKERY_VOICE_FAKE_PCM: '', TINKERY_VOICE_SMOKE_WAV: path });
  assert.equal((await c.wait('error')).kind, 'no-pi-voice');
});
test('empty and filler finals remain data for the Rust insertion guard', async t => {
  for (const text of ['', 'thank you', 'you', '.']) {
    const c = client(t, { text }); await c.wait('ready'); c.send('start'); await c.wait('partial'); c.send('stop'); assert.equal((await c.wait('final')).text, text);
  }
});
