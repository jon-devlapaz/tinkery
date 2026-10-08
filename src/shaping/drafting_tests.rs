use super::*;

const DRAFT: &str = r#"{"goal":"Find decision reasons near the work.","outcome":"People can understand why a choice was made.","assumptions":[],"options":[{"label":"Graph","benefit":"See connections","cost":"Maintain links","undo_cost":"Unknown until an approach is chosen"}]}"#;

fn request() -> DraftRequest {
    DraftRequest {
        thoughts: vec![Thought {
            id: "one".into(),
            title: "Note 1".into(),
            text: "I want a graph, so I can find reasons.".into(),
        }],
        previous_draft: None,
        feedback: None,
        earlier_feedback: Vec::new(),
    }
}

#[test]
fn draft_values_cannot_supply_authority_or_terminal_controls() {
    let draft = WorkingDraft::parse(DRAFT).unwrap();
    let paper = draft.sections(&request().thoughts).join("\n\n");
    assert!(paper.contains("Possible approaches / not accepted"));
    assert!(paper.contains("Graph / candidate only"));
    assert!(paper.contains("No goal, seed, or approach is confirmed"));
    let mut labelled = draft.clone();
    labelled.assumptions =
        vec!["PROVISIONAL: PROVISIONAL: The work can be linked to specific decisions.".into()];
    let labelled_paper = labelled.sections(&request().thoughts).join("\n\n");
    assert!(
        labelled_paper.contains("- PROVISIONAL: The work can be linked to specific decisions.")
    );
    assert!(!labelled_paper.contains("PROVISIONAL: PROVISIONAL:"));
    for invalid in [
        "",
        "```json\n{}\n```",
        r#"{"goal":"x","outcome":"y","assumptions":[],"options":[],"status":"confirmed for intake"}"#,
        r#"{"goal":" ","outcome":"y","assumptions":[],"options":[]}"#,
        r#"{"goal":"\u001b[31m","outcome":"y","assumptions":[],"options":[]}"#,
    ] {
        assert!(WorkingDraft::parse(invalid).is_err(), "Accepted {invalid}");
    }
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
fn pi_process_receives_only_bounded_drafting_inputs_and_disabled_resources() {
    let script = format!(
        r#"
import json, os, sys
required = ['--print', '--no-session', '--no-tools', '--no-extensions', '--no-mcp', '--no-skills', '--no-prompt-templates', '--no-themes', '--no-context-files', '--no-approve', '--offline']
assert all(flag in sys.argv for flag in required)
assert 'PI_SESSION_FILE' not in os.environ
assert 'PI_SESSION_ID' not in os.environ
instructions = sys.argv[sys.argv.index('--system-prompt') + 1]
assert 'ACTUAL DRAFT INSTRUCTIONS' in instructions
assert 'DO NOT RUN THIS LATER STEP' not in instructions
assert "what becomes better, and for whom" in instructions
assert "never state a proposed solution, tool, interface, artifact, or implementation as the goal or outcome" in instructions
assert "Solutions appear only in options" in instructions
assert "never recast them as uncertain beliefs" in instructions
assert "plain text without a PROVISIONAL prefix" in instructions
assert sys.argv[sys.argv.index('--model') + 1] == 'anthropic/claude-haiku-4-5'
data = json.load(sys.stdin)
assert len(data['thoughts']) == 1
assert data['thoughts'][0]['text'] == 'I want a graph, so I can find reasons.'
assert data['feedback'] == 'Cut the graph; keep the intention.'
assert data['previous_draft']['goal'] == 'Find decision reasons near the work.'
assert not any('I want a graph' in arg for arg in sys.argv)
print({DRAFT:?})
"#
    );
    let (directory, host) = host(&script);
    let before = std::fs::read_dir(directory.path()).unwrap().count();
    let mut input = request();
    input.previous_draft = Some(WorkingDraft::parse(DRAFT).unwrap());
    input.feedback = Some("Cut the graph; keep the intention.".into());
    let draft = host.draft(input, &AtomicBool::new(false)).unwrap();
    assert_eq!(draft.goal, "Find decision reasons near the work.");
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), before);
}

#[cfg(unix)]
#[test]
fn failures_timeout_overflow_and_cancellation_never_return_a_draft() {
    for script in [
        "import sys; sys.exit(7)",
        "print('not JSON')",
        "print('x' * 40000)",
        "import time; time.sleep(10)",
    ] {
        let (_directory, mut host) = host(script);
        host.timeout = Duration::from_millis(100);
        assert!(
            host.draft(request(), &AtomicBool::new(false)).is_err(),
            "Accepted {script}"
        );
    }
    let (_directory, limited) =
        host("import sys; print('429 rate_limit_error', file=sys.stderr); sys.exit(1)");
    assert!(
        limited
            .draft(request(), &AtomicBool::new(false))
            .unwrap_err()
            .contains("rate limit (429)")
    );
    let mut oversized = request();
    oversized.thoughts[0].text = "x".repeat(LIMIT + 1);
    assert!(
        limited
            .draft(oversized, &AtomicBool::new(false))
            .unwrap_err()
            .contains("input exceeds")
    );
    let (_directory, host) = host("import time; time.sleep(10)");
    let job = DraftJob::start(Arc::new(host), request());
    thread::sleep(Duration::from_millis(50));
    let started = Instant::now();
    drop(job);
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "Exit waited for the model instead of cancelling"
    );
}
