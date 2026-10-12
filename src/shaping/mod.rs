pub mod brain_dump;
mod canvas;
pub mod confirmed_goal;
pub mod drafting;
mod note;
#[cfg(test)]
mod note_tests;

pub use note::{Motion, Note};
