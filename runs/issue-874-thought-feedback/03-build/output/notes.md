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
