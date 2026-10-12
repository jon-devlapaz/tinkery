//! Think: turning a messy brain dump into a goal the person confirms, then saving it to Seed Me.
//!
//! Think reads no files or repository: it sees only what the person types or says, which keeps
//! the goal theirs. [`brain_dump`] is the screen and conversation, [`drafting`] the bounded
//! model call. Think hands a saved goal on only through `seed_me`.

pub mod brain_dump;
mod canvas;
pub mod drafting;
mod note;
#[cfg(test)]
mod note_tests;

pub use note::{Motion, Note};
