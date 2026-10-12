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
        note.text, "keep x\ny[31m keep",
        "controls stripped, CRLF is one newline"
    );
    assert_eq!((note.cursor, note.selection()), (5 + 7, None));
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
fn oversize_replacement_is_rejected_atomically_and_counts_removed_bytes() {
    let base = "a".repeat(4090) + "BCDEFG";
    let mut note = at(&base, 4090);
    for _ in 0..2 {
        note.navigate(Motion::Right, true, 40);
    }
    let before = (note.text.clone(), note.cursor, note.selection());
    assert!(note.insert(&"z".repeat(9)).is_err(), "4096 - 2 + 9 > 4096");
    assert_eq!((note.text.clone(), note.cursor, note.selection()), before);
    assert!(
        note.insert("zz").is_ok(),
        "same size as the removed bytes fits"
    );
    assert_eq!(note.text.len(), 4096);
}

#[test]
fn replacement_at_exactly_the_cap_is_allowed() {
    let base = "a".repeat(4090) + "BCDEFG";
    assert_eq!(base.len(), 4096);
    let mut note = at(&base, 4090);
    for _ in 0..6 {
        note.navigate(Motion::Right, true, 40);
    }
    assert!(note.insert(&"z".repeat(7)).is_err());
    assert_eq!(sel(&note), Some("BCDEFG"));
    assert!(note.insert(&"z".repeat(6)).is_ok());
    assert_eq!(note.text.len(), 4096);
    let mut full = Note::new(&"a".repeat(4096));
    assert!(full.insert("b").is_err());
    assert_eq!(full.text.len(), 4096);
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
