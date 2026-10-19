//! Tinkery: a calm, keyboard-first front door for agentic work.
//!
//! Each place in the menu is a phase with its own module (`think`, `tend`, `shape`, `approve`, `measure`); the menu
//! is the only module that knows them all. Phases never use each other: anything two of them
//! share (`goal`, `repo`, `seed_me`, `think_record`, `voice`, and the palette and sizes below) is shared code that names
//! no phase. `docs/design/architecture.md` has the map and `tests/architecture.rs` enforces it.
#![deny(missing_docs, unreachable_pub)]

use ratatui::style::{Color, Modifier, Style};

mod approve;
pub mod goal;
pub mod measure;
pub mod menu;
pub mod pi;
pub mod repo;
pub mod seed_me;
pub mod shape;
pub mod tend;
pub mod think;
pub mod think_record;
pub mod voice;

/// The narrowest terminal any screen draws for. Below it, a screen says what size it needs and
/// keeps every word; nothing is laid out half-visible.
pub const MIN_WIDTH: u16 = 80;
/// The shortest terminal any screen draws for; see [`MIN_WIDTH`].
pub const MIN_HEIGHT: u16 = 24;

/// How far PgUp/PgDn move a view `height` rows tall: a page less one row, so the last line read
/// stays in sight. Never less than one row.
pub fn page(height: u16) -> u16 {
    height.saturating_sub(1).max(1)
}

/// The four styles every screen draws with, one job each: ink for words, jade for the one thing to
/// act on, muted for what can wait, rust for consequences. With no colour each keeps its meaning
/// through weight, dimming or underline, so no cue depends on colour alone.
#[derive(Clone, Copy)]
pub struct Palette {
    ink: Style,
    jade: Style,
    muted: Style,
    /// Consequence only (`!`, loss prompts). Never the sole cue: always paired with `!` and words.
    rust: Style,
}

impl Palette {
    /// The palette for this terminal; `no_color` honours `NO_COLOR` and `--no-color`.
    pub fn new(no_color: bool) -> Self {
        let base = if no_color {
            Style::default()
        } else {
            Style::default()
                .fg(Color::Rgb(16, 15, 15))
                .bg(Color::Rgb(255, 252, 240))
        };
        Self {
            ink: base,
            jade: if no_color {
                base.bold()
            } else {
                base.fg(Color::Rgb(31, 122, 114)).bold()
            },
            muted: if no_color {
                base.add_modifier(Modifier::DIM)
            } else {
                base.fg(Color::Rgb(87, 86, 83))
            },
            rust: if no_color {
                base.bold().underlined()
            } else {
                base.fg(Color::Rgb(160, 62, 36)).bold()
            },
        }
    }
}

#[cfg(test)]
mod palette_tests {
    use super::*;
    #[test]
    fn palette_uses_its_colours_and_no_color_keeps_meaning_without_any() {
        let colour = Palette::new(false);
        assert_eq!(colour.ink.fg, Some(Color::Rgb(16, 15, 15)));
        assert_eq!(colour.ink.bg, Some(Color::Rgb(255, 252, 240)));
        assert_eq!(colour.jade.fg, Some(Color::Rgb(31, 122, 114)));
        assert_eq!(colour.muted.fg, Some(Color::Rgb(87, 86, 83)));
        assert_eq!(colour.rust.fg, Some(Color::Rgb(160, 62, 36)));
        let mono = Palette::new(true);
        for style in [mono.ink, mono.jade, mono.muted, mono.rust] {
            assert_eq!((style.fg, style.bg), (None, None));
        }
        assert_eq!(mono.jade.add_modifier, Modifier::BOLD);
        assert_eq!(mono.muted.add_modifier, Modifier::DIM);
        assert_eq!(
            mono.rust.add_modifier,
            Modifier::BOLD | Modifier::UNDERLINED
        );
    }
}
