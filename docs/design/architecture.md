# Architecture

Read [the constitution](../yohaku.md) first. This page owns module responsibilities,
dependency rules and the design score. [Architecture tests](../../tests/architecture.rs)
enforce the limits below; a limit may only get stricter.

## Phases and the dependency rule

Each built [place](menu.md) is a **phase**, with its own compartment:

- A phase uses only shared code, never another phase.
- Only the menu knows every phase.
- Shared code names no phase.

Each top-level module belongs to one area, listed in `MODULES` in the check. A new module must be listed
there, so a new phase can't count as shared code by default.

Published task specifications live in GitHub Issues. The
[parked backlog](../research/shared/backlog.md) is the single home for unpublished
ideas; the area entry points below contain navigation only.

| Area | Code | Work entry point |
| --- | --- | --- |
| menu (also lists approve, watch and taste, which aren't built) | `src/menu.rs` | [menu](../research/menu/backlog.md) |
| think | `src/think/` | [think](../research/think/backlog.md) |
| tend | `src/tend/` | [#123](https://github.com/jon-devlapaz/tinkery/issues/123) |
| measure | `src/measure.rs` | [measure](../research/measure/backlog.md) |
| shared | `src/lib.rs` (palette, sizes), `src/goal/`, `src/seed_me/`, `src/voice/` | [shared](../research/shared/backlog.md) |

Shape returns later as the Seed Me step with its own module; its entry point is
[#106](https://github.com/jon-devlapaz/tinkery/issues/106), which still needs approval.
Approve, watch and taste also get their own modules when built. There are no exceptions
to the dependency rule: think hands a saved goal to tend only through `seed_me`.

## Modules today

| Module | Owns |
| --- | --- |
| `main.rs` | Command-line options and the terminal loop; wires the menu or think to a real or practice host. |
| `lib.rs` | The palette (one job per colour) and the minimum terminal size. |
| `menu.rs` | The menu: the six places and navigation between them. |
| `measure.rs` | The ledger's read-only report. |
| `tend/mod.rs` | The saved-goal garden's discovery states, selection, detail and copying the continuation prompt again. |
| `tend/garden.rs` | The garden's deterministic pages, hollow rings, labels and cell hit-testing. |
| `tend/discovery.rs` | Scanning for sessions off the paint path, with unknown, complete, incomplete and failed kept apart. |
| `seed_me/` | Every fact about Seed Me: saving a confirmed goal through its helper, reading a session's files and stage, and the continuation prompt (shared). |
| `goal/mod.rs` | Goal parts, constraint normalisation and composing the goal block (shared). |
| `goal/confirmed.rs` | Rendering a confirmed goal the same way in review, receipt and tend detail (shared). |
| `think/brain_dump.rs` | Think's `BrainDump`, which routes keys, mouse and ticks to its four parts, paints them, and reports one `Exit` to the menu. |
| `think/brain_dump/conversation.rs` | The conversation, turn by turn: words, extracts, reading, the rules for questions, skips, scope and the next request, and the ordered turns the think record keeps. Fields are private; nothing about the screen or jobs. |
| `think/brain_dump/reading.rs` | The reading's contract with the model: the request, the prompt, the reply's wire shape (including open choices linked to questions) and its lenient decoder. |
| `think/brain_dump/review.rs` | The decision review: its prompt, what it sees, and how its decisions join the reading's. |
| `think/brain_dump/prompts/` | The two prompts as text, one rule per line, with a note naming the finding behind each tuned rule. Notes never reach the model. |
| `think/brain_dump/host.rs` | Who answers a reading request: Pi (a shaping call, then the review) or the practice simulation; a stateful host starts a separate history for each Think visit. |
| `think/brain_dump/board.rs` | Verifying a decoded reading; a `Board` only exists once verified. |
| `think/brain_dump/checks.rs` | The per-send record shown in F5 "why". |
| `think/brain_dump/goal_view.rs`, `intact_view.rs`, `voice_input.rs`, `layout.rs` | Think's review screen, original pane, dictation input, and where everything goes on screen with help words, key bar and notices. |
| `think/brain_dump/help_view.rs` | Think's bounded help overlay: visual hierarchy, scrolling body, fixed navigation and the boundary against hidden actions. |
| `think/drafting.rs` | The Pi call boundary: one bounded, cancellable request, optionally limited by a call budget. |
| `think/record.rs` | The think record's format and its one `write`: a confirmed session's turns, checks and goal in one file. |
| `think/canvas.rs`, `note.rs` | Extracted cards, and the raw text box with safe display, grapheme-offset mapping and machine-sized wrapped rows. Editor storage does not own send policy. |
| `voice/` | Local dictation: the host boundary, helper process, protocol and owned setup. |

## Design score

The yardstick is the eight-question diagnostic from John Ousterhout's *A Philosophy of Software Design*: the score
is how many pass, times 1.25. Each PR that changes structure says which rows it moves.

| # | Question | Recorded assessment |
| --- | --- | --- |
| 1 | Can each module be described in one sentence? | Yes: see the module map; the reading's types, prompt and decoder live in `reading.rs` (#38) |
| 2 | Are interfaces simpler than implementations? | Yes |
| 3 | Can an implementation change without touching callers? | Yes: each model contract lives beside its decoder; tests check their field names agree. Seed Me's format lives only in `seed_me/`. The review decodes only its own reply; the reading's decoder joins it (#89) |
| 4 | Do interface comments describe the abstraction? | Yes: modules and `pub` items carry interface comments; the build denies `missing_docs` and `unreachable_pub` (#39) |
| 5 | Is design discussion part of review? | Yes: [the working guide](../../AGENTS.md#guardrails) requires a changeability check and names the rows moved (#39) |
| 6 | Does each module hide an important decision? | Yes: `Conversation` hides question, skip and scope rules behind methods |
| 7 | Can a newcomer see the boundaries without reading code? | Yes: this page |
| 8 | Is design improvement part of ordinary work? | Yes |

**Recorded score: 10/10** (2026-10-12, after #39; first scored at 2d25a7c, 5/10).
This is a dated assessment, not a fresh measurement. Remaining debt:
`brain_dump.rs`'s narrow interface over a large amount of routing.
Paste reliability improves diagnostics 3 and 6 by keeping raw/display mapping in
Note and using conversation staging for both encoded-size preview and sending.
It preserves the simple editor interface and documented promises in diagnostics
2 and 4. Cached preview size stays inside Think's input/reply routing, preserving
those boundaries; the request boundary now has one definition in drafting (3 and 6).
No new score is claimed.
On 2026-10-14, [#123](https://github.com/jon-devlapaz/tinkery/issues/123) keeps one
module per phase and holds diagnostics 1 and 7: each module retains a one-sentence
job and the map shows the garden and future Seed Me boundary.

## Limits enforced by `tests/architecture.rs`

| Limit | Today | Lowered by |
| --- | --- | --- |
| Phase reaches across phases | none allowed | done in #36 |
| `BrainDump` fields / `pub` fields | 13 / 0 | done in #37 |
| Non-test source lines over 200 characters | 13 | done in #38 (the prompts moved to text files) |
| Library files without a `//!` comment, `pub` items without a doc comment | none allowed | done in #39 |

A check fails when the code gets worse than its limit, and also when it gets better, so the PR that improves it
lowers the limit too.
