# Change brief

Source: https://github.com/jon-devlapaz/tinkery/issues/864
Issue body sha256: 29f447833355967617805c1fc76789f53c35598279aa8b5a4f7ba921cf6aa3b1
Title: Tend: Keep a raw project thought before any goal is confirmed

The approved issue below is this run's brief, unchanged. Each Done-when line is one item in `checklist.json`.

---

### Primary area

tend

### Kind

enhancement

### Goal

The operator needs one place per project to keep ideas, goals and thoughts, including thoughts that are not yet goals.
Saving a thought must not claim that the operator confirmed a goal.

### Evidence

Operator words, 2026-11-15, exact:

> i just want a place that i can saturate my ideas/goals and thoughts in that are project specific and I can allow LLM to find surfaced connections, refactoring and determination if a goal(s) are ready for shaping. thats my main goal. I think idempotency is an idea that is relevant

> think of an entry to tend be a neuron, and with enough neurons a action potential activates over those neuron connections, and then its refined and insulated better through shape like myelination and then then the action is tink-sdlc which is like the behavior..

Asked whether Tend must keep a raw thought before any goal is confirmed from it, the operator answered: "Yes, raw thoughts".

Observed in code at `04c2bc4`:

- Tend lists saved sessions only. Think exits without confirmation leave no record (`docs/design/think.md`, Think record).
- `tinkery seed new` saves any text without Think or a model call, but records it as a confirmed goal (`src/shape/command.rs`, `new_goal`).
- That command writes no repository attribution. One active goal with unknown attribution makes Find connections refuse
  for every repository (`src/tend/connection.rs`, `snapshot_with`).

Proposal, not observation: a thought is a separate kind of entry, not a goal with a weaker label.
Not established: that more entries make connection discovery useful. The one real trial gave a false alarm.

### Scope and boundaries

One outcome: the operator saves exact words as a thought in the current repository's garden, sees it there after restart,
and later turns it into a goal only through the existing goal confirmation.

Changes:

- A key in Tend's repository garden opens one exact-word editor. No Think conversation is needed.
- One explicit save of exact words, with the repository shown before saving. No model call.
- Thoughts appear in the repository garden, visibly distinct from goals with and without colour.
- A thought records where it was saved. It never has unknown attribution.

Stays unchanged:

- Goal confirmation, seed confirmation and implementation permission. A thought is none of them.
- Shape accepts confirmed goals only. A thought cannot open Shape.
- Find connections input, its 20-goal and 32 KiB bounds, the saved correction and all saved-goal formats.
  Thoughts do not enter a provider request in this slice.
- No automatic interpretation, grouping, merging, readiness judgment, wiki or background work.
- No editing or deletion of a saved thought in this slice.

Subtract first: reuse the exact-word editor, the repository boundary, the private atomic write and the garden view.
If the build needs a new storage service, a session-format migration or a change to the shared goal saver, stop and report.

### Done when

- [ ] From the repository garden, the operator saves a thought with one disclosed action, and the saved words equal the typed words byte for byte.
- [ ] After restart, the thought is listed in that repository's garden, labelled as a thought and not as a goal.
- [ ] A saved thought has no goal confirmation record, cannot open Shape, and is absent from the Find connections request.
- [ ] Find connections still runs in a repository that holds thoughts. A thought never causes incomplete discovery.
- [ ] Saving the same exact words twice in one repository results in one entry, and the screen says so.
- [ ] Turning a thought into a goal uses the existing goal confirmation, keeps the thought's words as the original, and leaves the thought on record.
- [ ] Terminal journeys at 80×24 and 100×30 with colour off cover save, restart, duplicate save, cancel and an unwritable store, with temporary TEST roots and no provider.
- [ ] Reversion probes fail when a thought is recorded as a confirmed goal, when saved words differ from typed words, and when a thought reaches a provider request.

### Context and dependencies

- [Yohaku](https://github.com/jon-devlapaz/tinkery/blob/main/docs/yohaku.md), Articles 1 and 2.
- [Tend and handoff](https://github.com/jon-devlapaz/tinkery/blob/main/docs/design/shape-and-handoff.md#tend-garden-and-detail) and [Think](https://github.com/jon-devlapaz/tinkery/blob/main/docs/design/think.md).
- #860 and #862 are merged. This slice does not change them.
- #852 covers pruning and restoring goals. It is related and not a blocker.
- Later, separate slices: thoughts as Find connections input, volume above 20 entries, refactoring proposals,
  readiness judgments and dismissal that persists. None is part of this issue.

### Open decisions and permissions

Decided by the operator on 2026-11-15: a thought enters through a key in Tend's garden. Operator words: "Key in Tend garden".
The key name, its label and its place in the key bar remain design details for review.

Defaults proposed by the agent and accepted by the operator for this slice on 2026-11-15:

1. A saved thought cannot be edited or removed in this slice. Pruning is separate work (#852).
2. "Project" means the launch repository, as goals use today.
3. A duplicate means the same exact bytes in the same repository. Whitespace and case differences make a different thought.

Each default can change in a later slice. No other decision is open for this slice.

This issue records a design request. Publication, implementation, provider budget and real-data trials each need separate operator approval.

