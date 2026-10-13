# First-release tasks

These ten tasks implement the [first-release plan](first-release.md). R1 and R2
are done. R3 is published as [#96](https://github.com/jon-devlapaz/tinkery/issues/96).
R4–R10 remain unpublished drafts without implementation permission. R1–R10 are stable local
labels. Review each remaining draft with the operator before publishing it and
keep dependencies current.

## Rules shared by every draft

- Follow [the constitution](../yohaku.md) and [CLAUDE.md](../../CLAUDE.md).
  Preserve original words and every distinct human decision.
- Start from the verified dependency revision. Work on one approved slice per
  branch/PR; never merge your own work. Name the issue and dependencies in the PR.
- Code changes need relevant regression checks, important reversion probes and
  the documented zero-call verification. Report actual Rust/Node counts, failures
  and terminal journeys/sizes. Design-only work reports document/fixture checks;
  it must not imply that production behavior was tested.
- Test under temporary TEST roots with fake providers and voice. Never use real
  audio or alter operator sessions, installs, launcher or another tool's checkout.
- Real calls need separately authorized data use and an explicit budget. The
  default is zero. Historical authorization from #58 does not carry into a draft.
- Preserve phase boundaries. Update the affected design documentation in the same
  PR. Read existing tests before changing behavior and reuse existing helpers.

<a id="r1"></a>
## R1 — Make Think evaluation repeatable without live calls

**Done:** [#62](https://github.com/jon-devlapaz/tinkery/issues/62), merged in
[PR #67](https://github.com/jon-devlapaz/tinkery/pull/67). See
[Think evaluation](think-evaluation.md). The issue keeps the completion checks.

<a id="r2"></a>
## R2 — Map one job through the existing tools

**Done:** [#63](https://github.com/jon-devlapaz/tinkery/issues/63), merged in
[PR #65](https://github.com/jon-devlapaz/tinkery/pull/65). The
[proposed design](job-records.md) awaits the operator's acceptance before R5.

<a id="r3"></a>
## R3 — Add an explicitly budgeted live evaluation mode

**Published:** [#96](https://github.com/jon-devlapaz/tinkery/issues/96); see
[live runs](think-evaluation.md#live-runs).
**Milestone:** M1. **Depends on:** R1. **Parallel work:** R2/R5.
**Owner:** the evaluation builder. Building and testing this slice needs no live calls.

**Scope:** Connect the R1 runner to the existing drafting boundary. Reuse capture
support in the live example; remove reliance on default dumps/answers in this
entry point. Provider invocation is permitted only for supplied, authorized data.

**Completion checks:**

- [ ] Missing supplied data, permission or a positive authorized call limit causes
  refusal before any provider process/request starts. Default behavior is fake.
- [ ] One budget boundary covers every shaping and review request, retries and
  failures. Reserve/count before invocation; exhaustion stops before the next
  request, including the second call of a send. Test this with fake executables.
- [ ] Tests cover limits 0, 1 and 2, cancellation, provider failure and retry.
  Reverting the pre-invocation limit makes a regression test fail. Reusing an
  evaluation run cannot silently reset its remaining budget.
- [ ] Records identify the supplied dataset, provider/model, prompt and source
  revisions, configured limit, attempted calls and run outcome. Keep provider
  execution provenance separate from the operator's quality verdict.
- [ ] Private input and raw output stay in an explicitly selected, permitted
  evaluation location outside the repository; no operator sessions are written.
- [ ] Report fake-only verification as such. Any real smoke run requires its own
  data permission and budget and is not necessary to claim the limit tests pass.

**Leave out:** Automatically running live examples, harvesting sessions/history,
new provider integrations and permission inferred from a configuration flag alone.

<a id="r4"></a>
## R4 — Record the current goal quality and human effort

**Milestone:** M1. **Depends on:** R3; operator-supplied examples, data permission,
explicit budget and operator participation. **Parallel work:** R5/R6/R7/R8.
**Owner:** an evaluation agent with the operator judging meaning.

**Scope:** Establish a baseline through the current manual workflow, from goal
capture to a checked result and the operator's acceptance/correction decision,
before using the new full-loop views. Each task retains its own required work
authorization. The operator chooses comparable tasks and a useful release threshold;
propose three tasks without treating that sample as already approved.

**Completion checks:**

- [ ] Record the permitted dataset/task selection, comparison method, budget and
  chosen success threshold before real calls. No such permission means no run.
- [ ] The operator judges whether each goal preserves their meaning. Record this
  separately from exact-word, constraint, handoff and other automated checks.
- [ ] Record active explaining, deciding, reviewing and correcting time, repeated
  explanations and waiting separately. Do not infer attention from session duration.
- [ ] Follow each baseline task through the existing external work/review process.
  Record the final result revision, matching checks and explicit operator verdict,
  or its failed/abandoned state. Count accepted, checked results and total active
  effort through that verdict, including correction cycles. Think-only observations
  do not qualify as the full-workflow comparison baseline.
- [ ] Record failed/abandoned attempts and corrective effort as well as successes.
  Missing time, cost or judgments stay unknown. Keep a non-private result summary
  and references to permitted local evidence, without committing private text.
- [ ] Exact goal saving and manual continuation are exercised in isolated roots.
  Any demonstrated release blocker gets a bounded issue proposal with a failing
  example; the baseline task does not silently expand into implementation.
- [ ] The operator confirms the baseline is suitable for R10. Where evidence is
  insufficient, record the gap and the next observation rather than a quality score.

**Leave out:** Simulated human approval, unapproved provider spend, fabricated
baseline numbers and fixes not covered by a separately approved issue.

<a id="r5"></a>
## R5 — Read the records for one linked job

**Milestone:** M2. **Depends on:** accepted R2 design and any upstream prerequisites
it identifies. **Parallel work:** R3/R4. **Owner:** one shared-reader builder.

**Scope:** Implement the smallest shared read-only connection established by R2,
with one stable job identity and source/revision information for each state.

**Completion checks:**

- [ ] R2's fixtures trace one exact goal through its seed, decisions, run, checks
  and result. No link is guessed from a title, list position or agent status text.
- [ ] Missing, unreadable, outdated, failed and interrupted states remain distinct;
  none grants permission or counts as verified completion.
- [ ] Changed source/result revisions invalidate derived readiness and checks.
  Important identity and revision protections have failing reversion probes.
- [ ] Tests can reopen the same job after interruption without creating replacement
  sessions, changing the goal or repeating an external action.
- [ ] Reads do not block navigation indefinitely. Late results cannot reopen a
  departed view or overwrite another job; cleanup/failure behavior is tested.
- [ ] Shared code owns external formats; no phase depends on another phase.
  Update architecture documentation/checks without loosening their limits. The
  merged input contract and fixtures are sufficient for R6/R7/R8 to work separately.

**Leave out:** New permissions, provider calls, agent launching, phase UI work,
parallel writers in the shared reader and unsupported upstream migrations.

<a id="r6"></a>
## R6 — Show the exact decision in Approve

**Milestone:** M3. **Depends on:** R5 and the R2 approval-handoff design.
**Parallel work:** R7/R8. **Owner:** an Approve builder; one integrator owns menu wiring.

**Scope:** Show the current plan/seed revision, the next required decision and its
consequences. Provide a deliberate copyable handoff to the existing external
approval process, preserving its authority and all separate permissions.

**Completion checks:**

- [ ] The goal, displayed revision and requested decision refer to the same job.
  Goal confirmation, seed confirmation, intake approval and implementation
  permission are named accurately and never substituted for one another.
- [ ] The handoff identifies the exact revision and requires the existing mechanism
  to re-read current state before writing. If it cannot enforce this, R6 remains
  blocked on a separate dependency; a copied prompt is not an approval record.
- [ ] Missing/outdated decisions are explicit. Changes after display cannot reuse
  the old approval as permission; cover this with a reversion probe.
- [ ] Reading, navigating and copying never approve or launch work. The view gives
  one next action and an exit; ordinary UI keeps storage details in technical help.
- [ ] Terminal journeys cover normal and small sizes, keyboard routes, NO_COLOR,
  unavailable sources and returning to the same job. The design doc matches the UI.

**Leave out:** In-app approval writes, a launch button, auto-approval or auto-merge,
and a new approval receipt format.

<a id="r7"></a>
## R7 — Follow one external job in Watch

**Milestone:** M4. **Depends on:** R5. **Parallel work:** R6/R8.
**Owner:** a Watch builder; the integrator owns shared wiring.

**Scope:** A quiet read-only view of one job's recorded progress, source freshness,
blocker and next useful action.

**Completion checks:**

- [ ] Running, waiting for a decision, failed, finished and unknown are distinct,
  with the evidence source and freshness available on demand.
- [ ] Missing/stale records cannot show current success. An agent saying “done”
  does not establish passed checks or operator acceptance.
- [ ] Refresh/navigation never launch, restart, cancel or mutate external work.
  Unchanged state introduces no new demand for operator attention.
- [ ] Blocked/failed work has an evidence-backed next step or says what is unknown;
  no invented repair command is presented as safe.
- [ ] Terminal tests cover changing state, slow/failed reads, leaving during a read,
  returning to the same job, small/normal sizes and NO_COLOR. Reverting freshness
  protection makes the relevant regression fail. Update the design doc.

**Leave out:** Multi-run monitoring, background agent launches, new notification
services, restart controls and polling the operator for unchanged state.

<a id="r8"></a>
## R8 — Judge a result against the goal in Taste

**Milestone:** M5. **Depends on:** R5. **Parallel work:** R6/R7.
**Owner:** a Taste builder; the integrator owns shared wiring.

**Scope:** Present the original goal, its completion conditions, the result/PR
revision, matching checks and unresolved problems. Hand acceptance or correction
back to the existing review process; the operator retains the merge decision.

**Completion checks:**

- [ ] Each claimed completion condition points to evidence for the exact result
  under review. Unsupported conditions and missing checks stay unresolved.
- [ ] New code or a different result revision makes previous checks visibly out
  of date; a reversion probe proves the mismatch is caught.
- [ ] The confirmed goal is unchanged. Automated checks, an independent review
  and the operator's acceptance remain separate kinds of evidence.
- [ ] Acceptance and correction each have a clear deliberate handoff referring
  to the displayed result. Copying it records no verdict, posts nothing and merges
  nothing. Recheck current state before the external process records a decision.
- [ ] Terminal journeys cover incomplete/failed checks, revision changes, correction,
  exit/return, small/normal sizes and NO_COLOR. Update the design doc.

**Leave out:** Automatically accepting a result, posting reviews or merging,
rewriting the original goal and treating a model's praise as human acceptance.

<a id="r9"></a>
## R9 — Show recorded results and attention in Measure

**Milestone:** M6. **Depends on:** R5 and operator-accepted measurement definitions
from the R2 survey/R1 baseline form. **Preparation:** inspect fields with R2; the
completed R4 baseline is needed for R10's comparison, not to build this report.
**Owner:** a Measure builder.

**Scope:** Extend the existing ledger-backed view only as needed to compare
accepted, checked results and recorded active operator effort. Use R2's actual
source mappings. A needed upstream change requires its own approved dependency.
If the existing report meets every check, record that evidence and close this
slice without a code change.

**Completion checks:**

- [ ] Count a result only when matching checks and explicit operator acceptance
  exist. Duplicate records do not count the same result twice.
- [ ] Show active explaining, deciding, reviewing and correcting time, with waiting
  separate. Session start/end timestamps are never labeled active attention.
- [ ] Show the counts/minutes behind accepted changes per operator hour. Missing
  data or zero recorded time yields unknown, not an invented rate or infinity.
- [ ] Separate fake runs, actual provider runs and human judgments using verified
  provenance; do not infer it from `origin: live`. Actual cost, estimates and
  unavailable cost have distinct labels.
- [ ] Fixture checks cover duplicates, missing time/acceptance, zero time, corrected
  results, revision mismatch and mixed fake/real records. Reversion probes protect
  the accepted-result count and attention interpretation.
- [ ] Unavailable ledger behavior stays honest; the view remains read-only. Update
  the design and test the visible report at supported terminal sizes and NO_COLOR.

**Leave out:** A new analytics service, second ledger, passive attention tracking,
invented historical data and unsupported claims of productivity improvement.

<a id="r10"></a>
## R10 — Prove the complete release on permitted real work

**Milestone:** M6. **Depends on:** R4 and R6–R9; their code merged and checked;
operator-approved tasks, data use and budgets for the trial.
**Owner:** one journey coordinator with an independent reviewer and the operator.

**Scope:** Prove one complete six-stage journey, then run the agreed small
repeatability trial against R4's baseline. Retain manual handoff. This is a
release decision based on observed work, not an automatic launch or release.

**Completion checks:**

- [ ] First pass the complete journey with fake providers and isolated records,
  including failed checks, outdated approval, interruption and recovery. Then
  obtain the trial's specific data and budget authorization before real work.
- [ ] Every trial follows Think → Shape → Approve → external permitted work →
  Watch → Taste → Measure, retaining the exact goal and separate human decisions.
- [ ] The operator does not have to reconstruct intent at handoffs. The same job
  and exact checked result can be recovered after interruption without duplicate
  actions. Record any failure instead of excluding the attempt from the report.
- [ ] Checks and independent review cover the current result; the operator judges
  goal quality, acceptance and merge. Actual provider calls stay within budget.
- [ ] Compare active effort, corrections, repeated explanations, accepted results
  and recorded cost with the baseline; show missing values and task differences.
  The agreed improvement threshold is met without correctness/permission loss.
- [ ] The operator decides release readiness. If improvement is absent or evidence
  is inadequate, record that result and the smallest next proposal; do not mark
  the release complete. Preserve a non-private report with evidence references.

**Leave out:** Declaring fake success to be human validation, hiding failed trials,
automatic release/merge and expanding the trial into unapproved fixes.
