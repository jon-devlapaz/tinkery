//! Differential contract with the pinned Python helper: refusals, reviews and exact ledger bytes.

use std::{fs, path::PathBuf, process::Command};
use tinkery::seed_me::ledger::{self, Value};

const STAMP: &str = "2026-10-19T00:00:00+00:00";

fn same_json(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Float(a), Value::Float(b)) if a.is_nan() && b.is_nan() => true,
        (Value::Array(a), Value::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(a, b)| same_json(a, b))
        }
        (Value::Object(a), Value::Object(b)) => {
            a.len() == b.len()
                && a.iter().all(|(k, v)| {
                    b.iter()
                        .find(|(other, _)| other == k)
                        .is_some_and(|(_, other)| same_json(v, other))
                })
        }
        _ => a == b,
    }
}

fn message_template(expr: &syn::Expr) -> String {
    match expr {
        syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Str(s),
            ..
        }) => s.value(),
        syn::Expr::Macro(m) => {
            use syn::parse::Parser;
            let args = syn::punctuated::Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated
                .parse2(m.mac.tokens.clone())
                .unwrap();
            if m.mac.path.is_ident("concat") {
                args.iter().map(message_template).collect()
            } else if m.mac.path.is_ident("format") {
                message_template(args.first().unwrap())
            } else {
                panic!("unrecognized refusal template; extend the coverage check")
            }
        }
        _ => panic!("refusal must have a checkable message template"),
    }
}

fn assert_refusal_coverage(root: &std::path::Path, refused: &[String]) {
    use syn::visit::Visit;
    struct Refusals(Vec<String>);
    impl<'ast> Visit<'ast> for Refusals {
        fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
            if matches!(&*call.func, syn::Expr::Path(p) if p.path.is_ident("require")) {
                self.0.push(message_template(&call.args[1]));
            }
            syn::visit::visit_expr_call(self, call);
        }
    }
    let source = fs::read_to_string(root.join("src/seed_me/ledger.rs")).unwrap();
    let mut rules = Refusals(vec![]);
    rules.visit_file(&syn::parse_file(&source).unwrap());
    let mut missing = vec![];
    let placeholder = regex::Regex::new(r"\\\{(?:[a-z_][a-z_0-9]*)?\\\}").unwrap();
    for template in &rules.0 {
        let escaped = regex::escape(template);
        let pattern = placeholder.replace_all(&escaped, ".*").into_owned();
        let pattern = regex::Regex::new(&format!("^{pattern}$")).unwrap();
        if !refused.iter().any(|m| pattern.is_match(m)) {
            missing.push(template);
        }
    }
    assert!(
        missing.is_empty(),
        "Rust refusal rules without cases: {missing:?}"
    );
    println!("Rust refusal coverage: {} sites", rules.0.len());
}

#[test]
fn pinned_python_ledger_rules_match() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let output = Command::new("python3")
        .arg(root.join("tests/ledger/oracle.py"))
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .env("PYTHONHASHSEED", "0")
        .env("PYTHONINTMAXSTRDIGITS", "4300")
        .output()
        .expect("python3 is required for the ledger differential test");
    assert!(
        output.status.success(),
        "pinned oracle failed closed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let temporary = tempfile::Builder::new()
        .prefix("TEST-ledger-rust-")
        .tempdir()
        .unwrap();
    let mut count = 0;
    let mut summary = false;
    let mut differences = vec![];
    let mut refusals = vec![];
    for line in String::from_utf8(output.stdout).unwrap().lines() {
        let answer = Value::parse(line).expect("oracle JSON");
        if answer.get("summary").is_some() {
            summary = true;
            println!(
                "ledger differential: {}",
                answer["summary"].pretty().unwrap()
            );
            continue;
        }
        count += 1;
        let case = &answer["case"];
        let name = case["name"].string().unwrap();
        let result = match case["op"].string().unwrap() {
            "load" => {
                let raw: Vec<u8> = if let Some(hex) = case.get("raw_hex").and_then(Value::string) {
                    hex.as_bytes()
                        .as_chunks::<2>()
                        .0
                        .iter()
                        .map(|bytes| {
                            u8::from_str_radix(std::str::from_utf8(bytes).unwrap(), 16).unwrap()
                        })
                        .collect()
                } else {
                    case.get("raw")
                        .and_then(Value::string)
                        .map(str::to_owned)
                        .unwrap_or_else(|| case["ledger"].pretty().unwrap())
                        .into_bytes()
                };
                fs::write(temporary.path().join("ledger.json"), &raw).unwrap();
                let result = ledger::load(temporary.path());
                assert_eq!(
                    fs::read(temporary.path().join("ledger.json")).unwrap(),
                    raw,
                    "load wrote: {name}"
                );
                result
            }
            "validate" => ledger::validate(&case["state"])
                .map(|ready| Value::Array(ready.iter().map(|s| Value::from(s.as_str())).collect())),
            "transition" => ledger::transition(
                &case["ledger"],
                &case["state"],
                &case["expected_version"],
                &case["reason"],
                case.get("revalidated"),
                STAMP,
            ),
            op => panic!("unknown case operation {op}"),
        };
        let fail = |detail: String| format!("{name}: {detail}");
        match (answer["verdict"].string().unwrap(), result) {
            ("refuse", Err(message)) => {
                refusals.push(message.clone());
                let allowed = answer["allowed_messages"]
                    .array()
                    .is_some_and(|messages| messages.iter().any(|m| m.string() == Some(&message)));
                if message != answer["message"].string().unwrap() && !allowed {
                    differences.push(fail(format!(
                        "refusal {message:?}, Python {:?}",
                        answer["message"]
                    )));
                }
            }
            ("crash" | "divergence", Err(message)) => {
                refusals.push(message);
            }
            ("accept", Ok(result)) if case["op"].string() == Some("validate") => {
                if result != answer["ready"] {
                    differences.push(fail(format!("ready {result:?} != {:?}", answer["ready"])));
                }
            }
            ("accept", Ok(result)) => {
                if ledger::review_flags(&result["nodes"]) != answer["flags"] {
                    differences.push(fail("review flags differ".into()));
                }
                if !same_json(&ledger::editable(&result).unwrap(), &answer["editable"]) {
                    differences.push(fail("editable fields differ".into()));
                }
                if answer.get("serialization_refuses").is_some() {
                    match result.pretty() {
                        Ok(_) => differences.push(fail("invalid serialization accepted".into())),
                        Err(message)
                            if Some(message.as_str())
                                != answer["serialization_message"].string() =>
                        {
                            differences.push(fail(format!(
                                "serialization refusal {message:?}, Python {:?}",
                                answer["serialization_message"]
                            )));
                        }
                        Err(_) => {}
                    }
                } else {
                    let bytes = result.pretty().unwrap();
                    let expected = answer["bytes"].string().unwrap();
                    if bytes != expected {
                        let line = bytes
                            .lines()
                            .zip(expected.lines())
                            .position(|(a, b)| a != b)
                            .unwrap_or(0);
                        differences.push(fail(format!(
                            "bytes differ at line {}: Rust {:?}, Python {:?}",
                            line + 1,
                            bytes.lines().nth(line),
                            expected.lines().nth(line)
                        )));
                    }
                }
            }
            (verdict, result) => differences.push(fail(format!(
                "Python {verdict}: {:?}, Rust {result:?}",
                answer["message"]
            ))),
        }
    }
    assert!(
        summary && count >= 8495,
        "empty or incomplete corpus: {count}"
    );
    assert!(
        differences.is_empty(),
        "{} differential failures:\n{}",
        differences.len(),
        differences
            .iter()
            .take(30)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
    assert_refusal_coverage(&root, &refusals);
}

#[test]
fn oracle_fails_closed_without_fixture_or_with_wrong_pin() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let temp = tempfile::Builder::new()
        .prefix("TEST-ledger-pin-")
        .tempdir()
        .unwrap();
    fs::create_dir(temp.path().join("scripts")).unwrap();
    fs::write(temp.path().join("scripts/session.py"), "TEST wrong pin").unwrap();
    for wrong_pin in [false, true] {
        let mut command = Command::new("python3");
        command
            .arg(root.join("tests/ledger/oracle.py"))
            .env("CI", "1")
            .env("PYTHONDONTWRITEBYTECODE", "1")
            .env_remove("SEED_ME_TEST_SKILL");
        if wrong_pin {
            command.env("SEED_ME_TEST_SKILL", temp.path().join("SKILL.md"));
        }
        let output = command.output().unwrap();
        assert!(
            !output.status.success(),
            "missing/wrong fixture must not silently pass"
        );
        assert!(
            output.stdout.is_empty(),
            "no cases may run before the pin is checked"
        );
        let message = String::from_utf8(output.stderr).unwrap();
        assert!(
            message.contains(if wrong_pin {
                "not the c9d7667 pin"
            } else {
                "SEED_ME_TEST_SKILL is required"
            }),
            "{message}"
        );
    }
}
