use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Paragraph, Wrap},
};

pub mod home;
pub mod shaping;
pub mod voice;

pub const MIN_WIDTH: u16 = 80;
pub const MIN_HEIGHT: u16 = 24;

struct Work {
    title: &'static str,
    document: &'static str,
    question: &'static str,
    next: &'static str,
}

const WORK: [Work; 3] = [
    Work {
        title: "Tinkery",
        document: include_str!("../fixtures/cabinet.md"),
        question: "Does this feel like a place you want to work?",
        next: "Read the brief. Try the navigation. Nothing here records acceptance.",
    },
    Work {
        title: "Seed handoff",
        document: include_str!("../fixtures/seed.md"),
        question: "Separate artifacts or one combined document?",
        next: "Compare the two options in the seed. No decision has been recorded.",
    },
    Work {
        title: "Route receipts",
        document: include_str!("../fixtures/route.md"),
        question: "What context makes a routing choice inspectable?",
        next: "Read the receipt proposal. No routing service is connected.",
    },
];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Pane {
    #[default]
    Work,
    Artifact,
    Attention,
}

#[derive(Default)]
pub struct App {
    pub pane: Pane,
    pub selected: usize,
    pub evidence: bool,
    pub help: bool,
    pub quit: bool,
    return_pane: Pane,
    scroll: u16,
    attention_scroll: u16,
    max_scroll: u16,
    max_attention_scroll: u16,
}

impl App {
    pub fn handle_key(&mut self, key: KeyEvent, area: Rect) {
        if key.kind == KeyEventKind::Release {
            return;
        }
        if matches!(key.code, KeyCode::Char('q' | 'Q'))
            || (key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL))
        {
            self.quit = true;
            return;
        }
        if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
            return;
        }
        if self.help {
            if matches!(key.code, KeyCode::Esc | KeyCode::Char('?') | KeyCode::Enter) {
                self.help = false;
            }
            return;
        }
        if key.code == KeyCode::Char('?') {
            self.help = true;
            return;
        }
        if self.pane == Pane::Attention {
            match key.code {
                KeyCode::Char('a' | '3') | KeyCode::Esc | KeyCode::Tab | KeyCode::BackTab => {
                    self.pane = self.return_pane;
                }
                KeyCode::Down | KeyCode::Char('j') => self.move_vertical(true, 1),
                KeyCode::Up | KeyCode::Char('k') => self.move_vertical(false, 1),
                KeyCode::PageDown => self.move_vertical(true, 8),
                KeyCode::PageUp => self.move_vertical(false, 8),
                KeyCode::Home => self.move_vertical(false, u16::MAX),
                KeyCode::End => self.move_vertical(true, u16::MAX),
                _ => {}
            }
            return;
        }
        match key.code {
            KeyCode::Char('a' | '3') => {
                self.return_pane = self.pane;
                self.pane = Pane::Attention;
                self.attention_scroll = 0;
            }
            KeyCode::Tab | KeyCode::BackTab => {
                self.pane = if self.pane == Pane::Work {
                    Pane::Artifact
                } else {
                    Pane::Work
                };
            }
            KeyCode::Char('1') | KeyCode::Esc => self.pane = Pane::Work,
            KeyCode::Char('2') | KeyCode::Enter => self.pane = Pane::Artifact,
            KeyCode::Left | KeyCode::Right | KeyCode::Char('h' | 'l')
                if self.pane == Pane::Artifact =>
            {
                self.evidence = !self.evidence;
                self.scroll = 0;
            }
            KeyCode::Down | KeyCode::Char('j') => self.move_vertical(true, 1),
            KeyCode::Up | KeyCode::Char('k') => self.move_vertical(false, 1),
            KeyCode::PageDown => self.move_vertical(true, 8),
            KeyCode::PageUp => self.move_vertical(false, 8),
            KeyCode::Home => self.move_vertical(false, u16::MAX),
            KeyCode::End => self.move_vertical(true, u16::MAX),
            _ => {}
        }
    }

    fn move_vertical(&mut self, down: bool, amount: u16) {
        match self.pane {
            Pane::Work => {
                self.selected = if down {
                    self.selected
                        .saturating_add(usize::from(amount))
                        .min(WORK.len() - 1)
                } else {
                    self.selected.saturating_sub(usize::from(amount))
                };
                self.evidence = false;
                self.scroll = 0;
            }
            Pane::Artifact => self.scroll = advance(self.scroll, down, amount, self.max_scroll),
            Pane::Attention => {
                self.attention_scroll = advance(
                    self.attention_scroll,
                    down,
                    amount,
                    self.max_attention_scroll,
                )
            }
        }
    }
}

fn advance(current: u16, down: bool, amount: u16, max: u16) -> u16 {
    if down {
        current.saturating_add(amount).min(max)
    } else {
        current.saturating_sub(amount)
    }
}

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

pub fn render(frame: &mut Frame, app: &mut App, palette: Palette) {
    let area = frame.area();
    frame.render_widget(Block::default().style(palette.ink), area);
    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        frame.render_widget(Paragraph::new(format!(
            "tinkery / demo / read only\n\nMake a little room.\nResize to at least {MIN_WIDTH} x {MIN_HEIGHT}.\nCurrent: {} x {}\n\nq or Ctrl-C to quit", area.width, area.height,
        )).style(palette.ink).wrap(Wrap { trim: false }), area);
        return;
    }
    let page = Rect::new(
        area.x + area.width.saturating_sub(110) / 2,
        area.y + area.height.saturating_sub(36) / 2,
        area.width.min(110),
        area.height.min(36),
    )
    .inner(Margin {
        horizontal: 2,
        vertical: 1,
    });
    let rows = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(2),
        Constraint::Min(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .split(page);
    let header = Layout::horizontal([Constraint::Min(1), Constraint::Length(18)]).split(rows[0]);
    frame.render_widget(
        Paragraph::new("tinkery").style(palette.ink.bold()),
        header[0],
    );
    frame.render_widget(
        Paragraph::new("demo / read only")
            .style(palette.muted)
            .alignment(Alignment::Right),
        header[1],
    );

    let columns = if app.pane == Pane::Attention && !app.help {
        Layout::horizontal([
            Constraint::Length(20),
            Constraint::Min(18),
            Constraint::Length(28),
        ])
        .spacing(3)
        .split(rows[2])
    } else {
        Layout::horizontal([Constraint::Length(20), Constraint::Min(1)])
            .spacing(4)
            .split(rows[2])
    };
    let mut navigation = vec![
        Line::styled(
            "Work",
            if app.pane == Pane::Work {
                palette.jade
            } else {
                palette.muted
            },
        ),
        Line::default(),
    ];
    for (i, work) in WORK.iter().enumerate() {
        let selected = i == app.selected;
        navigation.push(Line::styled(
            format!("{}{}", if selected { "> " } else { "  " }, work.title),
            if selected && app.pane == Pane::Work {
                palette.jade
            } else if selected {
                palette.ink.bold()
            } else {
                palette.muted
            },
        ));
        navigation.push(Line::default());
    }
    frame.render_widget(Paragraph::new(navigation).style(palette.ink), columns[0]);

    let artifact_rows = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .split(columns[1]);
    let document = if app.evidence {
        include_str!("../fixtures/evidence.md")
    } else {
        WORK[app.selected].document
    };
    if app.help {
        let source = if app.evidence {
            "evidence.md"
        } else {
            ["cabinet.md", "seed.md", "route.md"][app.selected]
        };
        let help = format!(
            "Keys\n\nTab / Shift-Tab   Switch work and page\nUp / Down, j / k  Browse or scroll\nEnter            Read selected work\nLeft / Right     Switch brief and evidence\na / 3            Open attention\nEsc              Close drawer, or return to work\nPgUp / PgDn      Page scroll\nHome / End       First / last\n1 / 2            Focus work / page\nq / Ctrl-C       Quit\n\n? / Esc / Enter  Close help\n\nSource: embedded / {source}\nAll content is embedded. No files are written."
        );
        frame.render_widget(
            Paragraph::new(help)
                .style(palette.ink)
                .wrap(Wrap { trim: false }),
            columns[1],
        );
    } else {
        render_scrolled(
            frame,
            Paragraph::new(markdown(document, palette)).wrap(Wrap { trim: false }),
            artifact_rows[0],
            &mut app.scroll,
            &mut app.max_scroll,
        );
        let active_style = if app.pane == Pane::Artifact {
            palette.jade
        } else {
            palette.ink.bold()
        };
        let tabs = Line::from(vec![
            Span::styled(
                if app.evidence { "Brief" } else { "[Brief]" },
                if app.evidence {
                    palette.muted
                } else {
                    active_style
                },
            ),
            Span::raw("   "),
            Span::styled(
                if app.evidence {
                    "[Evidence]"
                } else {
                    "Evidence"
                },
                if app.evidence {
                    active_style
                } else {
                    palette.muted
                },
            ),
        ]);
        frame.render_widget(Paragraph::new(tabs).style(palette.ink), artifact_rows[2]);
        if app.pane == Pane::Attention {
            let attention = Paragraph::new(vec![
                Line::styled("Attention", palette.jade),
                Line::default(),
                Line::styled("Example question", palette.muted),
                Line::default(),
                Line::styled(WORK[app.selected].question, palette.ink),
                Line::default(),
                Line::styled("Next", palette.ink.bold()),
                Line::default(),
                Line::styled(WORK[app.selected].next, palette.ink),
                Line::default(),
                Line::styled("Reading is not approval.", palette.muted),
            ])
            .style(palette.ink)
            .wrap(Wrap { trim: false });
            render_scrolled(
                frame,
                attention,
                columns[2],
                &mut app.attention_scroll,
                &mut app.max_attention_scroll,
            );
        }
    }

    let footer = Layout::horizontal([Constraint::Length(28), Constraint::Min(1)]).split(rows[4]);
    frame.render_widget(
        Paragraph::new(if app.pane == Pane::Attention {
            "a / Esc close attention"
        } else {
            "1 question waiting [a]"
        })
        .style(palette.muted),
        footer[0],
    );
    let keys = if app.help {
        "? close help   q quit"
    } else {
        match app.pane {
            Pane::Work => "j/k browse   Tab page   ? help   q quit",
            Pane::Artifact => "j/k scroll   Tab work   ? help   q quit",
            Pane::Attention => "j/k scroll   ? help   q quit",
        }
    };
    frame.render_widget(
        Paragraph::new(keys)
            .style(palette.muted)
            .alignment(Alignment::Right),
        footer[1],
    );
}

pub fn snapshot(
    width: u16,
    height: u16,
    app: &mut App,
    no_color: bool,
) -> Result<String, std::convert::Infallible> {
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(width, height))?;
    terminal.draw(|frame| render(frame, app, Palette::new(no_color)))?;
    let buffer = terminal.backend().buffer();
    Ok((0..height)
        .map(|y| {
            (0..width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n"))
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
