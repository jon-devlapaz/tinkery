# Terminal setup

Tinkery draws in whatever terminal hosts it. It takes every font face from that
terminal and cannot install or pick one itself. This page covers the host settings
that matter: font, function keys, mouse, clipboard and colour. Key behaviour here
follows the in-app help (F1, then F1 again for every key); if they ever disagree,
the help and [docs/design/](design/principles.md) win.

## Font

Any monospace font with box-drawing glyphs works. Tinkery assumes a fixed cell grid.
[Paper Mono](https://paper.design/mono) ([releases](https://github.com/paper-design/paper-mono/releases/latest))
is the font it was designed with. It is SIL Open Font License 1.1 and is not bundled.

- Install at least the Regular and Bold static faces, then select **Paper Mono** as
  the terminal's font family. Restart the terminal if it does not find the family.
- Leave its optional **duospace (`ss02`)** and **narrow-space (`ss03`)** features off.
- Paper Mono has no italic face. Tinkery's reading is italic (a guess), so check that
  your terminal synthesizes a slant. If it shows regular text instead, the reading
  is still marked by its "I think you mean…" heading.

Where to set it:

- **macOS Terminal.app:** Terminal → Settings → Profiles → Text → Font → Change.
  Enable **Use bold fonts**.
- **Ghostty:** add `font-family = Paper Mono` via **Settings / Open Configuration**,
  then check `ghostty +list-fonts`. See its
  [font configuration](https://ghostty.org/docs/config/reference).
- **Embedded terminal panels** (editors, desktop apps): use the panel's own terminal
  font setting if it has one. A chat or editor font setting may not apply to the
  terminal. Otherwise use an external terminal.
- **Multiplexers** (tmux, Herdr and similar): set the font in the outer terminal.

### Typography check

Run this in the terminal that will host Tinkery:

```sh
printf '\033[1mbold line\033[0m\n\033[3mitalic line\033[0m\n┌────────────┐\n│ Paper Mono │\n└────────────┘\n┄┄┄┄ ┆ ↑ ↓\n'
stty size
```

Bold stays aligned, italic is readable without clipping, box corners join, and no
letters are missing. Think needs at least 80×24; in a smaller window it shows
"Need 80×24. Words retained." until resized. The menu has no such notice and is
simply clipped.

## Repaint problems

Frames use synchronized updates. If the view looks damaged (missing letters,
stale cells), press **Ctrl-L** to repaint; it changes nothing else. If damage
persists, launch with `--full-redraw`, which repaints every frame at the cost of
more terminal output. When reporting a rendering problem, include the terminal and
its version, `stty size`, the selected font, and a screenshot before and after
Ctrl-L. Leave out credentials and private data.

## Function keys

Think is driven by function keys: F1 help, F2 send, F3 review the goal (real drafting only), F8 skip,
F9 copy, F10 back to the menu. F4 (originals), F5 (why) and F6 (speak, local
voice) are listed in help. On a Mac keyboard you may need to hold **fn**, or enable
**Use F1, F2, etc. keys as standard function keys** in System Settings → Keyboard.
The terminal must also pass function keys through rather than binding them itself.

Ctrl aliases exist for hosts that capture function keys: **Ctrl-N** new dump
(also Shift-F2), **Ctrl-G** review, **Ctrl-O** originals, **Ctrl-D** why. Local
voice also answers to **Ctrl+Alt+Z** (⌃⌥Z on macOS).

**Tab** (or Shift-Tab) moves between writing and your original once one exists, and
the same key returns. Shift with the arrow keys, Home or End selects in either one;
**Ctrl-A** selects all of the one you are in. In writing, typing, Enter, Backspace,
Delete and paste replace the selection; the original is read-only, and typing there
returns to writing.
Extracted cards (Ctrl-E extracts a selection) show and hide with **F7** or **Ctrl-X**; while they
cover the original it cannot be selected or copied. A paste or edit past the 4,096-byte
limit is refused whole, with a one-line notice that clears on your next key.
**Ctrl-C** copies the selection in the surface you are in, or cancels a running
request; it never leaves think. Dictation adds text at the caret and never deletes
a selection.
F10 leaves think and asks first (`y` / `n`) when leaving would lose unsaved work.
It returns to the menu, where `q` quits. The one exception: launched without
`--home`, F10 before a goal is saved exits Tinkery; after saving, it opens the menu.

## Mouse

Tinkery captures the mouse while it runs and releases it on exit. In think, the
wheel scrolls the reading and the original, and dragging across your original
selects text for Ctrl-C to copy, in your original or your writing (Tab and Shift-arrows
do the same without a mouse). Because Tinkery captures the mouse, the host's own
click-to-select needs whatever bypass modifier the host offers (Option in many
macOS terminals). Shift works only in hosts that ignore the Shift forwarding
request below. A native selection copies terminal cells, boxes and wrapping included.

Tinkery also asks the terminal to forward Shift-clicks
([XTSHIFTESCAPE](https://ghostty.org/docs/vt/csi/xtshiftescape)) and restores the
default on exit. Hosts may ignore that request. Only think's extracted cards
(technical help) use Shift-click, to toggle a card. If a multiplexer consumes
those clicks, **Ctrl-A** selects all cards instead.

## Clipboard (OSC 52)

Copying (F9 in think, `c` on the receipt and in shape) sends text through the OSC 52
escape. Tinkery cannot confirm the host clipboard changed. F9 says so ("Copy sent;
host may require clipboard permission"); the receipt and shape just say "Copied."
once the escape is written. Either way, both the terminal and any multiplexer must
allow clipboard writes. Test by pasting into a scratch editor.
Tinkery never reads the clipboard or changes host permissions. In think, a copy
over 128 KiB is refused, never truncated.

## Colour

Tinkery uses colour for meaning (see the colour table in [the constitution](yohaku.md))
but never only colour. Set `NO_COLOR=1` or pass `--no-color` and the same meanings
carry through bold, dim and underline. The palette is light; dark mode is not built.

## Launch

From the repository root, practice mode needs no providers and saves nothing:

```sh
cargo +stable run --locked -- --home
```

Without `--home` the binary opens think directly. `--help` lists every option.

Real drafting sends your submitted words to a model and can cost money. It needs
an explicit budget from the operator, Pi, an explicit model, and a Seed Me
checkout. `./scripts/cloud-setup.sh` checks out the Seed Me commit CI pins into
`.ci-seed-me/`:

```sh
cargo +stable run --locked -- --shape-pi \
  --model provider/model-id \
  --seed-me .ci-seed-me/skills/seed-me/SKILL.md \
  --seed-session-root "$(mktemp -d /tmp/tinkery-TEST-goal-XXXXXX)"
```

`--seed-session-root` keeps trial sessions in a temporary directory; leave it out
only when you mean to save real sessions. `--thinking` sets Pi's reasoning level
(default `low`).

A send makes at most two provider calls: one shaping call and one decision review,
both before the reading is shown, with no automatic retry. Some sends make none, such as a new dump that first asks how its words fit.
F3 opens the review. Scroll to its end (PgDn), then type `confirm` and Enter to
save the goal to a new Seed Me session. Saving a goal is not seed confirmation or permission to implement, and
nothing launches an agent. See [think](design/think.md) and
[shape and handoff](design/shape-and-handoff.md).
