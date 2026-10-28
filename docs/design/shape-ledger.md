# Shape owns the ledger

**Accepted design; slices 1–8 implemented.** The operator accepted it on 2026-10-19
([#269](https://github.com/jon-devlapaz/tinkery/issues/269)). Each slice, Seed Me changes and any
provider budget remain separate decisions. Read at tinkery `3e167d9`, seed-me `c9d7667`
and tink-sdlc `1c6ffd5` on 2026-10-19.

## Why

The operator, 2026-10-19: "the goal will be a complete deprecation of seed me because
a skill is not the correct method for that crucial step of fact finding and shape
making for tinkery prior to tink sdlc." Yohaku agrees: *move each correction left*,
make the mistake impossible in the code, and write a rule only when code can't hold it.

The former Seed Me helper is a historical reference, pinned above to 2026-10-19.
Completion review and teach-back still contain model judgments, not semantic proofs;
the ledger enforces their typed records, coverage and explicit actions. This design
replaced the former rule that Seed Me alone owns the session, ledger and contract
([Shape](shape-and-handoff.md#shape)).

## 1. One owner for the ledger

**Settled by the operator (2026-10-19): port the helper's rules to Rust, in one shared
module that replaces `seed_me/`.** Vendoring `session.py` behind the subprocess was the alternative.

Rust keeps one typed boundary for every caller rather than a second language on the
critical path. The migration used differential fixtures against the pinned helper;
completed slice 8 retired that harness. Rust regressions now own these protections.

The Rust ledger preserves these former helper protections:
- **Validation:** node schema and statuses; prerequisites exist, are acyclic, and settle first;
  authority kinds; evidence behind `evidence` and `delegated` settlements; deferrals carry a
  reason and a revisit condition; inferences cite settled observations; `contradicts` only on facts.
- **Transitions:** stale versions refused; ended sessions read-only; the confirmed goal pinned;
  nodes superseded, never deleted; kind immutable; reopening needs a reason; a changed
  settled premise forces each descendant to be revalidated with evidence; history and revision kept.
- **Review flags** from changed, withdrawn or contradicted evidence, blocking confirmation and completion.
- **The seed:** check records, the canonical heading sequence, the Decisions and Assumed
  blocks rendered from the ledger, confirmation and its record, verification, and ending.
- **Writes:** one writer lock per session, atomic renames.
- **Receipt screening:** secret-shaped text is refused with the existing Unicode
  normalization and Rust character boundaries, even when regex's Unicode tables differ.
  Screening makes one search per pattern, so
  ordinary long receipts do not require rescanning every remaining suffix.

Dropped with the skill: the browser viewer (Tend's garden and Shape's So far already show
the ledger), the Lean path, simulated operators and `--no-viewer`.
They stay in the "Not adopted" table until a revisit condition holds.

## 2. What Shape writes

Each item is an operator key that says what it does
([Authority](../yohaku.md#2-action-stays-within-explicit-authority) and
[Composition](../yohaku.md#4-make-room-for-informed-judgment)); the model still
only investigates and drafts, and nothing it replies can confirm, settle a decision or
resolve a review flag.

- **Typed findings.** A step's findings become fact nodes: `observation` with its receipt,
  `inference` citing the observations it rests on, or `unknown` ("this investigation has not
  established X"). A question cites its facts through `supported_by`. "Not found" is never
  written as "does not exist". The seed's evidence section renders from these facts.
  This absorbs [#241](https://github.com/jon-devlapaz/tinkery/issues/241).
- **Resolved unknowns (#310 live recheck).** The next step may link an open unknown fact
  to the settled user or delegated decision whose answer covers it. The ledger alone validates
  those IDs and authority, refusing any invalid link without changing the fact or saving the
  reply. Accepted links retain the original claim, answer and receipts with `resolved_by` naming
  that decision; publication preserves the unlinked state in history. Resolved unknowns leave
  active findings and both ledger-rendered seed blocks (Decisions and Evidence), not the ledger.
  If the covering answer changes or reopens, publication removes its link and preserves it in
  history, making the unknown active again. No model reply settles a decision or resolves a
  review flag; semantic coverage remains the model's judgment, not a code proof.
  These defaults are Claude's call, not jondev's. The fixed pre-mortem cannot make a link at
  question time; a later reply names the answer already recorded.
- **Contradictions.** A fact may name the settled node it contradicts. The module flags that
  node and its dependents, and Shape shows the first flagged node as the current question:
  keep it and say why, or revise an operator-answered decision (see the [revision boundary](shape-and-handoff.md#answering-and-confirming)).
- **Answers beyond F2.** Defer a non-blocking question with a reason and a revisit condition
  ([#240](https://github.com/jon-devlapaz/tinkery/issues/240)). Take the suggestion as a
  delegated choice, only on a question with facts and a suggestion
  ([#239](https://github.com/jon-devlapaz/tinkery/issues/239)). Revise a settled answer,
  which reopens its dependents one at a time.
- **Prerequisites.** A question reply may name the earlier questions it depends on. The
  module refuses cycles and holds the question until they settle.

## 3. Closing a draft

The ledger computes confirmation risks from recorded choices, excluding the confirmed
goal and facts from choice counts. Completion review and
optional, explicitly budgeted teach-back remain drafting judgments, not semantic
proofs or implementation permission. For their keys, coverage, receipts, failure
behavior and budget gate, read [Answering and confirming](shape-and-handoff.md#answering-and-confirming).

## 4. The tink-sdlc intake contract

Tinkery is tink-sdlc's sole seed verifier. Intake runs `PATH seed status --json DIR`,
`PATH seed checks DIR`, then status again for stability, requiring `handoff.verified`.
Status omits viewer and snapshot records and opens no network connection; completion
needs no viewer. CI and the handoff journey pin tink-sdlc 1.41.0 at
`6eee1f1d3f2035983c01b3c61b51f0afb5dccf19`.
Think's working-draft guidance and artifact reference remain bundled verbatim from
Seed Me `c9d766760dbeac0c05c92108165f9e968bec0a66`; no runtime reads an installed skill.
The intake change shipped in [tink-sdlc #90](https://github.com/jon-devlapaz/tink-sdlc/issues/90)
(PR #91). For navigation and copying behavior, read
[SDLC handoff](shape-and-handoff.md#sdlc-handoff).
Copied commands use `--tinkery PATH`; the consumer refuses the old helper flag
([tink-sdlc #88](https://github.com/jon-devlapaz/tink-sdlc/issues/88)).
tink-sdlc re-checking the digests itself would make two owners of one rule, so it doesn't.

## 5. Completed migration and limits

The eight-slice migration accepted in [#269](https://github.com/jon-devlapaz/tinkery/issues/269)
completed with [#294](https://github.com/jon-devlapaz/tinkery/issues/294).
For current session-location, migration and continuation behavior, read the
[continuation contract](shape-and-handoff.md#continuation-contract).

The missing-old-path history identity rule was Claude's call, not jondev's.
Old-format ledgers moved with their folder bytes untouched; moving never parsed or
repaired sessions. Receipt times now require RFC 3339; Unicode categories replaced
the Python table. JSON publication adaptation is typed. Editable reads retain their
numeric and refusal compatibility.

The shared ledger names no phase and hides transition, review and unknown-resolution
rules behind typed calls. This improves caller independence and decision ownership
([architecture diagnostics](architecture.md#design-score), rows 2, 3 and 6).
The reply contract keeps the boundary visible (row 7). Missing evidence still refuses;
no automatic retry or architecture-test limit change follows from this design.

## Decisions

The operator settled these on 2026-10-19:

1. Port the helper's rules to Rust.
2. Teach-back is an optional key: the confirmation summary already covers what code can see.
3. Sessions move to `~/.local/share/tinkery/sessions` in slice 8, one owner for Tinkery's data;
   no backward-compatibility reader.
4. Archive the Seed Me repository after one real happy-path run (Shape confirms, Approve
   starts, Watch shows the goal) passes with Seed Me's helper not installed.

Code-level parity doesn't show that Shape asks the right questions; that needs a budgeted
evaluation such as [#179](https://github.com/jon-devlapaz/tinkery/issues/179).
