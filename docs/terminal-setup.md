# Terminal setup

This page covers terminal-host settings, launch commands and the
[headless commands](#headless-commands) that need no screen. Tinkery uses the
host's font; it cannot install or choose one. For key behaviour, use in-app help
(F1, then h for how it works). Report disagreements with the
[interaction contract](design/think.md#interaction-contracts), rather than treating
this setup guide as a new rule.

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

## Keys

Think and Shape share one key map: **Enter** runs the action the key bar names, and
**Shift+Enter** adds a new line in terminals that report it (kitty keyboard protocol).
Think's key bar shows that key when supported, otherwise **Ctrl+J**. Ctrl+J always works.
Alt+Enter also adds a line when reported. Pasted line breaks never run an action. F1 lists every
other action allowed now with a number (press the number to run it), and Esc goes back.
In a fresh Think input, Enter opens two choices: **1 Clarify into a goal** (a model reads
your words) or **2 Save as a thought** (nothing reads them). Esc returns to writing unchanged.
Enter, repeated keys and paste never pick. Without `--home`, saving a thought is unavailable.
After the first send, Enter sends directly. **Ctrl+S** still saves a fresh thought directly.

F2, Cmd+Enter (in terminals that report it) and Ctrl+Alt+S still run the key bar's action,
and F3 still keeps a thought. Other screens still name F2 on their key bars. On a Mac keyboard
F keys can need **fn**, and some hosts keep them; Herdr Desktop keeps F2.

Local voice also answers to **Ctrl+Alt+Z** (⌃⌥Z on macOS), for hosts that capture F6.

**Tab** (or Shift-Tab) switches between writing and the read-only original once
it exists.
**Ctrl-C** copies the active selection or cancels a request; it never leaves think.
Esc asks first (`y` / `n`) when leaving would lose unsaved work. With `--home` it
returns to the menu, where `q` quits. Without `--home`, it exits before a goal is
saved and opens the menu after saving. See the
[interaction contract](design/think.md#interaction-contracts) for selection,
editing limits and extracted cards.

## Mouse

Tinkery captures the mouse while running and releases it on exit. In think, the
wheel scrolls the reading and original; dragging selects text in the original or
writing for Ctrl-C to copy. Tab and Shift-arrows provide the keyboard route.
Host-native selection needs its bypass modifier (Option in many macOS terminals).
Shift works only in hosts that ignore the forwarding request below. Native
selection copies terminal cells, including boxes and wrapping.

Tinkery also asks the terminal to forward Shift-clicks
([XTSHIFTESCAPE](https://ghostty.org/docs/vt/csi/xtshiftescape)) and restores the
default on exit. Hosts may ignore that request. Only think's extracted cards
(technical help) use Shift-click, to toggle a card. If a multiplexer consumes
those clicks, **Ctrl-A** selects all cards instead.

## Clipboard (OSC 52)

Copying (F1's copy row in think, `c` in Approve or on a verified Shape seed) sends text through the OSC 52
escape. Tinkery cannot confirm the host clipboard changed. Think says so ("Copy sent;
host may require clipboard permission"); Approve and Shape just say "Copied."
once the escape is written. Either way, both the terminal and any multiplexer must
allow clipboard writes. Test by pasting into a scratch editor.
Tinkery never reads the clipboard or changes host permissions. In think, a copy
over 128 KiB is refused, never truncated.

## Colour

Tinkery uses colour for meaning (see the colour table in [the constitution](yohaku.md))
but never only colour. Set `NO_COLOR=1` or pass `--no-color` and the same meanings
carry through bold, dim and underline. Ctrl-T switches light and dark until you quit
([how](design/principles.md#visual-meaning)).

## Launch

From the repository root, practice Think needs no providers and saves no goals.
Tend can still save needs when you explicitly link or unlink:

```sh
cargo +stable run --locked -- --home
```

Without `--home` the binary opens think directly. `--help` lists every option.

Real drafting sends your submitted words to a model and can cost money. It needs
an explicit budget from the operator, Pi and an explicit model. Draft guidance and
the seed-contract reference are bundled; no external skill checkout is needed:

```sh
trial_root=$(mktemp -d /tmp/tinkery-TEST-trial-XXXXXX) &&
cargo +stable run --locked -- --shape-pi --shape-budget 0 \
  --model provider/model-id \
  --seed-session-root "$trial_root/sessions" \
  --record-root "$trial_root/think" \
  --link-root "$trial_root/tend"
```

For an agent trial, add `--operator agent`; everything it saves is
[recorded as the agent's](design/think.md#confirmation-attribution-185).
Use TEST roots. Replace the zero above only with a separately approved allowance.

All three overrides keep the trial isolated: sessions,
[think records](design/think.md#think-record), and Tend's needs links stay outside
operator data. Omit them only when you intend to use the real data directories.
`--link-root PATH` also works without `--shape-pi`; its default is
`~/.local/share/tinkery/tend` and requires an absolute HOME. In Tend's garden, `l`
starts [linking or unlinking a need](design/shape-and-handoff.md#linking-needs);
only Enter or a picked-goal click saves that change. Saved sessions stay unchanged.
Default sessions live in `~/.local/share/tinkery/sessions`. The first default-root
launch renames only the old sessions folder, leaving every session byte and Think record
unchanged. History matches ledger session IDs across the move. A visible notice names it.
If both roots exist, Tinkery refuses; move the leftover old root aside yourself, never merge blindly.
Explicit roots do not trigger the default move and cannot read the retired old root.
The installed launcher passes only `--home`, so the saved [model setup](design/menu.md#model-setup)
decides how a plain launch drafts. A launcher that also passes `--model` overrides the saved model
but not the saved harness, and a Pi model under a saved Claude Code harness is refused.

`--thinking` sets the reasoning level (default `low`). `--harness claude-code` drafts through
Claude Code (`claude` on PATH, on its own login) with a `--model` such as `claude-sonnet-5-5`
and an effort of `low` to `max`; `--pi-command` applies to Pi only. Claude Code runs with a
clean environment and no settings, MCP servers or session; Think gives it no tools and an
empty folder, and Shape gives it only Read, Grep and Glob in the displayed folder.

Instead of flags, `m` on the menu opens the [Model panel](design/menu.md#model-setup), and
`s` there saves the harness, model, thinking level and calls each launch in
`~/.config/tinkery/model.json`. Later launches draft with a saved setup that allows calls;
flags still override it for one launch and write nothing. Trials should keep a temporary
HOME so they never read or write the operator's saved setup.

Think and Shape share `--shape-budget RESPONSES` (or the saved allowance) for the whole launch;
the default is zero. F1's Model and responses row adds responses for this launch only.
The allowance counts model responses. Each Think shaping request and decision review uses
one. A Shape agent step that reads files may use several. Each started request counts
at least one, including failures. Opening another goal does not reset it. Shape uses
read-only tools in the displayed folder (F1 picks another); paths outside it refuse.
This allowance counts requests, not cost or tool-driven provider turns: a Shape
step may make several provider requests. Authorize that work and its spending separately.

A send makes at most two provider calls: one shaping call and one decision review,
both before the reading is shown, with no automatic retry. Some sends make none, such as a new dump that first asks how its words fit.
F1's first row opens the review. Scroll to its end (PgDn), then type `confirm` and Enter to
save the goal and its think record. Saving is not seed confirmation or permission
to implement, and nothing launches an agent. See [think](design/think.md) and
[tend and handoff](design/shape-and-handoff.md).

## Headless commands

An agent or a script can run these commands without a screen. Each one runs once and exits.
A read-only command writes nothing. A command that acts makes one explicit change and needs
`--operator human|agent` before `seed`. Options always go before `seed`.

| Command | What it does | Model calls |
| --- | --- | --- |
| `tinkery --help` | Read-only: prints every option. | None |
| `tinkery [--home] [--no-color] --snapshot` | Read-only: prints a 100×30 view. | None |
| `tinkery seed status --json DIR` | Read-only: prints the intake status. | None |
| `tinkery seed checks DIR` | Read-only: prints the strict check records. Before a draft, it refuses. | None |
| `tinkery seed show DIR` | Read-only: prints what Shape shows now, as JSON. | None |
| `tinkery [options] seed new GOAL_FILE [ORIGIN]` | Acts: saves the file's words as a confirmed goal and prints the session folder. | None |
| `tinkery [options] seed step DIR confirm DRAFT_SHA256` | Acts: confirms only the draft with that SHA-256. | None |
| `tinkery [options] seed step DIR look\|delegate\|answer WORDS\|answer --relayed FILE` | Acts: takes one Shape step. | Needs `--shape-pi`, `--model` and an approved `--shape-budget` |

[Shape without a screen](design/shape-and-handoff.md#without-a-screen) states what each `seed`
command promises and refuses. No command starts a worker. A worker starts only from Approve's
screen with F2, `l` or `u` ([start a worker](design/approve.md#start-a-worker)).
A headless worker start is proposed in [#956](https://github.com/jon-devlapaz/tinkery/issues/956)
and is not available.

This example makes no model calls. From the repository root, it builds once, then runs every
command with a temporary TEST HOME and TEST roots, away from operator data and saved setup:

```sh
cargo +stable build --locked
tinkery="$PWD/target/debug/tinkery"
test_root=$(mktemp -d /tmp/tinkery-TEST-headless-XXXXXX)
printf 'TEST: list the headless commands.\nDone when: one page lists them.\n' > "$test_root/goal.txt"
# Read-only.
HOME="$test_root/home" "$tinkery" --help
HOME="$test_root/home" "$tinkery" --operator agent --home --snapshot --no-color
# Acts: saves the TEST goal as the agent's and prints its session folder.
session=$(HOME="$test_root/home" "$tinkery" --operator agent \
  --seed-session-root "$test_root/sessions" seed new "$test_root/goal.txt" "TEST example")
# Read-only.
HOME="$test_root/home" "$tinkery" seed show "$session"
HOME="$test_root/home" "$tinkery" seed status --json "$session"
# Each of these refuses, changes nothing and exits non-zero.
# No draft exists yet, so there are no checks.
HOME="$test_root/home" "$tinkery" seed checks "$session"
# A look needs drafting options.
HOME="$test_root/home" "$tinkery" --operator agent seed step "$session" look
# There is no draft to confirm.
HOME="$test_root/home" "$tinkery" --operator agent seed step "$session" \
  confirm 0000000000000000000000000000000000000000000000000000000000000000
```

A real `look`, `answer` or `delegate` sends the goal to a model and can cost money. Run it
only with an explicit budget from the operator, as in the [trial above](#launch).
