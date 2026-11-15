# Change brief

Source: https://github.com/jon-devlapaz/tinkery/issues/860
Issue body sha256: 3a020ad4663fe4d6a106dbfba8e7a95651346a68d6ad6a6bdb3755f2e5921213
Title: Tend: find one useful connection and take its goal into Shape

The approved issue below is this run's brief, unchanged. Each Done-when line is one item in `checklist.json`.

---

### Primary area

tend

### Kind

enhancement

### Goal

Let Tend find a useful connection among my saved goals that I have not noticed. Show one possible next goal I can judge and take into Shape.
Keep the goal garden and source-linked wiki ambition. The first build covers one discovery and one safe handoff, not a wiki.

### Evidence

The operator said: "i don't want to have to pick the two goals myself. the point is for tend to find connections i haven't noticed."
The 14 November trial at `2c0c8f1` checked Tend at 100×30 and 80×24 using copied records and no provider calls.
Goal detail, available Think history, and read-only Shape handoff worked. No connection discovery was present.
The operator later retired the outdated goals. Pruning is no longer the first slice or a prerequisite.
The trial did not establish discovery usefulness or reduced human effort.

Source review found two integration gaps. Repository attribution currently comes from Think records; shared goal saving alone does not create one.
The existing saver retains its destination in memory, so restart-safe handoff needs a bounded shared extension.
The full specification below resolves both designs without changing code.

### Scope and boundaries

Add **Find connections** in Tend's repository view. Inspect all active goals in that scope without asking for a pair or search terms.
Disclose the goals, model, one-request cost, and provider transfer before explicit start.
Proposed bounds: 20 goals and 32 KiB of variable input. Refuse excess or incomplete input rather than silently omitting goals.
Fewer than two goals, ambiguous identities, or zero allowance make no call. Navigation and refresh make no calls.
Any active goal in the selected sessions root with unknown, invalid, or conflicting repository attribution makes discovery incomplete.
Check this before repository filtering. Refuse without a call rather than silently omit uncertain goals.
Verified other-repository goals and explicit no-repository goals do not block discovery.
Use one cancellable, tool-free request through the existing budget boundary, without automatic retries.

Return one source-backed proposal or **No useful connection found**. Explain the relationship, one inspectable result, and a useful judgment question.
Validate exact supporting quotations and source identities. Keep assumptions visible; the operator judges usefulness.
Offer **Prepare this goal**, **Dismiss**, and **Back**. Prepare opens editable wording for explicit confirmation, then saves through shared session code and opens Shape.
Preserve source goals and needs links. Neither drafting nor implementation starts on handoff.

Retain only the latest exploration per repository: frozen inputs, result, dismissal, and any saved destination.
Unchanged inputs reopen that result without another request. Changed inputs permit a fresh explicit pass.
Use one source-input definition for reuse and freshness through save and resume: included goal membership, exact text, checked identity, repository, and active status.
Unrelated Shape answers or whole-ledger version changes do not invalidate unchanged goal inputs. Incomplete discovery blocks a new request or handoff.
Do not accumulate dismissals or suppress proposals across older explorations. A later pass can rediscover a dismissed idea.
Keep unfinished handoff state until recovery establishes its outcome. Confirmed source evidence stays in the saved goal after exploration replacement.

**Repository attribution:** store typed Tend-origin metadata in the new goal's existing ledger evidence through the shared session owner.
It records the confirmation repository, exact goal, source references, and interpretation. Create no fake Think conversation or separate origin database.
Extend the common origin reader for Tend filtering, Shape's folder, and Approve's mismatch check. Keep existing Think-record rules for old goals.
Return invalid or conflicting Tend origins distinctly from missing legacy origins. Shape must refuse drafting without a fallback folder.
Approve must disable launch commands, and Tend must report incomplete discovery. Preserve existing legacy unknown-origin behavior.
The new goal must remain in its repository garden after restart.
History identifies a Tend-created goal honestly. Existing retry paths preserve creation origin without inheriting handoff ownership.

**Restart-safe saving:** persist one frozen confirmation receipt with a fixed destination before creating session storage.
The shared saver stages a complete validated session outside discovery, then publishes it without replacement on the same filesystem.
Resume uses that same receipt and destination. A published destination is verified and reused; conflicts refuse instead of overwriting or creating another.
Inspection after restart writes nothing. Explicit Resume completes pending writes. Cancel is allowed only when publication is proved absent.
The full specification below defines the interruption cases and ownership checks. No general transaction framework is included.

### Done when

- [ ] One explicit request selects its own source goals and returns one reviewable connection or no useful connection.
- [ ] Empty or oversized scope, incomplete reads, ambiguous identities, and zero allowance refuse without provider calls.
- [ ] Any active unknown, invalid, or conflicting repository origin blocks discovery, even when session reads are complete. Verified other-repository and explicit no-repository goals do not block it.
- [ ] Navigation, refresh, and unchanged-input reopening make no requests. Each started pass uses at most one request.
- [ ] Invalid references, malformed replies, cancellation, and failures preserve originals and existing Tend use.
- [ ] The latest result and dismissal survive reopening. Changed source inputs prevent a stale new handoff. No older dismissal history is required.
- [ ] Reuse and save checks use the same source-input definition. Unrelated Shape answers preserve reuse and handoff. Goal text, identity, repository, status, or membership changes invalidate them.
- [ ] Confirmed words and source evidence reach Shape unchanged. Opening Shape starts no drafting or delivery.
- [ ] The new goal remains in the same repository garden after restart; Shape uses that repository and Approve retains its mismatch protection.
- [ ] Old Think-origin attribution and legacy unknown-origin behavior remain unchanged. Tend-origin history invents no conversation.
- [ ] Invalid or conflicting Tend origins block discovery, refuse Shape drafting without fallback, and disable Approve launch commands.
- [ ] At each specified interruption point, restart and explicit resume produce at most one destination with the exact frozen goal and provenance.
- [ ] Concurrent attempts and conflicting paths cannot overwrite another session. Uncertain publication blocks another destination and unsafe cancellation.
- [ ] Public-interface tests use fake providers and TEST roots. Reversion probes cover consent, originals, attribution, and duplicate handoff.
- [ ] Full required checks and terminal journeys pass at 80×24 and 100×30, including color disabled.
- [ ] Report mechanical results separately from human discovery: what was newly revealed, whether it changed desired work, review effort, and cost.

### Context and dependencies

- [Yohaku](https://github.com/jon-devlapaz/tinkery/blob/main/docs/yohaku.md), [Tend and Shape](https://github.com/jon-devlapaz/tinkery/blob/main/docs/design/shape-and-handoff.md), and [architecture](https://github.com/jon-devlapaz/tinkery/blob/main/docs/design/architecture.md).
- Existing boundaries: `src/tend/`, `src/pi.rs`, `src/seed_me/save.rs`, `src/think_record/read/origins.rs`, and menu-owned navigation.
- Attribution consumers include `src/tend/discovery.rs`, `src/shape/step.rs`, and `src/approve.rs`.
- Tests: `tests/tend_terminal.py`, `tests/tend_repo_terminal.py`, existing save and origin tests, and `tests/architecture.rs`.
- #123 records the garden purpose; #127 introduced explicit needs. #852 remains separate pruning work, not a prerequisite.
- #853 proposes visible action controls. Follow its existing conventions where available without making a product-wide control redesign part of this issue.
- #820 defines a proposed human-effort baseline; this issue does not authorize that study.

Architecture review covers diagnostics 1, 2, 3, and 6. Stop if implementation requires broad history changes, migration, phase coupling, or a general transaction system.

### Open decisions and permissions

The operator authorized implementation of this reviewed scope, full checks, independent review, and a PR. Merging is not authorized.
Authorization: "yes, go ahead and have the worker implement #860. supervise it, run the checks, and get an independent review. open a PR but don't merge yet."
Repository attribution and restart-safe saving are specified below; these are chosen designs, not implemented or runtime-verified capabilities.
The 20-goal and 32 KiB limits remain adjustable engineering bounds. No broader architecture decision is deferred to implementation.
Provider budget remains zero. A real human trial requires separate authorization for goals, data use, participation, and calls.


---

<details>
<summary>Full goal garden specification</summary>

# Tend goal garden specification

Status: approved implementation scope, revised 14 November 2026. Implement, verify, independently review, and open a PR. Do not merge.
Application provider calls and a real usefulness trial remain unapproved.

## Purpose

Tend is a goal garden. Goals can remain unfinished, connect with other goals, and reveal useful work that the operator has not noticed.
The operator asks Tend to look. Tend finds a connection. The operator judges it. Shape develops the confirmed goal.
A source-linked wiki remains the longer-term ambition. Later experiments can test whether linked notes and corrections improve discovery beyond original-goal retrieval.

## Evidence

The 14 November trial at `2c0c8f1` exercised Tend at 100×30 and 80×24 with copied records and no provider calls.
Goal detail, available Think history, and read-only Shape handoff worked. Tend offered manual needs links but no connection discovery.
The historical garden contained 310 active goals. Of these, 294 shared three first-line titles; this did not establish duplicate full goals.
The operator subsequently retired all active goals. The action record reports 307 newly stopped goals and three archived legacy goals.
Pruning is no longer the first slice or a prerequisite. Human effort and useful connection discovery remain unmeasured.
Local evidence remains private. The issue publishes aggregate findings only.

## First slice

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

Do not accumulate dismissals, feed old decisions into later requests, or attempt suppression across different explorations.
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

Later experiments can add broader retrieval, linked wiki notes, and scoped correction reuse if they improve discoveries beyond simple retrieval.
Original words remain authoritative. Derived notes cannot become independent evidence for their own claims.
Background reflection, raw-idea capture, cross-repository discovery, and persistent dismissal history remain separate slices.
Research motivates the experiment but does not prove better goal selection: [Generative Agents](https://arxiv.org/abs/2304.03442),
[A-MEM](https://arxiv.org/abs/2502.12110), [Reflexion](https://arxiv.org/abs/2303.11366), and [Evaluating AGENTS.md v3](https://arxiv.org/abs/2602.11988v3).

This design belongs to [#860](https://github.com/jon-devlapaz/tinkery/issues/860). The operator authorized implementation, checks, independent review, and a PR. Do not merge.
Provider budget remains zero. A real trial requires separate data-use, participation, and call-budget authorization.

</details>

