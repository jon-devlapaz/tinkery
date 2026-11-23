//! Shape's F1 list: every action allowed on the open session now besides the F2 one the key bar
//! names. Picking a row runs exactly that action. Rows that write (delegate, endings) still go
//! through the session's own checks; ending a session asks again before it happens.

use super::*;
use crate::actions::Row;
use crate::seed_me::ledger::DeferredPrerequisite;

/// An action F1 can run on the open session, carrying what the row was offered on.
#[derive(Clone)]
pub(super) enum Choice {
    Approve(crate::seed_me::ready::SeedTarget),
    Reveal(String),
    Defer {
        id: String,
        expected: u64,
        gate: String,
    },
    Delegate(Act),
    Object,
    Revise {
        id: String,
        expected: u64,
    },
    Revisit(DeferredPrerequisite, u64),
    TeachBack,
    EditCheck(CheckEdit, String),
    Change,
    Folder(String),
    SoFar,
    End(Ending),
    Model,
}

impl Shape {
    /// The actions allowed now, most likely first. Empty without an open, idle, read session, or
    /// while a form, folder, teach-back, change, objection or ending is in progress.
    pub(super) fn rows(&self) -> Vec<Row<Choice>> {
        let Some(
            open @ Opened {
                mode: Mode::Idle(Editor::Stage),
                reading: Reading::Done(Ok(shaping)),
                ..
            },
        ) = &self.open
        else {
            return vec![];
        };
        let mut rows = vec![];
        if let Some(target) = &shaping.ready_target {
            rows.push(Row::new(
                "Open this Tink in Approve",
                Choice::Approve(target.clone()),
            ));
        }
        let agent = self.agent.is_some() && !self.held;
        let instinct = open.instinct_first();
        if let Shaped::Question { id, .. } = &shaping.stage
            && instinct
        {
            rows.push(Row::new(
                "Show the options and suggestion",
                Choice::Reveal(id.clone()),
            ));
        }
        match (&shaping.stage, shaping.version) {
            (Shaped::Question { id, gate, .. }, Some(expected)) => {
                if agent {
                    rows.push(Row::new(
                        "Defer, with a reason",
                        Choice::Defer {
                            id: id.clone(),
                            expected,
                            gate: gate.clone(),
                        },
                    ));
                    if !instinct {
                        rows.push(Row::new(
                            "Delegate to the suggestion",
                            Choice::Delegate(Act::Respond {
                                id: id.clone(),
                                expected,
                                action: AnswerAction::Delegate,
                            }),
                        ));
                    }
                }
                if !open.assumed.is_empty() {
                    rows.push(Row::new("Object to an assumption", Choice::Object));
                }
            }
            (
                Shaped::Review {
                    id,
                    revisable: true,
                    ..
                },
                Some(expected),
            ) if agent => rows.push(Row::new(
                "Revise the answer",
                Choice::Revise {
                    id: id.clone(),
                    expected,
                },
            )),
            _ => {}
        }
        if agent && let (Some(target), Some(expected)) = (&shaping.deferred, shaping.version) {
            rows.push(Row::new(
                "Answer deferred prerequisite",
                Choice::Revisit(target.clone(), expected),
            ));
        }
        if agent
            && let (
                Shaped::Drafted {
                    revision: Some(_),
                    text: Some(seed),
                },
                Some(expected),
            ) = (&shaping.stage, shaping.version)
        {
            for check in &shaping.checks {
                let number = check.index + 1;
                let edit = |method| {
                    Choice::EditCheck(
                        CheckEdit {
                            expected,
                            seed: seed.clone(),
                            target: check.clone(),
                            method,
                        },
                        check
                            .words()
                            .filter(|_| method == check.method())
                            .unwrap_or("")
                            .to_owned(),
                    )
                };
                rows.push(Row::new(
                    format!("Edit check {number}"),
                    edit(check.method()),
                ));
                if check.gap() == Some(CheckMethod::Command) {
                    rows.push(Row::new(
                        format!("Make check {number} a judgment"),
                        edit(CheckMethod::Manual),
                    ));
                }
            }
            rows.push(Row::new("Revise the draft", Choice::Change));
            rows.push(Row::new("Teach back what will be built", Choice::TeachBack));
        }
        if agent
            && open.offers_step()
            && let Folder::Known(step::Reads { folder, .. }) = &open.folder
        {
            rows.push(Row::new(
                "Choose the folder to read",
                Choice::Folder(folder.display().to_string()),
            ));
        }
        rows.push(Row::new("So far: goal and decisions", Choice::SoFar));
        if !self.held {
            // Completed once the seed is ready, stopped while it's still being shaped. Never for
            // an ended or unknown session.
            match shaping.stage {
                Shaped::Ready { ended: false, .. } => rows.push(Row::new(
                    "End this session as completed",
                    Choice::End(Ending::Completed),
                )),
                Shaped::Question { .. }
                | Shaped::Review { .. }
                | Shaped::Waiting
                | Shaped::Drafted { .. } => rows.push(Row::new(
                    "Stop this session for good",
                    Choice::End(Ending::Stopped),
                )),
                _ => {}
            }
        }
        if self.agent.is_some() {
            rows.push(Row::new("Model and responses", Choice::Model));
        }
        rows
    }

    /// Runs the row F1 numbers with this digit, if any; the caller closes help when it does.
    pub fn pick(&mut self, key: KeyEvent) -> bool {
        let rows = self.rows();
        if self.actions.turn(rows.len(), key) {
            self.help_scroll.reset();
            return false;
        }
        let Some(choice) = self.actions.pick(&rows, key) else {
            return false;
        };
        self.run(choice);
        true
    }

    fn run(&mut self, choice: Choice) {
        let Some(open) = &mut self.open else {
            return;
        };
        // The Model panel keeps what was said, for when Shape shows again.
        if !matches!(choice, Choice::Model) {
            open.notice = None;
        }
        match choice {
            Choice::Model => self.model = true,
            Choice::Approve(target) => self.approve = Some(target),
            Choice::Reveal(id) => open.reveal(id, self.agent.clone()),
            Choice::Object => {
                open.mode = Mode::Objecting;
                open.notice = Some(Notice::Said(
                    "Which assumption do you object to?".into(),
                    Some("Next: press its number; Esc keeps them all.".into()),
                ));
            }
            Choice::EditCheck(edit, words) => {
                let manual = edit.method == CheckMethod::Manual;
                open.mode = Mode::Idle(Editor::Aside(Aside::RepairCheck(edit), Words::new(&words)));
                open.notice = Some(Notice::Said(
                    String::new(),
                    manual.then(|| {
                        "A person signs off after seeing the built result; agent evidence can't sign it off.".into()
                    }),
                ));
                self.scroll = 0;
            }
            Choice::TeachBack => {
                let words = Words::new(open.typed.text());
                open.mode = Mode::Idle(Editor::Aside(Aside::TeachBack, words));
            }
            Choice::Change => {
                // Typed confirmation words are not a change.
                let typed = open.typed.text();
                let words = Words::new(if typed == CONFIRM { "" } else { typed });
                open.mode = Mode::Idle(Editor::Aside(Aside::Change, words));
            }
            Choice::Folder(folder) => open.mode = Mode::Choosing(Words::new(&folder)),
            Choice::SoFar => open.mode = Mode::SoFar(0),
            Choice::End(ending) => {
                open.mode = Mode::Asking(ending);
                open.notice = Some(asked(ending));
            }
            Choice::Revisit(target, expected) => {
                open.mode = Mode::Idle(Editor::Form(Form::revisit(target, expected)));
            }
            Choice::Defer { id, expected, gate } => {
                open.mode = Mode::Idle(Editor::Form(Form::defer(id, expected, gate)));
            }
            Choice::Revise { id, expected } => {
                open.mode = Mode::Idle(Editor::Form(Form::revise(id, expected)));
            }
            Choice::Delegate(act) => {
                if let Some(agent) = &self.agent {
                    open.act(act, agent);
                }
            }
        }
    }
}
