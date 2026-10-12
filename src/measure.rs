//! Measure: the ledger's read-only report. The menu shows it; nothing here writes.

use std::{path::Path, process::Command};

/// The ledger's text report from the installed tink-substrate. Unknown stays unknown.
pub fn ledger_report(substrate_dir: &Path) -> String {
    match Command::new("python3")
        .args(["-B", "-m", "tink_substrate", "ledger", "report"])
        .current_dir(substrate_dir)
        .output()
    {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).into_owned(),
        Ok(out) => format!(
            "Ledger unknown: {}",
            String::from_utf8_lossy(&out.stderr)
                .lines()
                .last()
                .unwrap_or("report failed")
        ),
        Err(error) => format!(
            "Ledger unknown: cannot run tink-substrate in {} ({error})",
            substrate_dir.display()
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_ledger_is_unknown_not_empty() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(ledger_report(&tmp.path().join("missing")).starts_with("Ledger unknown"));
    }
}
