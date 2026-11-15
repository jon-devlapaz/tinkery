# Issue 862 build handoff

## Situation

The bounded correction implementation is complete. Final SDLC verification passes on d21409c. No provider calls or operator goals were used. Do not merge.

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

Final focused logs and rendered terminal screens are under this output directory. The full configured check passed 1124 Rust tests, one ignored; 39 Node tests; all 85 Python files; formatting and Clippy. The first receipt awaited done-11 only. The final receipt now passes on corrected commit d21409c with all 11 attestations.

## Reversion evidence

Both probes compiled, failed the intended public terminal assertion, and restored the source. Their commands ran through tests/test_launch.py with a TEST HOME.

- `reversion-correction-input.log`: replace snapshot input's `correction` with `correction: None`, build, then run `tend_correction_terminal.journey((80,24))`. Saving feedback no longer permits a fresh request, and the expected F2 action is missing.
- `reversion-correction-freshness.log`: replace pending publication's `current == record.input` with `current.sources == record.input.sources`, build, then run `tend_correction_terminal.stale_goal_and_pending()`. Changed correction no longer blocks publication and Shape opens instead of showing the expected refusal.

The temporary mutation driver was outside the repository. The committed terminal test is the repeatable proof tool. Sources were restored and rebuilt before final focused checks.

## Failures resolved

The supervisor observed multiline correction text joined together by Line conversion. Raw spans now retain each line. A visible-row assertion verifies the second exact line, and the common current/supplied correction appears once.

A maximum-length correction hid the input-limit refusal and Clear action. Critical refusal and action lines now precede the complete guidance. Scroll remains visible. The adjusted terminal observation checks the single guidance label before scrolling instead of looking for a removed duplicate.

## Skills and limits

`principle-build-the-lever` produced the repeatable correction terminal suite. `principle-prove-it-works` required durable file inspection and settled terminal screens. `writing-for-agents` kept the behavior contract in the existing handoff design. `unslop` kept product labels and documentation direct.

Fake responses establish transfer mechanics, not relevance or usefulness. A human trial with different relevant goals and an unrelated case remains separately authorized work. Permission-based refusal requires a non-root process; the ordinary local user exercised it. No actual power-loss claim is made.

## Reader limit

Version dispatch parses JSON values before strict field validation. Unknown/required fields, version/tag mismatches, and duplicate origin entries refuse. Duplicate object keys follow JSON Value normalization rather than a new duplicate-key policy, consistent with the ledger parser. This is not tamper-proof storage.

## Disclosure correction

Inspection found that panel refresh read the edit snapshot separately from request input. A concurrent write between reads could disclose different guidance from the input sent. One correction read now supplies both states, and rendering uses Input.correction. The edit snapshot serves only stale mutation checks. This removes the divergent refresh state structurally.

The public test shows A, externally writes B, and verifies that F2 refuses without a call. Reopening shows B, and the next captured request contains exactly B. It does not pause between internal reads or use test-only access. Both reversion probes were refreshed after this change.

The second verification run on a01a524 was interrupted with SIGINT to make this correction. Its interrupted receipt is not passing evidence. The earlier complete check passed, and final verification must run again on the corrected commit.

## Final verification

The full configured SDLC verification passed on d21409c. It reports 1124 Rust tests passed and one ignored, 39 Node tests passed, all 85 Python test files passed, formatting passed, and Clippy passed with warnings denied. The receipt is current. Both required reversion probes failed as expected before restored-source checks.

Independent review and PR publication belong to the supervisor. No push, PR, merge, real provider request, real audio, or operator-goal access occurred in this worker.

## Independent review repairs

Round 1 found two Important defects in the correction editor. Reopening used `str::lines`, which discarded final blank lines. Loading with `split('\n')` now preserves the original text. Public tests save one and two final newlines, replace without editing, and inspect exact durable text after restart. No extra request occurs.

The capture view omitted the proposal and clipped the repository. It now wraps the full repository and proposal context above the existing editor. Tab changes focus to read-only context scrolling and back without discarding the draft. Public terminal tests at 80×24 and 100×30 inspect the proposal, repository, supporting quotes, focus hints, and footer. A longer repository test scrolls every path component and the proposal into view before saving.

Both new reversion probes compiled successfully and failed the intended public assertion. `reversion-exact-correction-end.log` restores `words.lines()` and detects changed exact text. `reversion-capture-context.log` restores the reviewed panel and detects the missing proposal. Restored-source focused checks follow in `review-repair-focused.log`. Captures are saved under `screens/`.

The first new capture assertion joined wrapped lines without spaces. It failed on correct displayed text. The assertion now normalizes screen whitespace. This was a test assertion defect, not an additional product defect.

These repairs add no provider calls or storage format changes. The existing design contract documents context navigation and final-line preservation. Human usefulness remains unobserved. Final full verification passed on repair commit 89b6509.

The first full repair verification failed the architecture source-line limit. One new format expression exceeded 200 characters. Splitting that expression resolves the observed cause without changing behavior. All seven architecture tests then passed. The failed full log remains in `review-repair-first-full-failure.log`.

## Final review-repair verification

The full configured SDLC verification passed on 89b6509. It passed 1124 Rust tests with one ignored, all 39 Node tests, all 85 Python files, formatting, and Clippy with warnings denied. The current receipt covers the repaired source. The only failed full repair run was the source-line limit described above.

The supervisor inspected the 80×24 capture and scrolled screens and found the repository, proposal, source context, and retained draft readable. Independent review of the repaired candidate remains the supervisor's next step. No push, PR, merge, or real model request occurred in this worker.

The supervisor reproduced a CI-only test dependency in a depth-one clone: the old prompt Git object was unavailable. The legacy test now records its verified SHA256 with the #861 source commit. Legacy recovery passed with the fixed identity. CI fetch policy is unchanged.

## Linux CI input diagnosis

PR #863 CI failed at the oversized correction test on 135fd43. The screen had an empty editor and no refusal. A clean export reproduced that exact failure on Linux/aarch64 with the cached Rust 1.95 Debian image, Python 3.11, two CPUs, TEST roots, and fake responses. Locked dependencies missing from the Mac cache were fetched into a disposable TEST cache. Host tools and caches were unchanged.

Temporary tracing showed a new completed frame for each editor opening. All 2061 paste bytes were written. The app received Ctrl-A and Ctrl-K, but neither the 2049-byte paste nor F2. A controlled combined write of 2066 bytes left exactly 1042 bytes unread at the terminal. Crossterm returned key events from its first 1024-byte read before draining the remaining input. A separate case waited for the clear-key frame, observed the full paste, and then received the same oversized refusal. See `linux-input-original.log`, `linux-input-traced.log`, `linux-input-events.log`, and `linux-input-cause.log`. Temporary product tracing was removed.

The correction test helper now waits for a completed clear-key frame before pasting. It checks write lengths and waits for pasted words before Save or cancellation. The 2049-byte refusal, unchanged storage, retained draft, and no-extra-request assertions remain intact. The repaired Linux focused cases passed, including the 32 KiB input refusal. This changes test synchronization only. The queued clear-plus-large-paste event limitation remains outside this feature's scope. No product or dependency source changed.

The full correction suite passed on Linux as the existing unprivileged `nobody` user. This includes failed-write retention and retry, with no permission case skipped. See `linux-correction-suite.log`. The first ordinary-user container setup used a numeric UID absent from its password database, so the TEST launch guard refused before launch. Using the image's existing user resolved setup. The earlier root run passed its other cases but skipped permission refusal.
