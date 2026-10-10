# PR #8 revision 2 — bounded dogfood evidence

Authority: operator-approved `Tinkery C16 revision 2 brief.md` and `Tinkery PR 8 dogfood 2026-10-10.md` in the factory reports directory. Scope O1–O9, Q1–Q2, S1–S4/S6; **S5 receipt unchanged. Open PR, no merge or installation.**

## What changed

- Goal: `[Who] [predicate][, when].` No imposed can. Same composer owns reading, paper and frozen confirmation.
- `must` / `must_not` arrays replace Keep/Avoid; one constraint per line. Exact repeated subject/can/timing and exact echo checks are normalized. **Semantic paraphrase equality is not solved by these string comparisons.**
- Required missing parts get question priority (who/outcome/why/done_when). Missing or exactly repeated checks stay absent. Generic instructions require concrete, human-decided questions, no confirmation/checklist/agent-findable facts; native questions still sometimes echo field definitions.
- Separate-goal/deferred arrays survive reading, paper, review and existing origin evidence. No Seed Me schema/helper/lifecycle change.
- Removed update banner. Local line comparison alone supplies changed-ink / unchanged-muted emphasis; NO_COLOR uses dim. Any key, paste or send clears it; unchanged answers do not mute the block. Actual rendered cells are tested, not only a style helper.
- Open notes/misfits and parts share readiness. No self-labels in displayed values. Review has one warning paragraph: `type confirm to save. You can't edit it after.` Full review plus separate typed confirm remains required; reading/readiness/answers are not authorization.
- S3 exposed a queue-empty-but-open case: default F2 text with no active question refines an existing composed block, even while notes leave it unready. Explicit add-more still asks scope. The old open-without-question test was deliberately migrated to this contract; no confirmation authority was added.
- Readable prose/candidate objects under supports do not claim citations: retained as uncited notes, no fabricated highlight. Any source/quote/occurrence claim remains strictly checked. Fact/history validation runs **before** question-priority filtering and is rechecked afterward.

## Calls, failures and provenance

**24/24 new invocations, 15 shape / 9 asynchronous audits; cumulative trace 83. No extension requested or used.** The first 59 trace lines are hash-unchanged. A post-budget wrapper invocation was refused before native Pi exec; it made zero provider calls and added no trace line. All calls used `openai-codex/gpt-5.6-luna`, explicit low, resource-disabled TEST configuration (not an OS sandbox).

1. First run: 16 calls, including all nine original inputs and both onboarding answers. Six readable boards were rejected because narrative supports were incorrectly treated as malformed citations; their null-board captures, raw responses and error evidence are retained. Books/onboarding also duplicated the subject.
2. Deterministic boundary/subject normalization fixed that without an automatic retry or extra model repair call. `offline-normalized/` renders the saved raw responses through ordinary submit/apply at four sizes, **zero provider calls**. This is not nine new final-prompt live first boards.
3. `ledger-followup/`: zero-call restoration of the first raw ledger response, equality-checked against its current normalized board; live answer one plus audit. S3's open notes then made answer two a local scope addition, not a provider refinement. This failed sequence, including F8 exclusion, is retained.
4. `final-checks/`: fresh native mom and multi-topic CI first boards/audits. Four calls.
5. `ledger-refinement/`: zero-call ordinary-submit restoration of the exact native answer-one board and source list (no private-field injection), then live answer two/audit after fixing no-question refinement. Two calls; final F8 local/no focused question, not evidence of resolving notes. First empty-question round remains one.

The final prompt was **not** rerun live for all six original first boards. `current-code-replay.json` checks all 15 saved shape responses under final code (seven exact board serializations; six prior null boards now normalize, two subject-only surface changes). `offline-final/` contains final-runtime four-size readings and frozen reviews for every comparison block, including both ledger answers and both onboarding answers; all provider calls there are cached/zero-cost. Final post-eval runtime changes are formatting of frozen review construction, preserving pre-priority fact/history validation, and restoring the readiness guard on the no-question status while broadening the composer label only. The review caught a false-ready message; its earlier captures/diff are retained, the final reading now says parts remain open, and actual rendered-text regressions cover both status and label. No provider call was used to hide a failure. Native/cached captures, generated provenance and source hashes remain separate.

The three dogfood input fixtures were transcribed from the designated a1/b1/c1 left panes, joining terminal soft wraps with one space. `inputs/provenance.json` records exact fixture bytes and capture hashes; it does **not** claim recovery of uncaptured keystroke whitespace. Original six fixtures are copied byte-for-byte. The literal dogfood revision-1 blocks below are transcribed from a3/b1/c1; the original screens remain in `dogfood-captures/`.

Nine log-only advisory judgments: **five passes / four flags** (onboarding's five emails, vague onboarding, ledger metric priority, CI rename/release pain). They never mutate, veto, repair, delay display or authorize confirmation. Passes are model judgments, not proof.

## Revision 1 alongside revision 2

Revision-1 originals use its saved current-code replay (including its already documented duplicate-can surface normalization), not a claim that its final prompt was rerun. Revision-2 provenance is labelled per block. Missing lines are omitted, not filled for the report. Flags below distinguish invention/promotion/negation from intact originals.

### books

Revision 1:
```text
I can keep track of the books I've read this year.
Why: I usually forget what I read by december.
Done when: I can see them all in one place with a short note on what I thought of each one.
Keep: a short note on what I thought of each one.
Avoid: nothing fancy.
```

Revision 2 (current-code offline render of saved native responses):
```text
I see all the books I've read this year in one place with a short note on what I thought of each one, This year, before December.
Why: I usually forget what I read by december.
Must: The books I've read this year are included.
Must: Each book has a short note on what I thought of it.
Must not: It must not be fancy.
```

O1: no forced can or doubled I. Done when remains missing. **Invented/strengthened:** ‘before December’ turns forgetting by December into a deadline; ‘current calendar year’ is an assumption. **Negated:** `Must not: It must not be fancy` is a double-negative under its label, not a faithful constraint. One-place display may still describe a solution.

### onboarding-emails

Revision 1:
```text
New users can receive onboarding emails without feeling spammed, during the first week.
Why: five emails arrive in the first week and the unsubscribe rate keeps going up.
Done when: new users do not feel spammed.
Keep: marketing value; onboarding emails.
Avoid: killing the emails entirely.
```

Revision 2 (current-code offline render of saved native responses):
```text
New users do not feel spammed by onboarding emails, during the first week.
Why: The unsubscribe rate keeps going up, while marketing values the emails.
Must: onboarding emails are not eliminated entirely.
Must not: new users feel spammed.
```

Beneficiary/outcome are clearer, and the missing check is asked first. **Invented/strengthened:** `onboarding emails are not eliminated entirely` makes marketing affinity a hard constraint; the original says five emails in the first week, but five is missing from the goal. The second constraint repeats the outcome. Marketing value no longer appears as Must, but that does not make this faithful.

### trap

Revision 1:
```text
Learners can understand why they got something wrong, on phones.
Why: learners currently see a red X and move on; people feel dumb and leave.
Done when: learners leave a wrong answer feeling smarter not dumber.
Keep: no gamification.
Avoid: one more chatbot nobody asked for; rebuilding the whole quiz engine.
```

Revision 2 (current-code offline render of saved native responses):
```text
Learners leave a wrong answer feeling smarter rather than dumber, after a wrong answer.
Why: Learners currently see a red X and move on, feel dumb, and leave.
Must: work on phones.
Must not: include gamification.
```

Learners remains the beneficiary. **Strengthened:** `must not include gamification` turns a tentative worry into a prohibition. **Misclassified:** explanation/tone, the tutor panel and quiz-engine rebuild are mechanisms/alternatives, not established separate goals. The marketing instruction is kept aside, not executed. After-a-wrong-answer wording is still semantically repetitive although not an exact duplicate. The possible question-4 drop-off survives as an unverified note, not a fact. No injection-immunity claim.

Quiet aside: Choose whether to improve explanations or tone; Decide whether to add an AI tutor chat panel; Rebuild the whole quiz engine; Write a marketing tagline

### ledger

Revision 1:
```text
I can know whether yesterday went well or not within like 5 seconds, in the morning when I open Tinkery.
Why: the ledger report is useless to me; its a wall of numbers and i dont know what to do with it; the 6am job sometimes fails silently and i only find out days later.
Done when: yesterday went well or not is clear in like 5 seconds; better than my usual week.
Keep: useful changes per hour; cost only if its way off.
Avoid: a dashboard; silent failures; targets.
```

Revision 2 (live final answer after checked cached continuation):
```text
I know in about 5 seconds whether yesterday went well when I open Tinkery in the morning.
Why: The ledger report is a wall of numbers, so I do not know what to do with it; the 6am job sometimes fails silently and I find out days later.
Done when: I can tell that yesterday was better than my usual week, without using targets.
Must: Useful changes per hour is included.
Must: A failed job overrides everything.
Must: Cost matters only if it is way off.
Must not: The result must not be a dashboard.
Must not: The result must not be a wall of numbers.
```

Both prescribed answers now reach the model and the block: usual-week/no-targets in Done when; failure override and way-off cost in Must. No invented numeric targets. **Strengthened:** wall-of-numbers becomes a prohibition. **Negated:** `Must not: The result must not be a dashboard/wall of numbers` has double negation under the label. Job-failure detection is also placed aside despite being compatible with the main signal. Useful-changes-per-hour priority is weaker than ‘the one that matters’; the advisory flagged that. ‘Way off is unspecified’ remains a note and blocks readiness without asking for precision.

Quiet aside: Detect when the 6am job fails

### voice

Revision 1:
```text
I can capture thoughts at night more easily, when I’m tired.
Why: talking is faster than typing for me when I’m tired; opening a terminal feels like work; i don’t start dumps at all at night.
```

Revision 2 (current-code offline render of saved native responses):
```text
I start capturing thought dumps at night, including by voice when talking is faster than typing.
Why: Opening a terminal feels like work, and typing is harder when I am tired.
```

**Mechanism promotion remains:** ‘including by voice’ selects voice while the dump left starting versus voice uncertain. No invented threshold. The source uncertainty survives in notes, but native supports had candidate objects in the wrong field: they are retained uncited, including their raw JSON representation, not highlighted. That representation is a presentation limitation, not a clean human-voice success.

### operator-goal

Revision 1:
```text
I can coordinate work across the entire continuum of agentic software development lifecycle from the brain dump to a PR ready to be reviewed by a human.
Why: make the next change easier while considering the health of the code base; think about the forest, not just the trees.
Done when: a PR is ready to be reviewed by a human.
Keep: human first; rigor; durability; cohesion; verification; a system that compounds itself and serves as a meta harness.
```

Revision 2 (current-code offline render of saved native responses):
```text
I receive a delicious change ready for human review that considers the health of the code base and makes the next change easier.
Why: I want work across the entire continuum of agentic software development to have rigor, durability, cohesion, and verification, from brain dump, prompt, hunch, or intuition through a review-ready pull request.
Must: The work serves as a meta harness.
Must: The work is human first.
Must: The work considers the forest, not just the trees.
```

The review-ready change and code-health intent remain. **Loss/misclassification:** self-compounding the harness, coordinating the continuum and parts working together are compatible aims, but the model puts them aside as other goals. Meta harness/human first/forest language becomes Must. The human-as-final-taster role is still less explicit than the person’s words. No numerical invention; missing success check remains open.

Quiet aside: Build a system that compounds itself.; Coordinate the entire agentic software development lifecycle.; Make the parts of the workflow just work together.

### vague-onboarding

Revision 1:
```text
People who sign up can increase second-session returns.
Why: people sign up then never come back; like 70% never do a second session.
Done when: more than half come back within a week.
```

Revision 2 (current-code offline render of saved native responses):
```text
People who sign up return for a second session within a week.
Why: People sign up then never come back; like 70% never do a second session.
Done when: More than half come back within a week.
```

Outcome now says people return, not that they ‘can increase returns’. Both exact prescribed answers remain in originals; like 70%, second session, more than half and within a week survive. No invented percentage target. **Readiness limitation:** the model’s bookkeeping note ‘70% … not independently verified’ leaves readiness false although the question queue is empty. The goal block itself is complete; this is not presented as full semantic success.

### mom-scams

Revision 1:
```text
My mom can tell if a text is fake before tapping anything, before tapping anything.
Why: she keeps getting scam texts and almost clicked one yesterday.
Done when: she can tell if a text is fake before tapping anything.
Keep: one step; no app installation.
Avoid: lecturing her.
```

Revision 2 (fresh native first board):
```text
My mom tells if a text is fake before she taps anything.
Why: my mom keeps getting scam texts and almost clicked one yesterday.
Must: She won't install apps.
Must: It has to be dead simple, like one step.
Must not: She must not be lectured.
```

No repeated timing, forced can, or repeated Done when; the distinct check is missing and asked about. No iPhone-version question. One-step and no-install intent survive. **Misfiled/negated:** no installation is a negative statement under Must, and `Must not: She must not be lectured` double-negates the preference. The unknown iPhone version is retained as an open note even though it is agent-observable; no readiness claim.

### multi-topic-ci

Revision 1:
```text
I can align the clippy version on CI with the laptop version, on every CI run.
Why: things pass locally and fail on github; happened twice this week.
Done when: things pass locally and fail on github no longer happens.
Keep: the CI time of like 2 minutes on mac.
Avoid: copying binaries around by hand; running an old build.
```

Revision 2 (fresh native first board):
```text
I get consistent clippy results locally and on GitHub.
Why: The clippy version on CI differs from the laptop, so checks pass locally and fail on GitHub; this happened twice this week.
```

Outcome is consistent results, not aligning a version. Release-build and tab-rename goals remain visibly aside, not in Must not. The two-minute CI time is NOT promoted to Must; no CI/toolchain change was made. No version/time fact question and no bundled scope question. **Loss:** ‘ledger maybe’ and the release-build pain (manual copying, old build for an hour) are not fully carried by the short aside labels, although originals remain. Done when is missing; the question remains generic. Priority is treated as clippy-first from the explicit ‘what bugs me’ wording, rather than asking which separate goal comes first.

Quiet aside: Rename the measure tab; Automate release builds after a PR merges

## Checks and remaining limits

Local Rust 1.99: **131 application / 41 Pinstar unit tests**, integrations/examples, two compile-fail docs, fmt, strict Clippy, build and all Python suites/four-size PTYs/official isolated TEST journeys. Fourteen fresh-copy probes each pass baseline and fail the exact regression under reversion: forced can, repeated check, timing/subject duplication, muting without change, NO_COLOR rendered ink, required-part priority, open-note readiness, per-line constraints, frozen deferred retention, history before priority, rendered false-ready status/no-question label, and eval-phrase leakage. Failed fixture migrations and the first probe-path mismatch are retained under validation.

No live confirmation/helper/seed occurred. Seven live/replay TEST trees have no ledger; official confirmation tests use separate isolated roots and explicitly say AUTOMATED TEST, NOT OPERATOR. Protected launcher, installed binary, existing ledger and dogfood session hashes remain unchanged. CI toolchain/workflow and canonical Seed Me untouched. No S5 receipt rollout.

**Not a clean semantic acceptance result.** Important unresolved deviations: double-negated Must not values; inferred constraints; candidate mechanisms presented as goals; compatible aims split aside; generic check questions; agent-observable/bookkeeping notes keeping complete blocks unready; shorthand asides losing qualifiers; raw candidate JSON in an uncited review note. No runtime semantic veto or automatic repair was added. There is no budget left for further evaluation; any extension requires a new Claude-session message. Green tests/CI do not establish human recognition, rendering on the operator's host, clipboard permission, universal fidelity or injection immunity.

Stop for PR review; no merge, installation, further model calls or next-slice work.
