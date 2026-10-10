# C16 plan — historical two-sentence version

**Superseded by the operator-approved pstack revision** in
`factory/reports/Tinkery C16 revision brief.md` and `Tinkery goal shape from pstack.md`.
Current implementation/evaluation: [`revised/README.md`](revised/README.md).
Seven parts replace the former five; who/outcome/why/done_when are required;
when/keep/avoid are optional. A labelled block replaces the two-sentence composer
and is identical in reading and frozen confirmation. Typing after readiness refines
the goal via explicit F2; source/history/consent remain unchanged. Success/constraints
are carried through existing official origin evidence, not a new Seed Me schema.
The original plan below is retained as history, not current implementation authority.

Authority: operator authorized Phase 2 after the Claude planning-session review in
`factory/reports/Tinkery C16 plan review.md` (2026-10-09). Baseline is main
`8818e63cc4811fc0935409580589ddbdc0612b7f`; 14/40 provider calls consumed, 26 remain.

## Scope

One structured goal, one composer, targeted clarification and a quiet frozen review.
Keep landed one-reading/one-voice/plain-original/stable-dump behavior. No D4 notecard
stack, C13 evidence UI, launcher/installed binary change, Seed Me repo change or
operator-session write. Branch `think-goal-shape` from main. Open a PR; do not merge.

## Types and lenient boundary

`GoalParts` owns five optional strings: situation, outcome, why, proof, boundaries.
`Part` names them; `Question.target` associates clarification with a part. `Guess`
and immutable `Board` carry parts plus `open`, the model's list of missing or
consequentially vague part names. Each next result **replaces** parts/open: no
separate issue ledger, semantic resolve/merge machinery or independent goal string.
Application state owns only skipped targets, ask counts, pending scope and existing
question/source history. Missing parts also remain observably missing regardless of
whether the model omits them from `open`.

Missing/null/empty parts are allowed; unknown fields/part names are ignored and
logged. Keep readable singleton/fence/control normalization. No exact-key schema,
quotas, word limits or mandatory citations. Exact claimed citations, grapheme
boundaries, history/scope identity and resource safety remain strict. All-empty parts
must display without fabricating a reading. No freeform legacy board path: historical
captures pass through the new parser, do not infer parts from old sentences, and can
have no composed reading. Retire exact old-board equality and legacy-only tests.

## Prompt

Request five natural clause-shaped parts in the person's voice, an ordered `open`
list and one targeted question when clarification would change the goal. Keep the
person's own words for criteria, quantities, constraints, referents and unverified
claims; never turn them into invented thresholds. Precision inside a present part
belongs to the seed and never blocks readiness or triggers a numbers question.
Mechanisms remain candidates; compatible aims need no forced either/or.

**No eval phrases, names or quantities in the prompt.** Specific preservation examples
belong exclusively in tests/evidence. Previous parts/open and application history are
input, not authority. One shape request per explicit F2; audits remain asynchronous
log-only. No repair/retry or additional semantic veto.

## Composer

One pure owner composes:

`[Situation], I can [outcome], so [why]. It works if [proof]; [boundaries].`

Omit empty clauses and their dangling connectives/punctuation; boundaries alone are
plain prose; all-empty yields no goal. Handle outer whitespace/terminal punctuation
without replacing content or quantities. Normal complete readings have two prose
sentences, with no part labels. Do not reject readable partial boards for prose
shape. Reuse exactly the composed string in reading, paper/copy, frozen review and
`Affirmation.goal`, never a separately authored framing.

## Targeting and stopping

- Each result's parts/open replace the previous result. No inferred semantic closure
  from an answer being historically `Settled`.
- Ask a part at most twice: the initial question and one follow-up. A still-open part
  beyond that stays open for the seed. A skipped target is not re-asked. Undo skip
  restores the original local question; it is not a new ask.
- Track actual exposed targeted questions, not provider attempts. Keep counts in the
  request and enforce bounds locally as well as instructing the model.
- Completion phrase appears only with all five parts present, no consequentially
  vague/open part, no skipped target/pending scope and no active question:
  **Nothing I'm unsure about. Review when you're ready.**
- Present-but-coarse wording is sufficient; do not demand numerical precision.
- Missing/skipped/exhausted parts remain under Still open; no completion phrase.
  Review and explicit confirm remain possible for a meaningful partial goal.
  An all-empty goal cannot be affirmed. No state above grants consent.

## Quiet F3 review and handoff

Freeze composed prose and parts together. Under `You are confirming` and the goal,
show **only present parts**, with short plain labels `when`, `I can`, `so`,
`it works if`, `not`. Missing/skipped/open parts appear once under `Still open`, with
retained questions/notes; no `Not supplied` lines and no duplicate open sections.
Keep wrapping/scroll gate, full frozen review, distinct typed confirm and goal-only
consent. No part labels in the main reading.

Pass composed prose in the existing `Affirmation.goal`; otherwise preserve official
helper/read-back/recovery, immutable origin, active human session, source evidence
and no-seed contracts. No schema extension or hand-written ledger.

## Tests and verification

Read existing boundary/UI/goal/handoff/terminal tests first. Add tests for every
composer omission/combination, boundaries-only, all-empty, punctuation/Unicode;
partial/extra/unknown parsing; all-five versus missing/vague readiness; precision
not being a missing part; capped asks, skipped targets/undo/result races; present-only
quiet review at 80x24/100x30/160x40/320x40; frozen exact reading-to-handoff identity.
Keep exact-word/quantity/constraint test fixtures out of the prompt. Replay historical
raw captures through the new parser for safe display, without inferring parts or
asserting old freeform-board equality. Drop legacy-only tests, explain replacements.

Run reversion probes that fail when boundaries disappear, wording changes,
empty queues falsely declare readiness, skip/cap guards are removed or the handoff
uses independent prose. Run fmt, locked workspace/all-targets/doc tests, strict
Clippy with CI's resolved stable toolchain (including `single_element_loop`), locked
build and all Python suites against official pinned TEST helpers. Obtain green
Ubuntu/macOS exact-head CI; do not equate local lint with CI.

After eval: identical six inputs and fixed ledger actions, Luna/low, temporary
`--seed-session-root` for every run, no confirmations. Save under `after/`, compare
retention, questions and stop rounds. Continue cumulative provider counting from 14;
normal ceiling is 8 shape + 8 log-only audit = 16 additional calls (30 total), with
absolute shared ceiling 40. No automatic repair/retry. Preserve failed evidence.

Pause after opening the PR and report commands, validation, after/baseline table,
counts, exact errors/limits and PR link. Never merge.
