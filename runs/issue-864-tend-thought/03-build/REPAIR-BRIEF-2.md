# Repair brief 2 for step build-fix-2 (supervisor, 2026-11-15)

Read this file first, then `05-deploy/output/REVIEW-findings.md` (independent review, round 1, verdict: changes required).
Approved scope is unchanged: `brief.md` (issue #864). You have at most 40 responses. Do the items in order.

## Item 1, blocking: finding I-1

In link mode in the repository garden, the graph is drawn at one place and hit-tested three rows lower.
`render` in `src/tend/mod.rs` moves the graph down only when `!tend.detail && !tend.is_linking() && tend.connection_scope()` and the summary is not empty.
`pointer` in the same file moves the hit rectangle down when `connection_scope()` and the summary is not empty, without the other two conditions.
The supervisor confirmed both conditions in the code. The reviewer reproduced a needs link written by a click on empty space.

- Use one condition for both paths: one small private helper that `render` and `pointer` both call. Do not copy the condition a second time.
- Add a terminal regression in repository scope with link mode: a click on a visible goal writes the intended needs link, and a click three rows
  below it writes nothing. Run it at 80×24 and 100×30 with colour off, with temporary TEST roots.
- Prove the regression fails when the helper is bypassed in `pointer`, then restore the source and rebuild. Record that probe.
- Check every other place that uses the garden body rectangle (drag, hover, release, scroll, selection) for the same mismatch, and say what you checked.

## Item 2, small: finding N-1

After `persist_noclobber` succeeds, a failed directory sync must not say "Thought not saved". Say that the thought is saved and that
persistence is uncertain, in the same manner as the correction store's message. Keep the words visible. No new mechanism.

## Item 3, small: finding N-3

Name the thought keys in Tend's help text, where the other garden keys are explained. Do not change the garden key bar.
Update `docs/design/shape-and-handoff.md` only if the help contract is written there.

## Not in this repair

Finding N-2 (Ctrl+C in the reader) and the reviewer's questions Q1 and Q2 are operator decisions. Leave them as built.

## Checks and boundaries

- Focused: `tests/tend_thought_terminal.py`, `tests/tend_terminal.py`, `tests/tend_repo_terminal.py`, `tests/tend_connection_terminal.py`,
  `tests/tend_correction_terminal.py`, `tests/goal_confirmation_terminal.py`, `tests/test_documentation.py`, fmt and Clippy with `-D warnings`.
- Full: `python3 scripts/full-check.py` from the checkout root, unchanged, with its own temporary TEST HOME in the system temporary directory.
  Never place TEST roots inside the checkout or in the operator HOME.
- Checklist marks made on the older candidate need a re-check: after the full check passes on committed work, mark again each item that this
  change can affect, with `sdlc.py mark`. Do not run `verify`, `step`, `stage` or `decide`.
- No scope beyond these items. No provider calls, no subagents. After three failed fixes of one defect, stop and report.
- Commit before you end. Write the report at the path in your step contract, result first, with counts.
