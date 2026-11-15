Round: 1
Reviewer session: /root/review_874_build

# Independent review of #874

No remaining Important findings. The candidate meets the approved issue scope and is ready for a PR. This review does not authorize merging.

Reviewed product candidate: `02d9a6c8152901d79400ff96c1bbce8f6e3da610`.
Evidence checkout: `5b41cb5a78bbe887f5bc782e148e4753103e1182`.
Review checkout: `/Users/jondev/dev/worktrees/tinkery/review-874`.
Scope: the approved `brief.md` and seven checklist items for issue #874. Issue #866 remains separate.

## Checks and evidence

- Think passes the exact saved words and successful new-or-duplicate result to Home. Tend owns the message and selection. Storage, refusal, uncertain-save handling, goal confirmation and provider authority stay unchanged.
- New and duplicate saves show the saved thought rather than the first alphabetical thought. The reader opens the selected thought. Duplicate saving preserves all existing thought-record bytes and the total count.
- The garden gives each summary line space. Drawing and pointer input subtract the same summary height. The retained 80×24 and 100×30 screens show save feedback, the selected thought, unreadable-record warnings and visible goals together.
- Opening the reader clears the save message. Leaving Tend for the menu drops the view. Both retained-Tend departures to Shape use the same private method, which clears the notice.
- The accepted regression baseline and helper hashes still match `test-lock.json`. I independently ran the unfixed regression and observed the intended missing-message failure, with the wrong thought displayed.
- `04-test/output/reversion-failure.txt` records the expected failure after the worker dropped the successful-save handoff. The worker restored it before final verification.
- I independently ran four save-feedback cases and two Shape-departure cases from the review checkout's tests against the worker's candidate binary. They passed at 80×24 and 100×30. These checks include reader selection, goal pointer selection, warnings, record preservation and zero provider requests.
- I independently ran the failed-refresh check at both sizes after the final guard. Both passed with zero provider requests.

The final `04-test/output/verification.json` reports passed for the product candidate above. `sdlc.py status` reports verification current. The bound `test-log.md` records 1127 Rust tests passed, one ignored, 39 Node tests passed and all 87 Python files passed. Build, guarded snapshots, formatting and Clippy passed. The full run includes both thought terminal test files and the existing refusal and uncertain-save protections.

## Findings resolved before final review

1. The save notice survived entering Shape and returning to the garden. I reproduced this through the real terminal. The added departure regression reproduced it before the fix. Both Tend-to-Shape routes now clear the notice through one existing navigation boundary.
2. A failed record refresh displayed the previous unreadable-record count in the garden. I reproduced this by replacing the TEST record directory with a file. The added regression reproduced it before the fix. A single condition now displays the count only after a successful read.

The tests added for these findings supplement the unchanged locked baseline. No accepted assertion was weakened.

## Evidence limits

The run metadata says `feature` because intake omitted `--kind bug`. Issue #874 is a bug. The metadata remains unchanged, and the accepted reproduction and enforced test lock remain present. The lock uses retained failure evidence rather than the optional `--reproduce` record. I independently observed the baseline failure, but this is not a forge-authenticated reproduction.

Checklist items are worker attestations backed by the tests and retained output. They are not operator acceptance. The final suite reran after the code corrections. Preliminary `full-check.txt` is not the final verification log.

This review used synthetic TEST records and local stubs. It used no real goals, audio, or provider requests. The independent focused runs used the worker's candidate executable, not a separately compiled executable. Screens are terminal text output, not native-terminal images. Real-thought usefulness and operator acceptance remain untested. Current forge CI remains a separate release check.

No source changes, PR actions or merge actions were made by this reviewer. Only this review file was written in the review checkout.
