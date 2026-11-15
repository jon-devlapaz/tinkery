# Build handoff

## Situation

The implementation is committed. The run is not ready for acceptance.
Full checks fail, and the confirmed-goal reversion probe remains incomplete.

- Run ID: `d50e7a87-1232-43e5-a29a-0cbf2512c1ab`.
- Checkout: `/Users/jondev/dev/worktrees/tinkery/issue-864-tend-thought`.
- Code candidate: `21277a33dc5d469c7ee9ad880fd013c8d31bd8ad`.
- Branch: `issue-864-tend-thought`.
- Approved input: `runs/issue-864-tend-thought/brief.md`, from issue #864.
- Issue body SHA-256: `29f447833355967617805c1fc76789f53c35598279aa8b5a4f7ba921cf6aa3b1`.
- This light run has no seed-contract binding in `run.json`.
- Step pin: `openai-codex/gpt-6.1-sol:medium`. I did not change it.
- No PR, independent review, operator acceptance or SDLC verification receipt was created.

## Background

Tend now keeps raw thoughts separately from saved goal sessions.

- In the repository garden, `t` opens an exact-word editor.
- The editor displays the repository before `F2` saves without a model call.
- The garden labels saved entries as thoughts. `v` opens the local thought reader.
- Identical bytes in one repository produce one entry and an explicit duplicate notice.
- `g` opens the selected words in Think, unsent. Existing goal confirmation remains necessary.
- Thoughts cannot open Shape. Find connections still reads saved goals only.
- Thoughts use private atomic files in the existing Tend link root.
- The new format stores only `schema`, `repo` and `words`.
- Thoughts accept 1 to 131072 bytes. They have no edit or delete action.

The exact-word editor moved from `src/shape/words.rs` to shared `src/words.rs`.
Its six existing tests moved unchanged. No shared goal saver or session format changed.
No storage service, provider endpoint or migration was added.

The architecture diagnostics affected are 1, 3, 6 and 7.
Thought persistence stays inside Tend. The menu alone connects a thought to Think.
Shape and Tend share one editor rather than separate copies of its line-ending rules.

Skills read in full:

- `principle-build-the-lever` led to a repeatable PTY test and `scripts/probe-thoughts.py`.
- `unslop` kept the documentation and notices direct about saving, confirmation and failure.

## Assessment

### Checked acceptance evidence

`tests/tend_thought_terminal.py` passed on the committed code candidate.
It runs six top-level journeys across 80×24 and 100×30 terminals, without colour.
It launches 12 terminal processes with temporary TEST roots.
Save, restart, duplicate, cancellation and store-refusal actions make no provider requests.
Connection and goal-confirmation checks use local executable stubs, not live providers.

1. Checked exact received UTF-8 bytes after explicit saving, including CRLF, CR, tabs, Unicode and trailing whitespace.
2. Checked restart discovery, thought labels and isolation between two repositories.
3. Checked unchanged goal-session files and confirmation records after thought saves. Pressing `s` does not open Shape.
4. Checked Find connections succeeds with thoughts present. Its request contains only the two original goal sources.
5. Checked duplicate saving leaves one record and displays `Thought already saved. One entry.`
6. Checked the existing Think review and typed confirmation save a goal. The first original equals the thought bytes.
7. Checked cancellation and an unwritable store at both sizes. A failed save keeps the words in the editor.
8. Partially checked. Changed-word and provider-source mutations fail their intended assertions. The confirmed-goal mutation is not proven.

Each saved thought file also passed a `0600` permission check.
These checks prove mechanics with synthetic words. They do not establish operator acceptance or real-data usefulness.

### Check results

The logs beside this report retain the observed results.

- `thought-journeys.log`: the focused journeys passed again after the code commit.
- `independent-probes.log`: changed-word and provider-source faults were rejected. Sources and binary were restored, then journeys passed.
- `node-check.log`: 39 Node tests passed, zero failed.
- `lint-check.log`: formatting and workspace Clippy passed with warnings denied.
- `python3 tests/test_documentation.py`: 12 tests passed after the documentation update.
- `git diff --check`: passed before the code commit.
- `full-check.log`: the required full check failed in the Rust wrapper. It stopped before its Node and Python steps.
- Rust wrapper totals: 1119 passed, one failed, across 20 reported test groups. This is not a passing workspace result.
- `python-check.log`: the separate full Python runner reported 86 files, with 62 passing and 24 failing.

All temporary roots were placed under `target/tmp/tinkery-TEST-build` to respect the checkout-only write boundary.
That layout conflicts with fixtures that require output outside a repository or expect launch outside Git.
The Rust failure is `tests::a_live_limit_counts_both_calls_of_a_send_and_records_the_run` in `examples/think_eval`.
Its error is `Live output stays outside the repository and the operator's Seed Me and Tinkery data.`

Several Python failures show the enclosing checkout as the launch repository.
Other failures include changed garden-key expectations, long-path display, label saving and launch-guard behavior.
Their individual causes are not all established. Do not classify every failure as an environment failure.
The complete failing-file list and terminal output remain in `python-check.log`.

An initial separate Python-runner attempt refused the real HOME before launching its checks.
The retained Python log comes from the later attempt with a checkout-local TEST HOME.

### Confirmed-goal probe blocker

Problem: the faulty confirmed-goal probe stops before it reaches the intended assertion.
Goal: prove the regression fails when saving a thought creates a confirmed goal session.

Three attempts failed:

1. A hand-written terminal declaration failed with `Invalid terminal confirmation declaration or action.`
2. The valid terminal constructor reached saving, but returned `The goal save could not be verified. Inspect the recovery session before retrying.`
3. A first-line goal instead of the raw multiline goal returned the same save-verification error.

Both final attempts passed an empty context to `Affirmed`.
The lower-level ledger refusal was not exposed, so its cause remains unknown.
I stopped further fixes after the third attempt. `probe-failure.log` retains the final failure.

`scripts/probe-thoughts.py` restores the original source files and rebuilds in its finalizer.
Its full command still fails at this first mutation.
The two independent mutations passed when run separately, without changing the script.

## Recommendation

Keep this candidate blocked from acceptance.

1. Inspect the lower-level refusal before another confirmed-goal mutation attempt. Then prove the intended assertion fails.
2. Establish baseline failures under the permitted TEST-root layout. Obtain permission before using roots outside this checkout.
3. Resolve actual regressions and run the full required checks again on committed work.
4. Obtain independent review, then continue with the run's verification and acceptance gates.

The approved brief, checklist and step contract were not edited.
The generated `AGENTS.md` block and stage/step runtime files were already uncommitted at entry and remain untouched.
No live endpoint requests, real audio recording, delegation, toolchain changes or Pi infrastructure edits occurred.
Every terminal process started by the focused journeys was closed. No local server was started.
