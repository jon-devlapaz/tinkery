use ratatui::style::{Color, Modifier, Style};

pub mod goal;
pub mod measure;
pub mod menu;
pub mod shape;
pub mod think;
pub mod voice;

pub const MIN_WIDTH: u16 = 80;
pub const MIN_HEIGHT: u16 = 24;

#[derive(Clone, Copy)]
pub struct Palette {
    ink: Style,
    jade: Style,
    muted: Style,
    /// Consequence only (`!`, loss prompts). Never the sole cue: always paired with `!` and words.
    rust: Style,
}

impl Palette {
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
