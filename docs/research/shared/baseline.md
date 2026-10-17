# Human baseline

The record for [#109](https://github.com/jon-devlapaz/tinkery/issues/109): what
one accepted, checked change costs the operator today, before Tinkery's full-loop
views exist. [#116](https://github.com/jon-devlapaz/tinkery/issues/116) repeats it
with those views and compares. Every value starts unknown; only the operator fills
judgments and times. Nothing here is a quality score.

## Plan

Status: **permitted.** The operator approved every choice below on 2026-10-16,
in the [project thread](https://claude.ai/code/project/chan_01HpLSkmbLdxh9PX1LVX2D7Y?thread=cmsg_01HpLSkmbLdxh9PX1LVX2D7YPKqTjBRmwTbi2qAnC7XvTw).

| Choice | Plan | Operator's decision |
| --- | --- | --- |
| Tasks | Wordle for tink-arcade (operator's swap for #176), [#177](https://github.com/jon-devlapaz/tinkery/issues/177), [#154](https://github.com/jon-devlapaz/tinkery/issues/154): small, bounded Tinkery changes with fake-only tests | approved |
| Workflow | Today's: say the change in Think, confirm the goal, copy it to Pi, Pi opens a PR, CI runs, the operator reviews and accepts or asks for a correction | approved |
| Budget | Up to 5 Think drafting calls per task; Pi implementation on the operator's existing subscription, cost recorded as subscription | approved; raised to 16 calls for Wordle |
| Threshold for #116 | The trial wins if active minutes per accepted change fall by at least a third, with no task lost that the baseline accepted | approved |

Choosing these tasks also approved building each one as a sample, through the
workflow above.

## How to record

Per task, the operator notes minutes at the end of each active stretch, by kind:

- **explaining:** saying or typing what they want, including repeats;
- **deciding:** answering questions, confirming the goal, approving;
- **reviewing:** reading the PR, checks and result;
- **correcting:** asking for and checking a correction.

Waiting is noted separately. Session timestamps are not attention. A forgotten
stretch stays unknown, never estimated.

The agent records, from the record itself: the goal's exact words, whether the
saved and handed-off goal stayed identical, the PR's final revision and its checks.
The operator alone records whether the goal kept their meaning and the verdict.
Private dump text stays local; this file keeps only summaries and references.

## Results

| Task | Goal kept meaning (operator) | Goal exact through handoff | Final revision and checks | Verdict | Active min (explain/decide/review/correct) | Waiting min | Corrections | Repeated explanations | Cost |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Wordle | yes | yes: saved and handed to Pi unchanged | [tink-arcade #22](https://github.com/jon-devlapaz/tink-arcade/pull/22) at `bcc658e`; `check_all` passed, played at 80×30 | accepted, merged | unknown | unknown | 4 operator-requested | budget raises typed after card taps (3) | 14 Think calls; Pi on subscription |
| #177 | unknown | unknown | unknown | unknown | unknown | unknown | unknown | unknown | unknown |
| #154 | unknown | unknown | unknown | unknown | unknown | unknown | unknown | unknown | unknown |

Accepted, checked changes per active operator hour: unknown until every row has a
verdict and recorded minutes. Failed or abandoned tasks stay in the table with
their effort.

Operator confirms this baseline is suitable for #116: unknown.

## Findings

Wordle, 2026-10-16/17:

- **Think budget.** Each answer cost two calls (draft plus review), so 5 calls fit two
  answers. A refused review failed silently and still showed the draft.
- **Lost session.** Think ended when its host process exited; nothing survives until
  confirm, so all four answers were replayed (5 more calls).
- **Readings vary.** The same first words read as empty once and "I play Wordle" once.
  The outcome line was reworded from the operator's words; Done when stayed close.
  An answer of "no requirement" stayed recorded as an open decision.
- **Loose goal, no Shape.** "Works reliably" went to Pi unshaped. Pi's first build
  passed its own tests but missed real guesses, typeable letters, Wordle key states
  and readable colors; the operator's play and a terminal replay found them, not tests.
- **Permission friction.** The device accepted budget raises and the build go-ahead
  only as the operator's own typed words, not card taps.

