use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::Paragraph,
};

pub mod home;
pub mod shaping;
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

fn markdown<'a>(source: &'a str, palette: Palette) -> Text<'a> {
    Text::from(
        source
            .lines()
            .map(|line| {
                let hashes = line.bytes().take_while(|byte| *byte == b'#').count();
                if (1..=6).contains(&hashes) && line.get(hashes..hashes + 1) == Some(" ") {
                    Line::styled(&line[hashes + 1..], palette.ink.bold())
                } else {
                    Line::from(markdown_spans(line, palette))
                }
            })
            .collect::<Vec<_>>(),
    )
}

fn markdown_spans(mut source: &str, palette: Palette) -> Vec<Span<'_>> {
    let mut spans = Vec::new();
    while let Some(start) = source.find(['*', '`']) {
        if start > 0 {
            spans.push(Span::styled(&source[..start], palette.ink));
        }
        source = &source[start..];
        let marker = if source.starts_with("**") {
            "**"
        } else if source.starts_with('`') {
            "`"
        } else {
            "*"
        };
        let content = &source[marker.len()..];
        if let Some(end) = content.find(marker).filter(|end| *end > 0) {
            let style = match marker {
                "**" => palette.ink.bold(),
                "*" => palette.ink.italic(),
                _ => palette.ink,
            };
            spans.push(Span::styled(&content[..end], style));
            source = &content[end + marker.len()..];
        } else {
            spans.push(Span::styled(marker, palette.ink));
            source = content;
        }
    }
    if !source.is_empty() {
        spans.push(Span::styled(source, palette.ink));
    }
    spans
}

fn render_scrolled(
    frame: &mut Frame,
    paragraph: Paragraph<'_>,
    area: Rect,
    scroll: &mut u16,
    max_scroll: &mut u16,
) {
    let total = paragraph.line_count(area.width).min(usize::from(u16::MAX)) as u16;
    *max_scroll = total.saturating_sub(area.height);
    *scroll = (*scroll).min(*max_scroll);
    frame.render_widget(paragraph.scroll((*scroll, 0)), area);
}

#[cfg(test)]
mod markdown_tests {
    use super::*;
    #[test]
    fn headings_and_emphasis_are_styled_without_markers_or_text_loss() {
        let source = "### Candidate\n**Benefit:** Preserve Jev.\n*Provisional only.*\nAn unmatched * stays visible.";
        let text = markdown(source, Palette::new(false));
        assert_eq!(text.lines[0].to_string(), "Candidate");
        assert!(text.lines[0].style.add_modifier.contains(Modifier::BOLD));
        assert_eq!(text.lines[1].to_string(), "Benefit: Preserve Jev.");
        assert!(
            text.lines[1].spans[0]
                .style
                .add_modifier
                .contains(Modifier::BOLD)
        );
        assert_eq!(text.lines[2].to_string(), "Provisional only.");
        assert!(
            text.lines[2].spans[0]
                .style
                .add_modifier
                .contains(Modifier::ITALIC)
        );
        assert_eq!(text.lines[3].to_string(), "An unmatched * stays visible.");
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
