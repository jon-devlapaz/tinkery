# Tinkery

A calm, keyboard-first terminal front door for agentic work:
think, shape, approve, watch, taste, measure.

## What works

- **Think** reads a messy dump into one goal block and asks one question at a time.
  Your words are kept exactly.
- **Confirming** saves that goal, unchanged, as a Seed Me session in
  `~/.local/share/seed-me/sessions/`, and writes a think record to
  `~/.local/share/tinkery/think/`.
- **Shape** lists saved goals and copies a prompt to continue in Claude, Codex or Pi.
- **Measure** shows a separately installed ledger, or says it is unknown.
- **Approve, watch and taste** are not built yet.

Nothing runs by itself. Copying a prompt starts no agent. Voice is dictation that
stays on your machine.

## Try it

On a fresh Debian/Ubuntu clone:

```bash
./scripts/cloud-setup.sh
export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PWD/.cloud-tools/node/bin:$PATH"
cargo +stable run --locked -- --home
```

This is practice: no model calls, nothing saved. F2 sends, Enter adds a line,
F1 helps, F10 returns to the menu. Add `--snapshot` to print the screen as text.

Real drafting needs `--shape-pi --model provider/model-id --seed-me PATH` and can
incur provider charges. `--help` lists every option.

## Read next

- [docs/yohaku.md](docs/yohaku.md): the constitution.
- [CLAUDE.md](CLAUDE.md): macOS setup, the full test run, and rules for changing Tinkery.
- [First-release plan](docs/design/first-release.md): the six-stage finish line,
  milestone dependencies and unpublished issue drafts.
- [docs/design/](docs/design/principles.md): current decisions.
  The [backlog](docs/design/backlog.md) holds drafts, not plans.

GPL-3.0-only; see [LICENSE](LICENSE).
