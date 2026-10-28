//! The one key map Think and Shape share. F2 runs the main action the key bar names, F1 lists
//! every other action allowed right now with a number, and Esc goes back. A digit runs exactly
//! its row on the displayed page, once. Left/Right reaches pages beyond nine actions.
//! Callers build the rows from their own state, so a row appears only when its
//! action is allowed; this module runs nothing itself and knows no phase.
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

/// The longest label that stays on one line in an 80-column help panel.
const LABEL_MAX: usize = 31;

/// One action allowed now: the plain words F1 shows, and what the caller runs when it's picked.
pub struct Row<A> {
    /// What the operator gets, in plain words.
    pub label: String,
    /// What the caller runs.
    pub act: A,
}

impl<A> Row<A> {
    /// A row showing `label` that runs `act`. Labels fit the narrowest help panel on one line
    /// (`LABEL_MAX` characters), so the digit and its words never split.
    pub fn new(label: impl Into<String>, act: A) -> Self {
        let label = label.into();
        debug_assert!(label.chars().count() <= LABEL_MAX, "{label:?} wraps in F1");
        Self { label, act }
    }
}

/// Nine numbered actions at a time. Paging never chooses an action; a fresh opening starts
/// on page one. A changed list is clamped to its last page before rendering or selection.
#[derive(Default)]
pub struct Page(usize);

impl Page {
    fn index(&self, len: usize) -> usize {
        self.0.min(len.saturating_sub(1) / 9)
    }

    /// Return to the first page without choosing anything.
    pub fn reset(&mut self) {
        self.0 = 0;
    }

    /// A plain first Left/Right press changes pages only. Modified and held keys do nothing.
    /// Returns whether this was page navigation; the caller can reset its separate scroll.
    pub fn turn(&mut self, len: usize, key: KeyEvent) -> bool {
        if len <= 9 || key.kind != KeyEventKind::Press || !key.modifiers.is_empty() {
            return false;
        }
        self.0 = match key.code {
            KeyCode::Left => self.index(len).saturating_sub(1),
            KeyCode::Right => (self.index(len) + 1).min((len - 1) / 9),
            _ => return false,
        };
        true
    }

    /// The action a first digit press names on this page. Held or modified keys, zero and
    /// digits past the displayed rows pick nothing; Shift retains the terminal digit behavior.
    pub fn pick<A: Clone>(&self, rows: &[Row<A>], key: KeyEvent) -> Option<A> {
        if key.kind != KeyEventKind::Press
            || !key.modifiers.difference(KeyModifiers::SHIFT).is_empty()
        {
            return None;
        }
        let KeyCode::Char(c) = key.code else {
            return None;
        };
        let n = c.to_digit(10)? as usize;
        rows.get(self.index(rows.len()) * 9 + n.checked_sub(1)?)
            .map(|row| row.act.clone())
    }

    /// This page as numbered lines (`1  Review the goal`), at most nine.
    pub fn text<A>(&self, rows: &[Row<A>]) -> String {
        rows.iter()
            .skip(self.index(rows.len()) * 9)
            .take(9)
            .enumerate()
            .map(|(i, row)| format!("{}  {}", i + 1, row.label))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// A page indicator only when there are more actions than one page holds.
    pub fn hint(&self, len: usize) -> Option<String> {
        (len > 9).then(|| format!("Actions {}/{}", self.index(len) + 1, len.div_ceil(9)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows() -> Vec<Row<&'static str>> {
        vec![
            Row::new("Review the goal", "review"),
            Row::new("Skip", "skip"),
        ]
    }

    #[test]
    fn a_digit_runs_only_its_own_row_once() {
        let press = |c| KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE);
        let page = Page::default();
        assert_eq!(page.pick(&rows(), press('1')), Some("review"));
        assert_eq!(page.pick(&rows(), press('2')), Some("skip"));
        for c in ['0', '3', '9', 'a'] {
            assert_eq!(page.pick(&rows(), press(c)), None, "{c}");
        }
        let mut held = press('1');
        held.kind = KeyEventKind::Repeat;
        assert_eq!(page.pick(&rows(), held), None);
        let ctrl = KeyEvent::new(KeyCode::Char('1'), KeyModifiers::CONTROL);
        assert_eq!(page.pick(&rows(), ctrl), None);
    }

    #[test]
    fn rows_are_numbered_from_one() {
        assert_eq!(Page::default().text(&rows()), "1  Review the goal\n2  Skip");
    }

    #[test]
    fn every_row_is_reachable_with_its_displayed_digit_and_pages_stop_at_the_ends() {
        let rows: Vec<_> = (0..19)
            .map(|n| Row::new(format!("Action {n}"), n))
            .collect();
        let mut page = Page::default();
        let mut selected = vec![];
        for _ in 0..3 {
            for line in page.text(&rows).lines() {
                let digit = line.chars().next().unwrap();
                selected.push(
                    page.pick(
                        &rows,
                        KeyEvent::new(KeyCode::Char(digit), KeyModifiers::NONE),
                    )
                    .unwrap(),
                );
            }
            assert!(page.turn(
                rows.len(),
                KeyEvent::new(KeyCode::Right, KeyModifiers::NONE)
            ));
        }
        assert_eq!(selected, (0..19).collect::<Vec<_>>());
        assert_eq!(page.hint(rows.len()).as_deref(), Some("Actions 3/3"));
        assert_eq!(page.text(&rows), "1  Action 18");
        assert_eq!(
            page.pick(&rows, KeyEvent::new(KeyCode::Char('2'), KeyModifiers::NONE)),
            None
        );
        // A changing list cannot leave the shown page or its digit out of range.
        assert_eq!(page.text(&rows[..2]), "1  Action 0\n2  Action 1");
        for _ in 0..3 {
            page.turn(rows.len(), KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));
        }
        assert_eq!(page.hint(rows.len()).as_deref(), Some("Actions 1/3"));
        page.turn(
            rows.len(),
            KeyEvent::new(KeyCode::Right, KeyModifiers::NONE),
        );
        page.reset();
        assert_eq!(page.hint(rows.len()).as_deref(), Some("Actions 1/3"));
        assert!(page.hint(9).is_none() && page.hint(0).is_none());
    }
}
