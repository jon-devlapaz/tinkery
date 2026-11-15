# Repair handoff

## Situation

Checked: all repair items pass on code commit `d3e0e84`.
The full zero-call check passes. All eight checklist items have evidence receipts.
The supervisor must run SDLC verification. Independent review and operator acceptance remain pending.

## Background

The approved scope remains issue #864 in `brief.md` and `checklist.json`.
This repair follows `03-build/REPAIR-BRIEF.md` and the `build-fix` step contract.
The model pin remains `openai-codex/gpt-6.1-sol:medium`.

Changes:

- Restored the existing garden key bars exactly. `esc back` remains visible at 80 columns.
- Kept thought keys in the summary. An empty thought list displays `t new thought`.
- Prevented thought shortcuts from taking input while the connection panel owns the screen.
- Replaced the broken confirmed-goal mutation with the public `shape::command::new_goal` path.
- Kept readable thoughts when another thought has malformed JSON or an unsupported schema.
- Added visible unreadable counts in the garden and reader.
- Added terminal regressions for the key labels and partial reads. Updated the existing design contract.

Observed cause of the correction failure: the garden intercepted letters in `TEST corrected` while the correction editor was active.
The first `t` opened the thought editor. Later input saved `ed` as a thought instead of saving the correction.
`correction-reproduce.log` records that screen. The repair checks that no connection panel owns the input before opening a thought.
The existing correction assertion remains unchanged.

Architecture diagnostics 3 and 6 apply. Thought read policy stays inside `thought.rs` without changing callers.
The menu owns input routing between the two panels. No new module or public interface was added.

## Assessment

### Checked results

`python3 scripts/full-check.py` passed after the code commit.
`full-check-fix.log` contains the complete result.

- Rust: 1124 passed, zero failed, one ignored, across 23 reported groups.
- Node: 39 passed, zero failed.
- Python: all 86 files passed with the full tier.
- Formatting and workspace Clippy passed with warnings denied.
- Both snapshot commands and the locked build passed.
- `git diff --check` passed for the source repair before its commit.
- The evidence commit's whitespace check flags raw terminal-row padding in logs. The logs retain that output unchanged.

All six requested focused Python files pass in the full check.
Separate checkout-local runs also passed the thought, correction, connection and documentation files.
The documentation file reported 12 tests.

The thought file runs eight top-level journeys and 16 terminal processes across 80×24 and 100×30, with colour off.
It checks save, restart, duplicate save, cancel, store refusal, repository isolation, goal promotion and partial reads.
Save, duplicate, cancel and refusal actions make no provider requests.
Connection and Think journeys use local executable stubs only.
Synthetic tests establish mechanics, not useful real-data connections or operator acceptance.

### Reversion evidence

`probes-fix.log` records all three required mutations rejected at their intended assertions:

1. Calling `new_goal` while saving a thought fails `A thought changed goal sessions`.
2. Trimming saved words fails `Saved words differ from typed words`.
3. Adding thoughts to connection sources fails `A thought reached the provider request`.

The script restores sources and rebuilds the binary, then passes the clean thought journeys.
It passed again after the code commit.

`repair-reversions.log` records two additional rejected reversions.
Removing the input-routing guard reproduces the correction failure.
Restoring fail-fast thought reading hides the valid thought and fails its restart assertion.
Both sources and the binary were restored before the full check.

### Earlier failed attempts

Two separate focused runs failed with TEST roots inside the checkout.
`goal-fix.log` shows the enclosing repository name instead of the fixture repository name.
`tend-fix.log` shows repository filtering where its fixture expected an outside-repository garden.
These runs do not establish passing evidence.
Both files subsequently passed in the unchanged full check with its permitted system-temporary TEST HOME.
No unresolved test failure remains in the final full check.

### Scope and records

No shared goal saver, session format, connection input or saved correction changed.
No approved brief, checklist definition, factory reference or step contract changed.
No named input was unreadable. No product-shape decision remains open in this repair.

`principle-build-the-lever` changed the probe decision: the checked-in script now proves all three required faults in one command.
`unslop` kept the failure notices and this handoff explicit about evidence limits.

No production or live provider endpoint was used. Local test endpoints used synthetic fixtures.
No real audio, delegation, toolchain change or Pi infrastructure edit occurred.
No background job or server was left running by this repair. Terminal fixtures close their processes.

The generated `AGENTS.md` block, stage rules, skill snapshot and step files were already uncommitted at entry.
They remain untouched and outside the repair commits.

## Recommendation

Run SDLC verification on the committed candidate and receipts, then obtain independent review.
I did not run `sdlc.py verify`, `step`, `stage` or `decide`.
No PR, merge or operator acceptance is claimed.
