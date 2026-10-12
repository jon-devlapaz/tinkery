# Architecture

Read [the constitution](../yohaku.md) first. This page is about code structure: which module owns what, which may
use which, and how good the design is today by one fixed yardstick. `tests/architecture.rs` enforces the limits
below; a limit may only get stricter.

## Phases and the dependency rule

Tinkery is one front door with six places: think, shape, approve, watch, taste, measure. Each built place is a
**phase**. The target is one compartment per phase:

- A phase uses only shared code, never another phase.
- Only the menu knows every phase.
- Shared code names no phase.

Each top-level module belongs to one area, listed in `MODULES` in the check. A new module must be listed
there, so a new phase can't count as shared code by default.

| Area | Code |
| --- | --- |
| menu (also lists approve, watch and taste, which aren't built) | `src/menu.rs` |
| think | `src/think/` |
| shape | `src/shape/` |
| measure | `src/measure.rs` |
| shared | `src/lib.rs` (palette, sizes), `src/goal/`, `src/voice/`; [#36](https://github.com/jon-devlapaz/tinkery/issues/36) adds `src/seed_me.rs` |

Approve, watch and taste get their own modules when they're built. Where a phase still reaches into another,
the check lists an exception that names the step that removes it. An exception that no longer matches any code
fails the check, so the list only shrinks. Today there is one: shape uses think's
`handoff::continuation_prompt`, and #36 moves it into one Seed Me module.

## Modules today

One sentence each: what it owns.

| Module | Owns |
| --- | --- |
| `main.rs` | Command-line options and the terminal loop; wires the menu or think to a real or practice host. |
| `lib.rs` | The palette (one job per colour) and the minimum terminal size. |
| `menu.rs` | The menu: the six places and navigation between them. |
| `measure.rs` | The ledger's read-only report. |
| `shape/mod.rs` | The saved-goal list and detail, and copying the continuation prompt again. |
| `shape/discovery.rs` | Reading Seed Me sessions off the paint path, with unknown, complete, incomplete and failed kept apart. |
| `goal/mod.rs` | Goal parts, constraint normalisation and composing the goal block (shared). |
| `goal/confirmed.rs` | Rendering a confirmed goal the same way in review, receipt and shape (shared). |
| `think/brain_dump.rs` | Think: the dump, the reading, the question, answers and skips, plus layout, input and jobs (too much; see #37). |
| `think/brain_dump/board.rs` | Decoding and verifying a model's reading; a `Board` only exists once verified. |
| `think/brain_dump/checks.rs` | The per-send record shown in F5 "why". |
| `think/brain_dump/handoff.rs` | Saving a confirmed goal into Seed Me and the continuation prompt. |
| `think/brain_dump/goal_view.rs`, `intact_view.rs`, `voice_input.rs`, `yohaku.rs` | Think's review screen, original pane, dictation input, and help and key bar. |
| `think/drafting.rs` | The Pi call boundary: one bounded, cancellable request. |
| `think/canvas.rs`, `note.rs` | Extracted cards, and the text box with grapheme-safe selection. |
| `voice/` | Local dictation: the host boundary, helper process, protocol and owned setup. |

## Design score

The yardstick is the eight-question diagnostic from John Ousterhout's *A Philosophy of Software Design*: the score
is how many pass, times 1.25. Each PR that changes structure says which rows it moves.

| # | Question | Today | What moves it |
| --- | --- | --- | --- |
| 1 | Can each module be described in one sentence? | No: all but `BrainDump` | #37 |
| 2 | Are interfaces simpler than implementations? | Yes, except `BrainDump` | #37 |
| 3 | Can an implementation change without touching callers? | No: Seed Me's format and the model's JSON contract are each known in more than one place | #36, #38 |
| 4 | Do interface comments describe the abstraction? | No | #39 |
| 5 | Is design discussion part of review? | Yes | #39 adds a structural question |
| 6 | Does each module hide an important decision? | No: `BrainDump`'s fields are shared across four files | #37 |
| 7 | Can a newcomer see the boundaries without reading code? | Yes | this page |
| 8 | Is design improvement part of ordinary work? | Yes | |

**Score: 5/10** (2026-10-12, re-scored after #35; first scored at 2d25a7c). The goal is at least 9/10; #33 tracks the steps.

## Limits enforced by `tests/architecture.rs`

| Limit | Today | Lowered by |
| --- | --- | --- |
| Phase reaches across phases | 1 listed exception | #36 |
| `BrainDump` fields / `pub` fields | 55 / 9 | #37 |
| Non-test source lines over 200 characters | 21 | #38 |

A check fails when the code gets worse than its limit, and also when it gets better, so the PR that improves it
lowers the limit too.
