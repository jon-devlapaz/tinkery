# Zero-call agent-operator dogfood, October 19, 2026

Commit `a3b3a6cb08a5e48dd3063319c2f587a3d9dc6144` (`main`, equal to `origin/main` after a fetch).
Debug binary SHA-256 `9aecb0803e766d9e1945383182f0bd77fcd82509e10a5b30ec966219938ce549`.
Seed Me fixture `.ci-seed-me` at `c9d766760dbeac0c05c92108165f9e968bec0a66` (CI's pin).
Rust 1.99.1 stable, Node 22.22.0, Python 3.13.16, Linux cloud container.

**This run is stub-driven.** The model and provider budget was zero. A stub `pi` returned fixed
readings and Shape steps, and a fake retired ledger module returned the Measure report. That
proves mechanics and copy, not real-dump goal quality, human recognition or colour appearance.
Every run used a `tinkery-TEST-dogfood-*` temporary root, with HOME, sessions, records, links
and the substrate all inside it. `~/.local/share/seed-me` and `~/.local/share/tinkery` were never
created. No live `pi`, `live_*` example, eval command, real audio or native voice addon was used.

## Full zero-call verification

The exact AGENTS.md block ran after `./scripts/cloud-setup.sh` and passed.

| Check | Result |
| --- | --- |
| Rust (`cargo test --workspace`) | 663 passed, 0 failed, 3 ignored (frame measurements) |
| Node (`voice/*.test.mjs`) | 36 of 36 passed |
| Python scripts | 28 of 28 exited 0; 64 unittest cases; about 220 PTY journey lines |
| Snapshots, fmt, Clippy `-D warnings` | clean |

## What was exercised

An AI-driven real-PTY walkthrough at 80×24, 100×30 and 160×40 with colour, plus 80×24 with
`NO_COLOR=1`. Every launch used `--operator agent --home --shape-pi --shape-budget 2` with the stub.

1. **Home and menu:** F1 help twice, Esc, `h`, the unbuilt places 4, 5 and 6, and `q` to exit.
2. **Think:** empty F2, then a pasted dump with leading and trailing spaces, a tab, a double space,
   `café`, a ZWJ emoji and a line break. Then help and Every key, F2 send, the reading and its
   question, answer `never`, F5 why, F4 originals, F3 review, Enter and `confirm` on the review,
   and the F10 lose-draft guard (`n`, then `y`).
3. **Tend** (on a Seed Me helper fixture session, because agent saving is held): garden, help,
   detail, `c` copy prompt, `r` history, `l` in detail.
4. **Shape:** open, help, F5, F2 pre-mortem (no call), two stub steps (question, then seed draft
   r1), PgDn, Enter hint, a pasted two-line confirmation with F3, then a one-line `I confirm r1.`
   with F3.
5. **Measure:** open, `?` help, Esc, `q`.

Held in every run:
- The dump and the answer reached the provider request byte for byte. F4 showed the dump with
  its tab (`⇥`) and spacing intact. Shape answers were stored verbatim in the ledger.
- The goal stayed unchanged through seed confirmation.
- Empty F2 made no call. Opening Tend and Shape, help, F5, history and copy wrote nothing to
  sessions and made no Shape calls. Copy emitted one OSC 52 and started no process.
- Each Think send made two stub calls (4 per run). Shape made exactly 2. Measure made 1 read
  per open.
- The two-line confirmation was refused with `Put your confirming words on one line; nothing
  was confirmed.`, and nothing was written.
- Exit code 0 with mouse and paste modes restored. No child processes were left running.
- Under NO_COLOR the binary emitted no colour SGR codes, only bold, dim and italic.
- No layout breakage was seen at any size. Key bars fit at 80 columns (Shape's tightens its
  spacing to two spaces).

## Findings

P2 and P3 use the [Oct 12 scale](README.md): P2 is a reproduced failure that blocks an action or
recovery, or here breaks an attribution guarantee. P3 is a lower-impact clarity defect.

### D-01 (P2, [#247](https://github.com/jon-devlapaz/tinkery/issues/247)): Under `--operator agent`, Shape records the agent's answers and seed confirmation as the operator's

Think holds agent-declared goal saving (`Delegated goal saving awaits Seed Me integration`).
Shape doesn't receive the launch declaration at all:
- Each F2 answer is written with `authority: user` and the source `the operator's own answer,
  typed in Tinkery Shape` (`src/seed_me/answering.rs:479-480`). The seed then shows
  `- storage [user] Local only.`
- F3 wrote `seed-confirmation.json` with `"operator": "human"` and `"source": "I confirm r1."`.
  The screen said `Seed ready: you confirmed r1`. The Shape title bar doesn't show
  `declared: agent`, though Think's does.

This conflicts with #185's accepted rule ("Never record an agent's confirmation as the operator's").
#185 closed on Think-only tests, and #227 explicitly leaves out delegated seed confirmation, so
nothing tracks it. Reproduced at all four sizes.

Reproduction: create a human Seed Me session with a confirmed goal in a TEST root. Launch
`tinkery --operator agent --home --shape-pi --model test/model --seed-me SKILL --seed-session-root
TEST-sessions --record-root TEST-records --link-root TEST-links --pi-command STUB --shape-budget 2`.
Press `2`, Enter, `s`, then F2 and answer each step. Type `I confirm r1.` and press F3. Then read
`ledger.json` and `seed-confirmation.json`.

A possible direction, for the operator to decide: hold Shape F2 answers and F3 under an agent
declaration the same way Think holds saving, until delegated seed confirmation is designed.

### D-02 (P3, [#248](https://github.com/jon-devlapaz/tinkery/issues/248)): Measure's report screen has no way out on screen, and its help says `q quits`

After a successful read, Measure shows only the report, with no key bar. Article 6 asks every
screen to say how to leave. `?` opens the menu's help, which says `q quits`, but in Measure `q`
returns to the menu (failure and waiting views do show `esc back   ? help`).
`src/measure.rs:316-321` renders the complete report without a footer.
Reproduction: press `7` with a working reader, then `?`.

### D-03 (P3, [#249](https://github.com/jon-devlapaz/tinkery/issues/249)): The one-line refusal stays on screen after the words are fixed

After a refused two-line F3, deleting the break and typing `I confirm r1.` still shows
`Put your confirming words on one line; nothing was confirmed.` under correct words, until the
next F3. Reproduction: in a seed draft, paste `I confirm⏎r1.`, press F3, then Backspace ×13 and
type `I confirm r1.`.

### D-04 (design question, [#250](https://github.com/jon-devlapaz/tinkery/issues/250)): Seed F3 can confirm without the draft having been read to the end

Think's goal review refuses Enter until the operator scrolls to the end (`Read to the end first`).
Shape's F3 confirmed r1 while the viewport was mid-seed (at `Evidence and uncertainty`). The
Suggested first slice (with its `cmd:`), Risks and Downstream handoff had never been shown.
The design doesn't require reading to the end for seeds, so this is an inconsistency to decide,
not a contract break.

### D-05 (P3, [#251](https://github.com/jon-devlapaz/tinkery/issues/251)): PgDn scrolls a fixed 5 lines in Shape and Tend

At 100×30 the seed viewport is about 21 rows, and PgDn moves 6 rows on screen. Eight presses
didn't reach the end of a short stub seed at any size. Steps differ by place: 5 in Shape and Tend
(`src/shape.rs:361,382,506`, `src/menu.rs:638`, `src/tend/history.rs:180`), 8 in Shape help and
10 in Measure. Reproduction: open any seed draft and press PgDn.

### D-06 (P3, minor, folded into [#247](https://github.com/jon-devlapaz/tinkery/issues/247)): Held review still takes typing

Under an agent declaration, the held review still shows a text field. Typing `confirm` and Enter
change nothing visible, since the hold message was already up. Nothing is written, which is
correct; the live field just invites an action that can't happen.

## Not established

- **ZWJ emoji ghost glyph.** After the reading appears, a stray `💻` stays at column 38 and
  persists into the menu and Tend. `tests/vt_screen.py` gives every code point one column, while
  Tinkery emits `👩‍💻` as one two-column cluster, so this is very likely the emulator. Real
  terminals that draw ZWJ sequences four columns wide could show the same leftover. It needs a
  check in a real terminal before anyone files it.
- **Think saving, the receipt and receipt copy under `--operator agent`.** These can't be reached
  by design (#227). The human-default path is covered by `tests/goal_confirmation_terminal.py`,
  which passed in the verification block.
- Real model readings, Pi tool use, voice, colour appearance and human recognition.

The harness and captured screens were kept outside the repository and are not included.
