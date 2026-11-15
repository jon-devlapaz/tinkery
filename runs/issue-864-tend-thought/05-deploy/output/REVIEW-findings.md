Round: 1
Reviewer session: 01a1de7c-36c7-7638-a5ef-fdffd6391de1

# Review findings: issue-864-tend-thought, round 1

Candidate: `279077f` (code at `d3e0e84`), diff `git diff 04c2bc4 HEAD -- . ':!runs'`.
Reviewer model vendor: Anthropic (`claude-opus-5-5`). The author used a different vendor.

## Verdict

**changes required**

One Important finding blocks: mouse picking in link mode is now offset by three rows in the repository garden.
A click on a visible goal does nothing. A click on empty space three rows below a goal writes a needs link.
The other Done-when lines hold on the evidence I checked. No other blocker was found.

## Findings, by severity

### I-1 (Important, blocks): link-mode mouse picks are offset by three rows in the repository garden

- Location: `src/tend/mod.rs:301-310` (pointer) against `src/tend/mod.rs:467-483` (render).
- Observed in code: `render` draws the thought summary and moves the graph down three rows only when
  `!tend.detail && !tend.is_linking() && tend.connection_scope()`.
  `pointer` moves the hit-test rectangle down three rows when `connection_scope()` is true. It does not check `is_linking()`.
  In link mode, the graph is drawn at one place and hit-tested at another.
  `Garden::summary()` is never empty, so every repository garden with a repository is affected, also with no saved thoughts.
- Checked, 100×30, temporary TEST root, binary built from `279077f`, fixture from `tests/tend_connection_terminal.py`:
  - All-goals scope (`a`, then `l`), click on the visible ring of the second goal at (73,14): `needs.json` = `{"needs":[{"goal":"goal-0","needs":"goal-1"}]}`.
  - Repository scope (`l`), the same visible ring at (73,14): `needs.json` absent. The click did nothing.
  - Repository scope (`l`), the label at (64,11) and (64,8): `needs.json` absent.
  - Repository scope (`l`), empty space at (73,17), three rows below the ring: `needs.json` = the link above.
- Effect: a functional regression outside the issue's scope. A click on empty space writes a needs link that the operator did not aim at.
  Linking is a needs write, so this touches saved data, not only selection.
- Why CI did not catch it: `tests/tend_terminal.py` exercises `link-mouse` outside a repository, where `thoughts` is `None`.
  `tests/tend_repo_terminal.py` clicks in the repository garden only outside link mode, where render and pointer agree.
- Repair direction (proposal): use one condition for both paths, for example one helper that both `render` and `pointer` call.
  Add a repository-scope `link-mouse` regression and prove that it fails with the guard reverted.
- Probe scripts (outside the repository): `/tmp/review864/link_click.py`, `/tmp/review864/link_ctrl.py`.

### N-1 (Nit, does not block): a failed directory sync reports "not saved" after the thought file is in place

- Location: `src/tend/thought.rs:146-151`.
- Observed in code: `persist_noclobber` succeeds, then `fs::File::open(root).and_then(sync_all)` can fail.
  The screen then shows `Thought not saved: …` while the file exists. A second F2 shows `Thought already saved`.
- Not reproduced. A directory fsync failure is rare. The words are not lost, but the screen claim is false in that case.

### N-2 (Nit, does not block): Ctrl+C does nothing while the thought editor or reader is open

- Location: `src/menu.rs:621-631`.
- Observed in code: the thought block returns before the generic `Ctrl+C` quit. In the editor, `Words` treats Ctrl+C as copy.
  In the read-only reader, Ctrl+C is swallowed. Esc or F10 closes the view first.
- This matches other text editors. In the read-only reader it is a small surprise. Operator preference.

### N-3 (Nit, does not block): Tend help does not name `t`, `v` or `g`

- The garden key bar is unchanged by design (`docs/design/shape-and-handoff.md`). The summary line names `t` and `v`.
  `grep -rn thought` finds no Tend help text. The keys are discoverable only from the summary line.

## Checked facts, by brief risk

1. Input routing. Checked in code (`src/menu.rs:621-650`, `:437-444`, `:1090-1092`, `:1195-1201`):
   - `t` and `v` act only when no connection panel is open, not in detail, not linking, not in help, in repository scope,
     with Press and no modifiers. History requires detail (`r` acts only in detail). Tend `how` acts only under help.
   - While the thought view is active, it receives all keys, paste and render. Mouse input is ignored. `g` acts only inside the reader.
   - No other Tend mode that owns text was found. The connection-panel correction leak is repaired and covered.
   - The pointer path is the leak that remains (I-1). It is a mouse path, not a key path.
2. Authority. Checked:
   - Only F2 in the editor calls `save`. The editor names the repository and says "No model call. No goal confirmation."
   - Saved JSON is `{schema, repo, words}` with `words` written unchanged. The journey checks bytes with CR, CRLF, tab, ZWJ emoji and edge spaces.
   - `thought.rs` imports no session, record, Shape or provider module. `g` builds a fresh `make_dump()` and pastes. `BrainDump::paste` → `Note::insert` keeps bytes.
   - The promotion journey shows no provider call before the operator presses F2 in Think.
3. "Listed in the garden". Count plus one summary line, full list behind `v`. See the operator questions below.
4. Storage. Checked in code:
   - The file name is `thought-<sha256(repo path)>-<sha256(words)>.json`. Read checks schema, repo, non-empty, size and the name. Duplicates are exact bytes.
   - `O_NOFOLLOW | O_NONBLOCK`, a regular-file check and a read cap of `LIMIT*6+4096` bytes. A temp file plus `persist_noclobber`, with file and directory fsync.
   - An existing different file at the same name is refused, not replaced.
   - A malformed or foreign file is counted as unreadable. The store error stays visible. An unwritable store refuses and keeps the editor words.
   - No collision: the same directory holds `needs.json`, `needs.lock` and `connection-<sha256(repo)>.json`. The `thought-` prefix is distinct.
   - The repository key is the raw `Repo::root()` path, the same as `connection-*.json`. It is not canonicalized.
   - Terminal control bytes in a saved thought do not reach the terminal.
     I wrote `ESC[2J`, `ESC[H` and an OSC title into a thought file. The summary showed the visible text only. The raw PTY output held no `ESC[2J`.
5. Promotion. Checked in the journey and code. `g` opens Think unsent, the confirmation record's first turn equals the thought bytes,
   and the thought file bytes are unchanged afterwards. The goal needs the existing typed `confirm`.
6. Tests. `tests/tend_thought_terminal.py` drives the real binary through a PTY. It reads files only as a user can see them. It adds no test-only accessor.
   I ran it at `279077f`: "Thought journeys passed at 80x24 and 100x30 without colour. Local stubs only."
   I read the three probe mutations in `scripts/probe-thoughts.py`. Each one targets the named protection.
   The expected assertion text is distinct per probe. `probes-fix.log` records all three rejections. I did not run the probes again.
7. Subtraction. `src/shape/words.rs` moved to `src/words.rs` without logic change: 4 lines changed, and `pub(super)` at crate root equals crate visibility.
   `tests/architecture.rs` lists it as `shared`. Two features use it, so the move is justified. No code outside the issue's need was found, apart from I-1's side effect.

## Done-when lines

| Item | Reviewer reading |
|---|---|
| done-1 save, one disclosed action, exact bytes | Holds (journey, code). |
| done-2 listed after restart, labelled thought | Holds as count plus summary line. See question Q1. |
| done-3 no confirmation record, no Shape, not in request | Holds (journey `no_goal`, `s` check, request sources = 2). |
| done-4 Find connections runs with thoughts | Holds (journey with provider stub). |
| done-5 duplicate gives one entry and says so | Holds after restart. A duplicate in one visit was not tested, but it uses the same code path. |
| done-6 promotion through existing confirmation, thought stays | Holds (promotion journey). |
| done-7 journeys at 80×24 and 100×30, colour off | Holds for save, restart, duplicate, cancel and unwritable store. |
| done-8 three reversion probes | Holds on recorded evidence. I did not run it again. |

The release still fails on I-1. It is a regression outside the Done-when lines.

## Hypotheses (not checked)

- If two Tinkery processes save different words at the same moment, `create_dir_all` and `persist_noclobber` are safe. I did not run concurrent writers.
- A launch from a symlinked path can give a different `Repo::root()` string from a launch by the real path. Then thoughts split by path. Goals and connections share this rule.

## Not checked

- `python3 scripts/full-check.py`, Clippy, fmt and the full Python suite. I relied on `full-check-fix.log` for those.
- `scripts/probe-thoughts.py`. I did not run it again, because it rewrites source files in the checkout.
- Behaviour at 80×24 for I-1. I reproduced it at 100×30 only. The code path does not depend on size.
- Very many thought files: `read` lists the whole directory on each Tend open and each `v`. There is no count bound.
- Non-UTF-8 repository paths.
- macOS behaviour of `persist_noclobber` under a full disk.

## Questions for the operator (product shape)

- Q1: The issue says the thought "is listed in that repository's garden". The garden shows `Thoughts: N`, one first-line summary, and the full list behind `v`.
  Is a count plus one line enough, or must each thought show in the garden itself?
- Q2: Thought keys appear only in the summary line, not in the key bar or F1 help. Is that discoverable enough?

## Skills

`principle-prove-it-works` changed one decision. I ran the real binary in PTYs, and did not rely on code reading for I-1.
That run confirmed the regression and ruled out the terminal-escape hypothesis.
