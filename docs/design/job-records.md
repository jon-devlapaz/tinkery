# One job through the existing records

Design for [#63](https://github.com/jon-devlapaz/tinkery/issues/63), following
[yohaku](../yohaku.md). **Proposed, not implemented or accepted for R5.** This
maps existing records; it adds no reader, permission, launcher or job ledger.

The smallest next step is to read Substrate's selected work item and show its
sources separately. It already connects a run, local checks, a PR and measured
results. The missing connection is from that work item back to a particular
Tinkery goal and its complete Seed Me decisions. A matching title is not a link.

## What was inspected

Only source and synthetic tests were read. No operator sessions, transcripts,
saved dumps or installed packages were inspected. No model or voice calls ran.
These are the inspected commits, not a claim about the latest available release.

| Source | Exact revision | Format at that revision |
| --- | --- | --- |
| Tinkery | `4f520b22c28cd109de3c7d3fb7263cfad8bc3ccb` | think record `schema: 1`; package `0.1.0` |
| Seed Me, Tinkery's existing CI fixture | `1a2f31a136d53bceff51ae8c2e0e787c1a8a90bb` | ledger `schema_version: 1`; Markdown seed |
| tink-sdlc | `8ffa2bdf2743c8904c5eebd27c8ff2f45ec972a3` | package `1.23.0`; machine API `1`; run `schema: 1` |
| tink-substrate | `0f1f7e552ccc0b35c40cb4c6880f5add8f1b0c02` | package `0.2.0`; snapshot `schema: 1`; surface `tink-surface/1` |
| GitHub CLI / GitHub | installed CLI `2.98.0`; official docs read 2026-10-12 | selected `gh pr view --json` fields; live service has no pinned record schema |

Substrate preparation currently names `tink-skills` as its bundled Seed Me
source. That is distinct from the standalone Seed Me commit pinned by Tinkery.
No prepared package was inspected, and compatibility between these revisions is
not established here. A run's own `tools.json` identifies what it actually used.

## Record owners and limits

Paths below describe source formats, not permission to discover private files.
Readers must receive the selected locations explicitly.

| Owner / record and version | Identity and revision | Read path or helper | Freshness rule | Missing or failed means |
| --- | --- | --- | --- | --- |
| Tinkery / think JSON, schema 1 | Filename uses confirmation time and sanitized Seed Me folder name; `seed_me_session`, `goal`, `started`, `confirmed`, `prompt_hash` | Explicit record file; writer is [`think/record.rs`][T-record] | Written once after goal save; no Seed Me ledger version or digest is stored | Record absent may mean the separate record write failed, or no goal was confirmed; it does not prove no Seed Me session exists |
| Seed Me / `ledger.json`, schema 1 | `session_id`, `version`, `revision`; node `id`, `history`, `revision`, `premise_version`, `reviewed_version` | [`session.py read <session>` / `load`][S-session]; `review_flags(nodes)` derives concerns | Publication checks `expected_version`; changes preserve history and require dependent decisions to be reopened or revalidated | Absent, invalid schema, malformed state and an ended session are different; `completed` ends discovery, not implementation |
| Seed Me / `seed-contract.md`, Markdown contract | `revision:` label/date; first line status; `Confirmed by:` words | Read alongside the same session's ledger; [`confirm_seed`][S-confirm] and [seed contract][S-contract] define the fields | Human confirms the displayed label; unresolved or flagged nodes prevent helper confirmation. There is no saved content digest tying that confirmation to later file bytes | Draft, simulated, missing and unreadable are not confirmed intake. A status line alone is insufficient; it cannot authenticate the human |
| SDLC / `runs/<slug>/run.json`, schema 1; decision JSON | Selected checkout plus slug; `profile`, `kind`; optional `seed_contract.{path,sha256,source}`; decision filename carries stage/time/id, body carries `inputs`, `decision`, `reviewer`, `source`, `reason` | [`sdlc.py status <slug> --json`][D-status], API 1; raw `run.json` supplies seed binding omitted from status metadata | Each stage compares the latest receipt's `inputs` digest with current inputs; light profile has gate 3, full profile gates 1–3 | No receipt is pending; changed inputs are stale; rejection remains changes-requested. Local receipts record a claim of human review, not authenticated release authority |
| SDLC / verification, checklist and marks | `candidate.{head,tree}`, `inputs`, `policy`, `test_lock`, optional `checklist`, `manual_marks`, `checklist_items`, `log`; marks have item/result/evidence/candidate/time | API status, `runs/<slug>/04-test/output/verification.json`, `test-log.md`, `checklist.json`, `marks/` | Runtime recomputes current evidence and log digest, checks gates and writer ownership; an automated item and a manual attestation have distinct proof kinds | `blocked`, `running`, `not-run`, `current`, `stale`, `failed`, `interrupted` are separate. A file saying passed is not enough |
| Substrate / selection JSON and work Markdown, schema 1; tools JSON, schema 1 | Selection: `work`, `checkout`, `trust_sdlc`; work: `title`, `project`, `owner`, `next_action`, optional `branch`, `run`, `pr`, `seed`, `handoff`; tools: `run`, `change`, pinned `components` | [`surface --json`][B-surface] for compact view; [`status` / `sources.snapshot`][B-sources] for evidence; `runs/<run>/tools.json` | Per-source `checked_at`, snapshot `observed_at`; reads are not atomic. Prepared resume compares portable tools record with Git-local `substrate-runs/<run>.json`, package integrity and workflow digest | No selection, no link and unavailable source stay distinct. Missing local preparation record cannot be treated as a prepared run |
| GitHub / PR and checks | Repository plus PR number/URL; `headRefOid`, `state`, `isDraft`, `reviewDecision`, `statusCheckRollup`, `mergedAt` | Substrate's pinned `pr_state`, using [`gh pr view --json`][G-cli] | Refresh and compare PR head with checkout HEAD before relying on CI. Existing read does not establish branch protection rules or the reviewed commit for each review | No checks, pending, failed, skipped and unknown never become Substrate `ci: passed`; network/auth failure means unknown, not no PR |
| Substrate / measurement JSONL, schema 1 | `change` is `owner/repo#number`; `key`, `phase`, `at`, `by`, `fields`; optional `fields.change_id` matches tools `change` | [`surface.ledger_view`][B-surface] reads the existing ledger; [`ledger.fold`][B-ledger] applies field updates | File order folds observations; `as_of` is file modification time, not proof each field was refreshed. A unique change-ID link precedes PR fallback; disagreement is unavailable | No ledger, no row and null measurements are unknown, never zero. A partial last line is ignored by the surface; a malformed earlier line is an error |

The current Tinkery shape reader is intentionally lighter than these authority
checks: it classifies the ledger status and seed's first line. Its `seed ready`
label is not a validated approval receipt. See [`seed_me/session.rs`][T-session].

## Trace one synthetic job

The [fixtures](../../tests/fixtures/job-records/README.md) use a made-up counter
change. Their approvals and results are test data, never human actions.

1. **Goal.** `think.json` retains the original words and confirmed goal. Its
   `seed_me_session` names the Seed Me folder. In that folder, `ledger.json`
   identifies the same session; `goal` equals the settled origin node's `answer`.
   Tinkery's save path checks that answer, its user authority and source after
   publishing. Answered or skipped Think questions remain context, not additional
   settled Seed Me decisions. [Source and test][T-save]
2. **Seed and separate discovery decisions.** Read each ledger decision's own
   answer, owner, authority/source and history. The fixture's scope decision is
   separate from its goal decision. Seed confirmation quotes a further decision
   about revision `r1`; completing the human session is another state transition.
   A completed human ledger plus confirmed seed is the documented intake handoff.
   The seed is not implementation permission. [Source][S-contract]
3. **Run.** The chosen Substrate work record names `test-job`, its checkout and
   copied seed path. SDLC's `run.json` records the copied seed's SHA-256 and the
   original source path. That is a byte link to the seed, not validation of the
   Seed Me ledger, confirmation or semantic preservation. The SDLC helper accepts
   a regular file without checking Seed Me status. [Source][D-seed]
4. **Implementation decision.** This example uses a light run, whose combined
   definition gate is stage 3. Its own receipt carries reviewer, source, reason
   and input digest. A full run has separate stage 1, 2 and 3 receipts. Neither
   the goal nor the seed decision can stand in for any of them. [Source][D-decide]
5. **Checks.** The verification receipt records a synthetic candidate and local
   check. Runtime status determines whether the receipt still matches. GitHub
   checks are another source and cover the reported PR head, not automatically
   the local tree. A check pass does not establish a person's acceptance.
6. **Result.** The example PR is open with passing synthetic checks and no GitHub
   review approval: it is not finished. `MERGED` and `CLOSED` are different
   results. A merged PR establishes a merge, not deployment or usefulness. The
   existing ledger calls merged and not detected as reverted “useful”; operator
   taste and real-world outcome are not measured by that rule. [Source][B-ledger]

There is **no common job identifier across all six steps**. The think record has
no work-record/run/change/PR reference. The work record has no Seed Me session ID
or confirmed-goal reference. Its `seed` path can lead to SDLC's copy, and
`seed_contract.source` can name the original folder, but neither guarantees a
session identity. A moved source, two matching goals, duplicate change IDs or
disagreeing PR links must remain unresolved. Do not infer identity from text,
timestamps or branch name alone. The fixtures keep the proposed connection
expectations separate from every source record.

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

Snapshot times mean “read at,” not “unchanged since.” Substrate collects sources
separately. Refresh before any consequential external action; if identities
change while reviewing, show that and require review of the current material.
No aggregate “approved,” “done” or “safe to resume” flag is justified here.

## Attention and cost

Tinkery's record has ordered turns, check decisions, start/confirmation times,
model/thinking and prompt fingerprint. It has no token usage, reported dollars,
active-attention clock or provider-call receipt. `confirmed - started` includes
waiting and time away. `origin: live` names the Pi code path: the existing
[confirmation PTY test][T-pty] uses a stub provider yet produces that value.
Neither timestamps nor `origin: live` prove paid calls or operator attention.

Substrate already stores estimated `operator.minutes.{c2,c5,c10}`,
`minutes_by_week`, `touches`, `phases`, `method: gap-cap` and linked session IDs.
Its source algorithm credits gaps before human prompts, capped at 2/5/10 minutes,
merges overlap and excludes launched workers from human touches. That is an
estimate; it misses the first prompt's prior effort and reading/thinking outside
the transcript. This inspection read only the algorithm and synthetic tests,
never those transcripts. [Source][B-sessions]

Existing `cost` fields include normalized token counts, `llm.cost.total` and
`cost_known_share`. Dollars come only from harness reports; absent dollars remain
null, and the known share is weighted by input/output tokens, not by call count.
At the inspected revision Codex contributes tokens but no reported dollars.
No source here proves provider billing, all actual calls, direct attention,
operator taste or customer value. Reuse existing measurements with their method
and coverage; do not create another ledger or infer missing values. The existing
surface suppresses useful/hour for fewer than five changes with operator data.

## Proposed R5: one explicit read

After the operator accepts this design, start with one selected Substrate work
item and the existing `surface --json` output. Use its `tink-surface/1` identity,
selection, brief, workflow, PR, needs-you items and ledger coverage. Show one
next step and an exit. Detailed evidence comes from the existing `status` snapshot
and SDLC API, not from a second calculation of gates or hashes.

The read itself needs an explicit operator action naming the source: Substrate
runs Git/`gh` commands and may contact GitHub; with existing `trust_sdlc: true` it
also executes the selected checkout's Python status helper. “Read-only” describes
the intended effect, not permission to execute arbitrary discovered code. Do not
turn trust on, run setup, read private transcripts, install a watcher, or launch
the reader merely by opening a screen. A missing/trust-disabled source stays
unavailable and points to the external owner. No new permission system is needed.

Keep full decisions available through the detailed source: the compact surface
reduces them to stage/decision/reviewer and omits source, reason, receipt ID and
history. It also omits Seed Me session identity and SDLC's seed binding. Until an
explicit source connection is established, show “Goal connection unknown” and
the external work as selected; do not attach it to a Think session automatically.
Any need to persist that connection or expose missing fields is a separately
reviewed upstream dependency. R5 must not invent fields to make the chain look whole.

Architecture diagnostics: no structural score changes in this documentation PR.
The proposal preserves one owner per source and simpler callers (rows 1, 2, 3
and 6 of [architecture](architecture.md)); phase views would use shared readers,
not reach into another phase. Acceptance of this design is still required before
an R5 implementation starts.

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

**Binding gap:** the current `decide` helper hashes inputs when the command runs.
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
evidence. Checks here establish fixture/source consistency only, not a production
reader or a real job outcome. Rust, Node and PTY suites were not run for this
design-only change; no production behavior or reversion probe is claimed.

[T-record]: https://github.com/jon-devlapaz/tinkery/blob/4f520b22c28cd109de3c7d3fb7263cfad8bc3ccb/src/think/record.rs#L158-L190
[T-save]: https://github.com/jon-devlapaz/tinkery/blob/4f520b22c28cd109de3c7d3fb7263cfad8bc3ccb/src/seed_me/tests.rs#L93-L146
[T-session]: https://github.com/jon-devlapaz/tinkery/blob/4f520b22c28cd109de3c7d3fb7263cfad8bc3ccb/src/seed_me/session.rs
[T-pty]: https://github.com/jon-devlapaz/tinkery/blob/4f520b22c28cd109de3c7d3fb7263cfad8bc3ccb/tests/goal_confirmation_terminal.py#L79-L98
[S-session]: https://github.com/jon-devlapaz/seed-me/blob/1a2f31a136d53bceff51ae8c2e0e787c1a8a90bb/skills/seed-me/scripts/session.py#L290-L367
[S-confirm]: https://github.com/jon-devlapaz/seed-me/blob/1a2f31a136d53bceff51ae8c2e0e787c1a8a90bb/skills/seed-me/scripts/session.py#L581-L649
[S-contract]: https://github.com/jon-devlapaz/seed-me/blob/1a2f31a136d53bceff51ae8c2e0e787c1a8a90bb/skills/seed-me/references/seed-contract.md
[D-status]: https://github.com/jon-devlapaz/tink-sdlc/blob/8ffa2bdf2743c8904c5eebd27c8ff2f45ec972a3/assets/_system/scripts/sdlc.py#L628-L829
[D-decide]: https://github.com/jon-devlapaz/tink-sdlc/blob/8ffa2bdf2743c8904c5eebd27c8ff2f45ec972a3/assets/_system/scripts/sdlc.py#L188-L224
[D-writer]: https://github.com/jon-devlapaz/tink-sdlc/blob/8ffa2bdf2743c8904c5eebd27c8ff2f45ec972a3/assets/_system/scripts/sdlc.py#L377-L402
[D-seed]: https://github.com/jon-devlapaz/tink-sdlc/blob/8ffa2bdf2743c8904c5eebd27c8ff2f45ec972a3/assets/_system/scripts/sdlc.py#L1205-L1217
[D-seed-tests]: https://github.com/jon-devlapaz/tink-sdlc/blob/8ffa2bdf2743c8904c5eebd27c8ff2f45ec972a3/tests/test_sdlc_stage.py#L257-L284
[D-protocol-tests]: https://github.com/jon-devlapaz/tink-sdlc/blob/8ffa2bdf2743c8904c5eebd27c8ff2f45ec972a3/tests/test_sdlc_protocol.py#L109-L180
[B-surface]: https://github.com/jon-devlapaz/tink-substrate/blob/0f1f7e552ccc0b35c40cb4c6880f5add8f1b0c02/tink_substrate/surface.py
[B-sources]: https://github.com/jon-devlapaz/tink-substrate/blob/0f1f7e552ccc0b35c40cb4c6880f5add8f1b0c02/tink_substrate/sources.py
[B-ledger]: https://github.com/jon-devlapaz/tink-substrate/blob/0f1f7e552ccc0b35c40cb4c6880f5add8f1b0c02/tink_substrate/ledger.py
[B-sessions]: https://github.com/jon-devlapaz/tink-substrate/blob/0f1f7e552ccc0b35c40cb4c6880f5add8f1b0c02/tink_substrate/sessions.py
[G-cli]: https://cli.github.com/manual/gh_pr_view
[G-protection]: https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches/about-protected-branches
