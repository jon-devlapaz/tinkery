# Findings

## Measure failure/recovery dogfood, October 14, 2026 (#119)

[Captured terminal screens and fixture checks](2026-10-14-measure-dogfood.json)
cover commit `d17aab109ae93d61a7f2e26d4bae456dad95cbb0`, including #140's session-root
fix. The artifact records the tested binary's SHA-256. These are AI-driven real-PTY
walkthroughs with synthetic ledger commands, not human usability trials or real
ledger validation. Screen-cell text is retained; terminal colour/style is not.

Two walkthroughs started at 80×24 with NO_COLOR and 160×40 with colour enabled,
then resized to 100×30 and 80×24 respectively. Each opened Measure four times:
large failed read, successful retry, empty-stderr failure, then help/exit checks.
The fixture contained 70,000 numbered Unicode lines, a final marker, unsafe
terminal/bidirectional controls and an invalid byte.

Observed in both runs:
- The ordinary failure showed unknown activity and `esc back`, without paths.
- Existing two-layer help disclosed the command, directory and status. Unsafe
  bytes stayed visible as escapes; the privacy warning preceded the details.
- PgDn reached row 69,999 and the final marker; PgUp returned to the start.
  Recovery instructions remained visible at the tail and after resizing.
- Esc stepped details → help → summary → menu. F1/? closed help; q returned
  directly to the menu. Reopening after changing the fake reader showed success.
- Four explicit opens caused exactly four fake-reader invocations; help,
  scrolling and resizing caused none. Source hashes were unchanged, no session
  root was created, and menu q exited successfully. Provider calls: zero.

The initial harness expected the tail to stay pinned when shrinking the viewport.
The screen instead preserved its top-row anchor; another PgDn reached the tail.
The corrected run records both screens. No production change was needed for resizing.
Final inspection separately caught three invisible bidi marks (U+061C/U+200E/U+200F).
A new assertion failed before the fix and when it was reverted; all 12 Unicode
Bidi_Control characters are now covered. The retained walkthrough was refreshed
after that fix, with those three marks and ordinary Arabic text included.

Separate mechanics verification passed: 389 Rust tests (one existing ignored frame
measurement), 36 Node tests, all 18 Python scripts with 44 unittest cases and 56
Measure PTY journeys across four sizes/both colour modes; formatting and Clippy.
Ten deliberate protection-reversion probes failed as intended, then passed with
the protections restored. An earlier suite overlapped a new test edit and stopped
at formatting; the final frozen-source full rerun passed.

No application defect was observed in these bounded walkthroughs. They do not
establish human recognition, colour appearance or nonblocking subprocess behavior:
a stuck read can still block input until it returns; lifecycle remains #120.

## Think dogfood, October 12, 2026

[Read the report](2026-10-12-think-dogfood-report.pdf).

The report covers commit `4f520b22c28cd109de3c7d3fb7263cfad8bc3ccb`:
10 synthetic scenarios and four repeat runs, operated by agents with live model
replies. It does not establish whether later revisions still have these defects.
The PDF is preserved unchanged; the larger local evidence bundle it references
is not included here.

| Finding | Priority | PDF page | GitHub issue |
| --- | --- | --- | --- |
| F-01: Long reviews silently block confirmation | P2 | 4 | [#70](https://github.com/jon-devlapaz/tinkery/issues/70) |
| F-02: Unsent words silently block review | P2 | 5 | [#71](https://github.com/jon-devlapaz/tinkery/issues/71) |
| F-03: Back overlaps the save-error message | P2 | 6 | [#72](https://github.com/jon-devlapaz/tinkery/issues/72) |
| F-04: The record-save warning is clipped | P3 | 7 | [#73](https://github.com/jon-devlapaz/tinkery/issues/73) |
| F-05: Settled separate-goal context appears unfinished | P3 | 8 | [#74](https://github.com/jon-devlapaz/tinkery/issues/74) |
| F-06: One decision appears three times | P3 | 9 | [#64](https://github.com/jon-devlapaz/tinkery/issues/64#issuecomment-6138386385) |
| F-07: Exclusions use double-negative wording | P3 | 10 | [#75](https://github.com/jon-devlapaz/tinkery/issues/75) |

P2 means a reproduced usability failure obstructs an action or recovery. P3
means a lower-impact clarity defect; the intended goal and controls held in
these trials. F-06's priority describes that finding, not all of issue #64.

The six new issues were filed with `bug`, `priority: P2` or `priority: P3`, and
`needs-approval`. Filing them does not approve implementation. F-06 is linked to
the already-approved work in #64 instead of creating a duplicate. Follow the
issues for current status, and recheck findings after overlapping fixes settle.

## Decision-review evidence (#34)

The review replaced a log-only background meaning audit that never changed a
reading; #34 deleted it. In the v3 eval, decisions asked rose from 23% to 40%,
with questions on clear dumps within the 0.5-per-run target. Silent guesses fell
from 18% to 14%, but that difference is within noise, not a demonstrated gain.

## Evidence-note regression check (#58)

AI-operator dogfood on 2026-10-12 put “I have not reproduced that historical
failure in this run” under Still open. A bounded replay through
`openai-codex/gpt-6.1-sol` with low thinking reproduced it with the old reading
prompt. The revised prompt distinguishes statements about the evidence from
requirements and choices about the work, without a keyword filter in the decoder.
The decision-review prompt and the two-call send contract are unchanged.

| Supplied dump | Observed reading |
| --- | --- |
| Same dogfood dump, old prompt | Evidence note stayed unresolved; no focused question |
| Same dump, revised prompt | No unresolved note or question; evidence text remained verbatim in the original |
| Evidence note plus undecided simulated-test/live-trial success check | Asked which evidence counts as success; Done when remained missing |
| Evidence note plus undecided permission to show private text | Asked about the private-text boundary; Must not remained open |

Four sends made eight provider calls through the subscription connection, within
the operator's authorized budget. No goal was confirmed or saved by this eval.
These are three revised-prompt examples and one baseline, not a general quality
rate or a human-recognition result. The small captured projections in
`tests/fixtures/evidence_notes/` exercise readiness, review and exact original
handoff with no calls. The prompt byte test guards the deliberate instruction
change; those replay tests alone do not establish future model behavior.

## Goal-history and review regression check (#64)

Follow-up AI-operator [dogfood](README.md) found a previous goal's model reply in the next
goal's think record, and one permission choice repeated as a question and two
notes. New visits now get a fresh host history; fresh standalone visits also keep
the chosen record folder. Explicit decision/question links let review show the
question once while retaining its alternatives in handoff. The review's `covers`
indices identify the shaping choices it replaces; invalid indices, different parts
and malformed replacement choices leave the originals intact. Different choices
are not collapsed just because they share a part.

The live check used four sends/eight subscription-provider calls with
`openai-codex/gpt-6.1-sol`, low thinking, in isolated TEST roots. The formerly
repeated private-text choice appeared once. A second goal retained two different
permission choices, one as the question and the other as an open note. Its record
held one raw model reply; the first goal retained its three replies, with no
cross-goal overlap. Originals, confirmed goal copies and existing-session handoff
remained intact. This is bounded AI-operator evidence, not a general model-quality
rate or human recognition.

The live run also produced “Must not: Always omit” for an instruction to omit
private text. Normal refinement corrected that draft before it was saved. This
slice does not fix general constraint-polarity errors; the overlap judgment in
`covers` also remains a model judgment, not proof that choices mean the same thing.
Fake-provider terminal journeys cover both entry routes at all four supported
sizes, including distinct same-part choices, saved history and record-folder
preservation. Reversion probes fail for history isolation, linked-note suppression,
decision coverage and its validation, and pass again with the protections restored.
