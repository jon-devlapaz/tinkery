# One job through the existing records

Design proposal based on [#63's source survey](../research/shared/job-record-survey.md), following
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

See [historical source evidence](../research/shared/job-record-survey.md#what-was-inspected).

## Record owners and limits

See [historical source evidence](../research/shared/job-record-survey.md#record-owners-and-limits).

## Trace one synthetic job

See [historical source evidence](../research/shared/job-record-survey.md#trace-one-synthetic-job).

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

See [historical source evidence](../research/shared/job-record-survey.md#attention-and-cost).

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

[D-writer]: https://github.com/jon-devlapaz/tink-sdlc/blob/8ffa2bdf2743c8904c5eebd27c8ff2f45ec972a3/assets/_system/scripts/sdlc.py#L377-L402
[D-seed-tests]: https://github.com/jon-devlapaz/tink-sdlc/blob/8ffa2bdf2743c8904c5eebd27c8ff2f45ec972a3/tests/test_sdlc_stage.py#L257-L284
[D-protocol-tests]: https://github.com/jon-devlapaz/tink-sdlc/blob/8ffa2bdf2743c8904c5eebd27c8ff2f45ec972a3/tests/test_sdlc_protocol.py#L109-L180
[G-protection]: https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches/about-protected-branches
