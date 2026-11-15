# Issue 862 build handoff

## Situation

The bounded correction implementation is ready for full verification. No provider calls or operator goals were used. Do not merge.

## Background

One exact correction is saved per repository. Later explicit connection requests include current goals and those words. The response remains one connection or null. No old goal context, wiki, relevance report, or automatic request was added.

## Assessment

Tend owns the private correction record and edits. A bounded read and file identity token reject stale Replace/Clear actions. The existing repository lock excludes requests, correction writes, and pending goal saves. New discoveries include the exact correction in input identity.

The shared origin reader accepts strict v1 and v2 evidence. Only corrected goals use v2. Legacy records and pending markers retain their wire fields. New explorations require correction, including explicit null; v1 refuses the member. The frozen origin passes through the existing receipt, staging marker, and ledger. The shared publication algorithm is unchanged.

Legacy pending recovery keeps the old prompt identity across this upgrade. It still checks goals, repository, correction absence, and the other request settings. Published goals remain openable even if the reusable correction is later malformed.

Architecture diagnostics 1, 2, 3, and 6 remain bounded: Tend owns one new storage module; the shared origin type owns compatibility; callers use the existing reader/save interfaces; publication mechanics stay hidden.

## Evidence so far

Public terminal journeys run at 80×24 and 100×30, without color, in TEST roots. They cover exact multiline correction display, Save/Replace/Clear, restart, different-goal request inputs, unrelated/no-result retention, no request on feedback edits, v2 handoff, settled Shape, oversized editor recovery, failed filesystem-write retry, stale editor and Clear, repository-lock refusal, malformed correction recovery, and unsafe paths.

The legacy terminal case reconstructs #861's wire format with its actual committed prompt hash. A pending receipt resumes without another provider request. Rust saver tests additionally check byte-stable v1 receipt serialization and ownership markers, v2 interrupted staging/restart/history, and seven corrupt evidence cases.

The terminal case checks a correction changed after goal editing and again before pending publication. Neither stale goal can publish. Strict exploration cases reject v1 extra fields, v2 missing correction, an unknown version, and a missing pending correction. Complete variable input over 32 KiB refuses without another request.

Final focused logs and rendered terminal screens are under this output directory. Full verification remains pending.

## Reversion evidence

Both probes compiled, failed the intended public terminal assertion, and restored the source. Their commands ran through tests/test_launch.py with a TEST HOME.

- `reversion-correction-input.log`: replace snapshot's `correction: self.correction()?.value()?` with `correction: None`, build, then run `tend_correction_terminal.journey((80,24))`. Saving feedback no longer permits a fresh request, and the expected F2 action is missing.
- `reversion-correction-freshness.log`: replace pending publication's `current == record.input` with `current.sources == record.input.sources`, build, then run `tend_correction_terminal.stale_goal_and_pending()`. Changed correction no longer blocks publication and Shape opens instead of showing the expected refusal.

The temporary mutation driver was outside the repository. The committed terminal test is the repeatable proof tool. Sources were restored and rebuilt before final focused checks.

## Failures resolved

The supervisor observed multiline correction text joined together by Line conversion. Raw spans now retain each line. A visible-row assertion verifies the second exact line, and the common current/supplied correction appears once.

A maximum-length correction hid the input-limit refusal and Clear action. Critical refusal and action lines now precede the complete guidance. Scroll remains visible. The adjusted terminal observation checks the single guidance label before scrolling instead of looking for a removed duplicate.

## Skills and limits

`principle-build-the-lever` produced the repeatable correction terminal suite. `principle-prove-it-works` required durable file inspection and settled terminal screens. `writing-for-agents` kept the behavior contract in the existing handoff design. `unslop` kept product labels and documentation direct.

Fake responses establish transfer mechanics, not relevance or usefulness. A human trial with different relevant goals and an unrelated case remains separately authorized work. Permission-based refusal requires a non-root process; the ordinary local user exercised it. No actual power-loss claim is made.
