Round: 2
Previous review: d3bf9ab
Reviewer session: /root/review_862_round2

## Situation

The repair review passes. Both previous Important findings are resolved. No new Important findings were found.
The operator authorized a PR and explicitly forbade merging.

Reviewed candidate: `0d9ad54`.
Repair comparison: `d3bf9ab..0d9ad54`.
Run: `e9065256-ca5f-4d11-ba8d-a74b1c30f153` (`issue-862-tend-correction`).
Review checkout: `/Users/jondev/dev/worktrees/tinkery/codex/review-issue-862-round2`.
This reviewer did not write the candidate. This issue-based run has no bound seed.

## Background

This round covered the repair diff and the two previous findings, as required by `_shared/REVIEW.md`.
The approved brief requires exact correction words and inspectable proposal and repository context during capture.
The repair changes the existing connection panel, its public terminal tests, and its design contract.
It adds no storage service, provider request, or publication path.

## Assessment

### Resolved: Untouched replacement removed final newlines

The correction editor now loads text with `split('\n')`, which preserves final empty lines.
The reviewer ran the public terminal regression against the current implementation binary.
One final newline survived an untouched replacement. Two final newlines survived replacement and restart at both terminal sizes.
Each test checked the saved correction text directly and confirmed that correction actions added no request.
The retained `reversion-exact-correction-end.log` shows this regression failing when `words.lines()` is restored.

### Resolved: Capture hid the proposal and clipped the repository

The editor now shows the repository and reviewed proposal in a wrapped context area.
Tab switches to context scrolling while retaining the draft. Escape still cancels the editor before other key handling.
The reviewer ran the capture checks at 80×24 and 100×30 and inspected the retained capture screens.
The checks found the proposal and full ordinary repository path before saving.
Scrolling exposed the remaining proposal context without changing the draft or starting a request.
The long-path regression exposed every repository segment and the proposal at 80×24, then saved the unchanged draft.
The retained `reversion-capture-context.log` shows the capture check failing with the previous panel.

### Checked: Legacy test works without Git history at runtime

The test now uses the recorded legacy prompt digest instead of requiring an old commit during execution.
The reviewer independently hashed `1d14afc:src/tend/connection-prompt.txt`. It matches the committed digest.
The focused legacy pending recovery test passed, including malformed-record Clear and unsafe-path refusal checks.

### Verification

The final receipt names `2d1afcacc62ec557c8627fc3d47ee41dc31f72da` and reports success.
Its source and tests match the reviewed candidate. `sdlc.py status issue-862-tend-correction` reports current verification.
The retained full log reports 1124 Rust tests passed, one ignored, 39 Node tests passed, and 85 Python files passed.
Formatting and Clippy passed. The full log includes the repaired correction terminal test.
The reviewer did not repeat the full suite or rerun mutation probes.
The reviewer checked the retained mutation failures and independently reran the repaired public paths.

## Limits and recommendation

The focused checks used the implementation checkout binary, synthetic fixtures, fake providers, isolated TEST roots, and the guarded TEST HOME launcher.
They establish mechanics, not useful correction transfer, model relevance, or operator acceptance.
No real provider call, operator goal, real audio, or operator application storage was used.
No code, verification receipt, checklist mark, PR, or release state was changed by this review.
The `principle-prove-it-works` skill required direct checks of saved text and terminal behavior before passing the repairs.

Open the authorized PR and check its required CI results. Do not merge it.
A separately authorized trial remains necessary to judge usefulness.
