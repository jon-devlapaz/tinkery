//! The model setup Think and Shape draft with: the harness (Pi or Claude Code), the model, its
//! thinking level and the calls each launch allows. One parser checks the saved file (`~/.config/tinkery/model.json`) and the launch
//! flags alike, so neither can hold a value the other would refuse. Flags win for one launch and
//! write nothing. A missing file means practice; an unreadable or invalid one is refused with its
//! path and Tinkery stays in practice rather than guessing. Only `s` in the Model panel writes the
//! file. Opening, reading or changing the panel starts no request; calls added there count for
//! this launch alone. Names no phase.

mod panel;

use std::{
    ffi::OsStr,
    io::Write,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

pub use crate::pi::Harness;
pub use panel::Controls;

/// The per-launch call allowances the panel offers. Zero is practice.
pub const CALLS: [usize; 4] = [0, 5, 10, 25];

/// A checked model setup. With calls it drafts with `model`; with none it is practice.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Setup {
    /// Which program drafts; a file without one means Pi.
    #[serde(default)]
    pub harness: Harness,
    /// A model the harness accepts, or empty when no model is chosen.
    pub model: String,
    /// One of the harness's thinking levels.
    pub thinking: String,
    /// Requests each launch allows, shared by Think and Shape.
    pub calls: usize,
}

impl Setup {
    /// The one parser for the four values, from flags or the file. Refuses a model or thinking
    /// level the harness doesn't accept, and calls without a model.
    pub fn new(
        harness: Harness,
        model: String,
        thinking: String,
        calls: usize,
    ) -> Result<Self, String> {
        harness.check_thinking(&thinking)?;
        if !model.is_empty() {
            harness.check_model(&model)?;
        }
        if model.is_empty() && calls > 0 {
            return Err("Choose a model before allowing calls.".into());
        }
        Ok(Self {
            harness,
            model,
            thinking,
            calls,
        })
    }

    /// Pi, no model, low thinking, no calls.
    pub fn practice() -> Self {
        Self {
            harness: Harness::Pi,
            model: String::new(),
            thinking: "low".into(),
            calls: 0,
        }
    }

    /// The setup this launch drafts with, or `None` for practice. `--shape-pi` (`live`) drafts
    /// even with zero calls; each flag replaces its saved value for this launch only, and the
    /// saved model and thinking apply only to the saved harness. Without `--shape-pi`, a saved
    /// setup with calls drafts, unless an explicit zero-call limit selects practice.
    pub fn launch(
        live: bool,
        harness: Option<Harness>,
        model: Option<String>,
        thinking: Option<String>,
        calls: Option<usize>,
        saved: Option<&Setup>,
    ) -> Result<Option<Self>, String> {
        if !live {
            return Ok(saved.filter(|s| calls != Some(0) && s.calls > 0).cloned());
        }
        let harness = harness.or(saved.map(|s| s.harness)).unwrap_or_default();
        let same = saved.filter(|s| s.harness == harness);
        let model = model
            .or_else(|| same.map(|s| s.model.clone()).filter(|m| !m.is_empty()))
            .ok_or_else(|| {
                format!(
                    "--shape-pi requires an explicit --model: {}",
                    harness.model_shape()
                )
            })?;
        let thinking = thinking
            .or_else(|| same.map(|s| s.thinking.clone()))
            .unwrap_or_else(|| "low".into());
        let calls = calls.or(saved.map(|s| s.calls)).unwrap_or(0);
        Self::new(harness, model, thinking, calls).map(Some)
    }
}

/// Where the setup is saved under an absolute `home`; `None` without one.
pub fn path(home: Option<&OsStr>) -> Option<PathBuf> {
    let home = Path::new(home?);
    home.is_absolute()
        .then(|| home.join(".config/tinkery/model.json"))
}

/// The saved setup at `path`: `None` when there is no file, or why it can't be used.
pub fn load(path: &Path) -> Result<Option<Setup>, String> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => {
            return Err(format!(
                "Cannot read {}. Check its permissions; using practice.",
                path.display()
            ));
        }
    };
    let refused = |why: String| {
        format!(
            "{} is not a model setup: {why} Save again from this panel to replace it; using practice.",
            path.display()
        )
    };
    let raw: Setup = serde_json::from_str(&text).map_err(|_| {
        refused("it needs model, thinking and calls, an optional harness, and nothing else.".into())
    })?;
    Setup::new(raw.harness, raw.model, raw.thinking, raw.calls)
        .map(Some)
        .map_err(refused)
}

/// Writes `setup` to `path` whole or not at all.
pub fn save(path: &Path, setup: &Setup) -> Result<(), String> {
    let cannot = |_| format!("Cannot save {}. Nothing changed.", path.display());
    let folder = path.parent().ok_or("Cannot save without a folder.")?;
    std::fs::create_dir_all(folder).map_err(cannot)?;
    let mut file = tempfile::NamedTempFile::new_in(folder).map_err(cannot)?;
    let json = serde_json::to_string_pretty(setup).expect("plain fields serialize");
    file.write_all(format!("{json}\n").as_bytes())
        .map_err(cannot)?;
    file.persist(path).map_err(|e| cannot(e.error))?;
    Ok(())
}

#[cfg(test)]
mod tests;
