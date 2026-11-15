# Repair brief for step build-fix (supervisor, 2026-11-15)

Read this file first, then `03-build/output/handoff.md` from the first build step. The approved scope is unchanged: `brief.md` (issue #864).
Candidate under repair: commit `21277a3` plus `a33c440` on branch `issue-864-tend-thought`. You have at most 80 responses. Do the items in order.

## Observed by the supervisor

`python3 scripts/full-check.py` ran on `a33c440` with its normal temporary TEST HOME outside the checkout.
Rust: all groups passed. Node: 39 passed. Lint and Clippy passed. Python: 84 of 86 files passed. Two files failed:

1. `tests/goal_confirmation_terminal.py`, line 196, at 80×24:
   `AssertionError: Missing '↑↓←→ move   enter open   f connections   l link   esc back'`.
   The screen showed `arrows move   enter open   t thought   v read   f connect   l link   esc bac`.
   The existing key-bar labels were shortened, and the exit label is cut at 80 columns.
2. `tests/tend_correction_terminal.py`, line 146 (`edits_and_conflicts`): `AssertionError: Missing 'F2 find one connection'`. Cause not yet observed.

The first build's 24 Python failures came from TEST roots placed inside the checkout. Do not repeat that layout.

## Repair items

1. Key bar. Restore the existing garden key-bar text exactly, at every width the existing tests cover. Do not edit existing tests to fit new text.
   Yohaku Article 4 requires a legible exit: `esc back` must never be cut. Show the thought keys without shortening or displacing existing labels.
   The thought summary line already names `v read` and `t new thought`; when no thought exists, the `t` key still needs one visible, uncut place.
2. `tend_correction_terminal.py`. Reproduce the failure, find the observed cause, and fix that cause. Do not weaken the assertion.
3. Reversion probe for "a thought is recorded as a confirmed goal". The first builder failed three times with a hand-built `Affirmed`.
   Fresh approach: make the mutation call the existing public `shape::command::new_goal` (the `tinkery seed new` path), which already writes a
   confirmed goal session with no Think record. Prove the journey's assertion fails under that mutation, then restore the source and rebuild.
   `scripts/probe-thoughts.py` must run all three mutations in one command and exit zero only when each mutation is rejected.
4. Robustness, small: one malformed or foreign-format thought file must not hide the other thoughts. Show the readable ones and say how many could not be read.
   If this needs more than a small change inside `src/tend/thought.rs`, do not build it; report it instead.

## Checks you run

- Focused: `tests/tend_thought_terminal.py`, `tests/goal_confirmation_terminal.py`, `tests/tend_correction_terminal.py`, `tests/tend_terminal.py`,
  `tests/tend_connection_terminal.py`, `tests/test_documentation.py`, then `cargo +stable fmt --all -- --check` and Clippy with `-D warnings`.
- Full: `python3 scripts/full-check.py` from the checkout root, unchanged. It creates its own temporary TEST HOME in the system temporary directory.
  The supervisor permits that temporary write for this command only. Never point TEST roots at the checkout, the operator HOME or application stores.
- After the checks pass on committed work, mark each checklist item with `python3 _system/scripts/sdlc.py mark issue-864-tend-thought <id> passed --evidence <text>`.
  Mark an item `failed` or leave it unmarked when its evidence is missing. Do not run `verify`; the supervisor runs it.

## Boundaries

No scope beyond #864. No provider calls, no subagents, no changes to the shared goal saver, session formats, Find connections input or the saved correction.
Product-shape choices (key letters, the word "thought") stay as built; list any doubt in the report for the operator.
After two failed fixes of one defect, add logging and collect observations. After three, stop and report problem, goal and failed attempts.
Commit before you end. Write the report at the path in your step contract, result first, with counts for Rust, Node, Python files, journeys and probes.
