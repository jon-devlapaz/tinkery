# Terminal setup

Tinkery inherits **every font face from its terminal**. It cannot install or select Paper Mono itself. No terminal/host settings or installed fonts were changed for this patch.

The reported dogfood host was the **Claude Code desktop terminal panel**. The coding harness here runs under Herdr; that is not evidence about the desktop panel. Ghostty was not detected, so its instructions below are optional, not a claim about the operator's setup.

## Install Paper Mono

1. Download the font from [Paper Mono](https://paper.design/mono), or its official [latest release](https://github.com/paper-design/paper-mono/releases/latest).
2. On macOS, open the static TTF files in Font Book and choose **Install**. Install at least Regular and Bold; installing the whole static family is fine. Pick **Paper Mono** as the terminal's family, not a proportional font. Restart the terminal if it does not discover the new family.
3. Leave the optional **duospace (`ss02`)** and **narrow-space (`ss03`)** features disabled. Tinkery assumes a fixed cell grid.

The font is SIL Open Font License 1.1; it is not bundled with Tinkery. The v1.0 assets inspected here contain native Regular/Bold faces, but no native italic face. Italics depend on the terminal synthesizing a slant or showing regular text.

## Set it in the actual host

### Claude Code desktop panel

An official, documented terminal-specific custom font control could not be established. Do not assume a chat font setting changes the terminal. If your desktop version exposes a **terminal font** setting, choose Paper Mono there and run the proof below. Otherwise use an external terminal with a supported font selector; no undocumented app-file/CSS edits are recommended.

### macOS Terminal.app

Open **Terminal → Settings → Profiles → Text → Font → Change**, select **Paper Mono**, and enable **Use bold fonts**. Set that profile as the default if you want it in new windows. Check italics in the proof below; a regular-looking italic line means that host did not synthesize it.

### Ghostty (if you choose it)

Use Ghostty's **Settings / Open Configuration** action rather than guessing a configuration path. Add:

```ini
font-family = Paper Mono
```

Verify discovery with `ghostty +list-fonts` (or the executable inside `Ghostty.app` if it is not on PATH), then open a new terminal. Ghostty searches the family for bold variants and documents synthesized italics when a native face is absent. Do not override bold/italic to a different family if you want Paper Mono consistently.

References: [font configuration](https://ghostty.org/docs/config/reference), [Shift mouse forwarding](https://ghostty.org/docs/vt/csi/xtshiftescape).

### Herdr

The note-selection/menu gestures below apply to the explicit `--scratchpad` legacy prototype, not the default brain-dump intake.

Set the font in the **outer terminal**, not in Tinkery or a pane. Herdr can also mediate mouse/clipboard traffic. Shift-click does not depend on the right-click forwarding setting. If the host consumes Shift-click for native text selection, use explicit **Ctrl-A** to select all notes outside editing; F2 never implicitly includes unselected notes.

Tinkery requests Shift mouse forwarding while running and restores the terminal's default override behavior on exit. Hosts may ignore that request. Existing right-click forwarding is only needed for the optional note menu/right-drag gestures; no Herdr configuration was changed.

## Check typography and the original glitch

Run in the terminal that will host Tinkery:

```sh
printf '\033[1mDraft / Note 1 / edit — bold\033[0m\n\033[3mProvisional / italic\033[0m\n┌────────────┐\n│ Paper Mono │\n└────────────┘\n┄┄┄┄ ┆ ↑ ↓\n'
stty size
```

Check that bold stays aligned, italic remains readable without clipping, boxes join, and none of the ASCII letters disappear. The legacy working paper's headings and field labels use bold; its provisional text uses italics. The default brain-dump panel distinguishes the agent's reading with jade. Unavailable characters in your own notes may still require the terminal's normal fallback font.

The inspected Regular/Bold font files contain the UI's box/dashed glyphs at the same advance width as ASCII. They lack Pinstar's double diagonal corner arrows, so Tinkery projects those selection markers onto supported box corners. This proves glyph coverage/metrics, **not visual correctness in the operator's host**.

Missing ASCII in the desktop panel has not been reproduced in differential buffers or the POSIX PTY journeys. Frames now use synchronized updates. **Ctrl-L** requests a full repaint without changing notes, selection, paper, or provider requests. If the problem persists, try **`--full-redraw`** (more terminal output; unsupported synchronized updates may flicker). Neither is yet a confirmed fix for that panel.

For a useful report, record the desktop/terminal version, `stty size`, whether Paper Mono is actually selected, and a screenshot before/after Ctrl-L. Compare the same case in an external terminal if possible. Do not include credentials or private bookmarks.

## Default brain-dump intake

- Start with **What's on your mind?** Type freely; **Enter** inserts a newline, **F2** submits. No provider request or interpretation before submit.
- After submission, exact fragments and a provisional centre reading appear. Answer the focused question or **Ctrl-N** to add another dump; F2 reshapes. A reply arriving during new typing waits for the next submit.
- **Ctrl-O** opens intact originals; **Esc** closes. **Tab** switches input/board focus. **Ctrl-D** or the visible header control opens details from any focus (including originals/help); plain `d` remains text in the input. On the board: **d** toggles details, **s** skips the focused question, **y** explicitly copies board Markdown, and **q** exits. **Ctrl-C** exits from input.
- Drag fragment cards to arrange them; later responses retain existing positions. Double borders highlight the current reading's supporting words; **[ / ]** on the board switches the highlighted reading without answering or confirming. Misfit cards are marked **doesn't fit yet**; **… Ctrl-O** flags clipped text. Card titles/ID lists are absent from the default view. **Ctrl-F** on the board explicitly fits the view. **PageUp/PageDown** scroll the agent reading.
- **Esc** cancels an active request; failure/cancellation retains originals and the previous reading. F2 is an explicit retry. Nothing is saved, confirmed, or handed off.
- No answer choices in this slice. Clusters, relationships, settled strip, centre editing, and confirmation are not implemented.

## Legacy paper selection (`--scratchpad`)

- **Shift-click** toggles notes; **Ctrl-A** selects all notes outside editing. Double-click edits a note. Selection survives F2.
- Drag **inside the paper body** to select complete logical Markdown lines/paragraphs, including their wrapped display rows. This is line selection, not character-level selection. **Esc** clears it without closing the paper.
- **y**, outside text editing, copies selected paper lines—or the entire paper when there is no selection. The **y copy** button also works while writing feedback.
- Copy uses the original Markdown, not terminal cells: headings/emphasis are retained, with no UI boxes or artificial wrap breaks. It is an explicit clipboard export, not a saved seed. Ctrl-S still does not save anything.
- The copy limit is 128 KiB; oversize selections are rejected, never silently truncated.

OSC 52 requires both the terminal and any multiplexer to allow clipboard writes. The message says **“Clipboard escape sent; host may require permission”**, not that the system clipboard was verified. Test by pasting into a scratch editor. Tinkery does not read the clipboard or change permissions. Native terminal mouse-copy may include wrapping/boxes; use **y** for clean Markdown.

## Launch

Seed Me's canonical home is [jon-devlapaz/seed-me](https://github.com/jon-devlapaz/seed-me), at `skills/seed-me/`. Clone it locally if missing; do not move/delete the historical tink-skills checkout:

```sh
git clone https://github.com/jon-devlapaz/seed-me.git "$HOME/dev/active/factory/seed-me"
```

For tests, add `--seed-session-root "$(mktemp -d /tmp/tinkery-TEST-goal-XXXXXX)"`; only explicitly affirmed goals create sessions. The canonical viewer fix is merged; CI pins `1a2f31a136d53bceff51ae8c2e0e787c1a8a90bb`. For matching dogfood behavior, check out that commit in the local clone. PR#3 is review-only after fresh CI; goal confirmation is not seed confirmation.

```sh
cargo run --locked -- --shape-pi \
  --model openai-codex/gpt-5.6-luna \
  --thinking low \
  --seed-me "$HOME/dev/active/factory/seed-me/skills/seed-me/SKILL.md"
```

Real mode now defaults to **low** reasoning, selectable with `--thinking`; Pi clamps it to model capabilities. Earlier exploratory `off` runs included a malformed response. This change is not proof of reliability and can change provider cost/latency. Each submitted F2 makes a bounded shaping request and meaning-audit request; answers may also require the bounded question-continuity check. No automatic repair/retry. Default launch remains simulated and unsaved. In real mode, **Ctrl-G** reviews the complete goal with remaining questions; type `confirm` then Enter after reviewing to create the real goal-only Seed Me session. Questions do not block affirmation. Confirmation is separate from F2 and never confirms a seed. Nothing implements the bookmarks idea.
