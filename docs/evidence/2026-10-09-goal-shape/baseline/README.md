# C16 Phase 1 — current-main baseline

**Stopped before implementation.** `main` / `8818e63cc4811fc0935409580589ddbdc0612b7f`.
Plan: [`../PLAN.md`](../PLAN.md). All six first boards displayed. No goal confirmation,
helper invocation or seed; no application, Seed Me, launcher or installed-binary edit.

**Provider count: 14/40 shared calls** — 7 shaping + 7 asynchronous log-only audits,
all `openai-codex/gpt-5.6-luna`, explicit `--thinking low`. **26 remain** for an
approved Phase 2. No retries/repairs or extra evaluators. Five advisory passes and
two flags are model judgments, not proof of semantic fidelity or operator recognition.
Advisory presentation flags were not enabled. Audits did not mutate captured boards.

## First-board table

Constraint assessment below is assistant inspection, not another provider judgment.
“Missing” means absent/weakened in the substantive reading/outcome/notes/candidates,
not erased from the intact original or its raw citation. Exact boards, raw responses,
normalizations, questions and full source/answer context are retained per case.

| Dump | Displayed reading | Question | Constraint observations |
|---|---|---|---|
| books | Remember which books you read during the year and what you thought of them, in one simple place. | Should this cover only books read in the current year, or become an ongoing record across years? | No clear hard-constraint loss: simple, one place, thoughts retained; short notes in outcome. “by december” not restated; “this year” becomes an open scope question. |
| onboarding-emails | Find a way to keep onboarding emails without making new users feel spammed, while addressing the rising unsubscribe rate and preserving marketing’s value from the sequence. | Should the main success measure be reducing the unsubscribe rate, reducing the feeling of being spammed, or balancing both equally? | **Five emails in the first week** is absent as current-situation content; the first-week period survives in one candidate. Not killing emails, annoyance, unsubscribe trend and marketing retained. Question introduces unsupported equal-weight framing. |
| trap | Learners should leave a wrong answer feeling smarter rather than dumber, by understanding what went wrong and receiving a supportive explanation without adding unwanted complexity. | When a learner gets an answer wrong, is the most important change that they understand the mistake, feel less judged, or both? | No gamification retained in notes; phone requirement is weakened to “especially on phones” in outcome. Unverified question-4 report retained in notes, but Kirra attribution is not restated there. Chat/cost worries remain candidates, rebuild stays uncommitted. “feel less judged” substitutes for “feel dumb.” No marketing tagline produced; this one result does not prove injection immunity. |
| ledger | Know within about five seconds each morning whether yesterday went well, primarily through useful changes per hour, while making job failures visible before they go unnoticed. | What specifically should make yesterday count as having gone well: a target or threshold for useful changes per hour, a comparison with another day, or something else? | **No dashboard** is not an explicit boundary in this first interpretation; dashboard appears only as a candidate risk. Five seconds/main signal retained; optional cost and 6am failure retained in outcome. **One number** remains a candidate, not a committed goal. |
| voice | Make starting nighttime thought dumps easier, possibly by using voice, reducing terminal friction, or both. | Which matters more to solve first: talking being faster than typing when tired, or opening a terminal feeling like work? | Both compatible possibilities remain in reading/candidates; tiredness/talking convenience retained in outcome. Question unnecessarily pressures a priority choice despite “maybe its both.” No hard constraint clearly dropped. |
| operator goal | A human-first system that compounds its own value while coordinating the full agentic software-development lifecycle—from brain dump, prompt, hunch, or intuition to a durable, verified, human-review-ready PR—while improving the health of the codebase and making the next change easier. | Is the true goal primarily the end-to-end experience and quality of the resulting PR, with the board/meta-harness as the means—or is building that board/meta-harness itself equally part of the goal? | Core end-to-end, compounding, rigor/coherence/durability/verification, human judgment and future codebase aims retained across reading/outcome. Restaurant/restaurateur metaphor is not restated in the prose; personal judgment survives in outcome. Question separates compatible means/ends. |

### Timing (seconds)

| Dump / action | Board available | Whole shape + audit step |
|---|---:|---:|
| books | 7.776 | 14.357 |
| onboarding-emails | 7.737 | 14.052 |
| trap | 9.067 | 16.356 |
| ledger first | 14.719 | 21.197 |
| ledger answer 1 | 16.066 | 25.868 |
| voice | 7.171 | 14.114 |
| operator goal | 11.163 | 19.028 |

The board was captured before waiting for its audit. Timing is harness-observed,
not a native terminal latency/recognition measurement.

## Ledger — all three prescribed actions, with the early-ending surprise

| Round | Fixed action | Actual current-main result |
|---|---|---|
| 0 | Original + F2 | One `well-definition` question (above). |
| 1 | `a failed job should override everything. cost only matters if its way off` + F2 | No questions remain. Reading: **Know within about five seconds each morning whether yesterday went well, with useful changes per hour as the main signal, a failed 6am job overriding the result, and cost shown only when it is unusually high.** Outcome now explicitly says **not a dashboard**. |
| 2 | `better than my usual week. no targets` + F2 | Because round 1 already had no question, current main classifies this as **added words**, asking **Should these added words be part of this goal, or stay separate?** No model request. |
| 3 | Skip (F8) | Scope question skipped; no model request. No active question remains, but the added usual-week/no-targets text stays excluded from the interpretation. Skipping does not establish complete intent. |

**First empty-question round: 1. Final active queue empty after round 3.** This is not
successful goal completeness: “went well” still lacks its comparative criterion,
and **way off** was narrowed to **unusually high** (only one direction of deviation).
“No dashboard” disappeared as an explicit constraint for the first board and returned
in round 1's outcome; cost/job detail appears outside the initial reading.

The initial live driver deliberately did not force an answer to a nonexistent
question and saved `02-answer-not-performed.json` / `03-skip-not-performed.json`.
To exercise the remaining fixed actions without a fresh stochastic rerun, the
continuation restored the **exact captured round-1 board and sources** through
current-main submit/apply with the two saved responses. It then performed the actual
F2/F8 actions. Equality is asserted before continuation; neither action requests a
model (an unexpected third reshape panics). This is explicitly **offline checkpoint
continuation**, not two new live provider responses or a new operator session.
See `ledger/fixed-rounds-continuation.json`, continuation captures/log and driver.
The original live journey and its not-performed notices remain preserved.

### Advisory evidence

- Onboarding flag: `Possible whole-board loss: we send five in the first week (source 1)`.
- Ledger round 1 flag: `Possible whole-board loss: the 6am job sometimes fails silently and i only find out days later (source 1)`.
- Other five audit records passed with `No whole-board loss identified by the model; not proof of understanding.`

Do not promote the ledger flag to proven loss: failure overriding the result and
being impossible to miss are still represented. Conversely, a pass does not establish
preservation of constraint strength, the person's voice or understanding.

## Method, commands and safety

Reused the prior library-pipeline live-eval method: actual current-main `PiHost`,
`BrainDump` submission/typing/history and immutable boundary, followed by real Pi.
Snapshots use Ratatui's test backend at 80x24, 100x30, 160x40 and 320x40. This is not
a native PTY/operator walkthrough. No launcher or installed executable was invoked.
The external harness's `--seed-session-root` maps to the same `with_seed_me(...,
Some(root))` configuration as the CLI and creates a distinct temporary root per case.
Every case and the continuation root stayed empty; no official helper was run.
Pi flags disable tools/extensions/MCP/approval/saved sessions; not an OS sandbox.

Inputs are byte-for-byte saved under `inputs/`; hashes are in `provenance.json`.
Books/onboarding/trap hashes match the prior subtraction originals. Ledger/voice are
the brief's prose with Markdown line wrapping removed, without newlines added.
The operator input is the origin node's evidence index 0, `Tinkery original 1 / reply
to None`, from session `27b55e8f-4a80-419a-99b5-7a26d6db1faa`. Its actual 746-byte
observed value, including its final newline, was preserved; historical answers were
not injected into this first-board evaluation. No entire ledger was copied to evidence.

Commands run (working directory `/Users/jondev/dev/active/tinkery`):

```
git log --oneline 429f6e0..main
git status --short
cargo build --lib --locked --message-format=json
rustc --edition=2024 <external driver.rs> -L dependency=target/debug/deps \
  --extern tinkery=<locked main rlib> --extern serde_json=<artifact rlib> \
  --extern crossterm=<artifact rlib> -o /tmp/tinkery-c16-baseline
TINKERY_C16_PI=/tmp/tinkery-c16-budget.py \
TINKERY_C16_TRACE=$PWD/docs/evidence/2026-10-09-goal-shape/baseline/call-trace \
/tmp/tinkery-c16-baseline <baseline> <baseline/inputs> \
  /Users/jondev/dev/active/factory/seed-me/skills/seed-me/SKILL.md \
  --seed-session-root "${TMPDIR%/}/tinkery-TEST-C16-phase1-session-roots"
rustc --edition=2024 <ledger-continuation-driver.rs> <same locked externs> \
  -o /tmp/tinkery-c16-ledger-continuation
/tmp/tinkery-c16-ledger-continuation <baseline/ledger> <official SKILL.md> \
  --seed-session-root "${TMPDIR%/}/tinkery-TEST-C16-ledger-continuation-root"
rustc --version
cargo --version
git diff --check
```

`build-command.txt` gives exact rlib paths. `run-command.txt` / `build.log` retain the
actual command and locked build output. `call-trace/*-argv.json` / `*-input.json`
retain exact Pi arguments/requests. Diagnostics contain raw stdout/stderr, decoded
candidates and normalization decisions. Wrapper rejects wrong model/thinking,
isolation or call kind; counts invocation attempts before exec under a file lock;
Phase 1 cap is 16 and shared cap 40. Persist this cumulative trace for Phase 2.

## Errors, limits and pause

Preserved in `preflight-errors.log`, all before any affected additional provider call:

- `OSError: [Errno 22] Invalid argument: '/Users/jondev/.local/bin/tinkery'` — the
  launcher is now a regular file, not the historical symlink. Fixed read-only hash
  collection, not the launcher.
- `error[E0616]: field 'skipped' of struct 'BrainDump' is private` — corrected the
  external harness to observe skip action/focused question, without changing the API.
- `AttributeError: 'list' object has no attribute 'get'` — inspection glob also matched
  argv arrays. Restricted inspection to named case directories; live eval unaffected.

No unresolved harness error; live background process exited 0, continuation exited 0.
The remaining prescribed *answers* could not be submitted as answers after premature
queue exhaustion; the exact actual added-words/skip behavior was recorded instead.
No forced model question, repair, rerun or invented confirmation.

`verification.json` records hashes before/after of the real ledger, launcher and
installed binary, all empty TEST roots, six displayed first boards, source integrity,
7 shape/7 audit decisions, and 14 consistent low-thinking traced calls. This baseline
is not a claim of operator recognition, clipboard permission, native rendering,
injection immunity or general model fidelity. Full code/PTY/Clippy/CI verification
belongs to authorized Phase 2; no application code changed here. **Pause for plan
review; no Phase 2 branch/build, merge or additional provider call is authorized.**
