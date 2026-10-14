use super::note::{Motion, Note};
use unicode_segmentation::UnicodeSegmentation;

fn boundary(text: &str, index: usize) -> bool {
    index == text.len() || text.grapheme_indices(true).any(|(i, _)| i == index)
}
fn sel(note: &Note) -> Option<&str> {
    note.selected_text()
}
fn at(text: &str, cursor: usize) -> Note {
    let mut note = Note::new(text);
    note.cursor = cursor;
    note
}
const TRICKY: &str = "ae\u{301}👩‍💻中b";

#[test]
fn shift_left_and_right_select_forward_reverse_and_collapse() {
    let mut note = at("hello", 2);
    note.navigate(Motion::Right, true, 20);
    note.navigate(Motion::Right, true, 20);
    assert_eq!((note.selection(), sel(&note)), (Some((2, 4)), Some("ll")));
    note.navigate(Motion::Left, true, 20);
    assert_eq!(sel(&note), Some("l"));
    note.navigate(Motion::Left, true, 20);
    assert_eq!(note.selection(), None, "collapsed selection is none");
    for _ in 0..3 {
        note.navigate(Motion::Left, true, 20);
    }
    assert_eq!(
        (note.selection(), sel(&note)),
        (Some((0, 2)), Some("he")),
        "reverse selection is ordered; edge of buffer stops"
    );
    note.navigate(Motion::Right, false, 20);
    assert_eq!(
        (note.cursor, note.selection()),
        (2, None),
        "Right collapses to end"
    );
    note.navigate(Motion::Left, true, 20);
    note.navigate(Motion::Left, false, 20);
    assert_eq!(
        (note.cursor, note.selection()),
        (1, None),
        "Left collapses to start"
    );
}

#[test]
fn endpoints_never_cut_a_grapheme() {
    let mut note = at(TRICKY, 0);
    let mut seen = vec![];
    while note.cursor < TRICKY.len() {
        note.navigate(Motion::Right, true, 20);
        let (a, b) = note.selection().unwrap();
        assert!(boundary(TRICKY, a) && boundary(TRICKY, b), "{a}..{b}");
        seen.push(note.selected_text().unwrap().to_owned());
    }
    assert_eq!(
        seen.last().unwrap(),
        TRICKY,
        "ZWJ emoji, combining mark and wide char are each one step"
    );
    assert_eq!(seen.len(), 5);
    for _ in 0..5 {
        note.navigate(Motion::Left, true, 20);
        let at_cursor = note.cursor;
        assert!(boundary(TRICKY, at_cursor));
    }
    assert_eq!(note.selection(), None);
}

#[test]
fn home_and_end_use_the_logical_line_not_the_wrapped_row() {
    let text = "alpha beta gamma\nsecond";
    let mut note = at(text, 8);
    note.navigate(Motion::Home, true, 6);
    assert_eq!(sel(&note), Some("alpha be"));
    let mut note = at(text, 8);
    note.navigate(Motion::End, true, 6);
    assert_eq!(sel(&note), Some("ta gamma"));
    let mut note = at(text, 20);
    note.navigate(Motion::Home, true, 6);
    assert_eq!(sel(&note), Some("sec"));
}

#[test]
fn shift_up_and_down_follow_wrapped_rows_and_reach_the_buffer_edges() {
    let text = "one two three four five";
    let mut note = at(text, 0);
    note.navigate(Motion::Down, true, 9);
    let (a, b) = note.selection().unwrap();
    assert_eq!(a, 0);
    assert!(b > 0 && b < text.len() && boundary(text, b));
    note.navigate(Motion::Up, true, 9);
    assert_eq!(
        note.selection(),
        None,
        "Up from the far end returns to the anchor row"
    );
    let mut note = at(text, 4);
    note.navigate(Motion::Up, true, 9);
    assert_eq!(
        sel(&note),
        Some("one "),
        "Shift-Up on the first row selects to the start"
    );
    let mut note = at(text, 18);
    note.navigate(Motion::Down, true, 9);
    assert_eq!(
        note.cursor,
        text.len(),
        "Shift-Down on the last row selects to the end"
    );
    let mut note = at(TRICKY, 1);
    note.navigate(Motion::Down, false, 4);
    assert!(boundary(TRICKY, note.cursor));
}

#[test]
fn replacement_deletion_and_paste_preserve_surrounding_bytes() {
    let mut note = at("keep MIDDLE keep", 5);
    for _ in 0..6 {
        note.navigate(Motion::Right, true, 40);
    }
    note.insert("x\r\ny\u{1b}[31m").unwrap();
    assert_eq!(
        note.text, "keep x\r\ny\u{1b}[31m keep",
        "received line endings and controls are retained"
    );
    assert_eq!((note.cursor, note.selection()), (5 + 9, None));
    let mut note = at("keep MIDDLE keep", 5);
    for _ in 0..6 {
        note.navigate(Motion::Right, true, 40);
    }
    note.backspace();
    assert_eq!((note.text.as_str(), note.cursor), ("keep  keep", 5));
    let mut reverse = at("keep MIDDLE keep", 11);
    for _ in 0..6 {
        reverse.navigate(Motion::Left, true, 40);
    }
    reverse.delete();
    assert_eq!((reverse.text.as_str(), reverse.cursor), ("keep  keep", 5));
    let mut note = at("a👩‍💻b", 1);
    note.navigate(Motion::Right, true, 40);
    note.insert("é").unwrap();
    assert_eq!(note.text, "aéb");
}

#[test]
fn deleting_between_a_base_and_its_mark_realigns_the_caret() {
    let mut note = Note::new("ex\u{301}");
    note.cursor = 1;
    note.navigate(Motion::Right, true, 40);
    assert_eq!(sel(&note), Some("x\u{301}"));
    let mut note = Note::new("ee\u{301}");
    note.cursor = 1;
    note.navigate(Motion::Right, true, 40);
    note.delete();
    assert_eq!(note.text, "e");
    assert!(boundary(&note.text, note.cursor));
    let mut note = Note::new("e\u{301}");
    note.cursor = 0;
    note.insert("x").unwrap();
    assert!(
        boundary(&note.text, note.cursor),
        "inserting before a mark keeps the caret whole"
    );
}

#[test]
fn large_replacement_counts_removed_bytes_without_a_request_policy() {
    let base = "a".repeat(4090) + "BCDEFG";
    let mut note = at(&base, 4090);
    for _ in 0..2 {
        note.navigate(Motion::Right, true, 40);
    }
    note.insert(&"z".repeat(9000)).unwrap();
    assert_eq!(note.text, "a".repeat(4090) + &"z".repeat(9000) + "DEFG");
    assert_eq!(note.cursor, 13090);
    assert_eq!(note.selection(), None);
}

#[test]
fn editing_crosses_the_historical_cap_including_selection_replacement() {
    let base = "a".repeat(4090) + "BCDEFG";
    assert_eq!(base.len(), 4096);
    let mut note = at(&base, 4090);
    for _ in 0..6 {
        note.navigate(Motion::Right, true, 40);
    }
    note.insert(&"z".repeat(7)).unwrap();
    assert_eq!(note.text.len(), 4097);
    let mut full = Note::new(&"a".repeat(4096));
    full.insert("b").unwrap();
    assert_eq!(full.text.len(), 4097);
}

#[test]
fn select_all_and_empty_cases() {
    let mut note = Note::new("");
    note.select_all();
    assert_eq!(note.selection(), None);
    let mut note = at("abc", 1);
    note.select_all();
    assert_eq!(sel(&note), Some("abc"));
    note.insert("").unwrap();
    assert_eq!(sel(&note), Some("abc"), "an empty paste changes nothing");
    note.insert("x").unwrap();
    assert_eq!(note.text, "x");
}

#[test]
fn a_stale_caret_inside_a_character_edits_at_the_boundary_instead_of_panicking() {
    let mut note = Note::new("aéc");
    note.cursor = 2; // inside the two-byte é
    note.insert("x").unwrap();
    assert_eq!(note.text, "axéc");
    assert!(boundary(&note.text, note.cursor));
    let mut note = Note::new("aéc");
    note.cursor = 2;
    note.backspace();
    assert_eq!(note.text, "éc");
    let mut note = Note::new("aéc");
    note.cursor = 2;
    note.delete();
    assert_eq!(note.text, "ac");
    let mut note = Note::new("aéc");
    note.cursor = 99;
    note.insert("x").unwrap();
    assert_eq!(note.text, "aécx");
}

#[test]
fn raw_controls_have_safe_display_and_exact_grapheme_coordinates() {
    let raw = "a\r\nb\rc\td\u{1b}[31m\0\u{7f}\u{85}e\u{301}中👩‍💻";
    let mut note = Note::new("");
    note.insert(raw).unwrap();
    assert_eq!(note.text.as_bytes(), raw.as_bytes());
    for width in [2, 32, 72, 96, 156, 316] {
        let wrapped = note.wrap(width);
        assert_eq!(wrapped.positions.len(), raw.graphemes(true).count() + 1);
        for ((offset, g), (_, row, col)) in raw.grapheme_indices(true).zip(&wrapped.positions) {
            assert_eq!(
                wrapped.index_at(*row, *col as u16),
                offset,
                "{g:?} at {row},{col}"
            );
        }
        assert!(
            wrapped
                .lines
                .iter()
                .all(|l| !l.chars().any(char::is_control))
        );
        assert!(wrapped.lines.join("\n").contains("␛"));
    }
    note.select_all();
    assert_eq!(note.selected_text(), Some(raw));
    note.cursor = 1;
    note.collapse();
    note.right();
    assert_eq!(note.cursor, 3, "CRLF is a whole grapheme");
    note.left();
    assert_eq!(note.cursor, 1);
    note.end();
    assert_eq!(note.cursor, 1, "End never stops inside CRLF");
}

#[test]
fn logical_and_wrapped_rows_reach_beyond_u16_without_aliasing() {
    for rows in [65534usize, 65535, 65536, 70000] {
        let text = "x\n".repeat(rows) + "tail";
        let began = std::time::Instant::now();
        let mut note = Note::new("");
        note.insert(&text).unwrap();
        let wrapped = note.wrap(72);
        assert_eq!(wrapped.cursor, (rows, 4));
        assert_eq!(wrapped.index_at(rows, 0), rows * 2);
        note.move_row(false, 72);
        assert_eq!(note.cursor, rows * 2 - 1);
        note.move_row(true, 72);
        assert_eq!(note.cursor, rows * 2 + 1);
        note.select_all();
        assert_eq!(note.selected_text(), Some(text.as_str()));
        eprintln!(
            "logical rows={rows}, bytes={}, elapsed={:?}",
            text.len(),
            began.elapsed()
        );
    }
    let text = "中".repeat(70000);
    let note = Note::new(&text);
    let wrapped = note.wrap(2);
    assert_eq!(wrapped.cursor, (70000, 0));
    assert_eq!(wrapped.index_at(65536, 0), 65536 * 3);
    assert_eq!(wrapped.index_at(69999, 0), 69999 * 3);
}
