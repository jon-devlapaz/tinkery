# Thought save feedback

The worker implements issue #874 only. Think keeps the accepted return to Tend. Issue #866 remains separate.

Think carries the exact saved words and new-or-duplicate result to Home. Home opens Tend. The thought reader owns selection and save-message wording. Storage and unsuccessful-save behavior stay unchanged.

The garden reserves space for the summary's actual line count. Rendering and pointer input use the same count. Opening the reader clears the save message and retains the selected thought.

Architecture diagnostics 2 and 6 improve: the small handoff passes facts, while Tend owns presentation. Diagnostic 4 is maintained through comments on the public exit fields and thought-view methods. No storage interface or record format changes.

## Evidence

- `04-test/output/baseline-failure.txt` records the unfixed terminal failure. Three thoughts existed, but the garden showed the first alphabetical thought and no save message.
- Independent reviewer `/root/review_874_build` accepted that regression on 2026-11-15 after its own reproduction. `test-lock.json` protects the unchanged test and its helpers.
- `04-test/output/reversion-failure.txt` records the same failure after dropping the successful save handoff. The worker restored the code and reran all four feedback cases.
- `04-test/output/screens/` holds eight terminal text screens: new and duplicate saves, with and without an unreadable record, at 80×24 and 100×30 without colour.
- The supervisor inspected the tightest warning layouts. The terminal test also clicks the visible second goal and checks its selection.
- Independent review found that entering Shape preserved the save notice in the retained Tend view. `04-test/output/departure-failure.txt` records the added public-interface regression. The reviewer accepted it on 2026-11-15. The original locked baseline remains unchanged. Both retained-Tend departures now use one private Home method that clears the save notice before opening Shape.

## Skills used

`principle-build-the-lever` and `principle-prove-it-works` led to repeatable real-terminal checks and retained screen output. `unslop` kept messages and evidence descriptions direct. `writing-for-agents` kept the changed design contract in its existing authoritative document.

These checks use synthetic records, temporary TEST roots, and local stubs. They do not establish operator acceptance or real-thought usefulness. No real model or provider request is authorized or made.

The preliminary `04-test/output/full-check.txt` passed before the departure repair. Final verification runs the full check again on the committed candidate and records its binding in `04-test/output/verification.json`. The departure regression is additional coverage accepted by the independent reviewer; it does not replace the original locked baseline.

Independent review also found that a failed refresh could append an old unreadable-record count to the current read error. The reviewer accepted `failed_refresh` on 2026-11-15 after independently observing the failure. `04-test/output/refresh-failure.txt` retains the reproduction. A single condition now restricts the garden count to successful reads. Reader error behavior is unchanged. Both focused terminal files pass after this repair.

The first SDLC verification passed candidate `b4c412aab21fa2d2627b3a7c3042857ad831fa04` with 1127 Rust tests passed, one ignored, 39 Node tests passed, and 87 Python files passed. That pass predates the failed-refresh repair. Final verification supersedes it for the final candidate.

## Final verification

SDLC verification passed candidate `02d9a6c8152901d79400ff96c1bbce8f6e3da610`. It ran the full zero-call check with temporary TEST roots: 1127 Rust tests passed, one ignored, 39 Node tests passed, and all 87 Python files passed. Build, guarded snapshots, formatting and Clippy passed. `04-test/output/verification.json` and `test-log.md` bind that result. The final full suite includes the original feedback checks, Shape departure checks, failed-refresh checks, and the existing refusal and uncertain-save protections.

The intake metadata says `feature` because the launcher omitted `--kind bug`. Issue #874 is a bug. Generated metadata remains unchanged. The independently accepted test baseline and `test-lock.json` remain present and verification enforces that lock. The lock command used retained reproduction evidence rather than its optional `--reproduce` command; the worker and independent reviewer both ran the unfixed regression separately.

The worker made no PR or merge. Independent final review and publication belong to the supervisor. No real-thought trial or operator acceptance is claimed.
