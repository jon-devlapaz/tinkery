# First-release tasks

R1–R11 and I1/I2 are stable local labels, not GitHub issue numbers. R1–R3 are
done; the selected release tasks below are published, unassigned and
`needs-approval`. GitHub owns their scope, completion checks and blocking links.
This page is an index, not a second task specification.

Publication does not accept the revised R2/binding design or authorize
implementation, data use, provider spending or merge. Unpublished optional ideas
have one home in the [parked backlog](../research/shared/backlog.md).

## Rules shared by every draft

Every task inherits [AGENTS.md](../../AGENTS.md#guardrails) and
[work intake](../../AGENTS.md#work-intake). Start from verified dependency
revisions and name them in the PR. Design-only evidence must not imply production
behavior was tested. Authorization given for #58 carries into none of these tasks.

<a id="r1"></a>
## R1 — Make Think evaluation repeatable without live calls

**Done:** [#62](https://github.com/jon-devlapaz/tinkery/issues/62), merged in
[PR #67](https://github.com/jon-devlapaz/tinkery/pull/67). See
[Think evaluation](think-evaluation.md). The issue keeps the completion checks.

<a id="r2"></a>
## R2 — Map one job through the existing tools

**Done:** [#63](https://github.com/jon-devlapaz/tinkery/issues/63), merged in
[PR #65](https://github.com/jon-devlapaz/tinkery/pull/65). The
[revised design](job-records.md) proposes Tinkery-owned shared reading and an
explicit goal-to-run binding. The survey is complete; design acceptance and
verified identity prerequisites still precede separately approved R5 work.

<a id="r3"></a>
## R3 — Add an explicitly budgeted live evaluation mode

**Done:** [#96](https://github.com/jon-devlapaz/tinkery/issues/96), merged in
[PR #97](https://github.com/jon-devlapaz/tinkery/pull/97). See
[live runs](think-evaluation.md#live-runs). The issue keeps the completion checks.

<a id="r4"></a>
## R4 — Record the current goal quality and human effort

Published: [#109](https://github.com/jon-devlapaz/tinkery/issues/109) · M1 · needs approval. The issue owns baseline scope,
operator participation, separate data/budget gates and completion evidence.

<a id="r5-identity"></a>
## R5 prerequisites — run identity and explicit binding

M2. The [identity contract](job-records.md#goal-to-run-identity-prerequisite)
remains an unaccepted design proposal.

**I1 — Conditional upstream capability.** Verify source-owned run identity in
SDLC before proposing a missing-capability issue there. No upstream issue was
published in this batch; capability verification is not implementation permission.

**I2 — Initial binding creation and duplicate detection:** [#110](https://github.com/jon-devlapaz/tinkery/issues/110) · needs
approval. The issue owns this initial-only slice; binding revisions and relocation
are deferred. The entry point and explicit storage-root configuration remain open.

<a id="r5"></a>
## R5 — Read the records for one linked job

Published: [#111](https://github.com/jon-devlapaz/tinkery/issues/111) · needs approval. See the issue for scope, dependencies,
open decisions and completion evidence.

<a id="r6"></a>
## R6 — Show the exact decision in Approve

Published: [#112](https://github.com/jon-devlapaz/tinkery/issues/112) · needs approval. See the issue for scope, dependencies,
open decisions and completion evidence.

<a id="r7"></a>
## R7 — Follow one external job in Watch

Published: [#113](https://github.com/jon-devlapaz/tinkery/issues/113) · needs approval. See the issue for scope, dependencies,
open decisions and completion evidence.

<a id="r8"></a>
## R8 — Judge a result against the goal in Taste

Published: [#114](https://github.com/jon-devlapaz/tinkery/issues/114) · needs approval. See the issue for scope, dependencies,
open decisions and completion evidence.

<a id="r9"></a>
## R9 — Show recorded results and attention in Measure

Published: [#115](https://github.com/jon-devlapaz/tinkery/issues/115) · needs approval. See the issue for scope, dependencies,
open decisions and completion evidence.

<a id="r10"></a>
## R10 — Prove the complete release on permitted real work

Published: [#116](https://github.com/jon-devlapaz/tinkery/issues/116) · needs approval. See the issue for scope, dependencies,
open decisions and completion evidence.

<a id="r11"></a>
## R11 — Turn a confirmed goal into a seed in Shape

Added by the operator on 2026-10-15 · M7. Published: [#149](https://github.com/jon-devlapaz/tinkery/issues/149)
(read-only Shape) and [#150](https://github.com/jon-devlapaz/tinkery/issues/150) (answering and seed
confirmation, blocked by #149) · needs approval. The [Shape design](shape-and-handoff.md#shape) is a proposal.
