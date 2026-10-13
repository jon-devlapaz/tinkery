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
| shared | `src/lib.rs` (palette, sizes), `src/goal/`, `src/seed_me/`, `src/voice/` |

Approve, watch and taste get their own modules when they're built. No phase reaches into another, and the check
has no exceptions: think hands a saved goal to shape only through `seed_me`.

## Modules today

One sentence each: what it owns.

| Module | Owns |
| --- | --- |
| `main.rs` | Command-line options and the terminal loop; wires the menu or think to a real or practice host. |
| `lib.rs` | The palette (one job per colour) and the minimum terminal size. |
| `menu.rs` | The menu: the six places and navigation between them. |
| `measure.rs` | The ledger's read-only report. |
| `shape/mod.rs` | The saved-goal list and detail, and copying the continuation prompt again. |
| `shape/discovery.rs` | Scanning for sessions off the paint path, with unknown, complete, incomplete and failed kept apart. |
| `seed_me/` | Every fact about Seed Me: saving a confirmed goal through its helper, reading a session's files and stage, and the continuation prompt (shared). |
| `goal/mod.rs` | Goal parts, constraint normalisation and composing the goal block (shared). |
| `goal/confirmed.rs` | Rendering a confirmed goal the same way in review, receipt and shape (shared). |
| `think/brain_dump.rs` | Think's `BrainDump`, which routes keys, mouse and ticks to its four parts, paints them, and reports one `Exit` to the menu. |
| `think/brain_dump/conversation.rs` | The conversation, turn by turn: words, extracts, reading, the rules for questions, skips, scope and the next request, and the ordered turns the think record keeps. Fields are private; nothing about the screen or jobs. |
| `think/brain_dump/reading.rs` | The reading's contract with the model: the request, the prompt, the reply's wire shape and its lenient decoder. |
| `think/brain_dump/review.rs` | The decision review: its prompt, what it sees, and how its decisions join the reading's. |
| `think/brain_dump/prompts/` | The two prompts as text, one rule per line, with a note naming the finding behind each tuned rule. Notes never reach the model. |
| `think/brain_dump/host.rs` | Who answers a reading request: Pi (a shaping call, then the review) or the practice simulation. |
| `think/brain_dump/board.rs` | Verifying a decoded reading; a `Board` only exists once verified. |
| `think/brain_dump/checks.rs` | The per-send record shown in F5 "why". |
| `think/brain_dump/goal_view.rs`, `intact_view.rs`, `voice_input.rs`, `layout.rs` | Think's review screen, original pane, dictation input, and where everything goes on screen with help words, key bar and notices. |
| `think/brain_dump/help_view.rs` | Think's bounded help overlay: visual hierarchy, scrolling body, fixed navigation and the boundary against hidden actions. |
| `think/drafting.rs` | The Pi call boundary: one bounded, cancellable request. |
| `think/record.rs` | The think record's format and its one `write`: a confirmed session's turns, checks and goal in one file. |
| `think/canvas.rs`, `note.rs` | Extracted cards, and the text box with grapheme-safe selection. |
| `voice/` | Local dictation: the host boundary, helper process, protocol and owned setup. |

## Design score

The yardstick is the eight-question diagnostic from John Ousterhout's *A Philosophy of Software Design*: the score
is how many pass, times 1.25. Each PR that changes structure says which rows it moves.

| # | Question | Today | What moves it |
| --- | --- | --- | --- |
| 1 | Can each module be described in one sentence? | Yes: the reading's types, prompt and decoder moved to `reading.rs` (#38); see the table above | |
| 2 | Are interfaces simpler than implementations? | Yes | |
| 3 | Can an implementation change without touching callers? | Yes: each model contract is written once, next to its decoder, and a test holds each prompt's field names equal to the decoder's. Seed Me's format lives only in `seed_me/`. One residue: the review merges into the raw reading by its top-level keys | |
| 4 | Do interface comments describe the abstraction? | Yes: every module opens with what it owns and promises, and every `pub` item has a comment; the build denies `missing_docs` and `unreachable_pub` (#39) | |
| 5 | Is design discussion part of review? | Yes: CLAUDE.md's guardrails ask whether a change makes the code simpler to change and which rows it moves (#39) | |
| 6 | Does each module hide an important decision? | Yes: `Conversation` hides think's question, skip and scope rules behind methods | |
| 7 | Can a newcomer see the boundaries without reading code? | Yes | this page |
| 8 | Is design improvement part of ordinary work? | Yes | |

**Score: 10/10** (2026-10-12, re-scored after #39; first scored at 2d25a7c, 5/10). Every row passes. What a
pass doesn't hide: row 3's residue above, and `brain_dump.rs` is still the largest file (about 1,500 lines), with
a narrow interface over a lot of routing.

## Limits enforced by `tests/architecture.rs`

| Limit | Today | Lowered by |
| --- | --- | --- |
| Phase reaches across phases | none allowed | done in #36 |
| `BrainDump` fields / `pub` fields | 13 / 0 | done in #37 |
| Non-test source lines over 200 characters | 13 | done in #38 (the prompts moved to text files) |
| Library files without a `//!` comment, `pub` items without a doc comment | none allowed | done in #39 |

A check fails when the code gets worse than its limit, and also when it gets better, so the PR that improves it
lowers the limit too.
