//! Read-only CLI refusals never create session records.
use std::{fs, process::Command};

#[test]
fn seed_cli_refusals_are_explicit_and_never_create_a_session() {
    let root = tempfile::Builder::new()
        .prefix("TEST-seed-cli-refusal-")
        .tempdir()
        .unwrap();
    let home = tempfile::Builder::new()
        .prefix("tinkery-TEST-cli-home-")
        .tempdir()
        .unwrap();
    for args in [vec!["status", "--json"], vec!["checks"], vec!["status"]] {
        let output = Command::new("python3")
            .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/test_launch.py"))
            .arg(env!("CARGO_BIN_EXE_tinkery"))
            .env("HOME", home.path())
            .env("TINKERY_NODE", "/tinkery-TEST-no-node")
            .args(["--operator", "agent", "seed"])
            .args(&args)
            .arg(root.path())
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        let reason = String::from_utf8(output.stderr).unwrap();
        assert!(
            reason.contains(if args == ["status"] {
                "usage:"
            } else {
                "Tinkery refused:"
            }),
            "{reason}"
        );
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
    }
}
