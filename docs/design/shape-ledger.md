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

Seed Me's robustness lives in two places. Shape initially inherited only the helper's
publication protections; shipped rows below now reach more of its rules:

| Seed Me protection | Where it lives | Shape today |
| --- | --- | --- |
| Stale writes refused, origin pinned, nodes never deleted | `session.py` `transition` (L320) | Inherited: Shape publishes through the helper. |
| Confirmation bound to seed bytes, decisions and checks | `confirm_seed` (L950), `verify_handoff` (L916) | Inherited. |
| Check records and the seed's heading sequence | `validate_checks` (L732), `document_headings` (L846) | Inherited. |
| Typed facts; an inference cites its observations | `validate_claim` (L78) | Shipped: typed fact nodes and `supported_by`, validated by the Rust ledger. |
| Prerequisites, cycles, dependents wait | `validate` (L132) | Shipped: question replies preserve prerequisite decision IDs; the Rust ledger refuses invalid dependencies and holds questions until ready. |
| Changed evidence or answers flag dependents for review | `review_flags` (L288), `transition` | Shipped: contradictory facts flag review; answer revision reopens settled dependents. |
| Deferral with reason and revisit condition; delegation only on grounded questions | `validate` | Shipped: explicit operator actions publish through the ledger's typed answer boundary. |
| Completion review, teach-back, naming the riskiest items at confirmation | Rust ledger and typed Shape replies | Shipped: ledger-computed risks, review within drafting, optional budgeted teach-back. |

Completion review and teach-back still contain model judgments, not semantic proofs;
the ledger enforces their typed records, coverage and explicit actions. This design
replaced the former rule that Seed Me alone owns the session, ledger and contract
([Shape](shape-and-handoff.md#shape)).

## 1. One owner for the ledger

**Settled by the operator (2026-10-19): port the helper's rules to Rust, in one shared
module that replaces `seed_me/`.** Vendoring `session.py` behind the subprocess was the alternative.

| | Port to Rust | Vendor `session.py` |
| --- | --- | --- |
| Owner | One module, in the language of every caller | A second language on the critical path |
| Boundary | Typed calls; rules testable with `cargo test` | JSON over a subprocess; every caller decodes it |
| Cost | About 1,100 lines of Python, plus porting its tests | Small now; every later rule change is Python |
| Risk | Behavior drift | Two owners if Tinkery adds rules beside it |

Contain the drift with a **differential test**: during migration, run the Rust module and
the pinned `session.py` on the same fixtures and require identical refusals and ledgers.
Seed Me's own suite (`tests/`, about 5,400 lines, mostly epistemics, session and checks)
supplies the fixtures. Slice 8 retired that differential harness and fixture; Rust-only regressions now own the protections.

The module promises what the helper promises today:
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

Dropped with the skill: the browser viewer (Tend's garden and Shape's F5 already show
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

## 3. Procedure that becomes code

The skill's closing steps move into Tinkery, computed where they can be:

- **Confirmation summary (zero calls).** The ledger computes the words above F3:
  answers differing from an earlier settled answer, delegated or assumed choices, and
  exact-byte acceptance of suggestions. Each risky choice appears once, with all its
  labels. Counts exclude the confirmed goal and facts: accepted as suggested versus
  chosen otherwise. A changed answer is a reversal indicator, not a semantic judgment.
- **Completion review (inside drafting, no extra call).** The drafting step checks
  settled choices for contradictions, separates *decided* from *done*, and names a
  case that must fail each check. The model supplies case strings in check order;
  Tinkery attaches the exact statements, leaving no wording to copy; named objects
  are not a model reply format. New blockers return as questions. The
  ledger enforces typed review coverage and retains the review as append-only,
  byte-bound confirmed-goal receipts through teach-back's recording path. Its decided,
  done and failure-case lines appear with the risk summary as drafting guesses;
  the ledger does not prove those judgments.
- **Teach-back (optional F7 on a draft).** The operator may describe what will and
  won't be built. One explicitly budgeted comparison lists mismatches as guesses,
  never blocking F3. The comparison has no tools or repository access.
  Words, mismatches and skips are preserved verbatim in the ledger;
  no result grants build permission.

[Answering and confirming](shape-and-handoff.md#answering-and-confirming) owns the
shipped keys, reply schema, receipt format, failure behavior and budget gate.

## 4. The tink-sdlc intake contract

**Shipped:** intake uses Tinkery as its sole verifier. The consumer runs exactly
`PATH seed status --json DIR`, `PATH seed checks DIR`, then status again for stability,
requiring `handoff.verified`. Tinkery's status omits viewer and snapshot records and
opens no network connection; completion needs no viewer.
Think's working-draft guidance and the artifact reference are bundled verbatim from
Seed Me `c9d766760dbeac0c05c92108165f9e968bec0a66`; no runtime reads an installed skill.

[tink-sdlc #90](https://github.com/jon-devlapaz/tink-sdlc/issues/90) shipped in PR #91,
scaffold 1.34.0. CI and the handoff journey pin current main (1.41.0) at
`6eee1f1d3f2035983c01b3c61b51f0afb5dccf19`. Copied intake commands use
`--tinkery PATH`; the old helper flag is refused by that consumer.
The session folder's files keep
their names (`seed-contract.md`, `ledger.json`, `seed-checks.json`,
`seed-confirmation.json`), so Approve and Watch's match on the seed's canonical path and
sha256 ([#258](https://github.com/jon-devlapaz/tinkery/issues/258)) keep working.
tink-sdlc re-checking the digests itself would make two owners of one rule, so it doesn't.
This renames the flag in [tink-sdlc #88](https://github.com/jon-devlapaz/tink-sdlc/issues/88).

## 5. Migration

The module reads today's `ledger.json` unchanged, so existing sessions keep working and
slice 8 moves only their location. Each slice had its own PR and approval:

1. **Rust ledger: read, validate, transition, review flags**, under the differential test.
   Shape still calls the helper; nothing visible changes.
2. **Shape publishes through the module** (answers, questions, objections, probes).
3. **Seed, confirmation, verification and ending in Rust** (implemented), plus the two
   read-only `tinkery seed` commands. The consumer flag change is separate (section 4).
4. **Typed findings and contradictions** (implemented; section 2). The ledger alone
   validates facts. Old untyped model replies are refused; existing untyped sessions
   still read unchanged. `F6 keep with reason` revalidates without changing an answer
   or calling a model; flags continue to block confirmation and completion.
5. **Deferral, delegation and revising an answer** (implemented; #284, #240, #239).
   The ledger alone owns required fields, grounding, revision eligibility and dependent reopening;
   Shape presents its refusals without duplicating the rules.
6. **Prerequisites** from question replies (implemented; #286). The ledger alone owns
   existence, cycles and ready order; F5 names the earlier answers held questions wait for.
7. **Confirmation summary, completion review and teach-back** (implemented; #292, section 3).
8. **Retire Seed Me** (implemented; #294): remove the skill flag, external CI fixture,
   skill continuation prompt and helper subprocesses, including Think saving and its viewer.
   Sessions move once to `~/.local/share/tinkery/sessions` on a default-root launch;
   a visible notice names the move. Existing old and new roots refuse rather than merge.
   There is no old-root reader. Watch's old-path joins become unknown.
   Think records remain byte-identical: history matches unique live ledger session IDs.
   A path-only record whose folder is missing may use its final folder name as that ID;
   duplicate live IDs refuse. This identity rule was Claude's call, not jondev's.
   Old-format ledgers move with all their folder bytes untouched, even when today's ledger
   reader refuses their stored fields; the move never parses or repairs sessions.
   Differential tests retire; Rust-only protections remain. Receipt times now require
   RFC 3339; Unicode categories replace the Python table, and JSON adaptation is typed.
   The installed launcher's removed flag is a separate held edit after merge.

## Design score and limits

- Row 3 (implementation changes without touching callers) improves: ledger rules sit
  behind typed calls, not a JSON subprocess every caller decodes.
- Row 6 (each module hides an important decision) improves: transitions, cascade,
  review flags and unknown-resolution eligibility hide in one module.
- Rows 2 and 3 improve: a step proposes ID links without owning their rules; active findings
  and seed projections consume the ledger's resolution marker, with no answer parser.
  The existing ledger and Shape contracts keep that ownership visible (row 7).
- #296 improves rows 2, 3 and 6: Shape decodes ordered case strings; the ledger alone
  pairs them with check statements and owns coverage, without changing saved review
  records, byte-bound receipts or their readers.
  The [reply contract](shape-and-handoff.md#answering-and-confirming) keeps that boundary
  visible (row 7). Missing evidence still refuses; no automatic retry is added.
- The phase rule holds: the ledger is shared code that names no phase; Think, Tend,
  Shape and Approve use it as they use `seed_me/` today.
- No limit in `tests/architecture.rs` changes in this document.

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
