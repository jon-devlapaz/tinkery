Round: 1
Reviewer session: /root/review_862_code

## Situation

Changes are required. Two Important findings block review completion. No merge is authorized.

Reviewed candidate: `f17b4b95b156f1622be3926a5b54535b95534036`.
Comparison base: `1d14afcd8c711285f5b909130a6bfcd870855e31`.
Run: `e9065256-ca5f-4d11-ba8d-a74b1c30f153` (`issue-862-tend-correction`).
Review checkout: `/Users/jondev/dev/worktrees/tinkery/codex/review-issue-862-round1`.
The reviewer did not write the candidate. There is no bound seed in this issue-based run.

## Background

The approved brief requires exact correction text and a capture screen that shows the reviewed proposal and repository.
The review covered the implementation diff, brief, checklist, source, tests, saved screens, verification log, and reversion logs.
The verification receipt names `d21409c3b0cfdd4445370ba463ce6f481a99d651`. Its source and test files match the reviewed candidate.

## Assessment

### Important: P2 — Reopening a correction removes its final newline

Location: `src/tend/connection/panel.rs:213`.

`TextArea::from(words.lines())` drops the final empty line from stored correction text.
The save action joins those editor lines with newline characters. A replacement therefore changes stored text without an edit.
This violates the exact operator text contract. This is a durable data change, not just a display difference.

Observed reproduction through the real terminal:

1. Discover a proposal with the existing synthetic fixture and fake provider.
2. Press `f` and paste `Exact guidance.\n` with bracketed paste.
3. Press F2. The correction file contains `"Exact guidance.\n"`.
4. Press `f` again, then F2 without editing.
5. The correction file now contains `"Exact guidance."`.

The request log contained one fake request throughout. The correction actions made no request.
Preserve the complete stored text when loading the correction editor. Add a public test for this replacement path.

### Important: P2 — The correction capture screen omits the required review context

Location: `src/tend/connection/panel.rs:282-335`.

The editor branch renders only a heading, hint, text field, notice, and footer. It returns before rendering the reviewed proposal.
The brief explicitly states: “The capture screen shows the proposal being reviewed”.
The operator cannot inspect that proposal while writing a correction. Escape discards the draft before returning to the proposal.

The same branch puts the full repository path in a single unwrapped line.
In the observed 100×30 TEST screen, the heading ended with `tinkery-TEST-review-` and hid the repository name.
The brief also requires showing the repository before explicit Save correction.

Observed reproduction: discover a proposal, press `f`, and inspect the editor at 100×30.
No proposal text appears. A normal macOS temporary TEST path does not fit the heading.
Keep the reviewed proposal and full repository inspectable during capture. Test the capture screen at 80×24 and 100×30.
The retained disclosure screenshots show the screen after saving, so they do not test this requirement.

## Checked protections and limits

The source retains one ordinary tool-free request. Correction storage uses the existing repository lock and atomic replacement.
Input equality includes correction revision and text. The pending save checks current input before publication.
The version readers reject forbidden or missing correction members. Legacy pending saves preserve their old prompt identity and origin format.
Corrected origins freeze guidance separately from source quotations. Publication continues through the existing shared saver.
These source checks produced no further Important finding.

The retained full verification reports 1124 Rust tests passed, one ignored, 39 Node tests passed, and 85 Python files passed.
Formatting and Clippy passed. The review did not rerun the full suite.
The retained PTY evidence covers 80×24 and 100×30 with fake providers.
Both retained reversion logs show the intended failures for correction input and pre-publication freshness.
These checks establish mechanics, not correction relevance or operator acceptance.

The reviewer ran the focused reproduction with synthetic TEST roots and the guarded TEST HOME launcher.
It used the existing candidate binary from the implementation checkout. The review checkout has no built binary.
No real provider, operator goal, real audio, or application storage was used.
`principle-prove-it-works` required checking stored bytes and the actual editor screen, rather than relying on the passing suite.

## Recommendation

Return these two findings to the worker for small repairs and public regression tests.
Refresh full verification after code changes, then review the repair diff.
Open the requested PR after review. Do not merge it.
