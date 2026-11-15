# Change brief

Source: https://github.com/jon-devlapaz/tinkery/issues/868
Issue body sha256: a8d41f64c4c92be5a4ee3d7c306dc6951d71e7d7f9e02fe06b1b2a6de32e5e8a
Title: Think: Save an unfinished thought without a model call

The approved issue below is this run's brief, unchanged. Each Done-when line is one item in `checklist.json`.

---

# Think: Save an unfinished thought without a model call

Status: published for review; `needs-approval`. Implementation is not authorized.

### Primary area

think

### Kind

enhancement

### Goal

Think is the single entry point for thoughts and intents. An operator can save unfinished words directly from its initial input without interpretation or goal confirmation.

### Evidence

On 2026-11-15, the operator reported capturing two thoughts in Tend, then questioned direct capture there:

> i also think the single entry point for thoughts and intents inot tend has to be through think.... it feels weird adding stuff in tend directly

The operator confirmed:

> yes, Think should be the single entry point, with saving a thought requiring no model call. update the spec and #866 to reflect that, keep capture and display as separate slices, and don't start building yet.

Independent source review found that Think already owns an unsent `Note` input. After submission, the same field also carries answers and additions.
The current thought writer belongs to Tend's reader/editor object. A bounded storage extraction avoids a second writer or a dependency on Tend screen state.
No new operator trial establishes the proposed Think action's usability yet.

### Scope and boundaries

One outcome: save the exact fresh Think input as a thought, then open the repository garden.

**Eligible words and states**

- Save only the current input buffer in a fresh, unsubmitted Think visit. Include an existing thought opened through `g` as an unsent copy.
- Exclude conversation history, generated readings, focused answers, and goal-review or confirmation text. A visit with a submitted turn is outside this slice, even if the input later becomes empty.
- Disable thought saving during voice capture, model work, goal review, and goal saving. Explain unavailability when the action is shown.
- Preserve exact bytes, including whitespace, Unicode, and final line breaks. Keep the existing empty-input and size rules.

**Visible action and return**

- Show a separate **Save thought and open Tend** action beside the initial input, with a disclosed key and destination repository. It cannot be available only through F1.
- Keep F2 Send and existing goal confirmation unchanged. The action makes no model call, consumes no allowance, and creates no goal or seed.
- On confirmed durable save or a verified exact duplicate, open the repository garden and refresh its thought view.
- On refusal or uncertain persistence, retain the complete draft in Think and explain the result. Do not navigate as if saving succeeded.
- Cancellation makes no write. A successful save must not lead to a false unsaved-work warning.
- If the repository is missing or invalid, explain the destination problem and keep the words. Never choose another repository implicitly.

**Routes and storage**

- Route Tend's `t` shortcut to a fresh Think visit. Remove its direct-capture editor path. Opening Think makes no write or model call.
- Preserve `g` as an unsent copy of the selected thought. Changing that copy cannot silently edit its original. Saving different words creates a new thought under the existing duplicate rule.
- Extract one repository-bound thought store from the current Tend screen object. Keep Think's input editor and Tend's existing reader.
- Preserve storage names, repository binding, exact-byte duplicate identity, limits, private file permissions, atomic publication, and refusal behavior.
- Return a structured save outcome for saved, duplicate, refused, and uncertain persistence states. Each screen supplies its own user message.
- Keep old thoughts readable without migration or read-time writes. No session or goal-origin format changes.

Subtract first: reuse the input field, thought store behavior, and menu routing. Remove the redundant Tend capture path rather than adding another capture mode.
Review architecture diagnostics 1, 2, 3, and 6: clear ownership, small interfaces, isolated changes, and hidden implementation details.
Stop and report if this needs a broad Think rewrite, a storage migration, a new service, or shared goal-saver changes.

Excluded: #866's mixed garden display, editing/deleting old thoughts, timestamps, ordering, model input changes, readiness classification, wiki features, and correction-context storage.

### Done when

- [ ] A fresh Think visit visibly offers the separate thought-save action and names its repository before saving, at 80×24 and 100×30 with colour off.
- [ ] The exact input saves as one thought and opens Tend after success. Restart reads the same bytes. No goal, confirmation record, or seed is created.
- [ ] A configured provider receives zero requests for saving, duplicate saving, navigation, cancellation, and refusal. Cover zero allowance and an available allowance.
- [ ] A thought opened through `g` can be saved unchanged without duplication. Editing and saving its copy preserves the original thought.
- [ ] Submitted visits, answer entry, voice capture, model work, goal review, and goal saving cannot accidentally invoke thought saving.
- [ ] F2 Send, answer submission, and exact goal confirmation retain their existing behavior in guarded tests with fake providers.
- [ ] Empty or oversized input, invalid repository, unsafe storage, write refusal, and uncertain persistence retain the input and do not report confirmed success.
- [ ] A durable save or duplicate returns to Tend without an unsaved-work warning. Existing thoughts remain unchanged on reads and failed saves.
- [ ] Tend `t` opens Think without writing. The direct Tend capture editor is no longer reachable. Reader navigation and `g` still work.
- [ ] Reversion probes detect changed saved words, an unintended provider call, and an unintended goal confirmation. Run required repository checks with temporary TEST roots.
- [ ] After separate implementation approval, the operator tries saving and reopening a real thought. Record their judgment separately from mechanics.

### Context and dependencies

- #864/#865 provide shipped thought storage, reading, and unsent Think handoff.
- #866 covers thoughts alongside goals in the garden. It is a separate slice, issue, approval, and PR.
- This slice can return to the currently shipped Tend reader/summary surface. It does not require #866 to ship first.
- If both are implemented, coordinate shared Tend/menu edits. Neither issue authorizes the other's scope or parallel implementation.
- [Yohaku](https://github.com/jon-devlapaz/tinkery/blob/main/docs/yohaku.md)
- [Think design](https://github.com/jon-devlapaz/tinkery/blob/main/docs/design/think.md)
- [Tend and handoff](https://github.com/jon-devlapaz/tinkery/blob/main/docs/design/shape-and-handoff.md)
- Local goal-garden spec: `spec.md`, section “Separate proposed capture slice”.

The open backlog was checked before drafting. #866 owns display, not this capture relocation; no matching separate capture issue was found.

### Open decisions and permissions

The operator accepted Think-only entry and model-free thought saving, and authorized publication with the updated spec.
The precise shortcut and placement must fit the existing input flow and remain visible; do not change F2 Send or add an undisclosed action.
Publication is authorized. Implementation, provider calls, real-data changes, and merge are not authorized. Keep needs-approval.


---

## Updated goal-garden specification

Snapshot attached at publication. Historical permissions and proposals below retain their original context; the issue scope above governs this slice.

<details>
<summary>Full updated specification</summary>

# Tend goal garden specification

Status: updated 2026-11-15 after both bounded slices shipped on `main`.
[#861](https://github.com/jon-devlapaz/tinkery/pull/861) merged as `1d14afc`.
[#863](https://github.com/jon-devlapaz/tinkery/pull/863) merged as `04c2bc4` and closed #862.
“Shipped” means merged repository behavior. This does not claim an operator installation was updated.
The source-context storage proposal is deferred. Synthetic comparisons are stopped.
The first real-backlog trial is complete. The correction barely changed the proposed work.
The first operator capture session exposed a navigation obstacle: thoughts felt separate from the garden.
The proposed next slice is now #866: show thoughts alongside goals and use the existing reader as their detail view.
The hill-climb plan (local artifact: `hill-climb.md#agreed-sequence`) owns the simpler sequence accepted on 2026-11-15.
This update authorizes no publication, implementation, or application data changes.
The synthetic budgets and both one-request real-work allowances are exhausted. This update uses no provider calls.

## Purpose

Tend is a goal garden. Goals can remain unfinished, connect with other goals, and reveal useful work that the operator has not noticed.
The operator asks Tend to look. Tend finds a connection. The operator judges it. Shape develops the confirmed goal.
Accepted entry design: Think captures thoughts and intents; Tend holds and connects them. Model-free thought saving in Think is proposed, not shipped.
A source-linked wiki remains the longer-term ambition. It can connect goals, evidence, and explicit corrections so later proposals benefit from human judgment.
The garden must make useful work emerge without requiring the operator to choose goal pairs.
The learning ambition is to make that judgment easy to give and reuse across relevant work, not merely to accumulate links.

## Evidence

The 14 November trial at `2c0c8f1` exercised Tend at 100×30 and 80×24 with copied records and no provider calls.
Goal detail, available Think history, and read-only Shape handoff worked. Tend offered manual needs links but no connection discovery.
The historical garden contained 310 active goals. Of these, 294 shared three first-line titles; this did not establish duplicate full goals.
The operator subsequently retired all active goals. The action record reports 307 newly stopped goals and three archived legacy goals.
Pruning is no longer the first slice or a prerequisite. Useful synthetic examples are now observed. Human effort remains unmeasured.
Original operator data remains private. The PRs contain synthetic trial evidence only.

## Shipped first slice: find one connection

This section records the bounded #860 / #861 contract. The second slice adds one correction without changing this handoff.

Find one useful connection across active goals in the current repository, then take its confirmed proposed goal into Shape.
Goals enter through Think as today. The operator does not choose pairs, label relationships, or supply search terms.

### Start and scope

In the repository view, **Find connections** reviews the included goal count, selected model, one-request cost, and provider disclosure.
The full source goals remain available to inspect. Navigation and refresh make no calls or exploration writes.
The shipped bounds are 20 active goals and 32 KiB of serialized variable input. The fixed prompt is separately versioned.
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
The two bounded shared changes above are shipped. The repository design contract owns their maintained implementation details.
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
The trial report is saved locally (local artifact: `../tinkery-861-dogfood-2026-11-14/report.md`).

### Shipped second slice: reuse one correction

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
The [published second-slice contract](https://github.com/jon-devlapaz/tinkery/issues/862) records the approved scope.
The [repository design](https://github.com/jon-devlapaz/tinkery/blob/04c2bc409124b4b45d97907ee02716943fe68106/docs/design/shape-and-handoff.md#one-saved-correction) owns the shipped correction and format decisions.
The local feedback draft (local artifact: `next-slice-feedback-draft.md`) is historical design material, not an unbuilt next slice.
New readers accept existing v1 records and strict v2 records. Corrected goals freeze the exact revision and words in v2 creation evidence.
Goals created without corrections keep v1 origin evidence. Existing pending saves resume in their original format without migration.
The same frozen evidence travels through the pending receipt, staging marker, and saved goal. Later feedback changes never rewrite it.
Older binaries refuse new v2 records. Compatibility means new code reads old data; it does not mean old code can read new data.

Broader retrieval, multiple corrections, automatic summaries, wiki pages, background reflection, and cross-repository learning remain later experiments.
Research motivates experiments but does not prove better goal selection: [Generative Agents](https://arxiv.org/abs/2304.03442),
[A-MEM](https://arxiv.org/abs/2502.12110), [Reflexion](https://arxiv.org/abs/2303.11366), and [Evaluating AGENTS.md v3](https://arxiv.org/abs/2602.11988v3).

The first design belongs to [#860](https://github.com/jon-devlapaz/tinkery/issues/860), implemented in [#861](https://github.com/jon-devlapaz/tinkery/pull/861).
Both slices are now merged. This specification update grants no further publication, build, provider, or real-goal access permission.


## What the second trial established

At `ba6606f`, the real terminal saved one correction and reopened it after restart.
The correction came from a replayed library-booking proposal from the earlier synthetic trial.
Three authorized real requests then tested different tool-reservation goals and an unrelated community-event case.
All inputs were synthetic. The testing agent wrote the correction. This was not feedback from the operator's own work.

| Case | Observed result | Meaning |
| --- | --- | --- |
| Different relevant goals without correction | A reservation diagnostic paired with volunteer instructions | The result bundled two deliverables. |
| Same relevant goals with correction | One diagnostic with observations for loss, display failure, or uncertainty | The result followed the requested smaller scope. |
| Unrelated goals with correction | A shared information sheet for an event poster and website | The result stayed on task without forced reliability advice. |

The baseline already distinguished data loss from a display problem. The improvement was narrower scope and more explicit observations.
Identical relevant source arrays made the comparison useful. One sample per condition does not establish repeatable causality.
Exact correction transfer, restart persistence, unchanged source ledgers, and retention after the unrelated request were checked.
No real goals changed. No model request ran during correction saving or navigation.

Evidence: local trial report (local artifact: `../tinkery-863-dogfood-2026-11-14/report.md`) and [published inputs, outputs, and terminal captures](https://github.com/jon-devlapaz/tinkery/pull/863).
The existing Linux limit for rapidly queued clear, large paste, and Save remains documented in the PR.
The dogfood setup also needed a visibility-wait correction in its script. Neither finding establishes a new saving defect.

## Position against the garden and wiki ambition

| Ambition | Current position | Remaining gap |
| --- | --- | --- |
| Goals connect without pair selection | Shipped, within one repository and the stated input bounds | Natural-goal surprise and usefulness remain unmeasured. |
| Useful work emerges into Shape | Shipped, with explicit review, exact confirmation, and restart-safe saving | Autonomous readiness detection is not present. |
| Reopening does not repeat the same work | Shipped for unchanged exploration inputs and confirmed handoffs | Changed explorations can rediscover dismissed ideas. |
| Human judgment improves later proposals | One explicit correction is reused; useful transfer was observed once | No automatic learning, correction collection, or measured operator benefit exists. |
| Memory has inspectable sources | Saved goals retain creation evidence and any supplied correction | The reusable correction does not retain the original proposal that prompted it. |
| A source-linked LLM wiki supports the garden | Not shipped | No wiki pages, retrieval, synthesis, or linked evidence collection exists. |
| The garden works in the background | Not shipped | Current discovery always requires an explicit request and budget. |

The action-potential analogy describes the ambition, not the implementation.
Today the operator triggers one pass and judges one proposal. There is no background process that detects readiness.
The existing repeat-protection rules are narrower than general idempotence across changing goals.
A wiki can later make context durable and inspectable. Its notes must remain interpretations tied to original evidence.
Neither a wiki nor a saved preference grants execution permission.

## Deferred proposal: inspect the proposal reviewed with a correction

Status: deferred by the operator on 2026-11-15. This is not the next build.
The operator found context useful in the booking example and chose Replace. That judgment remains recorded below.
One synthetic example does not establish a general need for another storage format.
The contrasting synthetic comparisons are stopped. Their unanswered cases provide no outcome evidence.

Revisit this proposal only if real work reveals a correction that is hard to judge without the proposal reviewed when it was saved.
First inspect that concrete problem and consider a smaller remedy. The design below is retained for reference, not accepted implementation scope.
The garden and source-linked wiki remain the ambition. Neither requires this storage change now.

### Bounded behavior, if justified

Keep one correction per repository. Use the label “Proposal reviewed when this correction was saved.”
This records what was displayed, not a claim about what caused the operator's words.
Bind each snapshot to the saved correction revision. The snapshot must survive restart and replacement of the latest exploration.

For a save or replacement with a reviewed proposal, capture that displayed proposal and its cited source goals.
Check the displayed proposal is still the same stored proposal before committing its context.
If replacement has no reviewed proposal, save the new correction with no context. Keep replacement available.
Do not inherit the old revision's context or invent a new source for revised words.
Show “Source context was not recorded” for old corrections or replacements without context.

Preserve exact correction words, source goal words, and quotations. Label model interpretations separately.
Use the existing correction review area to inspect saved context. Distinguish it from the latest proposal during replacement.
Inspection performs no writes and makes no provider call. Replace and Clear retain their explicit meanings.
Do not create a correction archive or replay old context in a model request.

### Proposed local storage boundary

Tend's correction store owns one versioned local envelope. The shared Correction value remains revision plus exact text.
A proposed schema-2 envelope contains the repository, current correction or null, and optional context bound to that correction's revision.
The context contains the reviewed proposal and only its cited Source records, with exact identities, repository, status, and goal words.
Exclude uncited goals, the full exploration record, prior corrections, request settings, save receipts, staging data, and saved destinations.
Validate citation identities and exact quotes against those captured source goals.

Proposed bound: 128 KiB for the complete serialized schema-2 correction file, checked before any replacement.
This is an engineering ceiling, not a measured user need. The existing request and response limits are each 32 KiB.
Verify serialized escaping and boundary cases before implementation acceptance. Refuse excess size without truncation or silent omission.
An oversized proposed save must leave the old correction intact and the draft available.

New readers accept strict schema 1 under its existing 16 KiB limit, with no context, and strict schema 2 under its new limit.
Reading old records performs no migration or write. Old binaries refuse schema 2; preserve the file and restore a supporting reader on rollback.
Missing optional context is valid. A malformed or revision-mismatched context is an explicit invalid-record error, not silent absence.
Preserve existing recovery through explicit Clear for malformed regular files. Unsafe paths still refuse.
Malformed feedback must not hide a previously verified saved goal.

Save correction and context together through the existing atomic replacement and repository lock.
Preserve stale-edit checks and the pending-handoff block. Pre-replacement failures retain the old file.
A parent-sync failure reports uncertain persistence and requires rereading, as today.

Keep local context outside model inputs, the shared Correction type, saved-goal origins, and pending goal receipts.
Current goals plus exact correction words and revision remain the only discovery input. Do not change goal-origin formats.
Existing saved goals and pending handoffs retain their exact evidence and recovery behavior.
If this requires shared saver changes, old-session migration, or another storage service, re-scope before implementation.

### Fit, verification, and stopping rule

Tend already owns correction storage and proposal review. Use these owners without introducing another phase or service.
Review architecture diagnostics 1, 2, 3, and 6: responsibility, interface size, isolated changes, and hidden storage details.
The intended benefit is that memory inspection can change without changing model requests or goal publication.

If approved, save a correction on proposal A, complete exploration B, restart, and inspect A from the current correction.
Check exact source words, proposal text, and revision association with no extra provider call.
Cover replacement with a new proposal, replacement without a proposal, old records, stale proposals, and Clear.
Cover byte limits, malformed context, revision mismatch, stale edits, and interrupted writes.
Confirm existing request and goal-saving contracts remain unchanged.

No issue is published and no build starts from this candidate. Multiple corrections, retrieval, automatic summaries,
background reflection, and wiki generation remain outside it.

## Observed no-build comparison

Completed artifact inspection on 2026-11-15. This was not a fresh model trial or a blinded human comparison.
The agent had already seen these artifacts. The comparison cannot measure an independent change in that agent's decision.
The original correction was authored by the testing agent for synthetic work.
The operator later accepted narrowing it in this comparison. This does not establish a universal preference for all reliability work.

**Correction alone:**

> For reliability work, stop at a diagnostic result before proposing training or instructions.
> State what observation would distinguish lost data from a display problem.
> Do not bundle the diagnosis and guidance into one goal.

**Added original context:**

The cited goals reported that a booking disappeared after refresh and volunteers could not tell whether it was saved.
The reviewed proposal was:

> Document one booking-and-refresh reproduction and use its observations to draft a short saved-booking verification checklist.

Its assumption was:

> A booking disappearing from view does not by itself prove that it was not saved.

This shows a proposal bundling diagnosis and instructions while the underlying failure remained uncertain.
It does not prove that every reliability task must stop before guidance.

| Choice | What the available evidence supports |
| --- | --- |
| Keep | The correction already expresses a clear diagnostic-first preference. Later relevant reuse followed it. |
| Replace | Original context suggests a narrower scope: uncertainty about saved data, rather than all reliability work. The agent recommended this option; the operator subsequently chose it. |
| Clear | The artifacts show no demonstrated harmful transfer. They provide no clear reason to discard the correction entirely. |

**Replacement direction accepted; application correction unchanged:**

> When saved data appears to disappear and the cause is uncertain, first propose one diagnostic result.
> State what evidence would distinguish data loss from a display problem, or leave the result inconclusive.
> Keep that diagnostic separate from training or instructions until the uncertainty is resolved.

The later reservation result separated diagnosis from guidance and specified loss, display failure, or uncertainty.
The unrelated event result stayed on task. These support retaining useful guidance, but do not test the proposed replacement.

Operator judgment on 2026-11-15:

> yeah, the context helps. replace is the right call. update the spec with that judgment, but keep the storage change as a proposal for now. don't publish or build yet.

Additional constraint:

> also make sure we arent overfitting

Observed: the operator found this context useful and chose Replace over Keep or Clear.
Accepted direction: narrow the guidance to uncertain data loss instead of all reliability work.
The wording above remains a documented example, not a stored application preference or a tested new model input.
No application correction was changed. The original synthetic evidence remains intact.

### Limits and checks against overfitting

This is one synthetic scenario, with related booking and reservation examples. Those examples are close variants, not broad domain coverage.
The agent wrote the original correction, selected the comparison, and recommended Replace before the operator judged it.
The comparison was not blinded. Agreement establishes usefulness here, not independent evidence of a general improvement.
The unrelated event case tested inappropriate correction transfer. It did not test the usefulness of stored context.
The narrower replacement has not been tested in a new connection request.

Do not encode diagnostic-first behavior or this replacement text as product policy.
The proposed feature must preserve arbitrary operator corrections and their reviewed context, without domain-specific rules.
Context is historical evidence about what was reviewed. It does not prove that a correction still applies.

The planned contrasting synthetic comparisons were stopped at the operator's request on 2026-11-15.
Three correction-only questions were presented. No case judgments or before-and-after comparison results were collected.
Do not treat those cases as passed, failed, or evidence for a storage change.
Further learning must come from the shipped feature in actual work, without selecting tasks to reproduce the booking example.
Record useful results, irrelevant guidance, unchanged decisions, and cases where no correction is needed.
Do not turn one correction or one successful example into a general product rule.

Sources: exact correction and trial plan (local artifact: `../tinkery-863-dogfood-2026-11-14/plan.json`),
original proposal (local artifact: `../tinkery-861-dogfood-2026-11-14/response-1.txt`),
relevant reuse (local artifact: `../tinkery-863-dogfood-2026-11-14/response-2.txt`), and
unrelated reuse (local artifact: `../tinkery-863-dogfood-2026-11-14/response-3.txt`).

## Observed real-backlog trial: limited improvement

On 2026-11-15, the operator approved four real backlog goals from #854, #853, #828, and #820.
The agent saved their exact displayed words and source references, then linked the same sessions to the Tinkery repository.
No duplicates were created. These were agent-drafted summaries accepted by the operator, not spontaneous unassisted goal captures.
The #854/#853 dependency was already documented and disclosed before the trial.

Two separately authorized requests used the shipped Find connections flow, Pi, `openai-codex/gpt-6.1-sol`, and medium thinking.
The operator did not select a pair. Both requests selected #854 and #853.

| Request | Proposed next work | Inspectable result |
| --- | --- | --- |
| Without correction | Define a joint action map for goal review and Shape entry. | A table of visible actions, transitions, confirmation checks, scope, and cost. |
| With correction | Review separate revise, confirm, and exit scenarios for those flows. | A scenario table of substantially the same actions and checks. |

The operator rejected the first proposal as an already-known connection and supplied this correction:

> don't just restate a dependency; show how the connection changes what we do next.

The app saved those exact words. The next request included the exact saved revision and text.
Checked request files contain identical source arrays, repository, model settings, harness, and prompt identity.
The baseline had no correction. No synthetic diagnostic-first correction was carried into this work.

The second proposal described a more specific review method. It did not establish a materially different next action from the known backlog.
The operator subsequently described the correction as barely changing the proposal.
This is evidence of limited usefulness in this case, not proof that all correction reuse fails.
One response per condition cannot separate the correction's effect from ordinary response variation.
Same-goal revision also does not establish transfer to different relevant goals.

Both responses retained valid source citations. Neither was accepted or saved as a new goal.
Existing goal words and session identities stayed unchanged. No implementation started.
The first exploration stayed in temporary storage. Its record was retained in the live Tend store when the operator authorized saving feedback.
The saved correction and second exploration now remain available there. This spec update changes neither.

### What this changes in our understanding

Exact correction delivery worked. A storage failure did not explain the weak result.
The known relationship was easy to select from this small set. The record does not establish why the model selected it again.
The trial used brief goal summaries, without tools, repository inspection, or full issue bodies in the model input.
It therefore does not establish that the model understood everything already known about these issues.
Do not infer that extra context, a stronger prompt, a different model, or a wiki would solve this result.
No missing historical correction context obstructed the operator's judgment. The storage proposal remains deferred.

A connection can be useful without selecting a novel pair. The test is whether it changes an actual decision or next action.
A new table format or more specific wording alone does not establish that change.
No useful connection remains a valid result. Do not keep requesting proposals until one sounds acceptable.

Evidence: first request (local artifact: `../tinkery-real-garden-goals-2026-11-15/first-connection/request.json`),
first response (local artifact: `../tinkery-real-garden-goals-2026-11-15/first-connection/response.txt`),
exact saved correction (local artifact: `../tinkery-real-garden-goals-2026-11-15/corrected-connection/saved-correction.json`),
corrected request (local artifact: `../tinkery-real-garden-goals-2026-11-15/corrected-connection/request.json`), and
corrected response (local artifact: `../tinkery-real-garden-goals-2026-11-15/corrected-connection/response.txt`).
These local real-work artifacts are not authorized for publication by this update.

## Earlier option: inspect one actual #854 decision

This option is superseded as the garden’s next step. It remains possible separate #854 work, not a garden prerequisite.
Proposed action: inspect the current seed-review screen at the point where revision text can be confused with confirmation.
Use an existing permitted draft in an isolated copy. Make no drafting request, confirmation, or product change.
If a usable draft is unavailable, report that limit rather than generate a replacement example or request a model draft.

Keep this to one decision: can the current screen make Revise and Confirm clear without a wider action-menu redesign?
Record the displayed actions, the known problem, and the smallest change the observation supports.
If the inspection only confirms what #854 already records, say so. Do not present that confirmation as a new connection.
Do not produce the model's broad table merely to complete its proposal.

This is ordinary work on the existing backlog problem. It is not acceptance of either connection proposal or permission to implement #854.
The inspection is proposed here, not performed by this documentation update.
Keep the saved correction unchanged for later naturally occurring work. Do not revise it repeatedly to force a preferred test answer.
When another real connection request is warranted and separately budgeted, judge whether it changes the work before considering new infrastructure.

For later review, ask: what would we do without this proposal, and what specifically changes because of it?
If there is no useful change, leave it unaccepted. This is a review criterion, not a new prompt rule or product mechanism.

## Decision after the shipped slices

Outcome: the mechanics work, but this first real-backlog correction produced only a modest change in proposed work.
Evidence: identical goals and settings, exact correction delivery, the same selected pair, and similar proposed deliverables.
Learning: useful correction reuse is not established for this case. More stored history is not supported as the remedy.
Decision: retain the garden and wiki ambition, defer source-context storage, and leave synthetic comparisons stopped.
Next: follow the agreed capture-first sequence in the hill-climb plan (local artifact: `hill-climb.md#agreed-sequence`).
The #854 inspection is separate work. No model calls, publication, or build are authorized by this update.

## Agreed approach to further garden investment

Updated 2026-11-15 after explicit operator acceptance of the simpler sequence.
The hill-climb plan (local artifact: `hill-climb.md#agreed-sequence`) owns the steps and the reassessment rules (local artifact: `hill-climb.md#continue-adjust-or-pause`).
This replaces the earlier proposed five-goal trigger, fixed waiting periods, and automatic stop after three unsuccessful requests.
It also removes the requirement that no existing link be documented among the entries.
Those were proposed learning gates, not validated thresholds. Existing engineering limits and permission boundaries remain unchanged.

Start with capture, finding, and reuse during one real work session. Repair only a concrete obstacle, not all of #866 by default.
The operator judges whether the material warrants a discovery attempt without selecting a pair or knowing the answer first.
Thoughts are part of that material but are not yet part of the model input.
If warranted, propose a bounded thought-input change before attempting discovery over thoughts.
A confirmed-goal-only opportunity can use the existing flow under a separate budget.

A known connection can be useful if it changes a decision or next action. Novelty alone is not the target.
A failed trial can reveal an obstacle without proving product value. Repeated weak results require reassessment, not automatic permanent abandonment.
Readiness, refactoring, and the wiki remain later options rather than a committed build queue.
Source-context storage remains deferred, and synthetic usefulness comparisons remain stopped.

This alignment changes documentation only. #866 remains unapproved. No issue publication, build, model request, or application-data change follows automatically.

### Operator goal and entry decision

Recorded 2026-11-15. The operator's words are exact. The readings are the agent's guesses.

> i just want a place that i can saturate my ideas/goals and thoughts in that are project specific and I can allow LLM to find surfaced connections, refactoring and determination if a goal(s) are ready for shaping. thats my main goal. I think idempotency is an idea that is relevant

> think of an entry to tend be a neuron, and with enough neurons a action potential activates over those neuron connections, and then its refined and insulated better through shape like myelination and then then the action is tink-sdlc which is like the behavior..

Question put to the operator: must Tend keep a raw thought before you confirm any goal from it?
Operator answer: "Yes, raw thoughts".

Question put to the operator: where must a raw thought enter Tend?
Historical operator answer: "Key in Tend garden".
This entry decision is superseded by the later Think-only entry decision below. The historical answer remains unchanged.

These answers are product decisions. They grant no publication, build, budget, or data permission.
The operator authorized publication on 2026-11-15, with three accepted defaults: no edit or removal in this slice,
project means the launch repository, and a duplicate means the same exact bytes.
Initially published as [#864](https://github.com/jon-devlapaz/tinkery/issues/864) with `needs-approval`; implementation was not authorized at publication.
The local draft is retained (local artifact: `../tinkery-garden-discovery-2026-11-15/issue-draft-thought-capture.md`).
The operator approved #864. [#865](https://github.com/jon-devlapaz/tinkery/pull/865) merged it as `69cdb3a` on 2026-11-15 after two independent review rounds.
"Shipped" means merged repository behavior. At merge, no operator installation update or operator use was established. Later installed-binary checks and the operator trial are recorded below.
An agent dogfood on `69cdb3a` with zero provider calls found the core flow working and seven points of friction.
The operator authorized publication of the follow-up as [#866](https://github.com/jon-devlapaz/tinkery/issues/866) with `needs-approval`. It is not approved.
Run records are archived at `~/dev/archive/tinkery-864-thought-run-2026-11-15.tgz`.
The hill-climb document (local artifact: `hill-climb.md`) now holds the simpler sequence accepted by the operator on 2026-11-15. It grants no build permission.
Reading: an entry can be smaller than a goal, and saving it must not record a confirmed goal.
The later plan alignment accepts thoughts as material for judging discovery opportunities. It removes the Think-only trigger.
The shipped discovery input still contains confirmed goals only.

Later the same day the operator described a wider model. These words are exploratory, with the operator's own "maybe". They are not a decision.

> wait so the think now really captures 'intent' and putting in thoughts happens direclty in tend, and maybe their are two categories of entities in tend, thoughts and intents, and a goal is born out of those with the process of agents recommending them or finding connections or surfacing them. the goal is then engineered properly and passed into shape to be grounded in reality before entering into the sdlc... this is proving to be radically interesting to me

Reading: what Tend stores as a goal today is an intent in this model. The proposal that Find connections prepares for confirmation is the goal.
Reading: the shipped path from proposal to confirmed goal to Shape already has this form. The thought-capture slice is the same under both models.
Question put to the operator: can one clear intent go straight to Shape without an agent proposal?
Operator answer: "Yes, direct path".
Reading: today's route from a Think-confirmed entry to Shape stays. An agent proposal is a second route to a goal, not a required gate.
Not decided: where "engineered properly" happens, and whether Tend's stored "goal" is renamed "intent".
Evidence limit: the real-backlog trial did not establish useful discovery. The agreed reassessment rules now apply.

Read-only discovery by two Pi workers found three gaps, in this order: entry, volume, then judgment and quietness.
Reports: code (local artifact: `../tinkery-garden-discovery-2026-11-15/report-code.md`) and design (local artifact: `../tinkery-garden-discovery-2026-11-15/report-design.md`).
Checked fact: `tinkery seed new` saves text as a confirmed goal with no repository attribution.
One such active goal makes Find connections refuse in every repository. It is not a safe capture path.

### Research basis and its limits

Added 2026-11-15 from web search summaries. The full papers were not read. Check each claim against its paper before relying on it.
These sources motivate the two records above. They do not establish learning thresholds, and none tests connection-finding across one person's goals.

- Later outcome over review acceptance: in [The Ideation–Execution Gap](https://arxiv.org/abs/2506.20803v1), 43 experts executed assigned ideas.
  LLM ideas scored higher before execution and fell more after it. The study covers NLP research ideas and 2025 models.
- False alarms and silence: [Proactive Agent](https://arxiv.org/pdf/2410.12361) and [PersonalAlign](https://arxiv.org/pdf/2601.09636) report high false-alarm rates.
  Models offer help when the user needs none. [PRISM](https://arxiv.org/html/2602.01532v1) treats unneeded intervention as a cost to weigh.
- No repeated requests: [Can LLMs Generate Novel Research Ideas?](https://arxiv.org/pdf/2409.04109) reports low diversity in repeated generation.
- One operator-written correction: [Learning on the Job](https://www.alphaxiv.org/abs/2607.22157.md) reports that corrections taught agents more than verdicts, with simulated users.
  [Evaluating AGENTS.md](https://arxiv.org/abs/2602.11988) reports that LLM-written context lowered task success and raised cost.
- Wiki deferral: the search found practitioner posts on LLM-maintained wikis and no controlled comparison.
- Goal engineering: the term appears in [one blog post](https://cobusgreyling.substack.com/p/goal-engineering). A workshop paper argues for
  [goals as first-class abstractions](https://www.seanrintel.com/publications/goals-as-first-class-abstractions-in-human-ai-collaboration.html). Neither is a controlled study.


## Operator capture trial and narrowed #866

Recorded 2026-11-15. The installed binary passed isolated capture, restart, find, and unsent Think-handoff checks with zero provider calls.
These were mechanical checks, not a substitute for the operator trial.
The operator opened the repository garden and reported:

> okay i did, added 2

The operator then reported:

> the UX feels weird.

And clarified:

> the thoughts feel separate from the garden. i expected to see them alongside the goals, not go into a separate reader. can we make that feel more natural without redesigning everything?

Observed through operator report: two thoughts were captured, but the separate reader did not match the expected garden experience.
Successful finding and useful reuse were not established. No private thought text was inspected or published for this update.
This is a concrete navigation obstacle, not evidence that capture lacks editing, timestamps, or more sophisticated memory.

The operator accepted a smaller design: thoughts alongside goals, ordinary arrow-key selection and Enter, and the existing reader as thought detail.
Keep an explicit thought label and the unsent `g` handoff to Think. Preserve exact stored words and existing goal operations.
Direct `t` capture is temporary shipped behavior; moving capture into Think belongs to the separate slice below.
Thoughts remain separate from confirmed goals in meaning, storage, needs links, readiness, Shape access, and model input.

[#866](https://github.com/jon-devlapaz/tinkery/issues/866) now owns the bounded acceptance criteria for this display and navigation change.
Its earlier editing, set-aside, timestamp, time-ordering, and unrelated message fixes are deferred rather than bundled into this slice.
The garden layout and thought format remain in place. No new storage service, goal-saver change, or broad graph rewrite is authorized.

This is step 2 of the agreed hill climb: address the obstacle observed during real use.
After a separately approved implementation, resume finding and reusing one real thought before expanding scope.
The garden and wiki ambition remains. Source-context storage and synthetic comparisons stay deferred or stopped as previously recorded.
Issue and documentation updates are authorized. #866 remains needs-approval; no build or provider call is authorized.


## Accepted entry design: Think captures, Tend holds and connects

Recorded 2026-11-15. The operator said:

> i also think the single entry point for thoughts and intents inot tend has to be through think.... it feels weird adding stuff in tend directly

The operator confirmed:

> yes, Think should be the single entry point, with saving a thought requiring no model call. update the spec and #866 to reflect that, keep capture and display as separate slices, and don't start building yet.

This supersedes the earlier direct-Tend-capture decision. It does not rename persisted goals to intents or introduce another entity type.
The garden and wiki ambition remains. No goal engineering, readiness, or model step becomes mandatory for saving unfinished words.

### Display slice: #866

Show thoughts alongside goals in the repository garden. Use the existing reader as thought detail.
The [issue](https://github.com/jon-devlapaz/tinkery/issues/866) owns implementation acceptance criteria.
Independent review requires identity-safe mixed selection, goal-only link actions, and return to the last viewed thought.
Keep layout and storage intact. Thought identities must not become goal session identities.
This slice does not move capture or add a new Think action. Existing capture is a temporary way to populate the view until the separate capture slice ships.
Testing display must not turn the current `t` editor into a permanent product requirement.

### Separate proposed capture slice: save a thought from Think without a model

Status: revised after independent review. The Think capture issue above owns detailed acceptance criteria. Publication is authorized; implementation is not.

**Save only fresh, unsent words.** Use the existing Think input in an unsubmitted visit, including a thought opened through `g`.
Save that buffer verbatim. Do not combine conversation history, save a generated reading, or treat an answer as a standalone thought.
Disable thought saving during voice capture, model work, goal review, and goal saving. Visits with submitted turns are outside this first slice.
This restriction avoids another capture mode and keeps the source of the saved words unambiguous.

**Show a separate Save thought and open Tend action.** Place it beside the initial input, with its key and destination repository visible.
Keep F2 Send and the existing goal-confirmation path unchanged. Saving makes no provider call and remains available at zero allowance.
On durable save or a verified exact duplicate, open the repository garden and refresh its thought view.
On refusal or uncertain persistence, keep the complete draft in Think and explain the outcome. Cancellation saves nothing.
A successful save must not trigger an unsaved-work warning. A missing repository must not cause an implicit destination choice.

Route Tend's `t` shortcut into a fresh Think visit, with no save or model call. Remove the direct Tend capture editor path.
Preserve `g` as an unsent copy. Editing that copy never silently replaces the original thought.
Saving different words uses the existing new-thought rule; saving identical words reuses the existing entry.

**Extract storage, not Tend's screen.** Move repository-bound thought persistence into one small shared store.
Keep Think's existing input editor and Tend's reader. Do not duplicate the writer or make Think depend on Tend's view state.
Preserve filenames, byte limits, exact-byte identity, private permissions, atomic publication, and existing refusal behavior.
Use structured save outcomes so each screen can distinguish saved, duplicate, refused, and uncertain persistence states.
No format change, migration, goal session, goal-origin change, or shared goal-saver change is proposed.
If this requires a broad Think rewrite or a new storage service, stop and report the scope.

The issue draft requires exact-word, duplicate, state-availability, refusal, and navigation checks with isolated TEST roots.
It also requires zero provider calls with a provider configured, no goal creation, and unchanged F2 Send and confirmation behavior.
The precise key and placement remain bounded implementation details, subject to visible-action verification at narrow terminal sizes.

Capture and display have separate implementation scopes, approvals, and PRs. Neither is approved for implementation by this update.
Capture can return to today's Tend surface; #866 need not ship first. Shared menu changes require coordination, not combined scope.
#866 remains needs-approval and unchanged by this update. The capture slice is published separately with needs-approval.
No provider allowance, real-data action, or implementation permission is granted. Source-context storage remains deferred and synthetic comparisons remain stopped.

</details>

