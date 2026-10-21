//! #176: real discovery must retain incomplete-read evidence through the menu into Shape.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::{
    collections::BTreeMap,
    fs,
    path::Path,
    thread,
    time::{Duration, Instant},
};
use tinkery::{
    menu::{Home, snapshot},
    think::brain_dump::BrainDump,
};

const EMPTY: &str = "No goals are waiting for a seed.";
const OFFERED: &str = "TEST eligible goal";
const HIDDEN: &str = "TEST ended goal";

fn files(root: &Path) -> BTreeMap<std::path::PathBuf, Vec<u8>> {
    let mut result = BTreeMap::new();
    for entry in fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            result.extend(files(&path));
        } else {
            result.insert(path.clone(), fs::read(path).unwrap());
        }
    }
    result
}

fn fixture(unreadable: bool, populated: bool) -> tempfile::TempDir {
    let parent = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/tmp");
    fs::create_dir_all(&parent).unwrap();
    let temp = tempfile::Builder::new()
        .prefix("tinkery-TEST-176-rust-")
        .tempdir_in(parent)
        .unwrap();
    if unreadable {
        let broken = temp.path().join("TEST-unreadable");
        fs::create_dir(&broken).unwrap();
        fs::write(
            broken.join("ledger.json"),
            "not JSON: TEST unreadable ledger",
        )
        .unwrap();
    }
    if populated {
        for (name, goal, status) in [
            ("TEST-eligible", OFFERED, "active"),
            ("TEST-ended", HIDDEN, "completed"),
        ] {
            let session = temp.path().join(name);
            fs::create_dir(&session).unwrap();
            fs::write(
                session.join("ledger.json"),
                serde_json::json!({
                    "session_id": name, "status": status, "goal": goal,
                    "updated_at": "2026-10-17T00:00:00Z"
                })
                .to_string(),
            )
            .unwrap();
        }
    }
    let found = tinkery::tend::scan(temp.path()).unwrap();
    assert_eq!(found.unreadable, usize::from(unreadable));
    assert_eq!(found.sessions.len(), if populated { 2 } else { 0 });
    temp
}

fn shape_screen(root: &Path, width: u16, height: u16, no_color: bool) -> String {
    let mut home = Home::new(
        Box::new(BrainDump::default),
        Ok(root.to_path_buf()),
        Err("TEST links disabled".into()),
    )
    .with_record_root(root.join("TEST-no-records"));
    home.handle_key(KeyEvent::new(KeyCode::Char('3'), KeyModifiers::NONE));
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        home.tick();
        let text = snapshot(width, height, &mut home, no_color).unwrap();
        if text.contains("tinkery › shape") && !text.contains("Reading your sessions") {
            return text;
        }
        assert!(
            Instant::now() < deadline,
            "TEST discovery did not settle:\n{text}"
        );
        thread::sleep(Duration::from_millis(1));
    }
}

fn incomplete_case(populated: bool) {
    for (width, height) in [(100, 30), (80, 24)] {
        for no_color in [false, true] {
            let temp = fixture(true, populated);
            let before = files(temp.path());
            let text = shape_screen(temp.path(), width, height, no_color);
            assert_eq!(
                files(temp.path()),
                before,
                "Shape changed TEST session files"
            );
            assert_eq!(
                text.contains(OFFERED),
                populated,
                "eligible goals changed:\n{text}"
            );
            assert!(
                !text.contains(HIDDEN),
                "an ended session was offered:\n{text}"
            );
            let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
            assert!(
                (flat.contains("1 record") || flat.contains("1 session"))
                    && flat.contains("couldn't be read"),
                "#176 missing incomplete-read notice/count (populated={populated}):\n{text}"
            );
            assert!(
                !flat.contains(EMPTY),
                "#176 false complete-empty claim:\n{text}"
            );
            assert!(
                flat.contains("F1") || flat.to_lowercase().contains("tend"),
                "no route to why:\n{text}"
            );
        }
    }
}

#[test]
fn unreadable_empty_list_reports_the_count() {
    incomplete_case(false);
}

#[test]
fn unreadable_populated_list_reports_the_count_without_changing_offers() {
    incomplete_case(true);
}

#[test]
fn complete_empty_root_keeps_the_existing_message() {
    for (width, height) in [(100, 30), (80, 24)] {
        for no_color in [false, true] {
            let temp = fixture(false, false);
            let text = shape_screen(temp.path(), width, height, no_color);
            assert!(
                text.contains(EMPTY),
                "complete-empty message missing:\n{text}"
            );
            assert!(
                files(temp.path()).is_empty(),
                "Shape wrote into an empty TEST root"
            );
        }
    }
}
