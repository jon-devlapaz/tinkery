# Repair handoff 2

## Situation

Checked: all three approved repair items pass on code commit `eb3efb6`.
The full check passes with 1124 Rust tests, 39 Node tests, and all 86 Python files.
Rust reports one ignored test and zero failures.
All eight checklist items have new evidence receipts after the committed full check.
The supervisor must run verification and obtain independent review.

## Background

The scope remains issue #864 and `03-build/REPAIR-BRIEF-2.md`.
I read the approved brief, checklist, step contract, stage contract, checkout instructions, and round-1 review.
The step model pin remains unchanged.
No approved input was unavailable.
I reproduced I-1 locally instead of relying on the reviewer's external probe scripts.

Changes:

- One private `thought_summary` helper controls summary visibility in both rendering and pointer handling.
- Repository link journeys reject empty-space writes and check the intended goal link at both terminal sizes.
- A successful thought publication followed by directory-sync failure now reports saved words with uncertain persistence.
- Tend help names `t`, `v`, and the reader's `g` action. The garden key bar is unchanged.
- The design contract records the help keys.
- `scripts/probe-thought-repair.py` reproduces the offset fault and injects directory-sync failure through the public terminal interface.

N-2 and questions Q1 and Q2 remain operator decisions. This repair does not change them beyond the approved help addition.
No shared goal saver, session format, provider input, or saved correction changed.

## Assessment

### Checked results

`03-build/output/focused-fix-2.log` records all seven requested focused Python files passing.
It also records formatting and workspace Clippy passing with warnings denied.
The documentation file reports 12 tests.

`03-build/output/full-check-fix-2-retry.log` records the unchanged full check passing on committed code `eb3efb6`.
The locked build, both snapshots, formatting, and workspace Clippy pass.
Rust reports 1124 passed, zero failed, and one ignored across 23 groups.
Node reports 39 passed and zero failed.
Python runs the full tier with 86 files. All files pass.

The thought test runs ten top-level journeys across 80×24 and 100×30, with colour off.
The added journeys check help, a click three rows below a visible goal, and a click on that goal.
They assert the exact saved needs pair and zero provider requests.
Existing journeys check save, restart, duplicate save, cancel, an unwritable store, repository isolation, promotion, and partial reads.
All TEST roots use the system temporary directory.
Local executable stubs establish mechanics, not real-data quality or operator acceptance.

### Reversion and fault evidence

`03-build/output/link-reproduce-2.log` records the original offset defect writing a needs link from empty space.

`03-build/output/repair-probes-2.log` records both sizes rejecting the bypassed pointer helper.
Each fault fails with `Empty-space click wrote a needs link`.
The same script injects directory-sync failure after publication at both sizes.
The screen reports `Thought saved, but persistence is uncertain.`
The saved words remain byte-for-byte equal to the editor words.
The script restores sources, rebuilds, and passes the clean thought journeys.

`03-build/output/thought-probes-2.log` records all three required mutations rejected:

1. Recording a thought as a goal fails `A thought changed goal sessions`.
2. Trimming the words fails `Saved words differ from typed words`.
3. Sending a thought to the provider fails `A thought reached the provider request`.

The script restores sources and the binary. The clean thought journeys then pass.
Both probe scripts pass after the final code commit.

### Pointer audit

I checked `Home::handle_mouse_at`, `Tend::pointer`, `garden::render`, and the graph adapter.
Mouse down, drag, release, and scroll all enter the same `Tend::pointer` rectangle adjustment.
Rendering and pointer handling now use the same summary helper before `sync_graph`.
Both paths retain the existing scope-row adjustment and graph footer subtraction.
Graph labels and ring picking use the resulting graph area.
Keyboard selection changes the selected identity, not the body rectangle.
Keyboard pan and zoom use the current graph state.
`release_pointer` clears drag state and does not calculate a rectangle.
Hover events reach the common adjustment, then the graph adapter ignores them.
Help, active thoughts, connection panels, and non-garden states retain their input guards.
No separate body-rectangle path needs a second correction.

Architecture diagnostics 3 and 6 apply.
Tend owns the summary visibility decision in one private helper.
Thought storage owns the saved-state notice without adding a public interface or storage mechanism.

### Failed attempt

The first committed full check failed the existing source-line-length limit.
The help string grew beyond 200 source characters.
`03-build/output/full-check-fix-2.log` preserves that failure.
Commit `eb3efb6` splits the source string without changing its displayed bytes.
The unchanged full check then passes. No unresolved check failure remains.

### Boundaries

`principle-build-the-lever` changed verification into a committed, repeatable fault script.
`unslop` kept the persistence notice explicit about the saved state and remaining uncertainty.

No live provider or production endpoint was used.
No real audio, delegation, model change, toolchain change, or Pi infrastructure edit occurred.
Test terminal fixtures close their processes. No server or background job remains from this repair.
I did not run `sdlc.py verify`, `step`, `stage`, or `decide`.

The generated `AGENTS.md` changes and active step files existed at entry.
They remain untouched and outside these repair commits.
Raw terminal logs retain row padding, which can trigger whitespace warnings.

## Recommendation

Run SDLC verification on the committed candidate and new receipts.
Then obtain independent review of I-1, N-1, and N-3.
Passing checks do not establish operator acceptance or release approval.
