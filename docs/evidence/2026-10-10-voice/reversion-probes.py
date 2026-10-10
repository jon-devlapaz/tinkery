"""Fresh-copy protection probes. Never run Node native addons, microphones or Pi."""
from pathlib import Path
import json
import os
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[3]
OUT = Path(tempfile.mkdtemp(prefix='tinkery-TEST-voice-probes-'))
PROBES = [
    ('current-cursor', 'src/shaping/brain_dump/voice_input.rs', '        let text = Note::sanitize(&result.text);', '        self.input.home();\n        let text = Note::sanitize(&result.text);', 'partial_and_final_follow_current_cursor_not_a_captured_anchor', 'rust'),
    ('f2-final-only', 'src/voice/mod.rs', 'let send = self.pending_send;', 'let send = false;', 'send_during_start_or_listening_waits_and_sends_exactly_once', 'rust'),
    ('stale-take', 'src/voice/mod.rs', 'Event::Final { take, text } if self.take == Some(take) =>', 'Event::Final { take, text } if true =>', 'cancel_discards_and_stale_events_cannot_touch_new_take', 'rust'),
    ('deadline', 'src/voice/mod.rs', 'self.deadline = Some(now + Duration::from_secs(30));\n            self.command(Command::Stop', 'self.deadline = Some(now + Duration::from_secs(31));\n            self.command(Command::Stop', 'finalization_deadline_does_not_extend_and_late_final_cannot_send', 'rust'),
    ('filler', 'src/voice/mod.rs', '    matches!(\n        text.trim()', '    false && matches!(\n        text.trim()', 'silence_fillers_are_exact_not_a_semantic_rewrite', 'rust'),
    ('frozen-review', 'src/shaping/brain_dump/voice_input.rs', '            && self.goal_review.is_none()\n', '', 'review_and_receipt_disable_voice_and_cannot_receive_final_or_confirm', 'rust'),
    ('muted-cells', 'src/shaping/brain_dump/voice_input.rs', '                                palette.muted\n', '                                palette.ink\n', 'actual_partial_cells_are_muted_and_final_cells_are_ink_in_color_and_no_color', 'rust'),
    ('immutable-pcm', 'voice/helper.mjs', 'const immutable = pcm.slice();', 'const immutable = pcm;', 'asynchronous native feeds borrow immutable PCM buffers', 'node'),
    ('stdin-eof', 'voice/helper.mjs', "process.stdin.on('end', exit);", "process.stdin.on('end', () => {});", 'stdin EOF exits immediately during load, listen and finalization', 'node'),
    ('node-major', 'voice/helper.mjs', 'if (expected && expected !==', 'if (false && expected && expected !==', 'Node major mismatch precedes engine loading with the prescribed message', 'node'),
]


def check(path, test, language, log):
    command = (['cargo', '+1.99.0', 'test', '--locked', '--lib', test] if language == 'rust'
               else ['node', '--test', '--test-name-pattern=' + test, 'voice/helper.test.mjs'])
    result = subprocess.run(command, cwd=path, env={**os.environ, 'CARGO_TARGET_DIR': str(ROOT / 'target')},
                            stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, timeout=60)
    log.write_text(result.stdout)
    return result.returncode, result.stdout


results = []
for name, file, before, after, test, language in PROBES:
    copy = OUT / name
    copy.mkdir()
    for directory in ['src', 'voice', 'vendor', 'fixtures']:
        shutil.copytree(ROOT / directory, copy / directory)
    for filename in ['Cargo.toml', 'Cargo.lock']:
        shutil.copyfile(ROOT / filename, copy / filename)
    # copytree preserves mtimes; refresh source times so shared Cargo artifacts
    # cannot make a pristine baseline execute a preceding copy's mutant.
    for directory in ['src', 'voice']:
        for source in (copy / directory).rglob('*'):
            if source.is_file():
                source.touch()
    baseline, output = check(copy, test, language, OUT / (name + '-baseline.log'))
    assert baseline == 0, (name, 'baseline failed')
    if language == 'rust':
        assert 'Compiling tinkery' in output, (name, 'baseline did not recompile its fresh sources')
    target = copy / file
    code = target.read_text()
    assert code.count(before) == 1, (name, 'mutation is not unique')
    target.write_text(code.replace(before, after))
    mutant, output = check(copy, test, language, OUT / (name + '-mutant.log'))
    assert mutant != 0 and ('FAILED' in output or 'fail 1' in output or 'not ok' in output or 'test timed out after 5000ms' in output), (name, 'protection did not fail at runtime')
    results.append(dict(name=name, test=test, baseline=baseline, mutant=mutant))
    print(json.dumps(results[-1]), flush=True)
(OUT / 'results.json').write_text(json.dumps(results, indent=2) + '\n')
print('Retained fresh copies and logs:', OUT)
