Previous review: f35efc9
Round: 2
Reviewer session: 01a1dfec-5756-73f2-a6b2-e0950409f4c9

# Review findings: issue-864-tend-thought, second review

Candidate: `868bdca` (code at `eb3efb6`). Repair diff: `git diff 279077f HEAD -- . ':!runs'`.
Reviewer model vendor: Anthropic. The author used a different vendor.
Binary under test: `cargo +stable build --locked` at `868bdca`. All TEST roots were in the system temporary directory.

## Verdict

**pass with notes**

I-1 is repaired. I reproduced the repair with the real binary at 80×24 and 100×30.
The repair added no render and pointer mismatch that I could find. N-1 and N-3 are repaired.
Three Nits remain. None of them blocks.

## Findings, by severity

### Important

None.

### N-4 (Nit, does not block): the help text keeps a second copy of the visibility condition

- Location: `src/tend/mod.rs:388`. The helper is `src/tend/mod.rs:252-260`.
- Observed in code: `help_text` tests `!self.detail && !self.is_linking() && self.connection_scope()`.
  This is the helper's condition without the summary check. The key guard at `src/menu.rs:633-636` holds a third copy. That copy is from round 1.
- Render (`src/tend/mod.rs:480`) and pointer (`src/tend/mod.rs:312`) both call `thought_summary()`. No geometry path holds a copy.
- Effect: none observed. The help copy controls text only, not a rectangle. If the helper changes, the help text can drift from the screen.

### N-5 (Nit, does not block): the uncertain-persistence notice drops "No goal confirmed"

- Location: `src/tend/thought.rs:149-151`.
- Observed: the normal notice says `Thought saved. No goal confirmed.` (`:153`). The uncertain notice does not repeat the goal disclosure.
  At 80 columns, the OS error text is cut after `Reopen before another save:`.
- The editor states "No goal confirmation" before the save. Thus the operator sees the disclosure once.

### N-6 (Nit, does not block): the new regression waits for text that is already on screen

- Location: `tests/tend_thought_terminal.py:230`.
- Observed in code: after the empty-space click, `flow.wait(term, 'esc')` returns on the next output that contains `esc`.
  The key bar already contains `esc`. The needs-file check can run before a wrong write lands.
- Checked: the bypass probe still failed at both sizes (item 5). The race did not hide the fault in my runs.

## Checked facts, in brief order

1. I-1 repaired. Real binary, fixture from `tests/tend_connection_terminal.py`, probe `/tmp/review864r2/probe2.py`.
   - Repository scope, link mode, 80×24 and 100×30, with zero thoughts and with one saved thought:
     a click three rows below the visible ring of the second goal left `needs.json` absent.
     A click on the ring wrote `{"goal":"goal-0","needs":"goal-1"}`. A click on the label row (three rows above the ring) wrote the same link.
   - All-goals scope gave the same results at the same rows. In link mode the summary is hidden, and the ring rows match in both scopes.
   - Code: `render` and `pointer` both call the private `thought_summary()`. No other place moves the garden body by three rows.
2. No new mismatch. Checked with the real binary in repository and all-goals scope, in and out of link mode, with zero thoughts and with thoughts, at both sizes:
   - Out of link mode in repository scope, the summary shows and the rings are drawn two rows lower than in all-goals scope at 100×30. This is expected.
   - Hover (`Moved`) over a ring changed no screen cell and wrote nothing.
   - A drag from empty space, released on a ring, wrote nothing in link mode.
   - Scroll events wrote nothing and did not move the rings.
   - Pointer paths in code: down, drag, up and scroll all enter `Tend::pointer`, then one `sync_graph(body)`. `release_pointer` calculates no rectangle.
     `garden::render` calls `sync_graph` with the rectangle that `render` computed with the same helper.
   - Each behaviour above was the same in repository scope and all-goals scope. A scope-dependent offset would show as a difference.
3. N-1 repaired. I built a copy of `868bdca` outside the repository with the directory sync replaced by an error.
   At both sizes the screen showed `Thought saved, but persistence is uncertain.` and not `Thought not saved`.
   One thought file existed, its words equalled the typed bytes, and the reader showed the words. No provider request was made.
   A second save of the same words showed `Thought already saved. One entry. No goal confirmed.`. The file count stayed at one.
   The claim on screen is true: `persist_noclobber` succeeded before the sync, so the file is in the directory.
4. N-3 repaired. F1 help in the repository garden shows `t new thought · v read thoughts` and `g in the thought reader opens Think, unsent.` at both sizes.
   Help does not name them in link mode or in all-goals scope, where the keys do not act.
   The garden key bars are `↑↓←→ move   enter open   f connections   l link   esc back` (repository) and
   `↑↓←→ move   enter open   l link   esc back` (all goals). The repair diff does not touch them.
   `docs/design/shape-and-handoff.md` gains one line that states the help keys.
5. Regression proof. In a copy outside the repository, I replaced the helper call in `pointer` with the old condition and rebuilt.
   `repository_link` failed at 80×24 and 100×30 with `AssertionError: Empty-space click wrote a needs link`.
   With the helper restored, it passed at both sizes. I did not run `scripts/probe-thought-repair.py`, because it rewrites the checkout.
6. Scope. The repair diff changes `src/tend/mod.rs`, `src/tend/thought.rs`, `tests/tend_thought_terminal.py`,
   `scripts/probe-thought-repair.py` and one line in `docs/design/shape-and-handoff.md`. All five map to repair brief 2 items 1 to 3.
   - Authority: unchanged. Only F2 in the editor calls `save`. `save` now returns the notice text instead of a flag. No session, record, Shape or provider import was added.
   - Storage: unchanged apart from the notice. File name, read checks, `persist_noclobber` and the refusal of a different existing file are as in round 1.
   - Promotion: the `g` path in `src/menu.rs` is not in the repair diff.
   - `verification.json` names candidate head `19eca99`. `eb3efb6..HEAD` changes only `runs/`. The eight new marks follow the repair.
   - `tests/tend_repo_terminal.py` passed in my run.

## Hypotheses and observations not attributed to this repair

- After the reader closes, the test terminal shows stray ZWJ-emoji cells (`‍💻`) above the graph in both scopes.
  I think the cause is a width difference between ratatui and the test emulator for `👩‍💻`. I did not test a real terminal. The repair diff does not touch this.
- In link mode, after a drag that starts on empty space, the next ring click wrote nothing. Both scopes behave the same. I did not check if this is intended or older than this run.
- Out of link mode, my single and double clicks on rings and labels did not change selection or open detail, in either scope.
  `tests/tend_repo_terminal.py`, which double-clicks a label in repository scope, passed. I think my probe timing is the cause. This result is inconclusive.

## Not checked

- `python3 scripts/full-check.py`, Clippy, fmt and the full Python suite. I relied on `full-check-fix-2-retry.log`.
- `scripts/probe-thoughts.py` and `scripts/probe-thought-repair.py` in the checkout. Both rewrite source files.
- A real directory fsync failure. I injected the error in a copy of the source.
- Keyboard selection, pan and zoom after the repair. They do not use the body rectangle in code.
- Concurrent writers, symlinked launch paths and very many thought files. These remain as in round 1.

## Skills

`principle-prove-it-works` changed one decision: I built a mutated copy outside the repository and ran the regression against it,
instead of trusting `repair-probes-2.log`.
