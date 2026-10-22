# Think dogfood review index — October 22, 2026

**Start here.** This is the durable index for this joint dogfood session, not a second
backlog or a claim that all findings are bugs. The [interaction proposal](../docs/research/think/interaction-proposal.md)
is the single proposed design for operator review. [#339](https://github.com/jon-devlapaz/tinkery/issues/339)
authorizes documentation consolidation only, not a new interaction implementation.
The later operator instruction permits verified current WIP to merge; merging these
documents does not accept the proposed interaction.

## Authority and review status

| Material | Home and meaning |
| --- | --- |
| Safeguards | [Yohaku](../docs/yohaku.md); unchanged by this review. |
| Shipped behavior | [Think](../docs/design/think.md), [goal block](../docs/design/goal-block.md), and merged PRs below. |
| Published task scope and live status | Linked GitHub issues/PRs; this index is a dated snapshot. |
| Proposed interaction and unresolved decisions | [One proposal](../docs/research/think/interaction-proposal.md); not accepted and not build permission. |
| Other unpublished work | [Parked backlog](../docs/research/shared/backlog.md#think); do not create a parallel specification here. |

New interaction implementation is paused in this thread. The operator subsequently
directed all verified current WIP to finish and merge under **Codex chat Evaluate
Think design and quality**, the master supervisor. **Codex chat Research drafted-goal
answer hints** owns #330/#332 and is finishing the approved Ctrl-R slice. This
review neither blocks nor expands that existing approval. Other phases' work stays
with its owners. This session owns #339 documentation only; a future ghost-text/Tab
or layout slice still needs explicit design and implementation approval.

## Shipped fixes

| Finding | Issue / merged PR | What is fixed and verified |
| --- | --- | --- |
| 22: clipped active question | [#319](https://github.com/jon-devlapaz/tinkery/issues/319) / [#320](https://github.com/jon-devlapaz/tinkery/pull/320), merge `2faf4ff` | Question height follows wrapped content; overflow has PgUp/PgDn and wheel access. Six new fake-Pi PTY journeys cover the exact question and longer overflow at 80×24, 100×30, 82×51; input/goal/original preservation and resize/selection checks pass. Old-renderer reversion fails. |
| 17: false experience-absence report | [#322](https://github.com/jon-devlapaz/tinkery/issues/322) / [#328](https://github.com/jon-devlapaz/tinkery/pull/328), merge `5568cc8` | The legacy `BoardData.outcome` holds Done when, not desired experience. Why/copy now say Success criteria / Not yet captured. Rejected summaries use unaccepted raw `parts.done_when`, retaining raw evidence. Eight new fake-Pi journeys cover absent/present criteria, colour/no-colour, 80×24/100×30; reversion fails. No new experience extraction. |

Both fixes passed local full zero-call verification, rustfmt/Clippy and Ubuntu/macOS
CI. #320 recorded 792 Rust tests passed / 1 ignored, 36 Node tests and 35 Python
entrypoints; #328 recorded 794 / 1 ignored, 36 Node and 36 Python entrypoints. Each
Python run included 74 unittest cases plus standalone journeys. These are historical
counts for those revisions, not current-main test totals. Rebuilt Herdr replays
verified the specific rendering changes; no operator visual acceptance or model-quality
improvement is inferred from those replays.

The first #328 macOS attempt exceeded an unchanged five-second Shape discovery test
wait. Investigation found no changed executed-path files; 30 local repetitions
passed (120 tests, 0.03–0.07s per suite). One bounded retry of the unchanged commit
passed the suite in 0.06s and full CI. The precise original slowdown cause remains
unproven; no timeout/code change was made to obtain green. See the
[recorded investigation](https://github.com/jon-devlapaz/tinkery/issues/322#issuecomment-6332157649).

## Active work — not shipped or design acceptance

Snapshot at main `44fec5285bea467602d40592637df6cb0f75d1c6`, checked October 22.
Follow the links for changes after this snapshot; do not maintain another live board here.

| Work | Verified status | Boundary for this review |
| --- | --- | --- |
| [#330](https://github.com/jon-devlapaz/tinkery/issues/330): editable suggested answers | Approved implementation/zero-call verification; issue open. [PR #332](https://github.com/jon-devlapaz/tinkery/pull/332) open at `246e8cf`; describes a separate suggestion/reason and **Ctrl-R** to copy into an empty draft. | Not merged at audit. Live evaluation remains deferred with zero budget under that issue. Reuse its suggestion/provenance boundary; do not build another recommendation system. |
| Operator's inline ghost-text / **Tab** request | Requested in this session's mockup, not approved for implementation. | Differs from #332 and existing Tab navigation. [Decision D2](../docs/research/think/interaction-proposal.md#approval-decisions) is a future design choice, not a blocker or replacement for the approved Ctrl-R slice. |
| [#109](https://github.com/jon-devlapaz/tinkery/issues/109): goal-quality/effort baseline | Related published research scope; consult its live issue before work. | Not permission to tune prompts or spend calls as part of the layout proposal. |

Related process cleanup [#336](https://github.com/jon-devlapaz/tinkery/issues/336) /
[#337](https://github.com/jon-devlapaz/tinkery/pull/337) merged separately. It is not a
layout decision or evidence that the remaining UX findings are fixed. Measure,
Shape and Approve follow-ups remain in their own issues/board, outside this review.

## Remaining findings — complete observation crosswalk

The numbers preserve the original session IDs. **Proposed** means a design direction
or observed friction awaiting a scoped decision; **Investigate** means evidence is
insufficient to claim a defect. Only 17 and 22 are marked shipped.

| ID | Observation / evidence qualification | Status and next home |
| --- | --- | --- |
| 1 | Think/Tend/Shape/Taste vocabulary is hard to predict; assistant assessment, not individually endorsed operator feedback. | Proposed: menu review later, not this layout slice. |
| 2 | Numbered menu may imply a sequence; shortcut purpose not inspected. | Investigate before removing numbering. |
| 3 | Selected menu description is distant from its choice. | Proposed: menu review later. |
| 4 | “tinkery in tinkery” ambiguously repeats product/directory names. | Proposed: menu context clarity later. |
| 5 | Seven destinations appear on an empty trial; their empty states were not tested. | Investigate; do not claim they are broken/unavailable. |
| 6 | Initial practice screen did not identify simulated replies before writing. | Proposed: accurate mode disclosure in the interaction proposal. |
| 7 | Persistent voice warning plus advertised speak action dominated the opening. Voice was deliberately disabled. | Proposed: clutter/setup review later; not an operator-install failure. |
| 8 | Operator needed an explanation of “declared: agent (not verified)”. | Proposed: clearer attribution copy; preserve identity/authorization safeguards. |
| 9 | Visible footer omitted editing guidance; operator confirmed caret/focus were clear. | Investigate help/conventions before adding hints. |
| 10 | Practice response did not engage with the sample idea. | Proposed: practice purpose/disclosure; not real-model quality evidence. |
| 11 | Practice fixture used awkward “I try Simulated sensemaking…” language. | Proposed: fixture copy later; not a live reading. |
| 12 | Practice fixture's generic question offered little direction. | Proposed: fixture purpose later. |
| 13 | `--shape-budget` controls Shape, not a launch-enforced Think cap. No overrun observed. | Investigate budget UX/enforcement separately. |
| 14 | First live response reported missing Done when but offered a generic revision field rather than a focused question. A relevant question emerged later. | Proposed: next-action clarity. Review with gaps is allowed; do not impose a questionnaire. |
| 15 | Long repeated Must checklist felt dense and weakly prioritized. | Proposed: heading/layout review; semantic consolidation is separate. |
| 16 | “I find the beginning / smallest satisfying…” reads unnaturally. | Proposed: goal-language/intent quality, not silent layout rewriting. |
| 17 | F5 called supplied experience absent because its field actually meant Done when. | **Shipped:** #322/#328 above. |
| 18 | Review showed Done when but not explicitly undecided scope/engine/art/realism. | Proposed: review context; retention on disk was not tested. Not every unknown must block. |
| 19 | “Work out…” and “without choosing the mechanics for me” appeared as game Must constraints. | Proposed: semantic distinction between instructions, discovery tasks and product requirements; separate from layout. |
| 20 | F3 said “You are confirming” while merely reviewing, then showed an agent launch-flag blocker. | Proposed: review wording; save refusal itself was correct. |
| 21 | Why mixed useful approaches with request IDs/gate diagnostics and “one: not found on board”. | Proposed: diagnostic disclosure; final phrase is not proof of a semantic failure. |
| 22 | Active satisfaction question was cut off above the answer field; F3 was needed to recover the tail. | **Shipped:** #319/#320 above. |
| 23 | Done when appeared populated and also “Still open”, without explaining refinement versus absence. | Proposed: honest state wording, not a demand to force closure. |
| 24 | Operator found “anything to add or change?” a detached fixture competing with review. | Proposed: question/answer adjacency and contextual revision input. |
| 25 | Operator disliked bottom-positioned thinking; that transient state was not captured. Initial-call status was captured at the top. | Investigate the reported state; proposed progress placement must cover initial/refining calls. |
| 26 | Operator wanted more noticeable titles and understandable sections. Plain-text captures do not prove exact visual contrast. | Proposed: stronger hierarchy/no-colour cues; operator visual review required. |
| 27 | “I think you mean…” then “I find…” switches the referent of “I”. | Proposed: explicit draft/guess heading; outcome wording remains separate. |
| 28 | Functional criteria mixed with an aspirational “maximize care” clause. Latest submitted answer was not recovered verbatim. | Investigate attribution before blaming a rewrite; clarify success without invented metrics. |
| 29 | Review had no Still open section while original choices remained undecided. | Proposed: carried-forward choices versus blockers; no evidence of lost persisted context. |
| 30 | Blocked agent review explained launch plumbing, not normal saving consequences. Human/authorized review was untested. | Proposed: review copy; preserve separate goal/seed/implementation consent. |
| 31 | Review's left pane showed the initial dump rather than the latest answer used in the goal. | Proposed: relevant-source access; no claim prior answers were deleted. |

## Evidence and limitations

This was one jointly inspected healthcare-game scenario, not an exhaustive usability
study. The first binary was an existing debug build with unverified source revision;
follow-up fixes used rebuilt task binaries and deterministic replays. The healthcare
dump was assistant-composed and revised after operator correction, not a verbatim
operator specification. No goal was confirmed or saved; saving/handoff and ordinary
human/authorized confirmation were not dogfooded.

The first live send's shaping and review stages were observed. Two later submissions
were inferred from changed readings and conservatively accounted at two calls each:
**6 of 40 authorized calls accounted for**, assuming no unreported sends—not an exact
provider audit or permission for #330 live evaluation. Consolidation/replays made no
new real model calls and no real audio recordings. Later answers were not recovered
verbatim; model fidelity to those exact words cannot be asserted.

The original live pane was later absent from Herdr; this agent did not close it.
The remaining pane was explicitly a **fake replay**, not continuation of the original
live session. Do not promise that unsaved draft can be recovered. Temporary logs
remain historical evidence only; this index and the linked proposal supersede the
temporary priorities/mockup as review entry points. Future readers need no `/tmp` file
to understand the findings, status distinctions or decisions.

The exact clipped question used for regression/replay was:

> What would make this loop satisfying to you: finding a workable balance for different patient populations, understanding consequences across the network, making difficult funding tradeoffs, or some combination?

### Operator feedback retained verbatim

> actually i want it to be managing a healthcare system in a city like central health (clinics, hospitals, administration, all of that) patient populations and providers and budgeting

> i also find it awkward that the 'anything to add or change?' is displayed like a fxiture and the thinking is at the bottom? its not good UX in my opinion. any thoughts on this?

> i also need the titles to be more noticable and also sections more understandable like the 'i think you mean' with the 'I find the smallest satisfying"
>
> what do you think?

> could you make the recommendation populate the your answer field like muted text that can be populated using tab key?

> im worried things are getting messy and disorganized. are you?

### Submitted sample and observed readings

The following is the assistant-composed sample actually submitted after the
operator corrected the citywide scope. It is not a verbatim operator-authored goal
or evidence that the operator accepted every added detail:

> I want to make a city healthcare system management game with the approachable simulation and management feel of RollerCoaster Tycoon. Think of managing an organization like Central Health: a network of clinics, hospitals, providers, and administration serving patient populations across a city, with budgeting as a central part of the game. The thing I want to manage is the healthcare system, not just the rooms and queues inside one clinic. I want decisions about where services go, which services and providers to fund, and how to distribute a limited budget to affect the people who depend on that system. Patient populations and providers need to matter, not just buildings and money. I want to see how choices across the network interact: for example, whether changing clinic capacity changes demand on hospitals, or whether spending more in one part of the city leaves another underserved. Those are possibilities to explore, not settled mechanics. I want to find the beginning of a playable game that captures this citywide management loop without trying to simulate all of healthcare at once. The first version should still feel like managing a healthcare system, rather than shrinking the idea into a single-clinic building game. I have not chosen the engine, visual style, level of realism, or exact first playable scope. Help me work out which decisions the player should make first, what feedback would make their consequences understandable, and what the smallest satisfying version of this system-level game could be.

First live reading, transcribed from the screen with terminal wraps joined—not
raw provider output, and never confirmed:

```text
I find the beginning of a playable city healthcare system management game with the approachable simulation and management feel of RollerCoaster Tycoon.
Why: I want to see how choices across a network of clinics, hospitals, providers, and administration affect the patient populations who depend on that system.
Must: Keep budgeting central to the game.
Must: Make patient populations and providers matter, not just buildings and money.
Must: Keep the first version feeling like managing a healthcare system.
Must: Work out which decisions the player should make first.
Must: Work out what feedback would make the consequences understandable.
Must: Find the smallest satisfying version of the citywide management loop.
Must not: shrink the first version into a single-clinic building game.
Must not: try to simulate all of healthcare at once.
```

Below it: “Still missing: Done when. Review when you're ready.” The input label
was “anything to add or change?”. A later inspected reading added this criterion:

```text
Done when: The first version lets me manage a small citywide network with clinics and a hospital; I can allocate a limited budget across services and providers; I can see consequences for different patient populations; the loop involves understanding the system and working with constraints to maximize care for a health population.
```

The exact later submitted answer was not recovered; do not silently correct
“a health population”, attribute it to the operator, or claim the model rewrote
known source words. The earlier single-clinic sample was the assistant's premature
narrowing, corrected before this submission—not a Tinkery model defect.

### What held up

Caret/focus and basic wrapping were clear. The live reading preserved citywide scope
rather than collapsing to a clinic; proposed mechanics stayed unaccepted. A useful
follow-up question eventually emerged. Agent saving was correctly held without
named authorization. These observations do not establish broad model quality,
colour recognition, persistence correctness or workflow improvement.

## Consolidation stopping rule

The demonstrated obstacle was fragmented review state: temporary chronology,
priorities and mockup files plus an approved #330 branch with a different keybinding.
The operator could no longer confidently assess what was agreed. This is a bounded
**consolidate** step under features-and-futures, not a new lifecycle: one index,
one proposal, published issues for live scope/status. It restores the option to
change the interaction without competing specifications. Stop when all IDs are
accounted for, links/checks pass and the unresolved choices are explicit; do not
add speculative refactors or unrelated cleanup. Only the separately requested
completed #319/#322 worktree safety audit follows publication.

## Next operator action

Review the [proposal's decision table](../docs/research/think/interaction-proposal.md#approval-decisions).
Approve or revise the future interaction design before another layout/keybinding
implementation. The named #330 owner finishes its approved current slice under the
master supervisor's integration coordination. Keep future design/build approval,
live-evaluation budget and merge permission distinct; merging this index does not
grant new feature or evaluation permission.
