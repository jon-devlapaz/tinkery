# Shape owns the ledger

**Accepted design; slices 1–5 implemented.** The operator accepted it on 2026-10-19
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
| Prerequisites, cycles, dependents wait | `validate` (L132) | Never fires: `ask` writes `"prerequisites": []` (`src/seed_me/answering.rs:516`). |
| Changed evidence or answers flag dependents for review | `review_flags` (L288), `transition` | Shipped: contradictory facts flag review; answer revision reopens settled dependents. |
| Deferral with reason and revisit condition; delegation only on grounded questions | `validate` | Shipped: explicit operator actions publish through the ledger's typed answer boundary. |
| Completion review, teach-back, naming the riskiest items at confirmation | `SKILL.md` prose only | Missing. |

The remaining inactive protections need later slices to write what triggers them;
the prose-only rows are still steps a model can skip. This design replaced the former
rule that Seed Me alone owns the session, ledger and contract
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
supplies the fixtures. When the last slice lands, the pinned helper and its fixture go.

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

Dropped with the skill: the browser viewer (Tend's garden and Shape's F5 already show
the ledger), the Lean path, simulated operators and `--no-viewer`.
They stay in the "Not adopted" table until a revisit condition holds.

## 2. What Shape writes

Each item is an operator key that says what it does (articles 4 and 5); the model still
only investigates and drafts, and nothing it replies can confirm, settle a decision or
resolve a review flag.

- **Typed findings.** A step's findings become fact nodes: `observation` with its receipt,
  `inference` citing the observations it rests on, or `unknown` ("this investigation has not
  established X"). A question cites its facts through `supported_by`. "Not found" is never
  written as "does not exist". The seed's evidence section renders from these facts.
  This absorbs [#241](https://github.com/jon-devlapaz/tinkery/issues/241).
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

- **Confirmation summary (code).** Above F3, name the riskiest items from the ledger:
  answers reversing an earlier decision, delegated or assumed choices, and choices
  accepted exactly as suggested. Also count accepted-as-suggested against chosen.
- **Completion review (one step).** Before drafting, a step reads the settled decisions
  against each other for contradictions, separates *decided* from *done*, and names one
  case that must fail each check word. New blockers return as questions.
- **Teach-back (optional key).** Before F3, the operator may say in their words what
  will and won't be built; a step lists mismatches with the draft. Skipping is recorded.

## 4. The tink-sdlc intake contract

Today intake runs Seed Me's helper twice (`status`, `seed checks show`) and requires
`handoff.verified` (tink-sdlc `assets/_system/scripts/sdlc.py:1479-1517`).

**Shipped:** Tinkery owns seed checks, confirmation, verification and ending.
`tinkery seed status --json DIR` and `tinkery seed checks DIR` print today's JSON shapes;
status omits `viewer`, `snapshot` and `snapshot_current` and never opens a network connection.
Completion needs no viewer record or `--no-viewer`. `contract()` still reads Seed Me's
installed `references/seed-contract.md` until retirement.

**Pending consumer change ([tink-sdlc #90](https://github.com/jon-devlapaz/tink-sdlc/issues/90)):**
intake will take `--tinkery PATH` in place of
`--seed-me-helper PATH`. Tinkery's copied intake command keeps today's helper flag until
tink-sdlc accepts the new one; that requires a separate tink-sdlc PR and pin bump, with the
pin bump waiting for the operator's "merge". The session folder's files keep
their names (`seed-contract.md`, `ledger.json`, `seed-checks.json`,
`seed-confirmation.json`), so Approve and Watch's match on the seed's canonical path and
sha256 ([#258](https://github.com/jon-devlapaz/tinkery/issues/258)) keep working.
tink-sdlc re-checking the digests itself would make two owners of one rule, so it doesn't.
This renames the flag in [tink-sdlc #88](https://github.com/jon-devlapaz/tink-sdlc/issues/88).

## 5. Migration

The module reads today's `ledger.json` unchanged, so existing sessions keep working and
the session root stays where it is. Proposed slices, each one PR and its own approval:

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
6. **Prerequisites** from question replies.
7. **Confirmation summary, completion review and teach-back** (section 3).
8. **Retire Seed Me:** remove `--seed-me`, the `.ci-seed-me` fixture, the continuation
   prompt that names the skill (`src/seed_me/mod.rs:56`) and the pin in tink-sdlc, and move
   sessions to `~/.local/share/tinkery/sessions` once, with no reader for the old root.
   Runs whose seed path was under the old root then show as unknown in Watch. This
   touches the operator's installs and launcher, so it waits for the operator's explicit "merge".

## Design score and limits

- Row 3 (implementation changes without touching callers) improves: ledger rules sit
  behind typed calls, not a JSON subprocess every caller decodes.
- Row 6 (each module hides an important decision) improves: transitions, cascade and
  review flags hide in one module.
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
