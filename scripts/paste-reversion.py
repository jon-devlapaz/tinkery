#!/usr/bin/env python3
"""Scoped paste protection probes. Restore source bytes in finally; never touch Git state.

Run separately from full verification, with no other writer or test runner active.
Uses Rust fixtures and the real PTY with fake processes, never providers or audio.
"""
from pathlib import Path
import os
import re
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
NOTE = ROOT / 'src/think/note.rs'
APP = ROOT / 'src/think/brain_dump.rs'
LAYOUT = ROOT / 'src/think/brain_dump/layout.rs'


def once(source, old, new):
    assert source.count(old) == 1, f'Probe anchor changed: {old!r}'
    return source.replace(old, new, 1)


def run(argv, expected, label):
    with tempfile.TemporaryDirectory(prefix='tinkery-TEST-paste-probe-') as directory:
        env = dict(os.environ)
        installed_home = Path(env['HOME'])
        env.setdefault('CARGO_HOME', str(installed_home / '.cargo'))
        env.setdefault('RUSTUP_HOME', str(installed_home / '.rustup'))
        env.update(HOME=directory, TINKERY_NODE='/tinkery-TEST-no-node', PYTHONDONTWRITEBYTECODE='1')
        result = subprocess.run(argv, cwd=ROOT, env=env,
                                capture_output=True, text=True, timeout=180)
    print(f'== {label}: exit {result.returncode}', flush=True)
    print(result.stdout[-6000:] + result.stderr[-2000:], flush=True)
    if argv[:4] == ['cargo', '+stable', 'test', '--locked']:
        assert re.search(r'running [1-9][0-9]* tests?', result.stdout), f'{label}: tests did not execute'
    assert (result.returncode == 0) == expected, f'{label}: unexpected result'


def rust(name):
    return ['cargo', '+stable', 'test', '--locked', '--lib', name, '--', '--nocapture']


def main():
    original = {p: p.read_bytes() for p in [NOTE, APP, LAYOUT]}
    try:
        source = original[NOTE].decode()
        cap = once(source, '        self.text\n            .try_reserve',
                   '        if self.text.len() - (end - start) + clean.len() > 4096 {\n'
                   '            return Err("historical editor cap");\n        }\n'
                   '        self.text\n            .try_reserve')
        NOTE.write_text(cap)
        run(rust('editing_crosses_the_historical_cap'), False, 'editor cap RED')
        run(['cargo', '+stable', 'build', '--locked'], True, 'build capped candidate')
        run(['python3', 'tests/selection_terminal.py'], False, 'actual bracketed paste RED')
        NOTE.write_bytes(original[NOTE])
        run(rust('editing_crosses_the_historical_cap'), True, 'editor cap restored GREEN')

        source = original[APP].decode()
        start = source.index('\n        if measured > 32768 {') + 1
        end = source.index('\n            self.refuse(if context_over', start)
        source = source[:start] + '        if false {' + source[end:]
        source = once(source, 'if serde_json::to_vec(&request).map_or(true, |b| b.len() > 32768)', 'if false')
        APP.write_text(source)
        run(rust('encoded_under_at_over_boundary'), False, 'preflight zero-start RED')
        APP.write_bytes(original[APP])
        run(rust('encoded_under_at_over_boundary'), True, 'preflight restored GREEN')

        source = once(original[APP].decode(), '        let mut cap = checks::Decision::new(',
                      '        let measured = measured.max(serde_json::to_vec(&staged.request).unwrap().len());\n'
                      '        let mut cap = checks::Decision::new(')
        APP.write_text(source)
        run(rust('waiting_reply_contraction'), False, 'stale-size contraction RED')
        APP.write_bytes(original[APP])
        run(rust('waiting_reply'), True, 'waiting replies restored GREEN')

        source = once(original[LAYOUT].decode(),
                      'matches!(\n        app.screen.refused,\n'
                      '        Some(Refusal::DraftTooLong | Refusal::EarlierWordsTooLong)\n    )',
                      'app.screen.refused.is_some() && app.notice.contains("32 KiB")')
        LAYOUT.write_text(source)
        run(rust('overflow_help_uses_semantic_kind'), False, 'notice-derived help RED')
        LAYOUT.write_bytes(original[LAYOUT])
        run(rust('overflow_help_uses_semantic_kind'), True, 'semantic help restored GREEN')

        source = once(original[NOTE].decode(), 'self.replace(start, end, text)',
                      'self.replace(start, end, &text.replace(\'\\r\', ""))')
        NOTE.write_text(source)
        run(rust('raw_controls_have_safe_display'), False, 'source fidelity RED')
        NOTE.write_bytes(original[NOTE])
        run(rust('raw_controls_have_safe_display'), True, 'source restored GREEN')

        source = once(original[NOTE].decode(), 'let display = Self::display(grapheme);',
                      'let display = grapheme.to_owned();')
        NOTE.write_text(source)
        run(rust('raw_controls_have_safe_display'), False, 'safe rendering RED')
        NOTE.write_bytes(original[NOTE])
        run(rust('raw_controls_have_safe_display'), True, 'safe rendering restored GREEN')
    finally:
        for path, content in original.items():
            path.write_bytes(content)
    run(['cargo', '+stable', 'build', '--locked'], True, 'restored binary')
    run(['python3', 'tests/selection_terminal.py'], True, 'restored actual PTY GREEN')
    for name in ['logical_and_wrapped_rows', 'editor_and_original_viewports',
                 'safe_control_cells', 'raw_display_mapping', 'large_fake_voice_final',
                 'waiting_reply', 'overflow_help_uses_semantic_kind', 'one_shape_then_one_review']:
        run(rust(name), True, f'current candidate {name} GREEN')
    print('Six protections RED independently, restored GREEN; source bytes restored.')


if __name__ == '__main__':
    main()
