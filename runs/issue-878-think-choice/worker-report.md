# Issue #878 implementation report

## Situation

Checked: the implementation is committed on `issue-878-think-choice`.
Commit: `c617b37b3aef97f062577f59508726ace9b64ae9`.
Checked: the worktree is clean.
Checked: full verification passed.
Independent review and operator acceptance remain outstanding.

## Background

The implementation uses the approved labels, "Clarify into a goal" and "Save as a thought".
No subagents, real provider calls, real recording, operator data, pushes, PRs, or GitHub edits were used.

## Assessment

### Done-when evidence

1. **Both choices at 80×24 and 100×30 without colour.**
   Checked by `fresh_enter_and_choice_enter_never_send_without_a_digit` in
   `src/think/brain_dump/thought_tests.rs`.
   Checked through real PTYs by `fresh_choice(size)` in `tests/tend_thought_terminal.py`.

2. **1 sends, 2 saves exact words and opens Tend with feedback, Esc preserves words.**
   The Rust regression checks the fake host's request count and exact submitted source.
   It checks the saved JSON and the public `Exit::Garden` result.
   The PTY regression checks `Thought saved.`, exact stored words, and copied bytes after Esc.
   `promotion(size)` checks the existing goal-save path after explicit choice 1.

3. **Enter, repeated Enter, and pasted line breaks never choose. Zero fake calls for Enter, Esc, and 2.**
   Both regressions above check these protections.
   The Rust regression also checks repeated digit 1, modified digit 1, F2, F1, and unrelated typing.
   The PTY regression checks provider records remain absent through cancellation and thought saving.

4. **Correct newline hint and both newline keys.**
   The Rust regression sets `with_shift_enter(false)` and `with_shift_enter(true)`.
   It checks each hint at both sizes.
   It sends Ctrl+J and Shift+Enter in both cases and checks both exact saved newline bytes.
   Startup queries support once in `src/main.rs`.
   Snapshot mode skips that query.
   `fresh_factory` preserves the capability for later Think visits.

5. **Enter sends directly after the first send.**
   The Rust regression checks a second fake request after plain Enter, without a choice.
   `tests/think_source_ref_terminal.py` checks five exact sources with only one initial choice.
   `tests/voice_terminal.py` checks direct answer sending after the first reading.

6. **Standalone option 2 is unavailable and writes nothing.**
   The Rust regression checks the displayed `Open --home` reason and zero fake calls.
   The PTY regression checks the unavailable row and unchanged thought records after digit 2.

7. **Both required reversion probes fail.**
   Each probe ran the named Rust regression in a temporary TEST HOME.
   Both returned exit 101.
   Both guards were restored before the successful full check.
   Exact failing output appears below.

8. **Full zero-call verification.**
   `python3 scripts/full-check.py` passed with exit 0.
   Rust: **1,133 passed, 0 failed, 1 ignored**, across 23 result summaries.
   Node: **39 passed, 0 failed**.
   Python: **87 files passed**, full tier, 8 workers.
   Build, both snapshot launches, rustfmt, and Clippy with denied warnings passed.
   The ignored Rust test is the existing optional large-paste frame measurement.
   Log: `/tmp/w878-final-full.log`.
   Herdr terminal: `t13w`.
   The check ran in a new process session, without an inherited controlling terminal.
   The wrapper supplied the temporary TEST HOME.

9. **Documentation agrees.**
   Updated `docs/design/think.md`, `docs/terminal-setup.md`, and `README.md`.
   After the full check, one documentation sentence was qualified to name Think's key bar, not Shape's.
   `python3 tests/test_documentation.py` then passed all **13 tests**.
   `git diff --check` passed before the commit.

### Additional protections and design judgment

- Pending voice submission uses the same choice boundary.
  `f2_waits_for_final_then_requires_a_choice_before_submitting_exactly_once` checks that route.
  Voice tests use fake helpers only.
- The choice shows its Esc route even when the normal key bar is hidden.
  `hidden_key_bar_keeps_the_choice_exit_visible` checks that preference returns after Esc.
- A missing keyboard capability reply uses the safe Ctrl+J hint.
  Crossterm returns an error when a terminal does not answer the query.
- No module, storage format, Shape behavior, goal-confirmation rule, or provider budget rule changed.
- Design diagnostics examined: **2** (interface simplicity), **3** (caller independence), and **6** (hidden decisions).
  Think owns the choice boundary.
  Layout owns its text and hint selection.
  Main owns terminal capability detection.
  Tests inject one boolean instead of querying a terminal.

### Verification failures resolved

Earlier runs exposed old first-send assumptions in existing tests.
Those journeys now explicitly choose 1 before their first send.
A voice-final route also bypassed the initial choice.
It now shares the writing action boundary.

An early capability-query error prevented unsupported terminals from starting.
The final implementation keeps Ctrl+J as the safe hint when detection fails.
The PTY screen parser now accepts the device-attributes query without painting it.

An initial Herdr-launched check inherited Herdr's controlling terminal.
Crossterm used that terminal instead of each test PTY.
The successful check used `subprocess.call(['python3', 'scripts/full-check.py'], start_new_session=True)`.
No application workaround or installed infrastructure change was added.

### Reversion probe A: fresh Enter sends directly

Mutation: replace `writing_action` with an unconditional `self.submit()`.
Expected failure: the first Enter does not display the choice.
Observed exit: **101**.

```text
   Compiling tinkery v0.1.0 (/Users/jondev/dev/worktrees/tinkery/issue-878-think-choice)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 2.19s
     Running unittests src/lib.rs (target/debug/deps/tinkery-457973c1358b1e14)

running 1 test

thread 'think::brain_dump::thought_tests::fresh_enter_and_choice_enter_never_send_without_a_digit' (158478909) panicked at src/think/brain_dump/thought_tests.rs:226:13:
                                                                                
  tinkery · tinkery-TEST-choice…                                             ?  
                                                                                
                                                                                
    TEST café 👩‍💻 ␍                       Thinking…                               
  Keep⇥these.                                                                   
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                        your answer                             
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
  esc cancel   F1 more                                                          
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test think::brain_dump::thought_tests::fresh_enter_and_choice_enter_never_send_without_a_digit ... FAILED

failures:

failures:
    think::brain_dump::thought_tests::fresh_enter_and_choice_enter_never_send_without_a_digit

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 959 filtered out; finished in 0.01s

error: test failed, to rerun pass `--lib`

```

### Reversion probe B: Enter inside the choice picks 1

Mutation: accept normalized F2 inside the choice, close it, and call `self.submit()`.
Plain Enter normalizes to F2.
Expected failure: Enter starts a fake model call without digit 1.
Observed exit: **101**.

```text
   Compiling tinkery v0.1.0 (/Users/jondev/dev/worktrees/tinkery/issue-878-think-choice)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 2.12s
     Running unittests src/lib.rs (target/debug/deps/tinkery-457973c1358b1e14)

running 1 test

thread 'think::brain_dump::thought_tests::fresh_enter_and_choice_enter_never_send_without_a_digit' (158479992) panicked at src/think/brain_dump/thought_tests.rs:254:13:
assertion `left == right` failed: Enter inside the choice started a model call
  left: 1
 right: 0
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test think::brain_dump::thought_tests::fresh_enter_and_choice_enter_never_send_without_a_digit ... FAILED

failures:

failures:
    think::brain_dump::thought_tests::fresh_enter_and_choice_enter_never_send_without_a_digit

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 959 filtered out; finished in 0.01s

error: test failed, to rerun pass `--lib`

```

### Files changed

```text
README.md
docs/design/think.md
docs/terminal-setup.md
src/main.rs
src/think/brain_dump.rs
src/think/brain_dump/confirmation_tests.rs
src/think/brain_dump/goal_tests.rs
src/think/brain_dump/help_exit_tests.rs
src/think/brain_dump/help_overlay_tests.rs
src/think/brain_dump/layout.rs
src/think/brain_dump/layout_tests.rs
src/think/brain_dump/readiness_tests.rs
src/think/brain_dump/refusal_tests.rs
src/think/brain_dump/scope_and_keys_tests.rs
src/think/brain_dump/selection_tests.rs
src/think/brain_dump/suggestion_tests.rs
src/think/brain_dump/thought_tests.rs
src/think/brain_dump/voice_input.rs
src/think/brain_dump/voice_tests.rs
src/think/brain_dump_tests.rs
tests/agent_operation_terminal.py
tests/brain_dump_terminal.py
tests/dogfood_terminal.py
tests/goal_confirmation_terminal.py
tests/goal_history_terminal.py
tests/help_exit_terminal.py
tests/hidden_review_terminal.py
tests/hidden_send_terminal.py
tests/model_setup_terminal.py
tests/reading_scroll_terminal.py
tests/selection_terminal.py
tests/shape_answer_terminal.py
tests/taste_terminal.py
tests/tend_thought_terminal.py
tests/think_budget_terminal.py
tests/think_condition_terminal.py
tests/think_criteria_terminal.py
tests/think_delivery_terminal.py
tests/think_escape.rs
tests/think_escape_terminal.py
tests/think_goal_filename_terminal.py
tests/think_history_terminal.py
tests/think_question_terminal.py
tests/think_quiet_who_terminal.py
tests/think_source_ref_terminal.py
tests/thought_feedback_departure_terminal.py
tests/voice_terminal.py
tests/vt_screen.py
```

## Recommendation

Review commit `c617b37b3aef97f062577f59508726ace9b64ae9` against issue #878.
No open implementation questions remain.
The two small design additions are the voice-final guard and the temporarily visible choice key bar.
Stub tests prove mechanics, not real-model goal quality.
No operator dogfood or independent review has occurred.
