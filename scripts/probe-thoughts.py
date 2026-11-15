#!/usr/bin/env python3
"""Prove thought protections by introducing three faults, then restore sources and binary."""
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT/'tests'))
import test_launch

THOUGHT = ROOT/'src/tend/thought.rs'
CONNECTION = ROOT/'src/tend/connection.rs'
FAULTS = [
    ('confirmed-goal', THOUGHT, '        let path = self.file(words)?;', '''        crate::seed_me::Config::new(Some(root.parent().unwrap().join("sessions"))).confirm_as(
            crate::seed_me::Affirmed {
                goal: words.lines().next().unwrap().trim().into(), outcome: String::new(), options: vec![], originals: vec![],
                context: String::new(), source: "TEST faulty thought confirmation".into(),
            },
            &crate::think_record::GoalConfirmation::terminal(Some("agent"))?,
        )?;
        let path = self.file(words)?;''', 'A thought changed goal sessions'),
    ('changed-words', THOUGHT, '            words: words.into(),',
     '            words: words.trim().into(),', 'Saved words differ from typed words'),
    ('provider-source', CONNECTION, '        sources.sort_by(|a, b| a.id.cmp(&b.id));', '''        for entry in fs::read_dir(&self.store).map_err(|e| e.to_string())? {
            let path = entry.map_err(|e| e.to_string())?.path();
            if path.file_name().unwrap().to_string_lossy().starts_with("thought-") {
                let thought: serde_json::Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
                if thought["repo"] == serde_json::json!(self.repo) {
                    sources.push(Source { id: "TEST-thought".into(),
                        goal: thought["words"].as_str().unwrap().into(), repo: self.repo.clone(),
                        status: "active".into() });
                }
            }
        }
        sources.sort_by(|a, b| a.id.cmp(&b.id));''', 'A thought reached the provider request'),
]


def main():
    env = test_launch.environment()
    output = ROOT/'target/thought-probes'
    output.mkdir(parents=True, exist_ok=True)
    originals = {p: p.read_text() for p in [THOUGHT, CONNECTION]}

    def build():
        test_launch.run(['cargo', '+stable', 'build', '--locked', '--quiet'], cwd=ROOT, env=env, check=True)

    try:
        for name, path, old, new, expected in FAULTS:
            source = originals[path]
            if source.count(old) != 1:
                raise SystemExit(f'{name}: mutation no longer matches exactly once')
            path.write_text(source.replace(old, new))
            try:
                build()
                result = test_launch.run([sys.executable, 'tests/tend_thought_terminal.py'], cwd=ROOT, env=env,
                                        capture_output=True, text=True, timeout=180)
                log = result.stdout + result.stderr
                (output/f'{name}.log').write_text(log)
                if result.returncode == 0 or expected not in log:
                    raise SystemExit(f'{name}: protection did not fail for the expected reason\n{log}')
                print(f'{name}: rejected ({expected})', flush=True)
            finally:
                path.write_text(source)
    finally:
        for path, source in originals.items():
            path.write_text(source)
        build()
    test_launch.run([sys.executable, 'tests/tend_thought_terminal.py'], cwd=ROOT, env=env, check=True)


if __name__ == '__main__':
    main()
