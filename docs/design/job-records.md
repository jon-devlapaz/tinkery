# One job through the existing records

This follows [#63's survey](../research/shared/job-record-survey.md) and [yohaku](../yohaku.md).
For the shipped, narrower read-only seed join, use [SDLC handoff](shape-and-handoff.md#sdlc-handoff).
It replaces the proposed Tinkery binding store for Approve and Watch; it does not implement R5/R6 below.

**Revised design proposal; not accepted or implemented.** Authorization covered
design, not the binding writer or R5. I2 [#110](https://github.com/jon-devlapaz/tinkery/issues/110)
and R5 [#111](https://github.com/jon-devlapaz/tinkery/issues/111) are published;
both issues remain `needs-approval`.

[Run retention and lessons](../research/shared/backlog.md#keep-runs-and-learn-from-them)
remain separate proposals. Seed Me, SDLC and GitHub retain their records and decisions.
The external aggregation tool is retired; Measure currently reads no source.
R5 requires [explicit binding](#goal-to-run-identity-prerequisite), never matching
titles, seed paths or a manually selected pair.

## What was inspected

See [historical source evidence](../research/shared/job-record-survey.md#what-was-inspected).

## Record owners and limits

See [historical source evidence](../research/shared/job-record-survey.md#record-owners-and-limits).

## Trace one synthetic job

See [historical source evidence](../research/shared/job-record-survey.md#trace-one-synthetic-job).

## Goal-to-run identity prerequisite

Refresh the survey before building. Verify existing capabilities instead of
reimplementing them. Missing capabilities need separate approval; this proposal
grants no implementation permission or provider budget.

1. **SDLC owns run-instance identity.** IDs must be persistent, globally unique and
   immutable across stages and interruption. Recreating a slug gets a new ID.
   Explicit source-owned legacy adoption may assign identity without execution or
   approval. Neither capability was established by the survey. Verify or propose
   them in SDLC; never assign IDs in the reader.
2. **I2 owns initial binding and duplicate detection.** Explicit confirmation connects
   a saved goal and session to an identified run in one immutable local record.
   Require selected storage outside repository history and operator sessions.
   Modify no Think record, Seed Me ledger or SDLC decision. Revisions, relocation,
   retargeting and migration are deferred. This is not another execution or measurement ledger.

Within that root, the relationship key is `(Seed Me session ID, globally unique SDLC run ID)`.
Identical text, seeds or slugs cannot collapse different keys. The same key with a
different frozen identity conflicts. Matching identity and locators return the
existing reference without writing; the caller owns opening. Different locators
refuse pending relocation support. Uniqueness covers this root only. Conflicting,
unreadable or unsupported records that prevent proving uniqueness refuse creation.
Never skip possible duplicates or pick the newest.

The binding has a format version, not revisions or predecessors. Creation evidence
does not prove the sources are still current:

| Field | Contract |
| --- | --- |
| Format version, job ID | Tinkery owns the format; unsupported versions fail explicitly. |
| Relationship key, locators, origin-node ID, slug | Source IDs establish identity; locators and slugs only find it. |
| Exact confirmed goal and UTF-8 SHA-256 | Compare the whole block with the ledger goal and settled origin answer, not a reconstructed headline. |
| Selected seed revision and content SHA-256; run seed reference and digest | The actual run copy and declared digest must match the selected seed at creation. |
| Ledger version, revision and digest; run-record digest | These describe creation, not later whole-file equality requirements. |
| Optional Think locator and digest | Supplied records must agree on goal and session; absence doesn't prove no goal was saved. |
| Creation time, explicit-link provenance | Retain actual source authority and action. Invent no human or authenticated implementation permission. |

**Write boundary.** Before confirmation, show the exact goal, session, seed revision
and run, with the destination in technical help. Capture reviewed sources, then
reread IDs, versions and byte digests. Refuse changes or unavailability. Leaving
writes nothing. Require byte-equal ledger goal and settled origin answer, resolved
seed confirmation, and a verified supplied Think record, selected seed and run copy.
Hash equality proves bytes, not meaning or confirmer identity. Legacy runs without
source identity remain unbound. The entry point and root configuration still need
review before I2 approval.

Atomically publish complete initial records with concurrent key uniqueness.
Neither partial validity nor overwrite is allowed. Losing creators return a valid
matching reference or conflict, never a second job. A missing destination refuses;
never default to session folders or history. Sources aren't a cross-tool atomic
snapshot. Final validation refuses observed changes, records captured evidence
and requires R5 revalidation. Publication cannot lock sources or prevent later
changes. Selection, navigation, copying, reads and retries never create or repair bindings.

**R5 read boundary.** Use stored locators only. Check relationship IDs, origin and
goal, and the run's bound seed reference and digest against frozen identity.
A changed ID under the same slug, or changed goal or seed, means a conflict or
outdated relationship. Never borrow its evidence. Missing or moved sources mean
unavailable, not permission to search, relocate or retarget. Explain missing update
or relocation routes and preserve bytes. Repeated creation cannot bypass refusal.

Ledger decisions and versions, stage or candidate progress, and whole run digest
changes do not alone invalidate identity. Apply source-owned freshness to current
checks and decisions separately from creation evidence. Report newer source seeds
as different; never substitute for the bound copy. A still-matching copy remains
the historical seed. Captured digests never prove current approval. Unverifiable
required identity stays unknown despite other readable sources.

PR references must explicitly belong to the run; disagreements conflict. Checks
include repository, number and result revision, never permanent binding approval.
No PR means no result evidence, not success. Missing, unreadable, unsupported,
outdated and conflicting evidence remain distinct. Failed or interrupted execution
does not justify another binding.

**Completion tests.** Use temporary synthetic sources to cover valid creation;
identical titles and seeds across sessions; same-slug runs with different IDs;
legacy ID absence; repeated and concurrent creation; same-key conflicts;
prepublication source changes; unsupported formats; partial-write failure; missing
destinations; and leaving without writing. Test revision and relocation refusal
without changing bytes. Distinguish normal ledger and stage progress from changed
run goals or seeds. Reversion probes protect confirmation, source-change detection,
ID checks and duplicate prevention. Generate initial binding and linked-job fixtures
for R5; unlinked fixtures stay negative cases.

## Changes, conflicts and interruption

| Source event | Required interpretation |
| --- | --- |
| Seed Me evidence or decisions change | Version checks reject conflicts; dependents need reopening or revalidation. Preserve history and the original decision alongside the concern. Derived `review_flags` never erase or silently replace decisions. |
| The original seed changes | SDLC binds its copy. Original changes alone do not stale approvals, but rebinding or editing the copy does. Name the used copy and differences; never silently rebind or call it latest. [Test][D-seed-tests] |
| The plan, specification, brief, context or checklist changes | Input digests stale relevant gates, possibly later gates in a full run. Show the first available review gate; retain old receipts as history. |
| The candidate, check policy, test lock, manual receipt or log changes | Verification may stale; a new failure replaces a pass. Show the source's next step, never reuse prior green. [Tests][D-protocol-tests] |
| PR head and checkout, or local and remote checks, differ | Name revisions and sources. Both observations may hold; never combine them into green. |
| GitHub review remains approved | Stale-dismissal rules and review commit identities aren't inspected. Report the observation, not candidate approval. [Rules][G-protection] |
| Verification stops | `running` without an active writer becomes `interrupted`; an active writer stays running. Lock-file existence alone proves nothing. Offer an external log inspection and rerun step; navigation never reruns. |
| The harness or application stops | Find SDLC state by explicit checkout and run; listing isolates invalid runs. Work files, handoffs and tools bindings aid recovery, but no inspected ID resumes agents. An active Seed Me session doesn't prove a worker is running. Reread selected records. |
| A source is missing, unparseable or unsupported | Keep other verified facts, name the gap and offer one repair or refresh. Never fall back to older success. |

Read times mean “read at,” not “unchanged since.” Collect sources separately and
validate the binding and each freshness rule. Refresh before consequential external
actions; identity changes require review of current material. No aggregate approved,
done or safe-to-resume flag is justified.

## Attention and cost

See [historical source evidence](../research/shared/job-record-survey.md#attention-and-cost).

## Proposed R5: one explicit read

Before R5, accept the design and implement, review and verify the
[prerequisites](#goal-to-run-identity-prerequisite). R5 reads one existing binding. The shared reader owns
selection, validation, bounded collection and typed evidence. Phases import no
other phase, parse no source formats and recalculate no SDLC gates. Seed Me owns
decisions, SDLC owns freshness, and GitHub owns remote observations.

Require no installed Substrate or `surface --json` service. Explicit legacy work
or tools references cannot replace a binding. Port useful readers, not the dashboard,
launcher, preparation pipeline or measurement service. Preserve source version,
identity, revision, read time and full decision source, reason and history for
Approve and Taste. Never reduce these to aggregate green.

Explicit reads disclose sources and execution or network access. Git and `gh` can
contact GitHub; selected SDLC status helpers execute checkout code. Read-only is
not execution permission. Use only already-permitted helpers and locations.
Never automatically trust, set up, discover transcripts, watch files or launch
readers on navigation. Disallowed sources stay unavailable. Tests use fakes and
TEST roots, not network or providers.

An invalid or absent binding says “Goal connection unknown” and separates facts.
This is required negative coverage, not successful R5 completion. Positive fixtures
trace the exact goal through its seed and separate decisions to the identified
run, matching checks and result. Reopening performs no write or external retry.
Late results cannot overwrite another job or reopen departed views.

Retention, lessons, binding creation and phase UI are separate slices. Reader
recovery cannot preserve deleted-worktree runs: report missing records and leave
retention to an explicit action. R6 expected-input approval is separate.
Design or prerequisite completion never approves R5 implementation.

## Proposed R6: external approval handoff

Show the first pending decision's exact source, revision and permission. Goal
confirmation saves the goal; seed confirmation accepts the intake revision;
an SDLC stage decision approves current inputs or requests changes; GitHub review
and release belong to the forge and operator. Keep these decisions distinct.

One explicit copy handoff names the checkout, run, stage, current evidence and
source review command. Copying neither runs nor approves. The operator decides
externally with their actual reviewer, source and reason; refresh reads the receipt.
Never invent approval words or approve the goal, seed, stages and release together.

**Separate approval-binding gap:** inspected `decide` hashes inputs at execution,
accepting no expected digest of previously displayed documents. Inputs can change
after copying or rereading. External rereview is possible, but approval of the
displayed revision is not guaranteed. R6 needs separately approved upstream rejection of
changed expected digests; refresh before or after cannot supply it. [Writer][D-writer]

The reader excludes Seed Me `init`, `publish`, `seed confirm`, `end` and viewer
startup; SDLC `new`, `decide`, `mark`, `verify`, `lock-tests` and `stage` (which can
commit and create a worktree); Substrate `init`, `prepare`, `workflow`, ledger
scheduling, updates and archives; and GitHub review or merge.
SDLC `actions.*.allowed` is a precondition, never authorization. R6 adds no hidden
execution or release authority. Missing authentic review identity, changed inputs
or unresolved links return to the external owner with a reason.

## Evidence and checks

[Fixture notes](../../tests/fixtures/job-records/README.md) distinguish source-shaped
records from proposed display expectations. Fixtures cover current, missing,
unreadable, outdated, failed, interrupted and conflicting states, but no implemented
binding. They establish consistency, not production reading or outcomes.
Prerequisite tests remain planned; no binding writer, reader, approval or live trial
runs here. Historical upstream checks are not fresh validation.

[D-writer]: https://github.com/jon-devlapaz/tink-sdlc/blob/8ffa2bdf2743c8904c5eebd27c8ff2f45ec972a3/assets/_system/scripts/sdlc.py#L377-L402
[D-seed-tests]: https://github.com/jon-devlapaz/tink-sdlc/blob/8ffa2bdf2743c8904c5eebd27c8ff2f45ec972a3/tests/test_sdlc_stage.py#L257-L284
[D-protocol-tests]: https://github.com/jon-devlapaz/tink-sdlc/blob/8ffa2bdf2743c8904c5eebd27c8ff2f45ec972a3/tests/test_sdlc_protocol.py#L109-L180
[G-protection]: https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches/about-protected-branches
