//! Pinned seed suites protect refusal messages, intake JSON and exact durable session bytes.

use std::{fs, path::Path, process::Command};
use tinkery::seed_me::ledger::{self, Value};

const STAMP: &str = "2026-10-19T00:00:00+00:00";
const FILES: &[&str] = &[
    "ledger.json",
    "seed-contract.md",
    "seed-checks.json",
    "seed-confirmation.json",
    ".writer.lock",
];

fn unhex(hex: &str) -> Vec<u8> {
    hex.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}
fn files(directory: &Path) -> serde_json::Value {
    let mut values = serde_json::Map::new();
    for name in FILES {
        if let Ok(bytes) = fs::read(directory.join(name)) {
            values.insert(
                (*name).into(),
                bytes
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>()
                    .into(),
            );
        }
    }
    values.into()
}
fn invoke(case: &serde_json::Value, directory: &Path) -> Result<String, String> {
    let args: Vec<_> = case["args"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap())
        .collect();
    let option = |flag| args[args.iter().position(|s| *s == flag).unwrap() + 1];
    let json = |value: Value| value.pretty();
    match case["op"].as_str().unwrap() {
        "decisions" => ledger::decisions_block(&ledger::load(directory)?),
        "checks-set" => json(ledger::set_checks(
            directory,
            &Value::parse(case["records"].as_str().unwrap())?,
        )?),
        "checks-show" | "checks-render" => {
            ledger::load(directory)?;
            let records = ledger::read_checks(directory)?;
            if case["op"] == "checks-show" {
                json(records)
            } else {
                ledger::checks_block(&records)
            }
        }
        "confirm" => json(ledger::confirm_seed(
            directory,
            option("--revision"),
            option("--source"),
            STAMP,
        )?),
        "status" => json(ledger::status(directory)?),
        "end" => json(
            ledger::end(directory, option("--status"), option("--reason"), STAMP)
                .map_err(|e| e.reason)?,
        ),
        op => panic!("unknown operation {op}"),
    }
}

#[test]
fn seed_cli_refusals_are_explicit_and_never_create_a_session() {
    let root = tempfile::Builder::new()
        .prefix("TEST-seed-cli-refusal-")
        .tempdir()
        .unwrap();
    for args in [vec!["status", "--json"], vec!["checks"], vec!["status"]] {
        let output = Command::new(env!("CARGO_BIN_EXE_tinkery"))
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

#[test]
fn pinned_seed_commands_and_intake_json_match() {
    let oracle = Command::new("python3")
        .arg("tests/ledger/seed_oracle.py")
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .env("PYTHONHASHSEED", "0")
        .output()
        .unwrap();
    assert!(
        oracle.status.success(),
        "{}",
        String::from_utf8_lossy(&oracle.stderr)
    );
    let root = tempfile::Builder::new()
        .prefix("TEST-seed-differential-")
        .tempdir()
        .unwrap();
    let directory = root.path();
    let (mut count, mut summary, mut differences) = (0, false, vec![]);
    for line in String::from_utf8(oracle.stdout).unwrap().lines() {
        let case: serde_json::Value = serde_json::from_str(line).unwrap();
        if let Some(value) = case.get("summary") {
            assert_eq!(value["upstream_tests"], 96, "pinned seed suites changed");
            println!("seed differential: {value}");
            summary = true;
            continue;
        }
        count += 1;
        for name in FILES {
            let path = directory.join(name);
            if path.exists() {
                fs::remove_file(path).unwrap();
            }
        }
        for (name, hex) in case["before"].as_object().unwrap() {
            fs::write(directory.join(name), unhex(hex.as_str().unwrap())).unwrap();
        }
        let normalize = |text: &str| {
            text.replace(directory.to_str().unwrap(), "TEST-SESSION")
                .replace(case["directory"].as_str().unwrap(), "TEST-SESSION")
        };
        let name = format!("{} / {} / {count}", case["name"], case["op"]);
        match invoke(&case, directory) {
            Ok(actual) if case["code"] == 0 => {
                let expected = case["output"].as_str().unwrap();
                let equal = if ["decisions", "checks-render"]
                    .contains(&case["op"].as_str().unwrap())
                {
                    actual == expected
                } else {
                    serde_json::from_str::<serde_json::Value>(&normalize(&actual)).unwrap()
                        == serde_json::from_str::<serde_json::Value>(&normalize(expected)).unwrap()
                };
                if !equal {
                    differences.push(format!("{name}: output mismatch\n{actual:?}\n{expected:?}"));
                }
            }
            Err(actual) if case["code"] != 0 => {
                let expected = case["message"].as_str().unwrap();
                if normalize(&actual) != normalize(expected) {
                    differences.push(format!(
                        "{name}: refusal mismatch {actual:?} != {expected:?}"
                    ));
                }
            }
            result => differences.push(format!(
                "{name}: verdict mismatch {result:?}, Python {} {}",
                case["code"], case["message"]
            )),
        }
        if files(directory) != case["after"] {
            differences.push(format!("{name}: durable bytes differ"));
        }
        if case["code"] == 0 && ["status", "checks-show"].contains(&case["op"].as_str().unwrap()) {
            let before = files(directory);
            let mut command = Command::new(env!("CARGO_BIN_EXE_tinkery"));
            command.args(["--operator", "agent", "seed"]);
            if case["op"] == "status" {
                command.args(["status", "--json"]);
            } else {
                command.arg("checks");
            }
            let output = command.arg(directory).output().unwrap();
            assert!(
                output.status.success(),
                "{name}: CLI {}",
                String::from_utf8_lossy(&output.stderr)
            );
            let actual: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
            let expected: serde_json::Value =
                serde_json::from_str(case["output"].as_str().unwrap()).unwrap();
            assert_eq!(actual, expected, "{name}: CLI JSON");
            assert_eq!(files(directory), before, "CLI wrote: {name}");
        }
    }
    assert!(summary && count >= 1275, "seed oracle incomplete: {count}");
    assert!(
        differences.is_empty(),
        "{} differences:\n{}",
        differences.len(),
        differences
            .iter()
            .take(80)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
