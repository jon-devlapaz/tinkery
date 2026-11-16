# Issue #866 implementation report

## Situation

Checked: the latest implementation is committed as `9cf601b25ec6f38f8ca8b77098426e0c1cb263e0`.
This new commit fixes both supervisor findings on `b57b40e109cbc56c241f113d8d5d6ddbc3ee3759`.
The original commit was not amended.
The branch is `issue-866-thought-nodes`.
The worktree is clean.
The final full zero-call verification passed.
Operator acceptance and real-thought dogfood remain unchecked.

## Background

The implementation follows the brief and the updated issue supplied with it.
No subagents ran.
No remote model or provider calls ran.
Tests used local stubs and temporary TEST roots.
No real audio was recorded.
No operator storage, installations, Pi configuration, GitHub issues, PRs, or remotes changed.
The exact authorized Violet row and amendment are the only changes to `docs/yohaku.md`.

## Assessment

### Implementation

- Thoughts appear as violet hollow diamonds beside goal rings.
- Thought labels prefer the space below their nodes.
- The scope line leads directly into the graph.
- The removed summary block has no replacement above the graph.
- The selected-item footer labels thoughts and shows save results and unreadable-thought warnings.
- Garden arrows select goals, then thoughts.
- Enter and double-click reuse the existing thought reader.
- Reader return retains the last viewed original, including after reader navigation and refreshed ordering.
- Missing originals produce a visible refusal and a safe selection.
- The all-goals view excludes thoughts. Returning to this repository restores its selected thought when available.
- Goal-link mode retains the same graph area. Thoughts remain visible but cannot become link targets.
- Thought selection refuses links and Find connections. It grants no readiness or Shape access.
- `v` remains available. `g` retains the existing exact-word, unsent Think handoff.
- Capture and shared storage formats remain unchanged.

The view distinguishes `Entry::Goal(PathBuf)` from `Entry::Thought(String)`.
Thought identity uses the complete exact words, not the label or list position.
The existing store already verifies the repository and exact-word content-addressed filename.
No persisted entity type or identity migration was added.

Graf received one optional render-cache shape map.
Its default shapes, input, hit-testing, physics, and layout rules remain unchanged.
This is not a broad graph rewrite.

### Files changed

- `README.md`
- `docs/design/architecture.md`
- `docs/design/shape-and-handoff.md`
- `docs/yohaku.md`
- `src/lib.rs`
- `src/menu.rs`
- `src/menu/menu_tests.rs`
- `src/tend/garden.rs`
- `src/tend/graph.rs`
- `src/tend/graph/tests.rs`
- `src/tend/linking.rs`
- `src/tend/mod.rs`
- `src/tend/thought.rs`
- `tests/tend_mixed_terminal.py`
- `tests/tend_thought_terminal.py`
- `tests/thought_feedback_departure_terminal.py`
- `vendor/graf/API.md`
- `vendor/graf/UPSTREAM`
- `vendor/graf/src/render.rs`

### Done-when evidence

Numbers follow the supplied issue's checklist.
All three named PTY files run at 80×24 and 100×30 with colour off.

1. Checked: no thought summary above the graph. `tend_mixed_terminal.mixed_identity_and_link_guards` rejects thought text above the footer. `thought_diamonds_keep_violet_and_hollow_geometry_in_both_themes_and_without_colour` checks rendered violet shapes and colour-independent geometry at both sizes. It distinguishes thought diamonds from goal rings.
2. Checked: the constitution diff contains the exact authorized additions. `palette_tests::both_themes_keep_five_distinct_readable_roles_on_their_own_paper` checks every role against its theme's paper, including Violet at ≥4.5:1.
3. Checked: `tend_thought_terminal.save_feedback` checks new and duplicate save results in the selected-item footer. It checks exact reader selection, unreadable records, unchanged duplicates, pointer selection, and notice departure. `thoughts_only_and_empty` opens a newly saved thought with Enter.
4. Checked: `mixed_identity_and_link_guards` shows goals and individual thoughts. The footer explicitly labels the selected thought `Thought:`.
5. Checked: `tend_mixed_terminal.thoughts_only_and_empty` captures, opens, restarts, and reopens a thought with no active goals.
6. Checked: `mixed_identity_and_link_guards` directly selects the second same-title thought before any reader visit. Enter must show its second-line original. It then selects the first thought and checks its different full words.
7. Checked: the same test navigates within the reader, returns, and reopens the last viewed original. It compares all session, record, and thought bytes. Removing the viewed thought produces `Thought unavailable. Select another entry.` and selects a visible goal.
8. Checked: `tend_thought_terminal.journey`, `capture_guards`, and `save_feedback` cover live save return, restart, exact bytes, distinct changed copies, and duplicate persistence. `thoughts_only_and_empty` covers restart without goals.
9. Checked: existing `tend_thought_terminal.promotion`, `fresh_choice`, and `capture_guards` retain exact original bytes through the unsent `g` handoff. They check no request before an explicit send. New mixed navigation runs with a zero budget and no stub requests.
10. Checked: `mixed_identity_and_link_guards` checks thought `l`, `f`, and `s` refusals, unchanged records, no readiness, no requests, and both forbidden thought clicks and valid goal links. Existing `journey` checks that only the two goals enter the fake Find connections request. Existing goal, Shape, readiness, and link suites passed.
11. Checked: `mixed_identity_and_link_guards`, `thoughts_only_and_empty`, `save_feedback`, `unreadable`, and `failed_refresh` cover mixed, thought-only, empty, same-title, refreshed-order, scope-toggle, reader-return, link-mode, restart, and partial/unavailable thought reads. These run at both required sizes without colour.
12. Checked: the mixed PTY checks thought diamond clicks and thought label clicks in link mode. Neither changes stored bytes. It also checks displayed goal-label links and thought-label selection. `repository_link` retains goal-node and empty-space click coverage. The mixed test compares graph glyph rows before and after entering link mode.
13. Checked: three mutation/reversion probes failed as expected. The guards were restored. The restored PTY and final full-check passed. Details follow.
14. Unchecked: no real-thought dogfood or operator judgment was collected. Automated checks establish mechanics, not acceptance or useful thought discovery.

### Palette

Both proposed values pass without adjustment.
Contrast uses WCAG sRGB relative luminance against each theme's paper.

| Theme | Violet RGB | Paper RGB | Measured contrast |
|---|---|---|---|
| Light | `(96, 82, 168)` | `(255, 252, 240)` | `6.2496162147:1` |
| Dark | `(176, 160, 232)` | `(38, 35, 32)` | `6.7274646286:1` |

With colour off, hollow diamonds differ from hollow goal rings.
Selected thought shapes remain violet with colour, including their selection outline.

### Final verification

Command, from this worktree:

```sh
python3 -c "import subprocess;raise SystemExit(subprocess.call(['python3','scripts/full-check.py'],start_new_session=True))"
```

Checked: exit status `0`.
Latest full output: `/tmp/w866-review-full-check.log`.
The earlier implementation check remains at `/tmp/w866-full-check-final.log`.

- Rust: **1,134 passed, 0 failed, 1 ignored** across workspace tests and doctests.
- Rust breakdown: Graf 52, Pinstar 40, Tinkery library 960, binary 15, integration tests 47, example tests 16, doctests 4.
- Node: **39 passed, 0 failed, 0 skipped**.
- Python: **88 test files passed, 0 failed**, full tier, eight workers. The runner reports files, not an aggregate assertion count.
- PTY evidence: `tend_mixed_terminal.py`, `tend_thought_terminal.py`, and `thought_feedback_departure_terminal.py` passed at **80×24 and 100×30**, with `NO_COLOR=1`.
- Build, isolated snapshots, Rustfmt, Clippy with `-D warnings`, architecture tests, and documentation checks passed.
- `git diff --check` passed before commit.

### Reversion probes

All probes used the real binary, the mixed PTY, temporary TEST roots, and a zero call budget.
Each faulty build exited through a failing test.
Each mutation was restored immediately after its probe.

#### Wrong-entry opening

Mutation: replace the exact reader selection `thoughts.selected = index` with `thoughts.selected = 0`.
Result: test exit `1`.
Full output: `/tmp/w866-probe-wrong-entry.log`.
Relevant failing output:

```text
AssertionError: Missing 'TEST exact second original':
  Saved thoughts
  Thought 1 of 2
  TEST same first words
  TEST exact first original
```

The direct second-thought Enter check rejected opening the first thought with the same displayed title.

#### Thought action entering goal-link mode

Mutation: replace the thought-link refusal with `self.selected = 0`, then continue into goal linking.
Result: test exit `1`.
Full output: `/tmp/w866-probe-thought-link.log`.
Relevant failing output:

```text
AssertionError: Missing 'Links connect goals, not thoughts.':
  › TEST goal 0
  TEST goal 0 needs … · 1 thoughts could not be read.
  ↑↓←→ pick   enter link   esc cancel
```

The thought action incorrectly entered goal-link mode. The test rejected it.

#### Thought click creating a goal link

Mutation: replace the thought-click refusal in link mode with selection of goal index `1` and continue to `commit_link`.
Result: test exit `1`.
Full output: `/tmp/w866-probe-thought-click.log`.
Relevant failing output:

```text
AssertionError: Missing 'enter link':
  goal saved · 2d ago · needs TEST goal 1 · not ready · 1 thoughts could not b
  ↑↓←→ move   enter open   f connections   l link   esc back   t think
```

The faulty view drew a needs edge and left link mode after the thought click. The test rejected it.

After restoration:

```text
Mixed garden identity, pointer/link guards, refresh, scope, reader return, restart and empty gardens passed at 80x24 and 100x30 without colour. Zero provider calls.
```

The final full-check also passed after restoration.

### Observed failures and corrections

- Initial compilation exposed visibility and borrow errors. These were corrected before verification.
- Existing goal-label tests caught a changed link-target marker. The source and target markers now retain their existing meanings.
- Save-feedback pointer testing exposed delayed save selection overwriting a later click. A temporary trace confirmed the correct hit before a later refresh reset selection. Deferred selection now exists only when the saved identity cannot yet be selected.
- The new geometry check caught scope-row removal when link mode started. Linking now retains that row and the same graph area.
- The new colour check caught a selection outline covering the small violet diamond. The selected thought outline now uses Violet.
- A later Help wording update failed one existing exact-text assertion. The assertion now names the selected original. Final verification passed.
- No unresolved test failures remain.

### Architecture diagnostics 1, 2, 3, and 6

1. Ownership: Tend owns mixed identity and authority. The existing thought module owns reader behavior. Graf owns drawing mechanics.
2. Interface size: the summary interface was removed. Menu routes saves, entry opening, and reader return through Tend operations. Graf adds one optional shape map.
3. Isolated changes: capture, shared thought storage, goal saving, sessions, and connection-source construction remain unchanged.
6. Hidden decisions: display indices and reader order stay behind exact-identity selection. Graf receives only render identities, labels, colors, and shape overrides.

### Deviations and open questions

No scope deviation or blocking design decision remains.
The render-cache override is a small vendored change, documented in its existing API and UPSTREAM files.
Save-result text omits the former `No goal confirmed.` suffix to leave room for the selected title and unreadable warning.
Storage and zero-request checks retain the underlying no-goal-confirmation protection.
No tests were removed. The save-feedback pointer check now uses the displayed goal label because thoughts also occupy the former right-hand goal area.
Goal-node clicks remain covered by `repository_link`. The new mixed test also checks thought-node clicks.

## Supervisor review fixes

Checked: both findings in `scratchpad/shots866/findings-1.md` are fixed in `9cf601b25ec6f38f8ca8b77098426e0c1cb263e0`.
The source findings file is under the same supervisor scratchpad root as this report.

### Finding 1: save feedback followed another thought

Observed cause: the footer attached a shared save message to every selected thought.
The save notice now contains the saved original's exact identity and the duplicate result.
The footer shows the result only when the selected original matches that identity.
Returning to the saved thought can show its result again until the existing departure rules clear it.
Keyboard and pointer selection use the same identity check.

New PTY regression: `tend_mixed_terminal.save_feedback_stays_with_original`.
It saves the weather thought, saves the museum thought, and selects weather with Left.
It rejects both save-result messages on the weather thought.
It also checks pointer selection and the duplicate-save result.
It runs at 80×24 and 100×30 without colour, with a zero call budget and no stub requests.

Before the fix, the regression failed at 100×30.
The baseline failure is in `/tmp/w866-review-save_feedback_stays_with_original-before.log`.
A separate reversion probe removed the exact-identity filter after the fix.
That probe also exited `1` with this failing output:

```text
assert 'Thought saved.' not in screen and 'Thought already saved.' not in screen, screen
AssertionError:
  Thought: TEST colour weather: forecast the colours I keep saving · Thought saved.
```

Full probe output: `/tmp/w866-review-probe-feedback.log`.
The identity filter was restored immediately.

### Finding 2: adjacent labels touched

Observed cause: placement rejected overlapping label cells but allowed adjacent rectangles with no blank column.
Placement now reserves one blank column on each horizontal side when checking another label.
The rendered label and its pointer target retain their actual text rectangle.
Existing clipping and alternate-row placement resolve conflicts without moving graph nodes.

New PTY regression: `tend_mixed_terminal.labels_keep_a_blank_column`.
It renders two long goal labels and two long thought labels.
It checks every selection at 80×24 and 100×30 without colour.
Every label must remain visible, and same-row labels must have a nonempty whitespace gap.
No provider request runs.

Before the fix, the regression failed at 100×30.
The baseline failure is in `/tmp/w866-review-labels_keep_a_blank_column-before.log`.
A separate reversion probe changed the placement check back to the unpadded rectangle.
That probe also exited `1` with this failing output:

```text
AssertionError: Garden labels touch at (100, 30): '  AAAAAAAAAAAAAAAAAAAAAAA…                       CCCCCCCCCCCCCCCCCCCCCCC…DDDDDDDDDDDDDDDDDDDDDDD…   '
```

Full probe output: `/tmp/w866-review-probe-spacing.log`.
The padded placement check was restored immediately.

### Review-fix files

- `src/tend/thought.rs`
- `src/tend/graph.rs`
- `tests/tend_mixed_terminal.py`
- `docs/design/shape-and-handoff.md`

The design contract now states both protections.
Architecture diagnostic 6 improves by binding feedback to identity instead of a shared message.
Diagnostics 1, 2, and 3 retain their existing boundaries and small interfaces.
No storage, palette, capture, link authority, or graph-layout rule changed in this follow-up.

### Review-fix verification

Checked after both reversion probes were restored:

- `tests/tend_mixed_terminal.py`: passed at both sizes. Output: `/tmp/w866-review-mixed.log`.
- `tests/tend_thought_terminal.py`: passed at both sizes. Output: `/tmp/w866-review-thought.log`.
- `tests/thought_feedback_departure_terminal.py`: passed at both sizes. Output: `/tmp/w866-review-departure.log`.
- The required full-check command ran in a new process session and exited `0`.
- Full-check counts remain **Rust 1,134 passed, 1 ignored; Node 39 passed; Python 88 files passed**.
- Rustfmt, Clippy, architecture checks, documentation checks, and `git diff --check` passed.
- The new commit contains only the four review-fix files. The worktree is clean.
- No subagents, remote model calls, real audio, or operator-data actions ran.

No open implementation question or unresolved test failure remains.
Supervisor acceptance of the fixes and operator real-thought dogfood remain unchecked.

## Recommendation

Review commit `9cf601b25ec6f38f8ca8b77098426e0c1cb263e0` and the retained initial commit against issue #866.
Collect the operator's real-thought judgment separately when they authorize that step.
No push, PR, merge, or cleanup was attempted.
