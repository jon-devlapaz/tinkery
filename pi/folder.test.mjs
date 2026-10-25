import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, writeFileSync, symlinkSync } from 'node:fs';
import { homedir, tmpdir } from 'node:os';
import { join } from 'node:path';
import guard from './folder.mjs';

function folder() {
  const root = mkdtempSync(join(tmpdir(), 'tinkery-TEST-pi-folder-'));
  const project = join(root, 'project'), outside = join(root, 'outside');
  mkdirSync(join(project, 'src'), { recursive: true }); mkdirSync(outside);
  writeFileSync(join(project, 'src/main.rs'), 'fn main() {}'); writeFileSync(join(project, '..notes'), 'kept');
  writeFileSync(join(outside, 'secret'), 'never sent');
  symlinkSync(join(outside, 'secret'), join(project, 'link'));
  symlinkSync(outside, join(project, 'escape'));
  return { project, outside };
}
function call(toolName, path, cwd) {
  let handler;
  guard({ on: (event, h) => { assert.equal(event, 'tool_call'); handler = h; } });
  return handler({ type: 'tool_call', toolCallId: '1', toolName, input: path === undefined ? {} : { path } }, { cwd });
}

test('reads inside the folder pass, by relative or absolute path', () => {
  const { project } = folder();
  for (const [tool, path] of [['read', 'src/main.rs'], ['read', join(project, 'src/main.rs')], ['read', '..notes'], ['ls', undefined], ['grep', '.'], ['find', ''], ['ls', 'src/../src']])
    assert.equal(call(tool, path, project), undefined, `${tool} ${path}`);
});

test('anything that would leave the folder is blocked', () => {
  const { project, outside } = folder();
  for (const [tool, path] of [['read', join(outside, 'secret')], ['read', '../outside/secret'], ['read', 'link'], ['grep', 'escape'], ['find', 'escape/'], ['ls', '..'], ['ls', '/'],
    ['read', '~/.ssh/id_ed25519'], ['ls', '~'], ['read', '@../outside/secret'], ['read', `file://${join(outside, 'secret')}`], ['read', `src /main.rs`], ['read', 'missing'], ['read', join(homedir(), '.ssh')]]) {
    const result = call(tool, path, project);
    assert.equal(result?.block, true, `${tool} ${path}`);
    assert.ok(!result.reason.includes('never sent'));
  }
});

test('tools other than the read-only four are blocked', () => {
  const { project } = folder();
  for (const tool of ['bash', 'write', 'edit', 'mcp__x'])
    assert.equal(call(tool, 'src/main.rs', project)?.block, true, tool);
});
