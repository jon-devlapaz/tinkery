use super::*;

fn input(text: &str) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({"words": [text]})).unwrap()
}

#[cfg(unix)]
fn host(script: &str) -> (tempfile::TempDir, PiHost) {
    use std::os::unix::fs::PermissionsExt;
    let directory = tempfile::tempdir().unwrap();
    let program = directory.path().join("pi");
    std::fs::write(&program, format!("#!/usr/bin/env python3\n{script}\n")).unwrap();
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
    let skill = directory.path().join("SKILL.md");
    std::fs::write(&skill, "# Seed Me\n\n## Write for the person reading\nUse common words.\n\n## 1. Triage the ask and extract context\n\n### Shape the working draft\nACTUAL DRAFT INSTRUCTIONS\n\n### Size gate\nDO NOT RUN THIS LATER STEP\n").unwrap();
    let host = PiHost::new(program, "anthropic/claude-haiku-4-5".into(), skill).unwrap();
    (directory, host)
}

#[cfg(unix)]
#[test]
fn pi_process_receives_only_bounded_inputs_and_disabled_resources() {
    let script = r#"
import json, os, sys
required = ['--print', '--no-session', '--no-tools', '--no-extensions', '--no-mcp', '--no-skills', '--no-prompt-templates', '--no-themes', '--no-context-files', '--no-approve', '--offline']
assert all(flag in sys.argv for flag in required)
assert 'PI_SESSION_FILE' not in os.environ
assert 'PI_SESSION_ID' not in os.environ
instructions = sys.argv[sys.argv.index('--system-prompt') + 1]
assert 'ACTUAL DRAFT INSTRUCTIONS' in instructions
assert 'DO NOT RUN THIS LATER STEP' not in instructions
assert sys.argv[sys.argv.index('--thinking') + 1] == 'low'
assert sys.argv[sys.argv.index('--model') + 1] == 'anthropic/claude-haiku-4-5'
data = json.load(sys.stdin)
assert data['words'] == ['I want a graph, so I can find reasons.']
assert not any('I want a graph' in arg for arg in sys.argv)
print('done')
"#;
    let (directory, host) = host(script);
    assert!(
        PiHost::new(
            directory.path().join("pi"),
            "test/model".into(),
            directory.path().join("SKILL.md")
        )
        .unwrap()
        .with_thinking("automatic")
        .is_err()
    );
    let before = std::fs::read_dir(directory.path()).unwrap().count();
    let prompt = host.working_instructions().to_owned();
    let output = host
        .complete(
            input("I want a graph, so I can find reasons."),
            prompt,
            &AtomicBool::new(false),
        )
        .unwrap();
    assert_eq!(output.trim(), "done");
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), before);
}

#[cfg(unix)]
#[test]
fn failures_timeout_overflow_and_cancellation_never_return_output() {
    let call = |host: &PiHost, text: &str, cancelled: &AtomicBool| {
        host.complete(input(text), "prompt".into(), cancelled)
    };
    for script in [
        "import sys; sys.exit(7)",
        "print('x' * 40000)",
        "import time; time.sleep(10)",
    ] {
        let (_directory, mut host) = host(script);
        host.timeout = Duration::from_millis(100);
        assert!(
            call(&host, "words", &AtomicBool::new(false)).is_err(),
            "Accepted {script}"
        );
    }
    let (_directory, limited) =
        host("import sys; print('429 rate_limit_error', file=sys.stderr); sys.exit(1)");
    assert!(
        call(&limited, "words", &AtomicBool::new(false))
            .unwrap_err()
            .contains("rate limit (429)")
    );
    assert!(
        call(&limited, &"x".repeat(LIMIT + 1), &AtomicBool::new(false))
            .unwrap_err()
            .contains("Encoded request exceeds 32 KiB")
    );
    let (_directory, host) = host("import time; time.sleep(10)");
    let job = Job::launch(move |cancelled| call(&host, "words", cancelled));
    thread::sleep(Duration::from_millis(50));
    let started = Instant::now();
    drop(job);
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "Exit waited for the model instead of cancelling"
    );
}
