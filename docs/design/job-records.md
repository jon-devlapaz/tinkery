# One job through the existing records

Based on [#63's survey](../research/shared/job-record-survey.md) and [yohaku](../yohaku.md).
**Revised design proposal; not accepted or implemented.** Authorization covered
design, not the binding writer or R5. I2 [#110](https://github.com/jon-devlapaz/tinkery/issues/110)
and R5 [#111](https://github.com/jon-devlapaz/tinkery/issues/111) are published;
both issues remain `needs-approval`.

[Substrate absorption](../research/shared/backlog.md#absorb-substrate-keep-runs-and-learn-from-them)
makes Tinkery the shared-reader owner, not record/decision owner. Seed Me, SDLC
and GitHub retain those. Substrate formats are compatibility inputs, not a permanent
service. R5 requires [explicit binding](#goal-to-run-identity-prerequisite), never
matching titles, seed paths or a manually selected pair.

## What was inspected

See [historical source evidence](../research/shared/job-record-survey.md#what-was-inspected).

## Record owners and limits

See [historical source evidence](../research/shared/job-record-survey.md#record-owners-and-limits).

## Trace one synthetic job

See [historical source evidence](../research/shared/job-record-survey.md#trace-one-synthetic-job).

## Goal-to-run identity prerequisite

Refresh the survey before building: verify existing capabilities instead of
reimplementing them. Missing capabilities need separate approval; this proposal
grants no implementation permission or provider budget.

1. **SDLC owns run-instance identity:** persistent, globally unique, immutable
   across stages/interruption; recreating a slug gets a new ID. Explicit source-owned
   legacy adoption may assign identity without execution/approval. Neither was
   established by the survey: verify/propose in SDLC, never assign IDs in the reader.
2. **I2 owns initial binding/duplicate detection:** explicit confirmation connects
   saved goal/session to identified run in one immutable local record. Require
   explicitly selected storage outside repository history/operator sessions; modify
   no Think record, Seed Me ledger or SDLC decision. Revisions, relocation,
   retargeting and migration are deferred. This is not another execution/measurement ledger.

Within that root, relationship key is `(Seed Me session ID, globally unique SDLC run ID)`.
Identical text/seed/slug cannot collapse different keys. Same key/different frozen
identity conflicts. Same identity/locators returns existing reference without
writing; caller owns opening. Different locators refuse pending relocation support.
Uniqueness covers this root only. Conflicting/unreadable/unsupported records that
prevent proving uniqueness refuse creation; never skip possible duplicates or pick
newest.

Binding format has a version, not revisions/predecessors. Creation evidence is not
perpetual currentness:

| Field | Contract |
| --- | --- |
| Format version, job ID | Tinkery-owned; unsupported versions fail explicitly |
| Relationship key, locators, origin-node ID, slug | Source IDs prove identity; locations/slug only find it |
| Exact confirmed goal and UTF-8 SHA-256 | Compare whole block with ledger goal and settled origin answer, not reconstructed headline |
| Selected seed revision/content SHA-256 and run seed reference/digest | Actual run copy and declared digest must match selected seed at creation |
| Ledger version/revision/digest, run-record digest | Creation evidence, not later whole-file equality requirements |
| Optional Think locator/digest | Supplied record must agree on goal/session; absence doesn't prove no goal was saved |
| Creation time, explicit-link provenance | Retain actual source authority/action, invent no human or authenticated implementation permission |

**Write boundary.** Before confirmation show exact goal, session, seed revision
and run; destination in technical help. Capture reviewed sources, then reread IDs,
versions and byte digests; refuse changes/unavailability. Leaving writes nothing.
Require byte-equal ledger goal/settled origin answer, resolved seed confirmation,
verified supplied Think record, selected seed and run copy. Hash equality proves
bytes, not meaning/confirmer identity. Legacy runs without source identity remain
unbound. Entry point/root configuration still need review before I2 approval.

Atomically publish complete initial records with concurrent key uniqueness; no
partial validity or overwrite. Losing creators return valid matching reference or
conflict, never a second job. Missing destination refuses, never defaults to session
folders/history. Sources aren't a cross-tool atomic snapshot: final validation
refuses observed changes, records captured evidence and requires R5 revalidation;
publication cannot lock sources or prevent later changes. Selection/navigation/
copying/reads/retries never create or repair bindings.

**R5 read boundary.** Use stored locators only; check relationship IDs, origin/goal
and run's bound seed reference/digest against frozen identity. Changed ID under
same slug or changed goal/seed means conflict/outdated relationship, never borrowed
evidence. Missing/moved sources mean unavailable, not search/relocate/retarget.
Explain missing update/relocation routes and preserve bytes; repeated creation
cannot bypass refusal.

Ledger decisions/versions, stage/candidate progress and whole run digest changes
do not alone invalidate identity. Apply source-owned freshness to current checks/
decisions separately from creation evidence. Report newer source seeds as different,
never substitute for the bound copy. A still-matching copy remains historical seed.
Captured digests never prove current approval; unverifiable required identity stays
unknown despite other readable sources.

PR references must explicitly belong to the run; disagreements conflict. Checks
include repository, number and result revision, never permanent binding approval.
No PR means no result evidence, not success. Missing, unreadable, unsupported,
outdated and conflicting evidence remain distinct. Failed/interrupted execution
does not justify another binding.

**Completion tests:** temporary synthetic sources cover valid creation; identical
titles/seeds across sessions; same-slug/different runs; legacy ID absence; repeat/
concurrent creation; same-key conflicts; prepublication source changes; unsupported
formats; partial-write failure; missing destination; leaving without writing;
revision/relocation refusal preserving bytes; normal ledger/stage progress versus
changed run goal/seed. Reversion probes protect confirmation, source-change/ID
checks and duplicate prevention. Generate initial binding/linked-job fixtures for
R5; unlinked fixtures stay negative cases. I2 remains separately unapproved.

## Changes, conflicts and interruption

| Source event | Required interpretation |
| --- | --- |
| Seed Me evidence/decision changes | Version checks reject conflicts; dependents need reopening/revalidation. Preserve history/decision alongside concern; derived `review_flags` never erase or silently replace decisions |
| Original seed changes | SDLC binds its copy; original changes alone do not stale approvals, but rebinding/editing copy does. Name used copy/differences, never silently rebind or call it latest. [Test][D-seed-tests] |
| Plan/spec/brief/context/checklist changes | Stage input digests stale relevant gates, possibly later gates in a full run. Show first available review gate; retain old receipts as history |
| Candidate/check policy/test lock/manual receipt/log changes | Verification may stale; new failure replaces pass. Show source next step, never reuse prior green. [Tests][D-protocol-tests] |
| PR head/checkout or local/remote checks differ | Name revisions/sources; observations may both hold, never combine into green |
| GitHub review remains approved | Repository stale-dismissal rules/review commit identities aren't inspected. Report observation, not candidate approval. [Rules][G-protection] |
| Verification stops | `running` without active writer becomes `interrupted`; active writer stays running. Lock-file existence alone proves nothing. Offer external log/rerun step; navigation never reruns |
| Harness/application stops | Explicit checkout/run locates SDLC; list isolates invalid runs. Work file/handoff/tools bindings aid recovery, but no inspected ID resumes agents and active Seed Me session doesn't prove a worker is running. Reread selected records |
| Source missing/unparseable/unsupported | Keep other verified facts, name gap, offer one repair/refresh. Never fall back to older success |

Read times mean “read at,” not “unchanged since.” Collect sources separately and
validate binding plus each freshness rule. Refresh before consequential external
actions; identity changes require current-material review. No aggregate approved,
done or safe-to-resume flag.

## Attention and cost

See [historical source evidence](../research/shared/job-record-survey.md#attention-and-cost).

## Proposed R5: one explicit read

After design acceptance and implemented, reviewed, verified [prerequisites](#goal-to-run-identity-prerequisite),
R5 reads one existing binding. Shared reader owns selection, validation, bounded
collection and typed evidence. Phases import no other phase, parse no source
formats and recalculate no SDLC gates. Seed Me owns decisions, SDLC freshness,
GitHub remote observations.

Require no installed Substrate or `surface --json` service. Explicit legacy work/
tools references cannot replace binding. Port useful readers, not dashboard,
launcher, preparation pipeline or measurement service. Preserve source version,
identity/revision/read time and full decision source/reason/history for Approve/
Taste, never aggregate green.

Explicit reads disclose sources and execution/network access: Git/`gh` can contact
GitHub; selected SDLC status helpers execute checkout code. Read-only is not
execution permission. Only already-permitted helpers/locations; no automatic trust,
setup, transcript discovery, watcher or reader launch on navigation. Disallowed
sources stay unavailable. Tests use fakes/TEST roots, no network/providers.

Invalid/absent binding says “Goal connection unknown”, separates facts and is
required negative coverage, not successful R5 completion. Positive fixtures trace
exact goal → seed/separate decisions → identified run → matching checks/result.
Reopen performs no write/external retry; late results cannot overwrite another job
or reopen departed views.

Retention, lessons, binding creation and phase UI are separate slices. Reader
recovery cannot preserve deleted-worktree runs: report missing, leave retention to
explicit action. R6 expected-input approval is separate. Diagnostics 1–3, 6, 7:
shared ownership simplifies callers/exposes boundaries; no module/limit/score
changes. Design/prerequisite completion never approves R5 implementation.

## Proposed R6: external approval handoff

Show first pending decision's exact source/revision and permission. Keep distinct:
goal confirmation saves goal; seed confirmation accepts intake revision; SDLC stage
decision approves current inputs/requests changes; GitHub review/release belongs
to forge/operator.

One explicit copy handoff names checkout/run/stage/current evidence/source review
command. Copying neither runs nor approves. Operator decides externally with actual
reviewer/source/reason; refresh reads receipt. Never invent approval words or approve
goal/seed/stages/release together.

**Separate approval-binding gap:** inspected `decide` hashes inputs at execution,
accepting no expected digest of previously displayed documents. Inputs can change
after copy/reread. External rereview exists; guaranteed same-displayed-revision
approval does not. R6 needs separately approved upstream expected-digest rejection;
refresh before/after cannot supply it. [Writer][D-writer]

Outside reader scope: Seed Me `init`/`publish`/`seed confirm`/`end`/viewer startup;
SDLC `new`/`decide`/`mark`/`verify`/`lock-tests`/`stage` (can commit/create worktree);
Substrate `init`/`prepare`/`workflow`, ledger scheduling/updates/archives; GitHub
review/merge. SDLC `actions.*.allowed` is a precondition, never authorization.
R6 adds no hidden execution/release authority. Missing authentic review identity,
changed inputs or unresolved links return to external owner with reason.

## Evidence and checks

[Fixture notes](../../tests/fixtures/job-records/README.md) distinguish source-shaped
bytes from proposed display expectations. Current/missing/unreadable/outdated/failed/
interrupted/conflicting fixtures have no implemented binding. They establish
consistency, not production reading or real outcomes. Prerequisite tests above are
planned, not executed binding/reader/approval/live-trial work. Historical upstream
checks are not fresh validation.

[D-writer]: https://github.com/jon-devlapaz/tink-sdlc/blob/8ffa2bdf2743c8904c5eebd27c8ff2f45ec972a3/assets/_system/scripts/sdlc.py#L377-L402
[D-seed-tests]: https://github.com/jon-devlapaz/tink-sdlc/blob/8ffa2bdf2743c8904c5eebd27c8ff2f45ec972a3/tests/test_sdlc_stage.py#L257-L284
[D-protocol-tests]: https://github.com/jon-devlapaz/tink-sdlc/blob/8ffa2bdf2743c8904c5eebd27c8ff2f45ec972a3/tests/test_sdlc_protocol.py#L109-L180
[G-protection]: https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches/about-protected-branches
