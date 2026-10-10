import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, mkdir, writeFile, readFile, stat, utimes, readdir, symlink } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { PACKAGES, MODEL_FILE, voicePaths, installVoice, installationRequirement, readVoiceConfig, findVoiceModel, discoverVoice, downloadVoiceModel, shortReason, runVoiceNpm } from './setup.mjs';

async function fixture() { return voicePaths(await mkdtemp(join(tmpdir(), 'tinkery-TEST-owned-')), {}); }
async function fakeNpm(paths) {
  for (const [name, version] of Object.entries(PACKAGES)) {
    const path = join(paths.root, 'node_modules', name); await mkdir(path, { recursive: true });
    await writeFile(join(path, 'package.json'), JSON.stringify({ version, ...(name === 'transcribe-cpp' ? { type: 'module' } : { main: 'index.js' }) }));
    if (name === 'transcribe-cpp') {
      await mkdir(join(path, 'dist'), { recursive: true });
      await writeFile(join(path, 'dist/index.js'), 'export const setLogHandler=()=>{};');
    } else await writeFile(join(path, 'index.js'), 'exports.PvRecorder=class {};');
  }
}
test('missing install is discovery-only: exact muted prompt and no files', async () => {
  const paths = await fixture(), result = await discoverVoice(paths, {});
  assert.equal(result.requirement.event, 'needsinstall');
  assert.match(result.requirement.message, /^Voice needs a one-time local install \(about \d+ MB\)\. Install now\? y \/ n$/);
  assert.equal(result.requirement.size_mb, process.platform === 'darwin' ? 20 : process.platform === 'linux' ? 90 : 85);
  await assert.rejects(stat(paths.root), { code: 'ENOENT' });
});
test('install success pins both packages and records this Node major, failure is not installed', async () => {
  const paths = await fixture(); await installVoice(paths, fakeNpm);
  assert.equal(await installationRequirement(paths), undefined);
  assert.deepEqual(JSON.parse(await readFile(join(paths.root, 'package.json'), 'utf8')).dependencies, PACKAGES);
  assert.equal(JSON.parse(await readFile(paths.metadata, 'utf8')).nodeMajor, Number(process.versions.node.split('.')[0]));
  const failed = await fixture(); await assert.rejects(installVoice(failed, async () => { throw new Error('registry unavailable'); }), /registry unavailable/);
  assert.equal((await installationRequirement(failed)).event, 'needsinstall');
  await assert.rejects(stat(failed.metadata), { code: 'ENOENT' });
});
test('native verification failure invalidates prior completion and permits explicit reinstall', async () => {
  const paths = await fixture(); await installVoice(paths, fakeNpm);
  await assert.rejects(installVoice(paths, fakeNpm, async () => { throw new Error('native binary missing'); }), /native binary missing/);
  assert.equal((await installationRequirement(paths)).event, 'needsinstall');
  await installVoice(paths, fakeNpm); assert.equal(await installationRequirement(paths), undefined);
});
test('installer invokes npm with the selected Node and explicit pins/ignore-scripts, never a shell', async () => {
  const paths = await fixture(); await mkdir(paths.root, { recursive: true });
  const binaries = join(paths.root, 'fake-bin'); await mkdir(binaries);
  await symlink(process.execPath, join(binaries, 'node'));
  await writeFile(join(binaries, 'npm'), 'require("node:fs").writeFileSync("argv.json", JSON.stringify(process.argv.slice(2)));');
  await runVoiceNpm(paths, join(binaries, 'node'));
  const args = JSON.parse(await readFile(join(paths.root, 'argv.json'), 'utf8'));
  assert.deepEqual(args, ['install', '--ignore-scripts', '--no-audit', '--no-fund', '--save-exact', 'transcribe-cpp@0.2.2', '@picovoice/pvrecorder-node@1.2.9']);
});
test('Node major mismatch offers a one-line reinstall and wrong package pins also reinstall', async () => {
  const paths = await fixture(); await installVoice(paths, fakeNpm);
  const requirement = await installationRequirement(paths, 999);
  assert.match(requirement.message, /Voice used Node \d+; found 999\. Reinstall/); assert.ok(!requirement.message.includes('\n'));
  await writeFile(join(paths.root, 'node_modules/transcribe-cpp/package.json'), '{"version":"0.2.3"}');
  assert.equal((await installationRequirement(paths)).event, 'needsinstall');
});
test('newest shared-cache snapshot wins, missing lookup does not download or create files', async () => {
  const paths = await fixture(); assert.equal(await findVoiceModel(paths, {}, {}), undefined);
  for (const [snapshot, modified] of [['zz-old', 100], ['aa-new', 200]]) {
    const directory = join(paths.cache, snapshot); await mkdir(directory, { recursive: true });
    await writeFile(join(directory, MODEL_FILE), 'GGUF'); await utimes(directory, modified, modified);
  }
  assert.equal(await findVoiceModel(paths, {}, {}), join(paths.cache, 'aa-new', MODEL_FILE));
  await assert.rejects(stat(paths.root), { code: 'ENOENT' });
});
test('model override precedes config/cache and invalid explicit override never falls back', async () => {
  const paths = await fixture(), configModel = join(paths.cache, 'configured.gguf'), override = join(paths.cache, 'override.gguf');
  await mkdir(paths.cache, { recursive: true }); await writeFile(configModel, 'GGUF'); await writeFile(override, 'GGUF');
  assert.equal(await findVoiceModel(paths, { model: configModel }, { TINKERY_VOICE_MODEL: override }), override);
  assert.equal(await findVoiceModel(paths, { model: configModel }, {}), configModel);
  await assert.rejects(findVoiceModel(paths, { model: configModel }, { TINKERY_VOICE_MODEL: '/absent.gguf' }), /Check TINKERY_VOICE_MODEL/);
  await assert.rejects(findVoiceModel(paths, {}, { TINKERY_VOICE_MODEL: '' }), /Check TINKERY_VOICE_MODEL/);
});
test('optional config defaults to system microphone/English; invalid values are explicit errors', async () => {
  const paths = await fixture(); assert.deepEqual(await readVoiceConfig(paths), { microphone: { type: 'system-default' }, language: 'en' });
  await mkdir(join(paths.config, '..'), { recursive: true });
  await writeFile(paths.config, '{"microphone":"USB microphone","language":"en","model":"/local/model.gguf"}');
  assert.deepEqual(await readVoiceConfig(paths), { microphone: { type: 'device', name: 'USB microphone', occurrence: 0 }, language: 'en', model: '/local/model.gguf' });
  for (const value of ['bad', '[]', '{"language":42}', '{"surprise":true}', '{"microphone":""}']) {
    await writeFile(paths.config, value); await assert.rejects(readVoiceConfig(paths), /Check ~\/\.config\/tinkery\/voice.json/);
  }
});
test('model missing reports download size; owned downloaded model is discovered', async () => {
  const paths = await fixture(); await installVoice(paths, fakeNpm);
  const missing = await discoverVoice(paths, {}); assert.equal(missing.requirement.event, 'needsmodel'); assert.equal(missing.requirement.size_mb, 732);
  await mkdir(join(paths.downloaded, '..'), { recursive: true }); await writeFile(paths.downloaded, 'GGUF');
  assert.equal((await discoverVoice(paths, {})).model, paths.downloaded);
});
test('explicit model download is bounded/atomic, failure retains no partial and does not replace a model', async () => {
  const paths = await fixture(), payload = Buffer.from('GGUFfake'); let calls = 0;
  const response = async () => { calls++; return new Response(payload, { headers: { 'content-length': String(payload.length) } }); };
  await downloadVoiceModel(paths, response, payload.length); assert.equal(await readFile(paths.downloaded, 'utf8'), 'GGUFfake'); assert.equal(calls, 1);
  await downloadVoiceModel(paths, response, payload.length); assert.equal(calls, 1);
  const failed = await fixture(); await assert.rejects(downloadVoiceModel(failed, response, 9), /size changed/);
  assert.deepEqual(await readdir(join(failed.downloaded, '..')), []);
  const truncated = await fixture(); await assert.rejects(downloadVoiceModel(truncated, async () => new Response(payload), 9), /incomplete/);
  assert.deepEqual(await readdir(join(truncated.downloaded, '..')), []);
});
test('voice helper and voice-owned Rust have no former dependency strings or file reads', async () => {
  const forbidden = ['Pi', 'pi-voice', '.pi/', '.agents', '/voice-settings'];
  const voiceRust = await readdir(new URL('../src/voice/', import.meta.url));
  for (const path of ['setup.mjs', 'helper.mjs', ...voiceRust.filter(name => name.endsWith('.rs')).map(name => '../src/voice/' + name)]) {
    const source = await readFile(new URL(path, import.meta.url), 'utf8');
    for (const text of forbidden) assert.ok(!source.includes(text), `${path} contains ${text}`);
  }
  for (const path of ['voice_input.rs', 'voice_tests.rs']) {
    const source = await readFile(new URL('../src/shaping/brain_dump/' + path, import.meta.url), 'utf8');
    for (const literal of source.match(/"(?:\\.|[^"\\])*"/g) ?? []) for (const text of forbidden) assert.ok(!literal.includes(text));
  }
});
test('technical install reasons are one line and bounded', () => {
  assert.equal(shortReason(new Error('registry\nfailed\x1b')), 'registry failed');
  assert.equal(shortReason(new Error('x'.repeat(200))).length, 100);
});
