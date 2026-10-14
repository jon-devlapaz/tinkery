# One job through the existing records

Source survey for [#63](https://github.com/jon-devlapaz/tinkery/issues/63), following
[yohaku](../yohaku.md). **Revised design proposal; not accepted or implemented.**
The operator authorized this design-only revision, not the binding writer or R5.
Publication of I2 [#110](https://github.com/jon-devlapaz/tinkery/issues/110) and R5
[#111](https://github.com/jon-devlapaz/tinkery/issues/111) does not accept this
contract; both issues remain `needs-approval`.

The later [Substrate absorption decision](../research/shared/backlog.md#absorb-substrate-keep-runs-and-learn-from-them)
makes Tinkery the owner of the shared reader. Seed Me, SDLC and GitHub still own
their records and decisions. Substrate's inspected formats are compatibility
inputs, not the permanent aggregation service. First establish an explicit
[goal-to-run binding](#goal-to-run-identity-prerequisite); then R5 can read one linked job.
A matching title, seed path or manually selected pair is not a verified link.

## What was inspected

Only source and synthetic tests were read. No operator sessions, transcripts,
saved dumps or installed packages were inspected. No model or voice calls ran.
These are the original survey's inspected commits, not a claim about the latest
available release. This design revision does not refresh upstream contracts;
recheck the relevant source versions before either prerequisite implementation or R5.

| Source | Exact revision | Format at that revision |
| --- | --- | --- |
| Tinkery | `4f520b22c28cd109de3c7d3fb7263cfad8bc3ccb` | think record `schema: 1`; package `0.1.0` |
| Seed Me, Tinkery's existing CI fixture | `1a2f31a136d53bceff51ae8c2e0e787c1a8a90bb` | ledger `schema_version: 1`; Markdown seed |
| tink-sdlc | `8ffa2bdf2743c8904c5eebd27c8ff2f45ec972a3` | package `1.23.0`; machine API `1`; run `schema: 1` |
| tink-substrate | `0f1f7e552ccc0b35c40cb4c6880f5add8f1b0c02` | package `0.2.0`; snapshot `schema: 1`; surface `tink-surface/1` |
| GitHub CLI / GitHub | installed CLI `2.98.0`; official docs read 2026-10-12 | selected `gh pr view --json` fields; live service has no pinned record schema |

Substrate preparation currently names `tink-skills` as its bundled Seed Me
source. That is distinct from the standalone Seed Me commit pinned by Tinkery.
No prepared package was inspected, and compatibility between these revisions is
not established here. A run's own `tools.json` identifies what it actually used.

## Record owners and limits

Paths below describe source formats, not permission to discover private files.
Readers must receive the selected locations explicitly.

| Owner / record and version | Identity and revision | Read path or helper | Freshness rule | Missing or failed means |
| --- | --- | --- | --- | --- |
| Tinkery / think JSON, schema 1 | Filename uses confirmation time and sanitized Seed Me folder name; `seed_me_session`, `goal`, `started`, `confirmed`, `prompt_hash` | Explicit record file; writer is [`think/record.rs`][T-record] | Written once after goal save; no Seed Me ledger version or digest is stored | Record absent may mean the separate record write failed, or no goal was confirmed; it does not prove no Seed Me session exists |
| Seed Me / `ledger.json`, schema 1 | `session_id`, `version`, `revision`; node `id`, `history`, `revision`, `premise_version`, `reviewed_version` | [`session.py read <session>` / `load`][S-session]; `review_flags(nodes)` derives concerns | Publication checks `expected_version`; changes preserve history and require dependent decisions to be reopened or revalidated | Absent, invalid schema, malformed state and an ended session are different; `completed` ends discovery, not implementation |
| Seed Me / `seed-contract.md`, Markdown contract | `revision:` label/date; first line status; `Confirmed by:` words | Read alongside the same session's ledger; [`confirm_seed`][S-confirm] and [seed contract][S-contract] define the fields | Human confirms the displayed label; unresolved or flagged nodes prevent helper confirmation. There is no saved content digest tying that confirmation to later file bytes | Draft, simulated, missing and unreadable are not confirmed intake. A status line alone is insufficient; it cannot authenticate the human |
| SDLC / `runs/<slug>/run.json`, schema 1; decision JSON | Selected checkout plus slug; `profile`, `kind`; optional `seed_contract.{path,sha256,source}`; decision filename carries stage/time/id, body carries `inputs`, `decision`, `reviewer`, `source`, `reason` | [`sdlc.py status <slug> --json`][D-status], API 1; raw `run.json` supplies seed binding omitted from status metadata | Each stage compares the latest receipt's `inputs` digest with current inputs; light profile has gate 3, full profile gates 1–3 | No receipt is pending; changed inputs are stale; rejection remains changes-requested. Local receipts record a claim of human review, not authenticated release authority |
| SDLC / verification, checklist and marks | `candidate.{head,tree}`, `inputs`, `policy`, `test_lock`, optional `checklist`, `manual_marks`, `checklist_items`, `log`; marks have item/result/evidence/candidate/time | API status, `runs/<slug>/04-test/output/verification.json`, `test-log.md`, `checklist.json`, `marks/` | Runtime recomputes current evidence and log digest, checks gates and writer ownership; an automated item and a manual attestation have distinct proof kinds | `blocked`, `running`, `not-run`, `current`, `stale`, `failed`, `interrupted` are separate. A file saying passed is not enough |
| Substrate / selection JSON and work Markdown, schema 1; tools JSON, schema 1 | Selection: `work`, `checkout`, `trust_sdlc`; work: `title`, `project`, `owner`, `next_action`, optional `branch`, `run`, `pr`, `seed`, `handoff`; tools: `run`, `change`, pinned `components` | [`surface --json`][B-surface] for compact view; [`status` / `sources.snapshot`][B-sources] for evidence; `runs/<run>/tools.json` | Per-source `checked_at`, snapshot `observed_at`; reads are not atomic. Prepared resume compares portable tools record with Git-local `substrate-runs/<run>.json`, package integrity and workflow digest | No selection, no link and unavailable source stay distinct. Missing local preparation record cannot be treated as a prepared run |
| GitHub / PR and checks | Repository plus PR number/URL; `headRefOid`, `state`, `isDraft`, `reviewDecision`, `statusCheckRollup`, `mergedAt` | Substrate's pinned `pr_state`, using [`gh pr view --json`][G-cli] | Refresh and compare PR head with checkout HEAD before relying on CI. Existing read does not establish branch protection rules or the reviewed commit for each review | No checks, pending, failed, skipped and unknown never become Substrate `ci: passed`; network/auth failure means unknown, not no PR |
| Substrate / measurement JSONL, schema 1 | `change` is `owner/repo#number`; `key`, `phase`, `at`, `by`, `fields`; optional `fields.change_id` matches tools `change` | [`surface.ledger_view`][B-surface] reads the existing ledger; [`ledger.fold`][B-ledger] applies field updates | File order folds observations; `as_of` is file modification time, not proof each field was refreshed. A unique change-ID link precedes PR fallback; disagreement is unavailable | No ledger, no row and null measurements are unknown, never zero. A partial last line is ignored by the surface; a malformed earlier line is an error |

The current Tinkery tend reader is intentionally lighter than these authority
checks: it classifies the ledger status and seed's first line. Its `seed ready`
label is not a validated approval receipt. See [`seed_me/session.rs`][T-session].

## Trace one synthetic job

The [fixtures](../../tests/fixtures/job-records/README.md) use a made-up counter
change. Their approvals and results are test data, never human actions.

1. **Goal.** `think.json` retains the original words and confirmed goal. Its
   `seed_me_session` names the Seed Me folder. In that folder, `ledger.json`
   identifies the same session; `goal` equals the settled origin node's `answer`.
   Tinkery's save path checks that answer, its user authority and source after
   publishing. Answered or skipped Think questions remain context, not additional
   settled Seed Me decisions. [Source and test][T-save]
2. **Seed and separate discovery decisions.** Read each ledger decision's own
   answer, owner, authority/source and history. The fixture's scope decision is
   separate from its goal decision. Seed confirmation quotes a further decision
   about revision `r1`; completing the human session is another state transition.
   A completed human ledger plus confirmed seed is the documented intake handoff.
   The seed is not implementation permission. [Source][S-contract]
3. **Run.** The chosen Substrate work record names `test-job`, its checkout and
   copied seed path. SDLC's `run.json` records the copied seed's SHA-256 and the
   original source path. That is a byte link to the seed, not validation of the
   Seed Me ledger, confirmation or semantic preservation. The SDLC helper accepts
   a regular file without checking Seed Me status. [Source][D-seed]
4. **Implementation decision.** This example uses a light run, whose combined
   definition gate is stage 3. Its own receipt carries reviewer, source, reason
   and input digest. A full run has separate stage 1, 2 and 3 receipts. Neither
   the goal nor the seed decision can stand in for any of them. [Source][D-decide]
5. **Checks.** The verification receipt records a synthetic candidate and local
   check. Runtime status determines whether the receipt still matches. GitHub
   checks are another source and cover the reported PR head, not automatically
   the local tree. A check pass does not establish a person's acceptance.
6. **Result.** The example PR is open with passing synthetic checks and no GitHub
   review approval: it is not finished. `MERGED` and `CLOSED` are different
   results. A merged PR establishes a merge, not deployment or usefulness. The
   existing ledger calls merged and not detected as reverted “useful”; operator
   taste and real-world outcome are not measured by that rule. [Source][B-ledger]

The inspected records have **no common job identifier across all six steps**.
The think record has no work-record/run/change/PR reference; the work record has
no Seed Me session ID or confirmed-goal reference. A seed path can locate bytes
but cannot establish session identity. The fixtures deliberately demonstrate
this gap; they are not the linked-job success fixture required for R5.

## Goal-to-run identity prerequisite

This is an unaccepted design proposal, not implementation permission. Two
capabilities must be verified before R5. Refresh the source survey first: an
existing capability needs evidence, not redundant implementation. Any missing
capability needs its own approved slice:

1. **SDLC owns run-instance identity.** Its run record must expose a persistent,
   globally unique, immutable run ID that survives stage changes and interruption.
   Recreating a run under the same slug creates a different ID. A source-owned,
   explicit adoption operation may identify a legacy run without executing or
   approving it. Neither capability was established by the original survey; propose and
   verify it in SDLC rather than assigning IDs from Tinkery's reader.
2. **I2: Tinkery owns initial binding creation and duplicate detection.** An
   explicit confirmation connects one saved goal/session to one identified run.
   It creates one immutable local binding under an explicitly selected job-record
   root outside repository history and operator session folders. It modifies no
   Think record, Seed Me ledger or SDLC decision. Binding revisions, relocation,
   retargeting and migration are deferred, not prerequisites for this initial slice.
   This identity record is not a second execution or measurement ledger.

**Relationship key.** Within the selected job-record root, the key is the pair
`(Seed Me session ID, globally unique SDLC run ID)`. Another session or run has
another key, even with identical goal text, seed bytes or slug. A matching key
with different frozen goal/seed identity is a conflict, not a new job or update.
With the same identity and stored locators, duplicate detection returns the
existing job reference without writing; opening a view belongs to its caller.
Different locators require the deferred relocation capability and are refused.
Uniqueness is scoped to this root, not across every root on the machine. Conflicting,
unreadable or unsupported records that prevent proving uniqueness cause refusal;
never choose the newest record or skip a possible duplicate silently.

The binding has a **format version**, but no binding revision or predecessor
chain. Its captured evidence describes creation, not perpetual currentness:

| Binding content | Promise |
| --- | --- |
| Format version and job ID | Tinkery owns the format; unsupported versions fail explicitly |
| Relationship key, source locators, origin-node ID and run slug | Source IDs establish identity; locators and slug find the records, never replace ID checks |
| Exact confirmed goal and its UTF-8 SHA-256 | Freeze the whole block; compare with the ledger goal and settled origin answer, not a reconstructed headline |
| Selected seed revision label and content SHA-256; run's seed reference/digest | Freeze which seed this run uses; require the actual run copy and declared digest to match the selected seed at creation |
| Captured ledger version/revision/digest and run-record digest | Evidence of the records reviewed at creation, not whole-file equality requirements on every later read |
| Optional Think-record locator and digest | Require its goal/session to agree when supplied; absence does not prove that no goal was saved |
| Creation time and explicit-link provenance | Record the action and retained source authority/provenance without inventing a human; this is not authenticated implementation permission |

**Write boundary.** Before confirmation, show the exact goal, session, seed
revision and run being linked, with the destination available in technical help.
Capture the selected records and re-read their IDs, versions and byte digests
against what was reviewed. Refuse observed changes or unavailable sources; leaving
writes nothing. Compare the ledger goal and settled origin answer byte-for-byte;
require resolved seed confirmation. Verify any supplied Think record, the selected
seed and run copy. A matching hash
proves bytes, not semantic fidelity or who confirmed them. A legacy run without
source-owned identity remains unbound. The operator entry point and explicit
storage-root configuration still need review before I2 implementation approval.

Publish the complete initial binding atomically with uniqueness enforced for the
relationship key, including concurrent creators. No partial record is readable as
valid; no existing binding is overwritten. A losing creator returns the valid
matching reference or a conflict, never a second job. A missing destination refuses
creation; there is no default to session folders or repository history. Source
files are not an atomic cross-tool snapshot: refuse observed changes on the final
validation, record the captured evidence and require R5 to validate again. Do not
claim that publication locks sources or prevents a change after the last read.
No selection, navigation, copying, read or reader retry creates or repairs a binding.

**Read boundary for R5.** Resolve only stored locators and verify the relationship
IDs, origin/goal and run's bound seed reference/digest against the frozen identity.
A changed run ID under the same slug or a different goal/seed connection is a
conflict or outdated relationship; do not borrow its evidence. A missing or moved
source is unavailable, not permission to search, relocate or retarget. Report the
reason and the absence of an implemented update/relocation route; preserve all
binding bytes. Repeated creation cannot repair a binding or bypass that refusal.

Ledger decision/version changes, stage progress, candidate changes and whole
run-record digest changes do not by themselves invalidate relationship identity.
Read current decisions and checks through source-owned freshness rules, separately
from creation evidence. A newer source seed is reported as different from the
bound run copy, not substituted for it; if that copy and declared connection still
match the binding, it remains the historical seed used by this run. Never treat
captured ledger/run digests as current approvals. If a required identity component
cannot be checked, keep that connection unknown even if other sources are readable.

A PR reference must explicitly belong to the identified run; disagreeing references
remain conflicting. Read PR checks with their repository, number and result
revision, not as permanent binding approval. No PR yet means no result evidence,
not success. Missing, unreadable, unsupported, outdated and conflicting bindings
and evidence stay distinct. Failed/interrupted execution is source state, not a
reason to create another binding.

**Prerequisite completion.** Test temporary synthetic sources for valid initial
creation, identical titles/seeds on different sessions, same-slug/different runs,
legacy identity absence, repeated and concurrent creation, conflicting identity
under the same key, observed changes before publication, unsupported formats,
partial-write failure, missing destination and leaving without writing. Test
revision/relocation refusal with existing bytes unchanged. Include normal ledger
and stage progress that preserves identity but changes source evidence, and a
changed run seed/goal that must not inherit the relationship. Reversion probes
protect confirmation, source-change detection, ID checks and duplicate prevention.
Provide a generated initial binding and linked-job fixtures for R5; existing
unlinked fixtures remain negative cases. This contract remains unaccepted;
I2 is a published proposal needing separate approval. These planned checks grant
no implementation or provider budget.

## Changes, conflicts and interruption

| What changed or failed | What the existing tools establish | What the proposed view should say/do |
| --- | --- | --- |
| Seed Me evidence or a decision changes | Version checks reject conflicting publication; dependent settlements need reopening/revalidation; history remains. `review_flags` is derived and does not erase decisions | Show the concern and the original decision. Never silently choose a replacement |
| Original seed changes after SDLC copied it | SDLC binds its own copy. Changing the original does **not** stale that run's approvals; rebinding or editing the copy does. [Regression test][D-seed-tests] | Show which copy was used and any observed difference; do not call it the latest seed or silently rebind it |
| Plan, specification, brief, stage context or checklist changes | Relevant stage input digests change; approvals become stale. A full run's earlier artifact can stale later gates too | Show the first available gate needing review; preserve earlier receipts as history |
| Candidate, check policy, test lock, manual receipt or verification log changes | Current verification can become stale; a new failed verification replaces a pass. [Tests][D-protocol-tests] | Show failure/staleness and the source's next step; do not reuse a prior green result |
| PR head and checkout differ; local and remote checks disagree | Substrate reports the mismatch. Both observations can be true for different revisions | Name both revisions and sources; do not combine them into a green job |
| GitHub review remains approved after changes | Dismissal of stale approvals depends on repository rules; the current reader does not inspect those rules or review commit identities. [GitHub rules][G-protection] | Report GitHub's observed state, never “this candidate is approved” from that field alone |
| Verification stops mid-run | A `running` receipt with no active writer becomes `interrupted`; an active writer means `running`. A lock file's mere existence is insufficient | Offer the external next step to inspect the log and rerun; never rerun on navigation |
| Application/harness stops | Explicit checkout + run locates SDLC state; list status isolates invalid runs. Substrate's selected work file, saved handoff and prepared tools bindings support recovery | Re-read the chosen records. No inspected ID resumes an in-flight agent; a still-active Seed Me session also does not prove a worker is running |
| Source disappears, cannot be parsed, or has an unsupported version | Source-specific error or unavailable state; other sources may remain readable | Keep verified facts visible, mark this gap plainly, and offer one repair/refresh step. No automatic downgrade to an older successful record |

Snapshot times mean “read at,” not “unchanged since.” The shared reader collects
sources separately and validates the binding as well as each source's freshness.
Refresh before any consequential external action; if identities change while
reviewing, show that and require review of the current material.
No aggregate “approved,” “done” or “safe to resume” flag is justified here.

## Attention and cost

Tinkery's record has ordered turns, check decisions, start/confirmation times,
model/thinking and prompt fingerprint. It has no token usage, reported dollars,
active-attention clock or provider-call receipt. `confirmed - started` includes
waiting and time away. `origin: live` names the Pi code path: the existing
[confirmation PTY test][T-pty] uses a stub provider yet produces that value.
Neither timestamps nor `origin: live` prove paid calls or operator attention.

Substrate already stores estimated `operator.minutes.{c2,c5,c10}`,
`minutes_by_week`, `touches`, `phases`, `method: gap-cap` and linked session IDs.
Its source algorithm credits gaps before human prompts, capped at 2/5/10 minutes,
merges overlap and excludes launched workers from human touches. That is an
estimate; it misses the first prompt's prior effort and reading/thinking outside
the transcript. This inspection read only the algorithm and synthetic tests,
never those transcripts. [Source][B-sessions]

Existing `cost` fields include normalized token counts, `llm.cost.total` and
`cost_known_share`. Dollars come only from harness reports; absent dollars remain
null, and the known share is weighted by input/output tokens, not by call count.
At the inspected revision Codex contributes tokens but no reported dollars.
No source here proves provider billing, all actual calls, direct attention,
operator taste or customer value. Absorption preserves existing measurement data
and its method/coverage, not a permanent dependency on Substrate's report command.
Migrate any writer/reader ownership in a separate approved slice; do not create a
competing measurement ledger or infer missing values. The inspected surface's
five-change reporting threshold is historical behavior, not an accepted R9 rule.

## Proposed R5: one explicit read

After design acceptance and the [identity prerequisites](#goal-to-run-identity-prerequisite)
are implemented, reviewed and verified, R5 reads one existing binding. Tinkery's
shared reader owns source selection, binding validation, bounded collection and
typed evidence for all phases. Seed Me owns its session/decisions, SDLC owns run
and gate freshness, and GitHub owns remote PR observations. No phase imports
another; phase callers neither parse source formats nor recalculate SDLC gates.

R5 must work without an installed Substrate package or its `surface --json`
service. Existing Substrate work/tools records may supply explicitly selected
legacy references, but cannot replace a verified binding. Port useful source
reading behavior behind Tinkery-owned boundaries, not the whole dashboard,
launcher, preparation pipeline or measurement service. Preserve source version,
identity, revision and read time, and full decision source/reason/history for
later Approve and Taste views. Never reduce these to one aggregate green flag.

The explicit read names its sources and any execution/network access: Git/`gh`
may contact GitHub, and a selected SDLC status helper executes checkout code.
“Read-only” is not permission to execute discovered code. Use only already
permitted helpers/locations; no automatic trust, setup, transcript discovery,
watcher or reader launch on navigation. A disallowed source stays unavailable.
Local contract tests use fakes and temporary TEST roots, not network or providers.

An absent or invalid binding says “Goal connection unknown” and keeps source
facts separate. This refusal is required negative coverage, **not successful R5
completion**. The positive fixture must trace the exact bound goal through its
seed and separate decisions to the identified run, matching checks and result
reference. Reopening the same binding performs no write or external retry; late
results cannot overwrite a different job or reopen a departed view.

Run retention, lesson proposals, binding creation and phase UI are separate
slices. Reader recovery does not promise that an external run survives deletion
of its worktree; report missing records, and leave retention to its explicit action.
The R6 expected-input approval gap is separate from this read-only identity work.

Architecture diagnostics: the proposal clarifies source ownership and simpler
callers (rows 1, 2, 3 and 6), and makes the new boundary visible (row 7). This
design-only revision changes no modules, limits or measured score. Neither design
acceptance nor prerequisite completion automatically approves R5 implementation.

## Proposed R6: external approval handoff

For the first pending decision, show the exact source document/revision and what
that decision permits. Keep these actions distinct:

- Goal confirmation saves that goal.
- Seed confirmation accepts the displayed intake revision.
- An SDLC stage decision approves its current inputs, or requests changes.
- GitHub review and release decisions belong to the forge and operator.

Offer one explicit **copy handoff** action for the selected external workflow.
Copying does not execute it or record approval. The handoff names the checkout,
run, stage, current evidence and source-owned review command; the operator makes
the decision there and supplies their actual reviewer/source/reason. A refresh
afterward reads the new receipt. Never prefill invented approval words or mark
the goal, seed, stages and release approved together.

**Approval binding gap, separate from job identity:** at the inspected revision,
the `decide` helper hashes inputs when the command runs.
It accepts no expected digest for the documents the operator previously saw.
Inputs can change between a copied handoff, rereading and that command. External
human rereview is available; guaranteed approval of the same displayed revision
is not. R6's same-revision completion check therefore needs an upstream command
that rejects a changed expected input digest, as a separately approved dependency. A refresh before or
after `decide` cannot supply that missing guarantee. [Decision writer][D-writer]

Source operations that would write or run are outside this reader: Seed Me
`init`/`publish`/`seed confirm`/`end` and viewer startup; SDLC `new`/`decide`/`mark`/
`verify`/`lock-tests`/`stage` (stage can commit and create a worktree); Substrate
`init`/`prepare`/`workflow`, ledger updates/scheduling and archives; GitHub
review/merge. `actions.*.allowed` in SDLC means a local precondition permits a
command, not that the operator authorized it. R6 adds no hidden execution or
release authority. Missing authentic review identity, changed inputs or an
unresolved link returns the operator to the external owner with the reason.

## Evidence and checks

The [fixture notes](../../tests/fixtures/job-records/README.md) state which bytes
are source-shaped records and which are proposed display expectations. They
cover current, missing, unreadable, outdated, failed, interrupted and conflicting
evidence, but no implemented goal-to-run binding. Those fixtures and the original
source checks establish consistency only, not a production reader or real outcome.
This revision specifies the additional prerequisite checks above; it executes no
binding writer, reader, approval action or live trial. Historical upstream
validation is not fresh validation of this revision.

[T-record]: https://github.com/jon-devlapaz/tinkery/blob/4f520b22c28cd109de3c7d3fb7263cfad8bc3ccb/src/think/record.rs#L158-L190
[T-save]: https://github.com/jon-devlapaz/tinkery/blob/4f520b22c28cd109de3c7d3fb7263cfad8bc3ccb/src/seed_me/tests.rs#L93-L146
[T-session]: https://github.com/jon-devlapaz/tinkery/blob/4f520b22c28cd109de3c7d3fb7263cfad8bc3ccb/src/seed_me/session.rs
[T-pty]: https://github.com/jon-devlapaz/tinkery/blob/4f520b22c28cd109de3c7d3fb7263cfad8bc3ccb/tests/goal_confirmation_terminal.py#L79-L98
[S-session]: https://github.com/jon-devlapaz/seed-me/blob/1a2f31a136d53bceff51ae8c2e0e787c1a8a90bb/skills/seed-me/scripts/session.py#L290-L367
[S-confirm]: https://github.com/jon-devlapaz/seed-me/blob/1a2f31a136d53bceff51ae8c2e0e787c1a8a90bb/skills/seed-me/scripts/session.py#L581-L649
[S-contract]: https://github.com/jon-devlapaz/seed-me/blob/1a2f31a136d53bceff51ae8c2e0e787c1a8a90bb/skills/seed-me/references/seed-contract.md
[D-status]: https://github.com/jon-devlapaz/tink-sdlc/blob/8ffa2bdf2743c8904c5eebd27c8ff2f45ec972a3/assets/_system/scripts/sdlc.py#L628-L829
[D-decide]: https://github.com/jon-devlapaz/tink-sdlc/blob/8ffa2bdf2743c8904c5eebd27c8ff2f45ec972a3/assets/_system/scripts/sdlc.py#L188-L224
[D-writer]: https://github.com/jon-devlapaz/tink-sdlc/blob/8ffa2bdf2743c8904c5eebd27c8ff2f45ec972a3/assets/_system/scripts/sdlc.py#L377-L402
[D-seed]: https://github.com/jon-devlapaz/tink-sdlc/blob/8ffa2bdf2743c8904c5eebd27c8ff2f45ec972a3/assets/_system/scripts/sdlc.py#L1205-L1217
[D-seed-tests]: https://github.com/jon-devlapaz/tink-sdlc/blob/8ffa2bdf2743c8904c5eebd27c8ff2f45ec972a3/tests/test_sdlc_stage.py#L257-L284
[D-protocol-tests]: https://github.com/jon-devlapaz/tink-sdlc/blob/8ffa2bdf2743c8904c5eebd27c8ff2f45ec972a3/tests/test_sdlc_protocol.py#L109-L180
[B-surface]: https://github.com/jon-devlapaz/tink-substrate/blob/0f1f7e552ccc0b35c40cb4c6880f5add8f1b0c02/tink_substrate/surface.py
[B-sources]: https://github.com/jon-devlapaz/tink-substrate/blob/0f1f7e552ccc0b35c40cb4c6880f5add8f1b0c02/tink_substrate/sources.py
[B-ledger]: https://github.com/jon-devlapaz/tink-substrate/blob/0f1f7e552ccc0b35c40cb4c6880f5add8f1b0c02/tink_substrate/ledger.py
[B-sessions]: https://github.com/jon-devlapaz/tink-substrate/blob/0f1f7e552ccc0b35c40cb4c6880f5add8f1b0c02/tink_substrate/sessions.py
[G-cli]: https://cli.github.com/manual/gh_pr_view
[G-protection]: https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches/about-protected-branches
