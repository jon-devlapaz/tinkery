//! TEST-only adapter to the real Rust ledger, for zero-call terminal fixtures. No alternate rules.
use std::{
    io::{self, Read},
    path::Path,
};
use tinkery::seed_me::ledger::{self, Value};

fn main() -> Result<(), String> {
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .map_err(|_| "Cannot read TEST request.")?;
    let request: serde_json::Value =
        serde_json::from_str(&input).map_err(|_| "Invalid TEST request.")?;
    let path = Path::new(request["path"].as_str().unwrap_or(""));
    let stamp = "2026-10-19T00:00:00+00:00";
    let value = Value::from(request["value"].clone());
    let result = match request["op"].as_str().ok_or("Missing TEST operation.")? {
        "init" => {
            if !path.is_absolute() || !path.to_string_lossy().contains("TEST") {
                return Err("A temporary TEST root is required.".into());
            }
            let directory = ledger::initialize(path, stamp)?;
            if request["operator"] == "simulated" {
                let mut ledger = ledger::load(&directory)?.to_json()?;
                ledger["operator"] = "simulated".into();
                std::fs::write(
                    directory.join("ledger.json"),
                    serde_json::to_vec_pretty(&ledger).unwrap(),
                )
                .map_err(|_| "Cannot save TEST operator.")?;
            }
            serde_json::json!({"session":directory})
        }
        "read" => ledger::load(path)?.to_json()?,
        "editable" => ledger::editable(&value)?.to_json()?,
        "flags" => ledger::review_flags(&value).to_json()?,
        "decisions" => ledger::decisions_block(&value)?.into(),
        "render" => ledger::checks_block(&value)?.into(),
        "checks" => ledger::read_checks(path)?.to_json()?,
        "set-checks" => ledger::set_checks(path, &value)?.to_json()?,
        "status" => ledger::status(path)?.to_json()?,
        "publish" => ledger::publish(
            path,
            &value,
            &Value::from(request["expected"].clone()),
            &Value::from(request["reason"].clone()),
            None,
            stamp,
        )
        .map_err(|e| e.reason)?
        .to_json()?,
        "confirm" => ledger::confirm_seed(
            path,
            request["revision"]
                .as_str()
                .ok_or("Missing TEST revision.")?,
            request["source"].as_str().ok_or("Missing TEST source.")?,
            stamp,
        )?
        .to_json()?,
        "end" => ledger::end(
            path,
            request["status"].as_str().ok_or("Missing TEST status.")?,
            request["reason"].as_str().ok_or("Missing TEST reason.")?,
            stamp,
        )
        .map_err(|e| e.reason)?
        .to_json()?,
        _ => return Err("Unknown TEST operation.".into()),
    };
    println!("{result}");
    Ok(())
}
