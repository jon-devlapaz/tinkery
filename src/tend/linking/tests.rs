//! Link state and wording contracts, with isolated stores and synthetic session identities.

use super::*;
use crate::tend::{Found, Stage, needs};
use std::fs;

fn fixture() -> (tempfile::TempDir, Tend) {
    let tmp = tempfile::Builder::new()
        .prefix("tinkery-TEST-link-state-")
        .tempdir()
        .unwrap();
    let mut tend = Tend::new(Ok(tmp.path().join("TEST-links")));
    tend.current = true;
    tend.apply(
        &Ok(Found {
            sessions: ["Calm theme", "Quiet garden", "Local records"]
                .iter()
                .enumerate()
                .map(|(i, title)| Session {
                    id: Some(format!("id-{i}")),
                    path: format!("/TEST/{i}").into(),
                    goal: format!("{title}\nWhy: intact"),
                    stage: Stage::GoalSaved,
                    updated: String::new(),
                })
                .collect(),
            unreadable: 0,
        }),
        true,
        1,
    );
    (tmp, tend)
}

#[test]
fn start_self_pick_cancel_commit_and_toggle_return_to_source() {
    let (tmp, mut tend) = fixture();
    tend.selected = 1;
    tend.start_link();
    assert!(tend.is_linking());
    assert_eq!(tend.selected_line(76, None, "").1, "Quiet garden needs …");
    tend.commit_link();
    assert!(tend.is_linking(), "self-pick stays in linking");
    tend.selected = 0;
    assert!(tend.source_marked(1) && !tend.source_marked(0));
    assert_eq!(
        tend.selected_line(76, None, "").1,
        "Quiet garden needs Calm theme"
    );
    tend.cancel_link();
    assert!(!tend.is_linking());
    assert_eq!(tend.selected, 1);
    assert!(!tmp.path().join("TEST-links").exists());
    tend.start_link();
    tend.selected = 0;
    assert_eq!(tend.link_verb(), "link");
    tend.commit_link();
    assert!(!tend.is_linking() && !tend.detail);
    assert_eq!(tend.selected, 1);
    assert!(
        tend.needs
            .read()
            .into_links()
            .unwrap()
            .contains("id-1", "id-0")
    );
    assert!(
        tend.selected_line(76, None, "")
            .1
            .ends_with(" · needs Calm theme · not ready")
    );
    tend.start_link();
    tend.selected = 0;
    assert_eq!(tend.link_verb(), "unlink");
    tend.commit_link();
    assert_eq!(tend.selected, 1);
    assert_eq!(
        tend.needs.read().into_links().unwrap(),
        needs::Links::default()
    );
    assert_eq!(
        tend.selected_line(76, None, "").1,
        "goal saved · time unknown"
    );
}

#[test]
fn missing_or_ambiguous_ids_refuse_without_writing_or_guessing() {
    for target in [false, true] {
        let (tmp, mut tend) = fixture();
        if target {
            tend.sessions[1].id = None;
            tend.start_link();
            tend.selected = 1;
            tend.commit_link();
        } else {
            tend.sessions[0].id = None;
            tend.start_link();
        }
        assert!(!tend.is_linking());
        assert_eq!(tend.selected, 0);
        assert_eq!(tend.notice, "This goal can't be linked yet.");
        assert!(tend.link_error.as_ref().unwrap().contains("session id"));
        assert!(!tmp.path().join("TEST-links").exists());
    }
    let (tmp, mut tend) = fixture();
    tend.all_sessions.push(tend.sessions[0].clone());
    tend.start_link();
    assert!(!tend.is_linking());
    assert!(tend.link_error.unwrap().contains("not unique"));
    assert!(!tmp.path().join("TEST-links").exists());
}

#[test]
fn unreadable_at_start_or_commit_preserves_bytes_and_discloses_only_in_help() {
    for at_commit in [false, true] {
        let (tmp, mut tend) = fixture();
        if at_commit {
            tend.start_link();
            tend.selected = 1;
        }
        let root = tmp.path().join("TEST-links");
        fs::create_dir_all(&root).unwrap();
        let bytes = b"TEST unreadable links: never replace me";
        fs::write(root.join("needs.json"), bytes).unwrap();
        if at_commit {
            tend.commit_link();
        } else {
            tend.start_link();
        }
        assert!(!tend.is_linking());
        assert_eq!(tend.selected, 0);
        assert_eq!(tend.notice, UNAVAILABLE);
        assert!(!tend.notice.contains("needs.json"));
        assert!(!tend.help_text(None).contains("needs.json"));
        tend.how = true;
        let help = tend.help_text(None);
        assert!(help.contains("Session: /TEST/0"));
        assert!(help.contains("Cannot read links: needs.json is not valid JSON."));
        assert_eq!(fs::read(root.join("needs.json")).unwrap(), bytes);
    }
}

#[test]
fn unavailable_root_and_non_directory_are_plain_refusals() {
    let (tmp, mut tend) = fixture();
    tend.needs = needs::Store::new(Err("TEST unavailable root".into()));
    tend.start_link();
    assert_eq!(tend.notice, UNAVAILABLE);
    assert!(!tend.is_linking());
    assert_eq!(tend.link_error.as_deref(), Some("TEST unavailable root"));
    // Missing file is readable, but its parent cannot be created below a regular file.
    let parent = tmp.path().join("TEST-file");
    fs::write(&parent, b"unchanged").unwrap();
    tend.needs = needs::Store::new(Ok(parent));
    tend.start_link();
    assert!(!tend.is_linking());
    assert_eq!(
        fs::read(tmp.path().join("TEST-file")).unwrap(),
        b"unchanged"
    );
}

#[test]
fn all_readable_sessions_supply_readiness_missing_needs_stay_unknown() {
    let (_tmp, mut tend) = fixture();
    let mut sessions = tend.all_sessions.clone();
    sessions[1].stage = Stage::Ended;
    sessions[2].stage = Stage::Unknown;
    tend.needs.toggle("id-0", "id-1").unwrap();
    tend.apply(
        &Ok(Found {
            sessions: sessions.clone(),
            unreadable: 0,
        }),
        true,
        2,
    );
    assert_eq!(tend.ended, 1);
    assert!(
        tend.selected_line(76, None, "")
            .1
            .ends_with(" · needs Quiet garden · ready")
    );
    tend.needs.toggle("id-0", "id-2").unwrap();
    tend.needs.toggle("id-0", "missing").unwrap();
    tend.apply(
        &Ok(Found {
            sessions,
            unreadable: 1,
        }),
        true,
        3,
    );
    let suffix = tend.selected_line(156, None, "").1;
    assert!(
        suffix.ends_with(" · needs Local records, Quiet garden · 1 need not found · not ready"),
        "{suffix}"
    );
    tend.needs.toggle("id-0", "another-missing").unwrap();
    tend.links = tend.needs.read().into_links();
    assert!(
        tend.selected_line(156, None, "")
            .1
            .contains("2 needs not found · not ready")
    );
}

#[test]
fn refreshed_sessions_cancel_picking_instead_of_redirecting_the_source() {
    let (tmp, mut tend) = fixture();
    tend.selected = 1;
    tend.start_link();
    tend.selected = 2;
    let mut sessions = tend.all_sessions.clone();
    sessions.reverse();
    tend.apply(
        &Ok(Found {
            sessions,
            unreadable: 0,
        }),
        true,
        2,
    );
    assert!(!tend.is_linking());
    assert_eq!(tend.current().unwrap().path.to_str(), Some("/TEST/1"));
    assert!(!tmp.path().join("TEST-links").exists());
}
