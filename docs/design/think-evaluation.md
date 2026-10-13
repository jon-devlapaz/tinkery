# Think evaluation: fake runs (#62)

This first slice checks repeatable mechanics with explicitly supplied text and
scripted replies. It cannot measure model quality or human recognition. It adds
no prompt, changes no production interaction, and has no live-provider option.
The historical `live_brain_dump_check` example still requires separate data-use
and call-budget authorization and remains outside the zero-call suite.

Run from the checkout being evaluated, rebuilding the example:

```bash
eval_root=$(mktemp -d)
cargo +stable run --locked --example fake_think_eval -- \
  --dataset tests/fixtures/fake-think-eval.json \
  --output "$eval_root/result" --max-requests 2
```

The named fixture is synthetic test data. The runner refuses missing input;
it never substitutes that fixture. Output must be a new absolute directory below
the system temporary directory, with an existing parent. It writes captures and
`report.json` there only. It never configures Seed Me, confirms a goal, or writes a
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

Every fake request consumes one unit before its scripted reply is read. Shaping
and decision review are counted separately, including failures and explicit
retries. Zero refuses the first request. Reaching the cap refuses the next
request without consuming its reply. A failed or refused decision review leaves
the shaping reply in place, as production does. No request retries itself.
Reply delivery and decoding are reported separately: a delivered invalid reply
still counts as a request, and the step reports the failed reading.

Each step reuses the live example's capture helper: sources, reading, cited spans,
paper, and 80×24, 100×30, 160×40 and 320×40 text snapshots. These are mechanics
captures, not terminal journeys or human validation.

The report names the dataset path, fingerprint and declared provenance, fake
execution, and zero real calls. It records the Git checkout revision and dirty
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
reports zero real calls and zero provider charges for this fake run only.

## Verification and structure

`cargo test --workspace --locked` includes the fake example's regression tests.
They cover the request cap before invocation (including failures and retries),
fake-only input, review merging, exact originals, unknown judgments and temporary
output. Reverting the cap check must fail those tests. No live example executes.

Architecture diagnostics 1–3 stay stable: the runner owns an explicit fake replay,
capture code is shared between examples, and the existing review input preparation
and merge are reused without duplicating their rules. Readable but invalid shaping
replies still receive a review before validation, just as in production. No provider framework or production phase is added.
