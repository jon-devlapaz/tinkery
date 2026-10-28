//! Shape's F1 list: every action allowed on the open session now besides the F2 one the key bar
//! names. Picking a row runs exactly that action. Rows that write (delegate, endings) still go
//! through the session's own checks; ending a session asks again before it happens.

use super::*;
use crate::actions::Row;

/// An action F1 can run on the open session.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Choice {
    Reveal,
    Defer,
    Delegate,
    Object,
    Revise,
    TeachBack,
    Folder,
    SoFar,
    End(Ending),
    Model,
}

impl Shape {
    /// The actions allowed now, most likely first. Empty without an open, idle session, or
    /// while a form, folder, teach-back, objection or ending is in progress.
    pub(super) fn rows(&self) -> Vec<Row<Choice>> {
        let Some(open) = &self.open else {
            return vec![];
        };
        if open.running.is_some()
            || open.writing.is_some()
            || open.form.is_some()
            || open.choosing.is_some()
            || open.teach_back.is_some()
            || open.objecting
            || open.asking.is_some()
            || open.so_far.is_some()
        {
            return vec![];
        }
        let mut rows = vec![];
        let agent = self.agent.is_some() && !self.held;
        if open.instinct_first() {
            rows.push(Row::new("Show the options and suggestion", Choice::Reveal));
        }
        if agent && open.offers_answers() {
            rows.push(Row::new("Defer, with a reason", Choice::Defer));
            if !open.instinct_first() {
                rows.push(Row::new("Delegate to the suggestion", Choice::Delegate));
            }
        }
        if open.offers_objection() {
            rows.push(Row::new("Object to an assumption", Choice::Object));
        }
        if agent && open.revisable() {
            rows.push(Row::new("Revise the answer", Choice::Revise));
        }
        if agent && open.offers_teach_back() {
            rows.push(Row::new("Teach back what will be built", Choice::TeachBack));
        }
        if agent && open.offers_step() && matches!(open.folder, Folder::Known(_)) {
            rows.push(Row::new("Choose the folder to read", Choice::Folder));
        }
        if open.so_far_ready() {
            rows.push(Row::new("So far: goal and decisions", Choice::SoFar));
        }
        if !self.held {
            match open.offers_end() {
                Some(Ending::Completed) => rows.push(Row::new(
                    "End this session as completed",
                    Choice::End(Ending::Completed),
                )),
                Some(Ending::Stopped) => rows.push(Row::new(
                    "Stop this session for good",
                    Choice::End(Ending::Stopped),
                )),
                None => {}
            }
        }
        if self.agent.is_some() && matches!(open.reading, Reading::Done(Ok(_))) {
            rows.push(Row::new("Model and calls", Choice::Model));
        }
        rows
    }

    /// Runs the row F1 numbers with this digit, if any; the caller closes help when it does.
    pub fn pick(&mut self, key: KeyEvent) -> bool {
        let rows = self.rows();
        if self.actions.turn(rows.len(), key) {
            self.help_scroll = 0;
            return false;
        }
        let Some(choice) = self.actions.pick(&rows, key) else {
            return false;
        };
        self.run(choice);
        true
    }

    fn run(&mut self, choice: Choice) {
        if choice == Choice::Model {
            self.model = true;
            return;
        }
        let agent = self.agent.clone();
        let Some(open) = &mut self.open else {
            return;
        };
        open.notice = None;
        match choice {
            Choice::Reveal => open.reveal(),
            Choice::Object => {
                open.objecting = true;
                open.notice = Some((
                    "Which assumption do you object to?".into(),
                    Some("Next: press its number; Esc keeps them all.".into()),
                ));
            }
            Choice::TeachBack => open.teach_back = Some(Words::new(open.typed.text())),
            Choice::Folder => {
                if let Folder::Known(folder) = &open.folder {
                    open.choosing = Some(Words::new(&folder.display().to_string()));
                }
            }
            Choice::SoFar => open.so_far = Some(0),
            Choice::End(ending) => {
                open.asking = Some(ending);
                open.notice = Some(asked(ending));
            }
            Choice::Model => {}
            Choice::Defer | Choice::Delegate | Choice::Revise => {
                let Some(agent) = agent else {
                    return;
                };
                match (&open.reading, choice) {
                    (
                        Reading::Done(Ok(Shaping {
                            stage: Shaped::Question { id, gate, .. },
                            version: Some(expected),
                            ..
                        })),
                        Choice::Defer,
                    ) => open.form = Some(Form::defer(id.clone(), *expected, gate.clone())),
                    (
                        Reading::Done(Ok(Shaping {
                            stage: Shaped::Question { id, .. },
                            version: Some(expected),
                            ..
                        })),
                        Choice::Delegate,
                    ) => {
                        let act = Act::Respond {
                            id: id.clone(),
                            expected: *expected,
                            action: AnswerAction::Delegate,
                        };
                        open.act(act, &agent);
                    }
                    (
                        Reading::Done(Ok(Shaping {
                            stage:
                                Shaped::Review {
                                    id,
                                    revisable: true,
                                    ..
                                },
                            version: Some(expected),
                            ..
                        })),
                        Choice::Revise,
                    ) => open.form = Some(Form::revise(id.clone(), *expected)),
                    _ => {}
                }
            }
        }
    }
}
