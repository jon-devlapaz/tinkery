//! Think's F1 list: every action allowed right now besides F2 sending, in plain words. Picking
//! a row closes help and runs exactly that action, with the same refusals it always had. Nothing
//! here sends words or saves a goal: reviewing still needs a typed `confirm`.
use super::*;
use crate::actions::Row;

/// An action F1 can run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Act {
    Review,
    UseSuggestion,
    Skip,
    UndoSkip,
    NewDump,
    Originals,
    Why,
    Cards,
    Copy,
    Model,
}

impl BrainDump {
    /// The actions allowed now, most likely first. Empty while saving, reviewing, after saving,
    /// while asking whether to leave, or during a voice take.
    pub(super) fn rows(&self) -> Vec<Row<Act>> {
        if self.screen.leave_prompt
            || self.handoff.job.is_some()
            || self.handoff.review.is_some()
            || self.handoff.receipt.is_some()
            || self.voice_active()
        {
            return vec![];
        }
        let mut rows = vec![];
        let board = self.conversation.board();
        if self.can_review() {
            rows.push(Row::new("Review the goal", Act::Review));
        }
        if self.can_use_suggestion() {
            rows.push(Row::new("Use the suggested answer", Act::UseSuggestion));
        }
        if board.is_some() && self.conversation.focused_question().is_some() {
            rows.push(Row::new("Skip this question", Act::Skip));
        }
        if !self.conversation.skipped().is_empty() {
            rows.push(Row::new("Bring back the skipped question", Act::UndoSkip));
        }
        if !self.conversation.sources().is_empty() {
            rows.push(Row::new(
                if self.add_more {
                    "Answer instead of a new dump"
                } else {
                    "Start a new dump instead"
                },
                Act::NewDump,
            ));
            rows.push(Row::new("Show your originals", Act::Originals));
            rows.push(Row::new("Why: attempts and checks", Act::Why));
            if !self.conversation.fragments().is_empty() {
                rows.push(Row::new(
                    if self.screen.show_extractions {
                        "Hide the extracted cards"
                    } else {
                        "Show the extracted cards"
                    },
                    Act::Cards,
                ));
            }
            if board.is_some() {
                rows.push(Row::new("Copy the reading", Act::Copy));
            }
        }
        if self.host.calls_left().is_some() {
            rows.push(Row::new("Model and runs", Act::Model));
        }
        rows
    }

    /// Close help, then run `act`.
    pub(super) fn run(&mut self, act: Act) {
        self.screen.help = false;
        self.screen.how = false;
        self.screen.original_scroll = 0;
        match act {
            Act::Review => self.review_goal(),
            Act::UseSuggestion => self.use_suggestion(),
            Act::Skip => self.skip_question(),
            Act::UndoSkip => self.undo_skip(),
            Act::NewDump => self.new_dump(),
            Act::Originals => {
                self.screen.details = false;
                self.screen.original = true;
            }
            Act::Why => {
                self.screen.original = false;
                self.screen.details = true;
                self.screen.paper_scroll = 0;
            }
            Act::Cards => self.toggle_cards(),
            Act::Copy => self.copy_board(),
            Act::Model => self.screen.model = true,
        }
    }
}
