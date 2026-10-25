use super::*;
use ratatui::{
    backend::TestBackend,
    buffer::Buffer,
    layout::Alignment,
    style::{Color, Modifier, Style},
    widgets::Wrap,
};
use unicode_segmentation::UnicodeSegmentation;

fn rendered(text: Text<'static>, width: u16, offset: usize, replacement: bool) -> Buffer {
    let mut terminal = ratatui::Terminal::new(TestBackend::new(width, 8)).unwrap();
    terminal
        .draw(|frame| {
            if replacement {
                Reading::new(text.clone(), width).render(frame, frame.area(), offset);
            } else {
                frame.render_widget(
                    Paragraph::new(text.clone())
                        .wrap(Wrap { trim: false })
                        .scroll((offset as u16, 0)),
                    frame.area(),
                );
            }
        })
        .unwrap();
    terminal.backend().buffer().clone()
}

#[test]
fn styled_reading_matches_existing_word_wrap() {
    let red = Style::default().fg(Color::Red).add_modifier(Modifier::BOLD);
    let blue = Style::default()
        .fg(Color::Blue)
        .add_modifier(Modifier::ITALIC);
    let inputs = [
        "",
        "alpha beta gamma delta",
        "  leading and trailing    ",
        "          ",
        "one\t two\u{00a0}three\u{2003}four",
        "a\n\nlast\n",
        "é中👩‍💻 e\u{301} longwordwithoutbreaks 👩‍💻",
        "a  bb   cccc     dddddddd",
        "a\0b\u{001b}[31m\r c\t d",
        "لا Arabic and e\u{301} text",
    ];
    for input in inputs {
        for width in [8, 20, 76] {
            for alignment in [Alignment::Left, Alignment::Center, Alignment::Right] {
                let mut lines: Vec<Line<'static>> = input
                    .split('\n')
                    .map(|line| {
                        let spans = line
                            .graphemes(true)
                            .enumerate()
                            .map(|(i, c)| {
                                Span::styled(c.to_owned(), if i % 2 == 0 { red } else { blue })
                            })
                            .collect::<Vec<_>>();
                        Line::from(spans).alignment(alignment)
                    })
                    .collect();
                // Keep a complete multi-scalar grapheme beside style boundaries too.
                lines.push(Line::from(vec![
                    Span::styled("e\u{301}👩‍💻中 ", red),
                    Span::styled("tail", blue),
                ]));
                let text = Text::from(lines);
                let rows = Reading::new(text.clone(), width).rows();
                assert_eq!(
                    rows,
                    Paragraph::new(text.clone())
                        .wrap(Wrap { trim: false })
                        .line_count(width),
                    "{input:?}, width {width}"
                );
                for offset in 0..=rows {
                    assert_eq!(
                        rendered(text.clone(), width, offset, true),
                        rendered(text.clone(), width, offset, false),
                        "{input:?}, width {width}, offset {offset}"
                    );
                }
            }
        }
    }
}

#[test]
fn already_fitting_ascii_rows_preserve_styles_alignment_and_boundary_spaces() {
    let style = Style::default().fg(Color::Green);
    for source in ["", "   ", "a  ", "  a", "alpha beta", "a![]`~"] {
        let line = Line::from(vec![
            Span::styled(source.to_owned(), style),
            Span::raw(" tail "),
        ])
        .style(Style::default().add_modifier(Modifier::BOLD));
        let exact = line.width() as u16;
        for width in [exact, exact + 1, 76] {
            for alignment in [Alignment::Left, Alignment::Center, Alignment::Right] {
                let text = Text::from(line.clone())
                    .style(Style::default().bg(Color::Blue))
                    .alignment(alignment);
                assert_eq!(
                    Reading::new(text.clone(), width).rows(),
                    Paragraph::new(text.clone())
                        .wrap(Wrap { trim: false })
                        .line_count(width)
                );
                assert_eq!(
                    rendered(text.clone(), width, 0, true),
                    rendered(text, width, 0, false),
                    "{source:?}, width {width}"
                );
            }
        }
    }
}

#[test]
fn one_physical_line_remains_reachable_beyond_terminal_row_counts() {
    let text = Text::from(format!("{}TEST tail 中", "word ".repeat(65_536)));
    let page = Reading::new(text, 5);
    assert!(page.rows() > usize::from(u16::MAX));
    let mut terminal = ratatui::Terminal::new(TestBackend::new(5, 3)).unwrap();
    terminal
        .draw(|frame| page.render(frame, frame.area(), page.rows() - 3))
        .unwrap();
    let shown = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(
        shown.contains("TEST") && shown.contains("tail") && shown.contains('中'),
        "{shown}"
    );
}

#[test]
fn inherited_styles_and_span_boundaries_match_the_existing_renderer() {
    // Whole-span width collapses Arabic pairs; wrapping must still count each grapheme.
    let arabic = Text::from("لالالالا");
    assert_eq!(Reading::new(arabic.clone(), 5).rows(), 2);
    let shown = rendered(arabic.clone(), 5, 0, true);
    assert_eq!(shown, rendered(arabic, 5, 0, false));
    assert_eq!(
        shown
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>()
            .replace(' ', ""),
        "لالالالا"
    );
    let style = Style::default().fg(Color::Green);
    let text = Text::from(vec![
        Line::from(vec![
            Span::styled("e", style),
            Span::styled("\u{301} test", style),
        ]),
        Line::from(vec![
            Span::styled("👩", style),
            Span::styled("\u{200d}💻 tail", style),
        ]),
        Line::from(vec![
            Span::raw("inherited "),
            Span::styled("red", Style::default().fg(Color::Red)),
        ])
        .style(Style::default().add_modifier(Modifier::BOLD)),
    ])
    .style(Style::default().bg(Color::Blue))
    .alignment(Alignment::Right);
    for width in [20, 76] {
        assert_eq!(
            rendered(text.clone(), width, 0, true),
            rendered(text.clone(), width, 0, false)
        );
    }
}

#[test]
fn narrow_mixed_width_reading_keeps_each_fitting_grapheme() {
    let widest = Text::from(format!("{}中", "x".repeat(65_534)));
    assert_eq!(Reading::new(widest, u16::MAX).rows(), 2);
    let text = Text::from("e\u{301}👩‍💻中");
    let buffer = rendered(text, 2, 0, true);
    let shown = buffer
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(
        shown.contains("e\u{301}") && shown.contains("👩‍💻") && shown.contains('中'),
        "{shown}"
    );
    for width in [1, 2, 3] {
        let ascii = Text::from("  a bb cccc    d");
        assert_eq!(
            rendered(ascii.clone(), width, 0, true),
            rendered(ascii, width, 0, false)
        );
    }
}
