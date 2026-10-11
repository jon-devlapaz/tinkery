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
letters are missing. Tinkery needs at least 80×24; smaller windows show
"Need 80×24. Words retained." until resized.

## Repaint problems

Frames use synchronized updates. If the view looks damaged (missing letters,
stale cells), press **Ctrl-L** to repaint; it changes nothing else. If damage
persists, launch with `--full-redraw`, which repaints every frame at the cost of
more terminal output. When reporting a rendering problem, include the terminal and
its version, `stty size`, the selected font, and a screenshot before and after
Ctrl-L. Leave out credentials and private data.

## Function keys

Think is driven by function keys: F1 help, F2 send, F3 review the goal, F8 skip,
F9 copy, F10 back to the menu. F4 (originals), F5 (why) and F6 (speak, local
voice) are listed in help. On a Mac keyboard you may need to hold **fn**, or enable
**Use F1, F2, etc. keys as standard function keys** in System Settings → Keyboard.
The terminal must also pass function keys through rather than binding them itself.

Ctrl aliases exist for hosts that capture function keys: **Ctrl-N** new dump
(also Shift-F2), **Ctrl-G** review, **Ctrl-O** originals, **Ctrl-D** why. Local
voice also answers to **Ctrl+Alt+Z** (⌃⌥Z on macOS).

**Ctrl-C** copies a selection or cancels a running request; it never leaves think.
F10 leaves think and asks first (`y` / `n`) when leaving would lose unsaved work.
Launched with `--home`, it returns to the menu, where `q` quits; launched without
`--home`, think is the whole session, so F10 exits Tinkery.

## Mouse

Tinkery captures the mouse while it runs and releases it on exit. In think, the
wheel scrolls the reading and the original, and dragging across your original
selects text for Ctrl-C to copy. Because Tinkery captures the mouse, the host's own
click-to-select usually needs a modifier (often Shift or Option); a native
selection copies terminal cells, boxes and wrapping included.

Tinkery also asks the terminal to forward Shift-clicks
([XTSHIFTESCAPE](https://ghostty.org/docs/vt/csi/xtshiftescape)) and restores the
default on exit. Hosts may ignore that request. Right-click and Shift-click
forwarding are only used by the legacy `--scratchpad` prototype (note menu,
right-drag, Shift-click to toggle notes); think does not need them. If a
multiplexer consumes those clicks, scratchpad's **Ctrl-A** selects all notes instead.

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

Each send makes one shaping call and, once a valid reading is shown, one
background audit that is log-only: at most two calls, with no automatic retry.
F3 opens the review; typing `confirm` then Enter saves the goal to a new Seed Me
session. Saving a goal is not seed confirmation or permission to implement, and
nothing launches an agent. See [think](design/think.md) and
[shape and handoff](design/shape-and-handoff.md).
