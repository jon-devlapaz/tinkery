//! One explicit connection discovery and its latest durable result. Source selection is automatic,
//! requests have no tools, and only the shared saver creates a confirmed goal.

mod correction;
mod panel;
use crate::{
    confirmation::GoalConfirmation,
    pi::Drafting,
    seed_me::{
        self,
        connection::{Correction, Origin, Source, Support},
        connection_save::Pending,
    },
    think_record::MadeIn,
};
pub(crate) use panel::Panel;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::PathBuf,
    sync::{Arc, atomic::AtomicBool},
};

const PROMPT: &str = include_str!("connection-prompt.txt");
const LIMIT: usize = 32 * 1024;
const RECORD_LIMIT: usize = 256 * 1024;

/// Captured launch destinations and shared drafting allowance. No phase object crosses this boundary.
#[derive(Clone)]
pub(crate) struct Context {
    pub(crate) sessions: PathBuf,
    pub(crate) records: PathBuf,
    pub(crate) store: PathBuf,
    pub(crate) repo: PathBuf,
    pub(crate) call: Option<Drafting>,
    pub(crate) actor: GoalConfirmation,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Input {
    repo: PathBuf,
    sources: Vec<Source>,
    model: String,
    thinking: String,
    harness: String,
    prompt: String,
    #[serde(default)]
    correction: Option<Correction>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Proposal {
    sources: Vec<Support>,
    relationship: String,
    goal: String,
    outcome: String,
    question: String,
    assumptions: Vec<String>,
}

impl Proposal {
    fn origin(&self, input: &Input, goal: &str) -> Origin {
        Origin {
            schema: if input.correction.is_some() { 2 } else { 1 },
            correction: input.correction.clone(),
            entry: "tend".into(),
            repo: input.repo.clone(),
            goal: goal.into(),
            sources: input.sources.clone(),
            support: self.sources.clone(),
            interpretation: format!(
                "{}\nJudgment: {}\nAssumptions: {}",
                self.relationship,
                self.question,
                self.assumptions.join("\n")
            ),
        }
    }
    fn validate(&self, input: &Input) -> Result<(), String> {
        if [
            &self.relationship,
            &self.goal,
            &self.outcome,
            &self.question,
        ]
        .iter()
        .any(|s| s.trim().is_empty())
        {
            return Err("The connection reply is incomplete. No goal was saved.".into());
        }
        self.origin(input, &self.goal).validate(&self.goal)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Record {
    schema: u8,
    input: Input,
    proposal: Option<Proposal>,
    dismissed: bool,
    pending: Option<Pending>,
    saved: Option<PathBuf>,
}

#[derive(Deserialize)]
#[serde(remote = "Record", deny_unknown_fields)]
struct RecordFields {
    schema: u8,
    input: Input,
    proposal: Option<Proposal>,
    dismissed: bool,
    pending: Option<Pending>,
    saved: Option<PathBuf>,
}

struct LegacyInput<'a>(&'a Input);

impl Serialize for LegacyInput<'_> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut fields = serializer.serialize_struct("Input", 6)?;
        fields.serialize_field("repo", &self.0.repo)?;
        fields.serialize_field("sources", &self.0.sources)?;
        fields.serialize_field("model", &self.0.model)?;
        fields.serialize_field("thinking", &self.0.thinking)?;
        fields.serialize_field("harness", &self.0.harness)?;
        fields.serialize_field("prompt", &self.0.prompt)?;
        fields.end()
    }
}

impl Serialize for Record {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut fields = serializer.serialize_struct("Record", 6)?;
        fields.serialize_field("schema", &self.schema)?;
        if self.schema == 1 {
            fields.serialize_field("input", &LegacyInput(&self.input))?;
        } else {
            fields.serialize_field("input", &self.input)?;
        }
        fields.serialize_field("proposal", &self.proposal)?;
        fields.serialize_field("dismissed", &self.dismissed)?;
        fields.serialize_field("pending", &self.pending)?;
        fields.serialize_field("saved", &self.saved)?;
        fields.end()
    }
}

impl<'de> Deserialize<'de> for Record {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(d)?;
        let present = value["input"].get("correction").is_some();
        if !matches!(
            (value["schema"].as_u64(), present),
            (Some(1), false) | (Some(2), true)
        ) {
            return Err(serde::de::Error::custom(
                "Unsupported exploration or correction field",
            ));
        }
        let record = RecordFields::deserialize(value).map_err(serde::de::Error::custom)?;
        if let Some(c) = &record.input.correction {
            c.validate().map_err(serde::de::Error::custom)?;
        }
        Ok(record)
    }
}

impl Context {
    fn file(&self) -> PathBuf {
        self.store.join(format!(
            "connection-{:x}.json",
            Sha256::digest(self.repo.as_os_str().as_encoded_bytes())
        ))
    }
    fn snapshot(&self) -> Result<Input, String> {
        self.snapshot_with(self.correction()?.value()?)
    }
    fn snapshot_with(&self, correction: Option<Correction>) -> Result<Input, String> {
        let found = super::scan(&self.sessions)?;
        if !found.complete()
            || found
                .sessions
                .iter()
                .any(|s| s.stage == seed_me::Stage::Unknown)
        {
            return Err(
                "Discovery is incomplete. Restore unreadable sessions before finding connections."
                    .into(),
            );
        }
        let active: Vec<_> = found.sessions.iter().filter(|s| !s.stage.ended()).collect();
        let origins = crate::seed_me::connection::origins(
            &self.records,
            &active
                .iter()
                .map(|s| (s.path.as_path(), s.goal.as_str()))
                .collect::<Vec<_>>(),
        )
        .map_err(|e| e.message)?;
        let mut sources = Vec::new();
        for (session, origin) in active.into_iter().zip(origins) {
            match origin {
                MadeIn::Unknown | MadeIn::Invalid(_) => return Err("An active goal has unknown or invalid repository attribution. Discovery is incomplete.".into()),
                MadeIn::Repo(repo) if repo == self.repo => sources.push(Source {
                    id: session.id.clone().ok_or("A source goal has no unique identity.")?, goal: session.goal.clone(), repo, status: "active".into(),
                }),
                _ => {},
            }
        }
        // The one rule linking uses: a source id held by any other readable session, ended ones
        // included, would make a saved need ambiguous.
        let ids = found.identities();
        if sources.iter().any(|s| ids.session(&s.id).is_none()) {
            return Err("Source identities are ambiguous.".into());
        }
        sources.sort_by(|a, b| a.id.cmp(&b.id));
        if sources.len() < 2 {
            return Err(
                "Save at least two goals in this repository before finding connections.".into(),
            );
        }
        if sources.len() > 20 {
            return Err(
                "This first connection pass supports at most 20 goals. Nothing was sent.".into(),
            );
        }
        let input = Input {
            repo: self.repo.clone(),
            sources,
            model: self
                .call
                .as_ref()
                .map_or("not configured".into(), |d| d.call.model.clone()),
            thinking: self
                .call
                .as_ref()
                .map_or(String::new(), |d| d.call.thinking.clone()),
            harness: self
                .call
                .as_ref()
                .map_or(String::new(), |d| d.call.harness.label().into()),
            prompt: format!("{:x}", Sha256::digest(PROMPT)),
            correction,
        };
        if serde_json::to_vec(&input).map_err(|e| e.to_string())?.len() > LIMIT {
            return Err("The complete goals exceed 32 KiB. Nothing was truncated or sent.".into());
        }
        Ok(input)
    }
    fn read(&self) -> Result<Option<Record>, String> {
        let path = self.file();
        let regular = "The exploration is not a regular local file.";
        let bytes = match crate::regular_file::read_bounded(&path, RECORD_LIMIT as u64, regular) {
            Ok(read) => read.bytes.ok_or("The saved exploration is oversized.")?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) if e.kind() == std::io::ErrorKind::InvalidInput => return Err(regular.into()),
            Err(e) => return Err(format!("The latest exploration is unreadable: {e}")),
        };
        let record: Record = serde_json::from_slice(&bytes)
            .map_err(|_| "The latest exploration is malformed. Preserve it for inspection.")?;
        if record.input.repo != self.repo {
            return Err("The exploration belongs to another repository or format.".into());
        }
        if let Some(proposal) = &record.proposal {
            proposal.validate(&record.input)?;
        }
        if let Some(pending) = &record.pending {
            if self.sessions.canonicalize().ok().as_ref() != Some(&pending.root)
                || pending.origin.repo != self.repo
                || pending.origin.sources != record.input.sources
            {
                return Err("The pending save differs from this launch or exploration.".into());
            }
            let proposal = record
                .proposal
                .as_ref()
                .ok_or("The pending save has no proposal.")?;
            if pending.origin != proposal.origin(&record.input, &pending.origin.goal)
                || pending.outcome != proposal.outcome
            {
                return Err("The pending save differs from the reviewed proposal.".into());
            }
            if let Some(saved) = &record.saved
                && pending.inspect()?.as_ref().map(|receipt| &receipt.session) != Some(saved)
            {
                return Err("The completed handoff cannot be verified. Resume the fixed destination before another pass.".into());
            }
        }
        if record.saved.is_some() && record.pending.is_none() {
            return Err("The saved destination has no confirmation receipt.".into());
        }
        Ok(Some(record))
    }
    fn lock(&self) -> Result<fs::File, String> {
        use std::os::fd::AsRawFd;
        fs::create_dir_all(&self.store).map_err(|e| e.to_string())?;
        let file = crate::regular_file::open(
            fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(false),
            libc::O_NOFOLLOW,
            &self.file().with_extension("lock"),
            "The connection lock is not a regular file.",
        )
        .map_err(|e| e.to_string())?;
        // SAFETY: File owns this descriptor through the operation; close releases the lock.
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            return Err("Another connection pass or save is active for this repository.".into());
        }
        Ok(file)
    }
    fn write(&self, record: &Record) -> Result<(), String> {
        let bytes = serde_json::to_vec(record).map_err(|e| e.to_string())?;
        if bytes.len() > RECORD_LIMIT {
            return Err("These edited words exceed the saved exploration limit. Shorten them and retry; the previous result is intact.".into());
        }
        match crate::regular_file::publish(
            &self.file(),
            &bytes,
            crate::regular_file::Existing::Replace,
        )
        .map_err(|e| e.to_string())?
        {
            crate::regular_file::Published::Durable => Ok(()),
            crate::regular_file::Published::Uncertain(e) => Err(e.to_string()),
        }
    }
    fn explore(&self, input: &Input, stop: &AtomicBool) -> Result<Record, String> {
        let _lock = self.lock()?;
        if let Some(previous) = self.read()? {
            if previous.pending.is_some() && previous.saved.is_none() {
                return Err("Resume or cancel the pending save first.".into());
            }
            if &previous.input == input {
                return Ok(previous);
            }
        }
        if &self.snapshot()? != input {
            return Err("The goal inputs changed. Reopen Find connections.".into());
        }
        let Drafting {
            call: configured,
            budget,
        } = self
            .call
            .as_ref()
            .ok_or("Choose a drafting model and allowance before finding connections.")?;
        let mut call = configured.clone();
        call.read_only_tools = false;
        call.folder = None;
        let reply = call.complete(
            budget,
            "connection",
            serde_json::to_vec(input).map_err(|e| e.to_string())?,
            pi_prompt(),
            stop,
        )?;
        if reply.len() > LIMIT {
            return Err("The connection reply is oversized.".into());
        }
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Reply {
            connection: Option<Proposal>,
        }
        let value: serde_json::Value =
            serde_json::from_str(&reply).map_err(|_| "The connection reply is not valid JSON.")?;
        if !value
            .as_object()
            .is_some_and(|o| o.len() == 1 && o.contains_key("connection"))
        {
            return Err("The connection reply must contain exactly one connection field.".into());
        }
        let reply: Reply = serde_json::from_value(value)
            .map_err(|_| "The connection reply is not valid structured data.")?;
        if let Some(proposal) = &reply.connection {
            proposal.validate(input)?;
        }
        if &self.snapshot()? != input {
            return Err(
                "The goal inputs changed during discovery. No new result was saved.".into(),
            );
        }
        let record = Record {
            schema: 2,
            input: input.clone(),
            proposal: reply.connection,
            dismissed: false,
            pending: None,
            saved: None,
        };
        self.write(&record)?;
        Ok(record)
    }
    fn dismiss(&self, shown: &Record) -> Result<Record, String> {
        let _lock = self.lock()?;
        let mut record = self.read()?.ok_or("The exploration is missing.")?;
        if &record != shown || record.pending.is_some() {
            return Err("The exploration changed or has a pending save. Reopen it.".into());
        }
        record.dismissed = true;
        self.write(&record)?;
        Ok(record)
    }
    fn save(&self, shown: &Record, words: &str) -> Result<(Record, seed_me::Receipt), String> {
        let _lock = self.lock()?;
        let mut record = self.read()?.ok_or("The exploration is missing.")?;
        if &record != shown {
            return Err("The exploration changed. Reopen it before saving.".into());
        }
        if record.pending.is_none() {
            if record.dismissed {
                return Err("This connection was dismissed.".into());
            }
            if self.snapshot()? != record.input {
                return Err("The source inputs changed. Find a fresh connection.".into());
            }
            let proposal = record
                .proposal
                .as_ref()
                .ok_or("There is no proposed goal to save.")?;
            record.pending = Some(Pending::prepare(
                &self.sessions,
                proposal.origin(&record.input, words),
                proposal.outcome.clone(),
                self.actor.clone(),
            )?);
            self.write(&record)?;
        }
        let pending = record.pending.as_ref().expect("prepared above");
        let receipt = pending.resume(|| {
            let mut current = self.snapshot()?;
            // A legacy pending handoff keeps its original prompt across this format upgrade.
            if record.schema == 1 {
                current.prompt.clone_from(&record.input.prompt);
            }
            if current == record.input {
                Ok(())
            } else {
                Err("The source inputs changed before publication.".into())
            }
        })?;
        record.saved = Some(receipt.session.clone());
        self.write(&record)?;
        Ok((record, receipt))
    }
    fn cancel(&self, shown: &Record) -> Result<Record, String> {
        let _lock = self.lock()?;
        let mut record = self.read()?.ok_or("The exploration is missing.")?;
        if &record != shown {
            return Err("The exploration changed. Reopen it.".into());
        }
        record
            .pending
            .as_ref()
            .ok_or("There is no pending save.")?
            .cancel()?;
        record.pending = None;
        self.write(&record)?;
        Ok(record)
    }
}

fn pi_prompt() -> String {
    crate::pi::assemble(PROMPT)
}
