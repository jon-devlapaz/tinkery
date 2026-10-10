use super::*;
use crate::voice::{Failure, Voice, VoiceHost, silence_filler};
use ratatui::text::{Line, Span, Text};
use std::time::Instant;

impl BrainDump {
    pub fn with_voice(mut self, host: Box<dyn VoiceHost>) -> Self {
        self.voice = Some(Voice::new(host));
        self
    }
    pub(super) fn voice_active(&self) -> bool {
        self.voice.as_ref().is_some_and(Voice::active)
    }
    pub(super) fn voice_allowed(&self) -> bool {
        !self.quit
            && !self.leave_prompt
            && self.goal_review.is_none()
            && self.receipt.is_none()
            && self.handoff_job.is_none()
    }
    pub(super) fn voice_available(&self) -> bool {
        self.voice_allowed() && self.voice.is_some()
    }
    pub(super) fn voice_key(&mut self, key: KeyEvent) -> bool {
        if self.voice_allowed()
            && key.kind == KeyEventKind::Press
            && key.modifiers.is_empty()
            && self.voice.as_ref().is_some_and(Voice::setup_prompt)
        {
            match key.code {
                KeyCode::Char('y' | 'Y' | 'n' | 'N') | KeyCode::Esc => {
                    self.voice
                        .as_mut()
                        .unwrap()
                        .answer_setup(matches!(key.code, KeyCode::Char('y' | 'Y')), Instant::now());
                    return true;
                }
                _ => {}
            }
        }
        if key.code == KeyCode::Esc && self.voice_active() && !self.leave_prompt {
            self.voice.as_mut().unwrap().cancel();
            return true;
        }
        if key.code == KeyCode::F(2) && key.modifiers.is_empty() && self.voice_active() {
            self.voice.as_mut().unwrap().stop(true, Instant::now());
            return true;
        }
        let speak = key.code == KeyCode::F(6)
            || key.code == KeyCode::Char('Ω')
            || (matches!(key.code, KeyCode::Char('z' | 'Z'))
                && key
                    .modifiers
                    .contains(KeyModifiers::CONTROL | KeyModifiers::ALT));
        if !speak {
            return false;
        }
        if key.kind != KeyEventKind::Press || !self.voice_allowed() {
            return true;
        }
        if let Some(voice) = &mut self.voice {
            voice.toggle(Instant::now());
            self.board_focus = false;
            self.help = false;
            self.original = false;
            self.details = false;
        }
        true
    }
    pub(super) fn voice_tick(&mut self) {
        let Some(result) = self.voice.as_mut().and_then(|v| v.tick(Instant::now())) else {
            return;
        };
        if !self.voice_allowed() {
            return;
        }
        let text = Note::sanitize(&result.text);
        if silence_filler(&text) {
            self.voice.as_mut().unwrap().missed();
            return;
        }
        if let Err(error) = self.input.insert(&text) {
            self.voice
                .as_mut()
                .unwrap()
                .reject(Failure::new("capture", error));
            return;
        }
        self.board_focus = false;
        self.changed_lines = None;
        if result.send {
            self.submit();
        }
    }
}

pub(super) fn input_text(
    app: &BrainDump,
    palette: Palette,
) -> (super::super::note::Wrapped, Text<'static>) {
    let partial = Note::sanitize(app.voice.as_ref().map_or("", Voice::partial));
    if partial.is_empty() {
        let wrapped = app.input.wrap(app.input_area.width);
        let text = Text::styled(wrapped.lines.join("\n"), palette.ink);
        return (wrapped, text);
    }
    let start = app.input.cursor;
    let end = start + partial.len();
    let mut preview = Note::new(&format!(
        "{}{}{}",
        &app.input.text[..start],
        partial,
        &app.input.text[start..]
    ));
    preview.cursor = preview
        .text
        .grapheme_indices(true)
        .map(|(i, _)| i)
        .take_while(|i| *i <= start)
        .last()
        .unwrap_or(0);
    let wrapped = preview.wrap(app.input_area.width);
    let mut scan = 0;
    let lines = wrapped
        .lines
        .iter()
        .map(|line| {
            scan += preview.text[scan..].find(line).unwrap();
            let line_start = scan;
            scan += line.len();
            Line::from(
                line.grapheme_indices(true)
                    .map(|(i, g)| {
                        let at = line_start + i;
                        Span::styled(
                            g.to_owned(),
                            if at < end && at + g.len() > start {
                                palette.muted
                            } else {
                                palette.ink
                            },
                        )
                    })
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();
    (wrapped, Text::from(lines))
}

pub(super) fn render_notice(frame: &mut Frame, app: &BrainDump, palette: Palette) {
    if !app.voice_allowed() || app.area.width < MIN_WIDTH || app.area.height < MIN_HEIGHT {
        return;
    }
    if let Some(voice) = &app.voice {
        frame.render_widget(
            Paragraph::new(voice.notice()).style(palette.muted),
            Rect::new(2, app.area.height - 2, app.area.width - 4, 1),
        );
    }
}
