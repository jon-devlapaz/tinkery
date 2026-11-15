# Change brief

Source: https://github.com/jon-devlapaz/tinkery/issues/862
Issue body sha256: 441b6b0ca6f6bd0c1ad3e7424f0984720e7a83d15090e041765837b34991d8b9
Title: Tend: save one correction and reuse it in the next relevant connection

The approved issue below is this run's brief, unchanged. Each Done-when line is one item in `checklist.json`.

---

Implementation authorized by the operator in this chat:
> okay have a worker implement #862. supervise it, run the checks, and get an independent review. open a PR but don't merge yet.

This authorization supersedes the draft-only status below. Scope remains the reviewed correction slice. No new application provider calls or real-goal access are authorized. PR #861 is merged. Do not merge the new PR.

# Tend remembers one correction for a later connection

Status: publication authorized. Implementation remains unapproved.
Labels: `area: tend`, `enhancement`, `needs-approval`.

## Goal

Let the operator correct one connection and see that correction improve the next relevant proposal, without choosing the source goals.
Test whether retaining human judgment adds value beyond finding links between goals.

## Evidence

The original Goal Engineering article places easy review and transferable human feedback at the center of goal design.
PR #861 makes one connection reviewable. Its Input contains goals and request settings; it does not contain corrections.
Dismissal stores a boolean. Editing a goal does not explain the operator's judgment or teach later requests.
The isolated trial found one useful dependency and rejected an unrelated set. It did not test learning.
The [garden spec](#updated-goal-garden-specification) owns the long-term ambition and first-slice boundaries.
An open-issue search for correction and feedback found #820, which concerns broader effort measurement. This draft is narrower.

## Scope and boundaries

### One explicit correction

Add an optional correction action while reviewing a connection proposal. It opens one text field.
A hint asks what must change and when it matters. Do not require a form, taxonomy, selected goal pair, or explanation for dismissal.
Example: “For booking reliability work, establish whether data was saved before proposing training material.”
Show the exact words and repository before explicit Save correction. Escape leaves the stored correction unchanged.
Goal editing, dismissal, and acceptance never silently become feedback. Saving a correction neither changes the goals nor confirms a new goal.

Keep one current correction per canonical repository, bounded to 2 KiB of UTF-8 text. Refuse excess without truncating.
The correction must stand on its own. The capture screen shows the proposal being reviewed, but this slice does not copy its old goal set into memory.
Keep operator words distinct from model interpretations. Do not summarize or expand the correction automatically.
Show the current correction with explicit Replace and Clear actions. Replacement discards the current reusable correction, not prior saved-goal evidence.
The correction survives reopening and replacement of the latest exploration. No correction history or separate memory browser is included.

### One ordinary discovery request

The next explicit Find connections request includes only the exact current correction alongside eligible current goals. It does not resend the old proposal or old source context.
Disclose the correction before sending it. Count the complete variable input within the existing 32 KiB limit; refuse rather than truncate.
Keep the 20-goal bound, exact quotations, existing allowance, and one tool-free request. Add no separate relevance call or automatic retry.
The model chooses goals as today. Instruct it to use the correction only for relevant work.
Keep the existing response shape: one connection proposal or null. Add no structured applicability report.
It can apply the correction to different goals about the same work. Matching original goal IDs is not required.
The correction is scoped guidance, not evidence that a factual claim is true and not permission to use tools or execute work.
Show the exact correction supplied with the result, labelled as operator guidance, not source evidence or proof of learning.
Judge relevance from the proposed work. Do not promise an identical response when a correction is unrelated.
No useful connection remains valid. The unrelated trial must check that the correction did not cause an inappropriate proposal.
Keep the correction available after an unrelated result. Do not consume it after one attempt or claim that the system has learned permanently.

### Preserve exact state and safe handoff

Include the exact correction revision in exploration input identity. Saving, replacing, or clearing it permits a new explicit pass with unchanged goals.
Unchanged goals and correction reuse the existing result without another call. A failed request preserves the previous result as outdated.
A correction change during a request or before new goal publication prevents use of the outdated result.
Use the existing repository lock. Refuse correction changes while a handoff is pending; explain how to resume or cancel first.
Already published goals keep their exact words and evidence when the reusable correction changes or is cleared.
Freeze the exact correction supplied with a confirmed goal's creation evidence. This records input, not whether the model followed it.
Opening Shape remains read-only. Saving feedback, confirming goals, confirming seeds, and granting implementation permission remain separate.

## Architecture and stopping rule

Keep correction capture, relevance disclosure, and input identity in the existing Tend connection boundary.
Use its existing repository lock and bounded atomic storage pattern. Do not add a general memory service or another session writer.
The proposed storage is one private, versioned correction record per repository, beside the latest exploration.
Publication must be atomic and restart-safe. Write failures before replacement leave the prior record intact.
If replacement succeeds but synchronization fails, report uncertain persistence and reread before another mutation. Do not claim a rollback occurred.
Existing explorations without corrections remain readable. Existing saved Tend origins retain their meaning.
A malformed correction must block new discovery with an explicit remedy, not silently disappear from the request.
It must not block opening a previously verified saved goal whose frozen evidence is valid. Provide an explicit Clear action for an invalid regular correction file under the repository lock; refuse unsafe paths.

Use the versioned record decision below. Existing readers deny unknown fields, so do not add new fields to version 1 records.
Name effects on architecture diagnostics 1, 2, 3, and 6: responsibility, interface size, isolation, and hidden implementation decisions.
Stop and re-scope if this needs a generic history rewrite, migrations of old goals, cross-phase imports, or a retrieval service.

## Saved correction and compatibility decision

This is a proposed format contract, not an implemented change. Keep the existing ledger envelope and shared save operation.

### Reusable correction

Store one private record per canonical repository beside its exploration: schema 1, repository, and correction.
Correction is either null after Clear, or an object with `revision` and `text`.
`revision` is a fresh opaque identity for each explicit Save or Replace. `text` contains the exact nonempty operator words, at most 2 KiB.
Clear atomically stores null rather than requiring a delete-and-recreate sequence. Treat a missing file as no correction.
No old proposal, goal set, model relevance report, source-context archive, or correction history is stored here.
Use the repository lock for all mutations. Replace validates the version shown to the operator so another writer's correction cannot be overwritten silently.

### Exploration records

Read the existing schema 1 strictly as before. Its input has no correction and normalizes to no correction for comparison.
Write new exploration results as schema 2. Schema 2 adds a required `correction` input member: null or the exact revision/text object supplied.
Keep all other input identity and response validation rules. The 32 KiB limit includes the serialized correction and current goals.
Version 1 rejects correction fields; version 2 rejects a missing member. Unknown versions refuse explicitly.
Opening an old record performs no migration. A legacy pending save stays version 1 through resume and completion.
A fresh request produces a new record in version 2 only after success; a failed request preserves the previous record.
Changes to the reusable correction invalidate an unsaved proposal through normal input identity checks, not by rewriting its frozen input.

### Confirmed goal evidence

Keep `tinkery:origin/tend-v1` and origin schema 1 exactly as #861 defines them.
A goal created without a correction continues to use that format, even when its exploration uses schema 2.
A goal created with a correction uses `tinkery:origin/tend-v2` and origin schema 2.
Version 2 contains the existing origin fields plus a required `correction` object with the exact revision and text from exploration input.
The correction is labelled operator guidance supplied during discovery. Do not put it inside the model's `interpretation` string.
It grants no authority and is not a supporting quotation from a goal.

The shared origin reader dispatches on the artifact tag and validates the matching schema and strict field set.
It accepts one valid v1 or v2 entry on the confirmed origin node. Duplicate versions, unknown tags, missing correction fields, and mismatched tags/schemas refuse.
Preserve the current exact-goal, source-quotation, repository, and identity checks for both versions.
History displays the frozen correction separately when present. Existing consumers still obtain repository attribution from the same shared reader.
No existing session needs migration or a new top-level ledger field.

### Freeze once, recover the same confirmation

At explicit goal Save, construct the appropriate versioned origin from the frozen exploration input, not the current mutable correction record.
Recheck that input against current goals and correction before creating the pending handoff and before publication.
The existing Pending receipt and staging ownership marker carry that complete origin, including version and correction when present.
Decode nested origins by their schema. Keep legacy receipt and marker serialization unchanged when resuming an old save.
The ledger evidence, receipt, and marker must agree on the exact version and correction. Missing data is not normalized away in version 2.
Use this same version-aware equality in exploration validation and destination inspection. Never attach a newer correction during resume.
Once a destination is verified as published, later changes to reusable feedback do not invalidate it or create a second goal.
Clearing or replacing reusable feedback never edits evidence already frozen in a confirmed goal.

### Compatibility boundary and tests

New readers must open and resume #861 data without rewriting or weakening its checks.
Old #861 readers cannot understand v2 explorations or corrected origins. They must refuse them; backward reading by the old binary is not promised.
A rollback must preserve those files and restore the supporting reader before using them. Do not strip correction evidence to make old code accept a goal.
Keep v1 fixture tests for saved, pending, and staged states. Add v2 equivalents and version/tag/correction mismatch refusal tests.
Test a v2 exploration with no correction publishing a v1 origin, and a corrected exploration publishing a v2 origin.
Test restart and later Replace/Clear against the frozen saved evidence. Test these through the existing public save and origin interfaces.
This changes one typed origin format, not the publication algorithm, ledger schema, or history system.

## Done when

- [ ] The operator explicitly saves one exact correction from a reviewed proposal. Restart preserves it without a call.
- [ ] Find connections discloses and sends the correction in one request without requiring a selected pair.
- [ ] The result shows the exact correction supplied. Its supporting goal quotations remain verifiable. The model response contract is unchanged.
- [ ] Replace and Clear change future request identity. Unchanged input reopens the same result without another call.
- [ ] Relevant feedback survives an unrelated result and can apply to different source goals in the same repository.
- [ ] Cancelled edits, oversized input, malformed records, conflicting writers, and interrupted saves preserve valid existing data.
- [ ] Correction changes cannot silently alter a pending or published goal. The confirmed goal retains the correction actually used.
- [ ] Older #861 explorations, pending receipts, ownership markers, and origin evidence remain readable and resumable. No-correction discovery and Shape handoff still work.
- [ ] Unknown versions, mismatched tags, missing correction data, and conflicting frozen copies refuse explicitly. Opening old data does not rewrite it.
- [ ] Public-interface tests cover these protections. Reverting the correction-input and freshness protections makes focused tests fail.
- [ ] Applicable full checks pass in TEST roots. Fake providers establish mechanics only.

## Usefulness trial

Use synthetic goals and a separately approved call allowance. Do not use operator goals by default.
Start with a real proposal, then capture a human correction verbatim. A same-goal retry is only a preliminary revision check.
Require a later request with different relevant goals, without selecting a pair. Judge whether its proposed work reflects the correction.
Do not count repetition of the correction as success. Include a different, unrelated goal set to detect inappropriate transfer. Record the exact correction, before/after proposals, request count, and review effort.
If feasible within the approved budget, compare the same input without the correction. Without that comparison, report improvement as observed, not causal proof.
A model's relevance statement or an agent's assessment cannot replace the operator's usefulness judgment.
Do not mark the learning hypothesis established solely because deterministic tests pass.

## Context and dependencies

This is a separate successor to #860 / PR #861. Do not append it to that PR.
Relevant code: `src/tend/connection.rs`, its panel and prompt, and `src/seed_me/connection.rs`.
Relevant checks: `tests/tend_connection_terminal.py` and shared origin/handoff tests.
Preserve the garden and wiki ambition. Multiple corrections, semantic search, automatic feedback extraction, cross-repository reuse, wiki generation, and background work are excluded.

## Open decisions and permissions

The behavior and bounds above are proposed, not implementation approval.
The compatible evidence format is decided below for this proposal. It still needs implementation review and tests when a build is authorized.
The operator must judge usefulness in a separately approved trial.
The user authorized publication with the updated spec. Implementation, merge, real-goal access, and new provider calls are not authorized. Keep PR #861 unchanged.


---

## Updated goal garden specification

# Tend goal garden specification

Status: the first slice is implemented in PR #861 and remains unmerged. Its approved scope is unchanged.
The feedback-learning extension below is proposed only. Documentation and issue publication are authorized; implementation is not.
The isolated #861 trial used its separately approved allowance of two real requests. That allowance is exhausted; further calls require a new budget.

## Purpose

Tend is a goal garden. Goals can remain unfinished, connect with other goals, and reveal useful work that the operator has not noticed.
The operator asks Tend to look. Tend finds a connection. The operator judges it. Shape develops the confirmed goal.
A source-linked wiki remains the longer-term ambition. It can connect goals, evidence, and explicit corrections so later proposals benefit from human judgment.
The garden must make useful work emerge without requiring the operator to choose goal pairs.
The learning ambition is to make that judgment easy to give and reuse across relevant work, not merely to accumulate links.

## Evidence

The 14 November trial at `2c0c8f1` exercised Tend at 100×30 and 80×24 with copied records and no provider calls.
Goal detail, available Think history, and read-only Shape handoff worked. Tend offered manual needs links but no connection discovery.
The historical garden contained 310 active goals. Of these, 294 shared three first-line titles; this did not establish duplicate full goals.
The operator subsequently retired all active goals. The action record reports 307 newly stopped goals and three archived legacy goals.
Pruning is no longer the first slice or a prerequisite. Human effort and useful connection discovery remain unmeasured.
Local evidence remains private. The issue publishes aggregate findings only.

## First slice

This section remains the bounded #860 / #861 contract. The proposed feedback slice does not expand this PR.

Find one useful connection across active goals in the current repository, then take its confirmed proposed goal into Shape.
Goals enter through Think as today. The operator does not choose pairs, label relationships, or supply search terms.

### Start and scope

In the repository view, **Find connections** reviews the included goal count, selected model, one-request cost, and provider disclosure.
The full source goals remain available to inspect. Navigation and refresh make no calls or exploration writes.
The proposed bounds are 20 active goals and 32 KiB of serialized variable input. The fixed prompt is separately versioned.
These are engineering bounds, not research-derived thresholds. Include full goal text and stable identities, not first-line labels alone.
With fewer than two goals, incomplete discovery, ambiguous identities, or excess input, refuse clearly without calling a provider.
Discovery is incomplete if any active goal in the selected sessions root has unknown, invalid, or conflicting repository attribution.
Check repository attribution as well as session readability before filtering. Refuse without a call rather than silently excluding uncertain goals.
Goals with a verified different repository or an explicit no-repository origin do not block discovery.
Never truncate or silently select a subset. Other garden scopes explain the repository-only restriction.
An explicit start permits one tool-free request through the existing drafting allowance. Zero allowance refuses it.
Cancellation remains visible. No repository tools, external search, automatic retries, or background work are included.

### Result and review

Return one connection proposal or **No useful connection found**.
A proposal cites at least two included goals and quotes their supporting text. It explains a relationship beyond shared vocabulary.
Include one proposed goal, a bounded inspectable result, a judgment question, and any assumptions or conflicts.
Check response size, structure, source identities, and exact quotations before displaying a usable proposal.
Those checks establish traceability, not truth or usefulness. Source material is data, not instructions or permission.
Recheck the source inputs after the request and before saving, using the input identity defined below.
Changed or missing inputs make the result outdated and prevent a new handoff.

Offer **Prepare this goal**, **Dismiss**, and **Back**, with source goals available.
Back retains the result. Dismiss records the decision without requiring a reason.
Prepare opens editable wording. An explicit save confirms the exact displayed words and the disclosed repository.
The menu opens Shape only after the shared save boundary verifies the destination.
Neither opening Shape nor confirming a goal starts drafting or delivery. Existing seed confirmation and approval controls remain unchanged.

## Minimal exploration memory

Keep only the latest exploration per repository, with its frozen inputs, request settings, result, dismissal state, and saved destination.
Input identity includes repository scope, the complete included goal set, each checked unique source identity and repository, and exact goal content and active status.
It also includes model settings and prompt version. Use this same source-input definition for result reuse and freshness checks through save and resume.
Whole-ledger versions are not source-input identity. Unrelated Shape answers or ledger updates must not invalidate unchanged goal inputs.
Any recorded ledger versions are historical evidence only. Changed goal text, identity, repository, status, or included goal membership requires a fresh explicit pass.
Incomplete discovery blocks a new request or handoff, even when the last readable inputs match.
Ordering alone does not change it. Unchanged input reopens the saved result without another call, including dismissal and no-connection results.
Changed input permits a fresh explicit pass. Its completed result replaces the previous exploration; a failed pass preserves the previous record as stale.
Allow only one active pass or pending handoff per repository. Another instance reads the existing state or refuses while its writer is active.

Within the first slice, do not accumulate dismissals, feed old decisions into later requests, or attempt suppression across different explorations.
A later pass can rediscover a dismissed idea. State this limit. Same-input repetition protection remains deterministic.
After a confirmed goal is saved, its source evidence stays with that goal even when the exploration record is replaced.
An unfinished or uncertain handoff blocks replacement until recovery establishes its outcome.
Interrupted requests require explicit retry and another budget slot. Exactly-once provider execution after a crash is not promised.

## Repository attribution decision

Current repository attribution comes from Think records. Calling `seed_me::Config::confirm_as` alone does not write such a record.
Do not create an empty or invented Think conversation to make Tend-born goals visible.

Add a typed, versioned goal-origin entry within the new goal's existing ledger evidence. The shared `seed_me` boundary writes and validates it.
This uses the existing evidence structure; it adds no top-level ledger field, separate origin database, or migration of old sessions.
The entry records `entry: tend`, the canonical launch repository, exact confirmed goal, source identities and frozen source inputs, and the reviewed connection as an interpretation.
Keep source quotations verbatim and distinct from the newly confirmed goal. Preserve the existing caller-declared confirmation attribution.
The repository is where the goal was confirmed, not a model guess about its subject.

Extend the shared origin-reading boundary so all consumers receive the same checked origin.
For Tend-origin goals, validate the entry, the exact goal, and unique live session identity.
Duplicate, malformed, or conflicting Tend-origin evidence must return an explicit invalid-origin result, distinct from missing legacy origin.
If a Think record also claims an origin, a mismatch or ambiguous match returns that invalid result. Neither source silently overrides the other.
Preserve this distinction through Tend, Shape, and Approve. Do not convert invalid Tend origins into legacy `Unknown` or a missing record.
For goals without the new entry, retain today's Think-record rules and refusal behavior. No broad extraction or renaming of history modules is required.

Use that common result in Tend filtering, Shape's folder selection, and Approve's existing repository mismatch check.
A valid Tend-origin goal must remain in the repository garden after restart. Shape must use that repository rather than an unrelated fallback.
If its recorded repository cannot be used or its new origin is invalid, Shape must refuse drafting without selecting a fallback folder.
Approve must disable launch commands for invalid or conflicting Tend origins. Tend must report incomplete discovery, not silently omit them.
Explain the refusal and its remedy. Preserve existing behavior for older goals without Tend-origin evidence, including legacy unknown origins.
History must say **Created from a Tend connection** and must not imply that a Think conversation occurred. Source details remain available.
When an existing retry carries the same goal forward, preserve its creation origin and evidence. Creation origin does not bind it to one session folder.
Handoff ownership is separate from origin evidence and is never inherited by later attempts.

## Restart-safe saving decision

The existing goal saver retains retry state in memory. Extend its shared saving boundary with one narrow prepare-and-resume operation.
Keep ledger validation and exact-word checks there. Tend must not implement a second ledger writer.

1. On explicit Save, lock the repository exploration record and freeze the confirmed words, actor, repository, and source evidence.
2. Allocate one handoff ID and one destination session identity. Persist a `saving` receipt before creating any session directory.
3. The shared saver builds the complete goal and its origin evidence in a private staging directory outside session discovery.
4. Recheck source inputs under the same freshness rule. Validate and read back the staged goal, then publish its directory without replacing the fixed destination.
5. Read back the destination and atomically mark the exploration `saved`. Only then offer or open Shape.

The staging directory and destination must use the same filesystem. Refuse unsupported publication conditions; do not fall back to partial copies.
Persist file contents and parent-directory updates at each required boundary before reporting success.
The receipt identifies both the staging location and destination. Validate paths against the configured session root and reject symlink or ownership conflicts.
The staged session carries a shared-saver ownership marker bound to the handoff ID, session identity, and frozen confirmation.

After interruption, **Resume saving** uses the existing receipt, never a new identity or the current editor contents.
If only the receipt exists, recreate its staging state. If staging exists, validate and finish that same destination.
If publication succeeded but the exploration update failed, verify the existing destination and record it as saved.
If a path belongs to another operation or the content conflicts, refuse without replacing or deleting it.
If either location is unreadable, preserve the uncertain state and prevent another destination from being created.
An interrupted staging write is rebuilt only inside its proven owned staging directory from the frozen receipt.
A partial unowned directory is a conflict, not permission to remove it.

Reopening after a pending save performs inspection only. An explicit Resume performs any remaining writes; it uses no model call.
Once the destination is verified, repeated Save opens it. Later source changes do not revoke an already confirmed goal or create a replacement.
A source-input change or incomplete discovery before publication blocks publication. Preserve the pending receipt and explain recovery instead of creating another goal.
An explicit **Cancel pending save** can clear a not-yet-published operation after proving the destination absent and removing only its owned staging directory.
If publication is uncertain, cancellation must refuse. The confirmed destination always remains intact.

This is one exploration record plus one bounded session-publication operation. It is not a general transaction framework or new workflow engine.

## Architecture and stopping rule

Tend owns proposal review and the latest exploration. The existing model boundary owns budgets and cancellation.
Shared session code owns provenance validation and prepare/resume saving. The shared origin reader serves Tend, Shape, and Approve.
The menu alone connects Tend to Shape. Existing needs links and readiness meanings stay unchanged.
Normal Think saves retain their current path. Existing data requires no migration. A failure in exploration must leave ordinary Tend use available.

Review diagnostics 1, 2, 3, and 6: clear responsibility, small interfaces, isolated changes, and hidden implementation decisions.
The two bounded shared changes above are explicit design requirements, not claims that those capabilities already exist.
If implementation needs a generic history rewrite, existing-session migration, phase-to-phase imports, or a general transaction manager, stop and report the scope change.

## Verification and human trial

Use public interfaces, fake providers, and temporary TEST roots. Run required full checks and terminal journeys at 80×24 and 100×30, including color disabled.
Cover empty/one/multiple goals, repeated titles with distinct identities, limits, incomplete reads, invalid quotations, and no-connection output.
Cover zero budget, cancellation, failure, same-input reopening, dismissal, and changed-source-input invalidation.
A readable active goal with unknown repository attribution must block discovery without a provider call, even when session reads are complete.
Verified other-repository and explicit no-repository goals must not block discovery.
Unrelated Shape answers must leave result reuse and handoff valid when source inputs remain unchanged.
Changes to goal text, identity, repository, active status, or included goal membership must invalidate reuse and prevent a stale handoff.
Assert exact goal preservation, repository visibility after restart, Shape's recorded folder, and Approve's repository mismatch refusal.
Check old Think-origin behavior and honest Tend-origin history.
Drive malformed and conflicting Tend-origin evidence through each consumer: Tend blocks discovery, Shape refuses drafting, and Approve disables launch commands.
Check that legacy unknown-origin behavior remains unchanged. No invalid Tend-origin result can become a legacy unknown result.

Inject interruption after receipt persistence, during staging, after staging validation, after directory publication, and before the saved-state update.
Restart and resume at each point. Establish one destination with exact confirmed words and provenance, without another provider call.
Cover concurrent saves, conflicting destination content, source changes before publication, cancel of a proved unpublished save, and unreadable recovery state.
Reversion probes must catch broken consent, changed originals, lost attribution, and duplicate handoff.

A separately authorized human trial uses several new goals without pre-labelled connections or a preselected pair.
The operator must be able to identify what the connection revealed and whether it changed the work they want to pursue.
A valid citation or a fluent summary is not evidence of discovery. Record rejection and no-connection outcomes as well as accepted proposals.
Record active review effort and provider cost. One useful example does not prove a general benefit.

## Later ambition and authority

The original Goal Engineering article argues for work scopes that make human feedback easy to give and useful across similar tasks.
#861 applies the review part: one proposal, evidence, assumptions, and a judgment question. It does not learn from edits or dismissals.
The garden's longer-term wiki can preserve original corrections, their source context, and evidence of where reuse helped or failed.
Human words remain authoritative. Model interpretations remain guesses. Derived notes cannot become independent evidence for their own claims.
A saved preference is neither a universal rule nor permission to act. Organize later learning around reviewable work, not repository membership alone.
Repository boundaries remain the first experiment's data boundary, not a claim about the best long-term learning scope.

### Observed #861 trial

At `10fa0a0`, two real requests used invented goals and isolated TEST storage.
Tend selected a useful dependency between reproducing a booking failure and writing volunteer guidance.
The exact confirmed goal reached Shape and survived reopening without another request. A separate unrelated set returned no connection.
All eight original source ledgers remained unchanged. The testing agent judged the first connection useful.
This does not establish operator acceptance, surprising discoveries from natural goals, reduced review effort, or learning from feedback.
The detailed trial report remains local. The aggregate observations above are the published evidence; no operator goals were used.

### Proposed next experiment

Save one explicit correction in the current repository and reuse it in the next relevant connection request.
The operator states the correction in their own words while reviewing a proposal. Saving it makes no provider call.
Tend retains one self-contained correction separately from the replaceable latest exploration. It does not retain or replay the old goal set.
An explicit later Find connections request includes only that correction and current goals. The same request proposes at most one connection.
The prompt asks the model to apply the correction only when relevant. The existing response format stays unchanged.
The operator still does not select a goal pair. A correction can describe a kind of work and transfer to different goals in the same repository.
Show the exact correction supplied. Do not require a model applicability report or treat one as proof of learning.
Check the proposed work for inappropriate transfer. Do not promise an identical result when a correction is unrelated.
No useful connection remains a valid result.

Success means the next relevant proposal reflects the correction in a way the operator can inspect and judge useful.
A repeated phrase, valid JSON, or a model claim that it learned does not establish success.
Require transfer to different relevant goals and an unrelated case in a separately authorized trial.
A retry on the same goals tests revision only. Compare the proposed work, not fluency alone.
The detailed proposed contract is [the next slice contract](#tend-remembers-one-correction-for-a-later-connection), including the authoritative format decision.
New readers accept existing v1 records and strict v2 records. Corrected goals freeze the exact revision and words in v2 creation evidence.
Goals created without corrections keep v1 origin evidence. Existing pending saves resume in their original format without migration.
The same frozen evidence travels through the pending receipt, staging marker, and saved goal. Later feedback changes never rewrite it.
Older binaries refuse new v2 records. Compatibility means new code reads old data; it does not mean old code can read new data.

Broader retrieval, multiple corrections, automatic summaries, wiki pages, background reflection, and cross-repository learning remain later experiments.
Research motivates experiments but does not prove better goal selection: [Generative Agents](https://arxiv.org/abs/2304.03442),
[A-MEM](https://arxiv.org/abs/2502.12110), [Reflexion](https://arxiv.org/abs/2303.11366), and [Evaluating AGENTS.md v3](https://arxiv.org/abs/2602.11988v3).

The first design belongs to [#860](https://github.com/jon-devlapaz/tinkery/issues/860), implemented in [#861](https://github.com/jon-devlapaz/tinkery/pull/861).
Do not merge or change #861. Publication of the next slice is authorized. Do not build it. No new provider allowance or real-goal access is granted.

