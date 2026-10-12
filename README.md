# Tinkery

A human-first, calm, keyboard-first terminal front door for agentic work:
**think → shape → approve → watch → taste → measure**.

Today: think turns a messy dump into one goal block and one consequential question.
Explicit confirmation saves that exact goal in a Seed Me session, and a think
record of the session (every turn, words verbatim) in `~/.local/share/tinkery/think/`. The receipt and
shape detail offer a repeatable copyable continuation prompt; nothing launches an
agent. Voice is local, editable dictation with its own consented setup, not a Pi
Voice dependency. Measure reads a separately installed ledger (otherwise unknown).
Approve, watch and taste are placeholders, not working factory controls.

## Run

On a fresh Debian/Ubuntu Linux clone, with Bash and Git:

```bash
./scripts/cloud-setup.sh
export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PWD/.cloud-tools/node/bin:$PATH"
cargo +stable run --locked -- --home
```

This is practice mode: no provider calls or saved sessions. F2 sends; Enter adds a
line; F1 helps; F10 returns to the menu. Snapshot without a terminal:

```bash
cargo +stable run --locked -- --home --snapshot
```

For macOS prerequisites, the full test recipe and contribution rules, read
[CLAUDE.md](CLAUDE.md). No installed launcher or operator-machine files are needed.

## Screens as text

Simplified views (bracketed text is a placeholder, not a saved operator record):

```text
  tinkery

    1  think
    2  shape
    3  approve
    4  watch
    5  taste
    6  measure
```

```text
  Saved.

  [the confirmed goal block, unchanged]

  Next: shape it into a seed with Seed Me.

  c copy prompt   s open in shape   10 menu
```

Current decisions live in [docs/design/](docs/design/principles.md); future work
is in the [draft backlog](docs/design/backlog.md). GPL-3.0-only; see [LICENSE](LICENSE).
