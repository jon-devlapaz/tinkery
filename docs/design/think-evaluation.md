# Think evaluation (#62, #96)

One runner, `think_eval`, replays supplied words through think. By default the
replies are scripted: that checks repeatable mechanics and cannot measure model
quality or human recognition. [Live runs](#live-runs) send the same words to Pi,
only with operator data, a written permission and a positive call limit. Neither
mode adds a prompt or changes production interaction.

Run from the checkout being evaluated, rebuilding the example:

```bash
eval_root=$(mktemp -d)
cargo +stable run --locked --example think_eval -- \
  --dataset tests/fixtures/fake-think-eval.json \
  --output "$eval_root/result" --max-requests 2
```

The named fixture is synthetic test data. The runner refuses missing input;
it never substitutes that fixture. A scripted run's output must be a new absolute
directory below the system temporary directory, with an existing parent. It writes captures and
`report.json` there only. It never creates a saved goal or writes a
Think session record. There is no second session-record format.

The dataset has a `provenance` (`kind`: `synthetic_test_data` or
`operator_supplied`, plus a description), ordered `steps`, and ordered `replies`.
A step is `{"action":"send","text":"exact words"}` or
`{"action":"retry"}` after a failed reading. Each send has at most 4,096 bytes,
the existing input limit. Replies have `kind` (`shaping` or `decision_review`)
and either `text` (a raw JSON response string) or `failure` (an error string).
There are no commands, model names, or executable paths in this format. Sending
while a question is focused uses the existing answer behavior; this slice does
not add scripted skip, scope-selection, confirmation, or save actions.

Every request consumes one unit of the same `CallBudget` that limits Pi in a live
run, before its scripted reply is read. Shaping
and decision review are counted separately, including failures and explicit
retries. Zero refuses the first request. Reaching the cap refuses the next
request without consuming its reply. A failed or refused decision review leaves
the shaping reply in place, as production does. No request retries itself.
Reply delivery and decoding are reported separately: a delivered invalid reply
still counts as a request, and the step reports the failed reading.

Each step reuses the live example's capture helper: sources, reading, cited spans,
paper, and 80×24, 100×30, 160×40 and 320×40 text snapshots. These are mechanics
captures, not terminal journeys or human validation.

The report names the dataset path, fingerprint, declared provenance and any
permission, the execution (`fake` or `live`), and each attempted or refused call. It records the Git checkout revision and dirty
state **at run time**, plus compiled runner and prompt-file fingerprints. The
compiled library revision is explicitly unknown: an old executable can run after
the checkout changes. Rebuild and record the tested commit when retaining evidence.
Prompt fingerprints identify the current templates; no prompt executes in a fake run.

Optional `expected` groups compare exact supplied values with the final reading:

- `actor_outcome`: `who`, `outcome`.
- `constraints`: `must`, `must_not`.
- `unresolved_deferred`: `open`, `questions`, `notes`, `deferred`, `other_goals`.

An absent expectation, missing reading, or reading that has not applied every
supplied send is unknown, not a pass. A local input or request-size refusal is
reported as a failed step even when Think retains an earlier reading. Matching a
fixture says nothing about whether the reading correctly interprets a person.
Original text preservation is checked separately. Injection resistance and exact
confirmation/handoff stay unknown because this runner does not exercise them.
The existing isolated `goal_confirmation_terminal.py` journeys check the latter
mechanics separately; their results are not invented or imported by this runner.

## Live runs

```bash
cargo +stable run --locked --example think_eval -- --live \
  --dataset /path/to/operator-dump.json --output /path/outside/repo/run-1 \
  --max-requests 4 --model provider/model-id
```

`--pi-command` (default `pi`) and `--thinking` (default `low`) are optional. Pi
options without `--live` are refused. Before anything starts, a live run refuses:

- a dataset whose `provenance.kind` isn't `operator_supplied`;
- a dataset without `permission`, a nonblank statement of who allowed these words
  to reach a provider. A flag alone is not permission;
- scripted `replies`, or a `--max-requests` of zero;
- output inside the repository or the operator's `~/.local/share/seed-me` or
  `~/.local/share/tinkery`.

The limit is one `CallBudget` held by Pi at the [drafting boundary](architecture.md#modules-today).
Each shaping call, decision review, failure and retry counts before Pi starts;
at the limit, the next request is refused unstarted, including a send's review.
A cancelled request counts only if Pi had started. A separate goal in the same
run shares the budget, and a run never reuses an output directory, so nothing
resets it. Without a budget, as in the app, requests are unlimited.

The report adds the Pi command, model, thinking level and prompt fingerprint, and
counts real calls. Provider charges are reported as unknown. Recognition and the
other human fields stay `null`: execution provenance is not a quality verdict.
No session is saved and no goal is confirmed.

Tests use a fake Pi executable, so the limit checks need no budget. Any real run
needs its own data permission and call budget from the operator.

## Human baseline form

The report's `human_baseline` is a form, separate from `automated_checks`. All
values start as JSON `null` (unknown). Copy the form for the person's current
workflow and for a later authorized Tinkery trial, using the same task:

| Field | What the person records |
| --- | --- |
| recognition | Does the final reading express their intent? Their words, not an automated score. |
| active_attention_seconds | Time spent typing, reading, deciding and correcting. |
| waiting_seconds | Time waiting for the tool, counted separately from active attention. |
| corrections | Their corrections or correction count; record which was used. |
| cost | Actual cost and currency, or subscription/unknown. |

Do not fill this form from fake reply timings. The separate `provider_cost`
reports zero real calls and zero provider charges for a scripted run only.

## Verification and structure

`cargo test --workspace --locked` includes the runner's regression tests and the
budget tests in `drafting_tests.rs`. They cover limits 0, 1 and 2 before Pi starts
(including failures, retries and cancellation), live refusals, review merging,
exact originals, unknown judgments and output location. Reverting the budget check
or the permission check fails them. No real provider runs.

Architecture diagnostics 1–3 stay stable: the budget lives once, at the only place
think calls a model, and scripted and live runs share it; the runner owns an explicit fake replay,
capture code is shared between examples, and the existing review input preparation
and merge are reused without duplicating their rules. Readable but invalid shaping
replies still receive a review before validation, just as in production. No provider framework or production phase is added.
