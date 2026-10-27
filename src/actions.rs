//! The one key map Think and Shape share. F2 runs the main action the key bar names, F1 lists
//! every other action allowed right now with a number, and Esc goes back. A digit runs exactly
//! its row, once. Callers build the rows from their own state, so a row appears only when its
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

/// The action a plain first press of a digit names in `rows`. Held, repeated or modified keys,
/// zero and digits past the list pick nothing.
pub fn pick<A: Clone>(rows: &[Row<A>], key: KeyEvent) -> Option<A> {
    if key.kind != KeyEventKind::Press || !key.modifiers.difference(KeyModifiers::SHIFT).is_empty()
    {
        return None;
    }
    let KeyCode::Char(c) = key.code else {
        return None;
    };
    let n = c.to_digit(10)? as usize;
    rows.get(n.checked_sub(1)?).map(|row| row.act.clone())
}

/// The rows as numbered lines (`1  Review the goal`), at most nine, so every row has a digit.
pub fn text<A>(rows: &[Row<A>]) -> String {
    rows.iter()
        .take(9)
        .enumerate()
        .map(|(i, row)| format!("{}  {}", i + 1, row.label))
        .collect::<Vec<_>>()
        .join("\n")
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
        assert_eq!(pick(&rows(), press('1')), Some("review"));
        assert_eq!(pick(&rows(), press('2')), Some("skip"));
        for c in ['0', '3', '9', 'a'] {
            assert_eq!(pick(&rows(), press(c)), None, "{c}");
        }
        let mut held = press('1');
        held.kind = KeyEventKind::Repeat;
        assert_eq!(pick(&rows(), held), None);
        let ctrl = KeyEvent::new(KeyCode::Char('1'), KeyModifiers::CONTROL);
        assert_eq!(pick(&rows(), ctrl), None);
    }

    #[test]
    fn rows_are_numbered_from_one() {
        assert_eq!(text(&rows()), "1  Review the goal\n2  Skip");
    }
}
