# Working on Tinkery

**Read [docs/yohaku.md](docs/yohaku.md) first.** It is Tinkery's constitution and
wins in conflicts with this file, design docs or briefs. Only the operator amends
it; report factual discrepancies rather than editing it.

Tinkery is a calm, keyboard-first terminal front door for agentic work. Read
[README.md](README.md) for what works today. This file is the agent working guide
and source of truth subordinate to the constitution; `AGENTS.md` points here.

## Guardrails

- Yohaku: show only what is needed now. Say each thing once, in plain words.
- The operator's words are authoritative. Preserve originals verbatim; a reading
  is a guess. The displayed, confirmed goal stays unchanged through saving and handoff.
- Nothing runs by itself: navigation and copying never launch agents. Explicit goal
  confirmation is not seed confirmation or implementation permission.
- Work from a GitHub issue on a branch; open a PR, **never merge**. Link the issue
  and report actual test counts, failures, and reversion probes. If the operator
  requests draft-only issues, leave them uncreated and identify the approved task instead.
- No model/provider calls without an explicit budget. Default budget is zero.
  Examples named `live_*` and eval commands are not part of the zero-call suite.
- Never record real audio. Voice tests use fake engines/recorders, not native addons.
- Tests use temporary TEST roots. Never write to `~/.local/share/seed-me` or
  `~/.local/share/tinkery`, or change operator sessions, installs, launcher, Pi,
  Pi Voice, or the operator's Seed Me checkout.
- Read relevant tests before changing behavior. Add regression tests and verify
  important protections go red when reverted. Restore the protection afterward.
- Update `docs/design/` when decisions change. Read the relevant design below
  before implementation; backlog proposals are not shipped behavior or build permission.
- Leave the repository at rest ("git equanimity"): on `main`, matching `origin/main` after a fetch,
  clean, with no stashes, extra worktrees or leftover branches. While a PR is open, say so instead of
  cleaning it up. Delete only merged branches (`git branch -d`); never force-delete, drop stashes or
  remove worktrees to look clean.

## Layout and design entry points

| Work | Code | Read first |
| --- | --- | --- |
| CLI, terminal loop, snapshots | `src/main.rs` | [principles](docs/design/principles.md) |
| Module boundaries, design limits | `tests/architecture.rs` | [architecture](docs/design/architecture.md) |
| Menu, ledger view | `src/home.rs` | [menu](docs/design/menu.md) |
| Shape list/detail | `src/home/shape.rs` | [shape and handoff](docs/design/shape-and-handoff.md) |
| Think state and views | `src/shaping/brain_dump.rs`, `src/shaping/brain_dump/` | [think](docs/design/think.md) |
| Typed goal parts/composition | `src/shaping/brain_dump/goal.rs`, `board.rs` | [goal block](docs/design/goal-block.md) |
| Shared review/receipt/detail goal rendering | `src/shaping/confirmed_goal.rs` | [goal block](docs/design/goal-block.md) |
| Confirmation and continuation prompt | `src/shaping/brain_dump/goal_view.rs`, `handoff.rs` | [shape and handoff](docs/design/shape-and-handoff.md) |
| Explicit Pi drafting boundary | `src/shaping/drafting.rs` | [think](docs/design/think.md) |
| Voice host/process/protocol and owned setup | `src/voice/`, `voice/setup.mjs`, `voice/helper.mjs` | [voice](docs/design/voice.md) |
| Extracted cards (canvas, notes) | `src/shaping/canvas.rs`, `note.rs` | [think](docs/design/think.md) |
| Rendering and input primitives | `vendor/pinstar/`, `src/lib.rs` | [principles](docs/design/principles.md), `vendor/pinstar/API.md` |
| Tests | inline Rust tests, `tests/`, `voice/*.test.mjs` | relevant tests; `tests/vt_screen.py` is a helper |

## Cold setup

From the clone's root, on a fresh Debian/Ubuntu Linux box with Bash and Git:

```bash
./scripts/cloud-setup.sh
```

The script installs missing build/Python prerequisites, stable Rust with rustfmt
and Clippy, and Node if missing. It checks out the **same Seed Me commit as CI**
into ignored `.ci-seed-me/` and prints shell exports. Re-running it is safe; it
refuses a dirty or unrelated fixture checkout rather than resetting it.
The fixture's local Git exclude ignores generated Python bytecode caches, so helper
tests do not make it dirty; tracked Seed Me files remain untouched. It fetches only
packages and Git repositories: no providers, microphones, native voice engines, or
speech models. A non-root box needs sudo for missing apt packages.

CI uses **stable Rust** on Ubuntu and macOS. Local Clippy must use the same updated
stable toolchain, not an older default. The setup script updates stable in the
fresh cloud box. On another development machine, arrange that toolchain explicitly
before checking; do not alter an operator installation without approval.

On macOS, provision Rust (stable, rustfmt, Clippy), Python 3 and Node separately;
the Linux bootstrap deliberately refuses to install there. Clone Seed Me into
`.ci-seed-me`, check out the fixture `ref` in `.github/workflows/ci.yml`, then use
the exports below. No access to another developer's files is needed.

## Full zero-call verification

Run this exact block from the repository root after setup. Cold-clone CI executes
this block in a fresh Ubuntu container; keep it runnable, not illustrative.
Node is needed **only** for `voice/*.test.mjs`. Voice PTYs run with no Node; the
invalid `TINKERY_NODE` below is deliberate and must remain set for the journeys.
Python runs the real binary in a PTY with stub providers and temporary session roots.

<!-- full-check:start -->
```bash
export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PWD/.cloud-tools/node/bin:$PATH"
export SEED_ME_TEST_SKILL="$PWD/.ci-seed-me/skills/seed-me/SKILL.md"
export TINKERY_NODE=/tinkery-TEST-no-node
rustc +stable --version
node --version
python3 --version
cargo +stable build --locked
cargo +stable run --locked -- --home --snapshot
cargo +stable run --locked -- --snapshot --no-color
cargo +stable test --workspace --locked
node --test voice/*.test.mjs
for t in tests/*.py; do
  case "$t" in tests/vt_screen.py) continue;; esac
  echo "== $t"
  python3 "$t"
done
cargo +stable fmt --all -- --check
cargo +stable clippy --workspace --all-targets --locked -- -D warnings
```
<!-- full-check:end -->

For interactive practice (no providers or saved sessions), use a terminal:

```bash
cargo +stable run --locked -- --home
```

Without `--home`, the binary opens think directly. The installed launcher is
separate and is not needed for development. `--snapshot` prints a 100×30 text view
without terminal mode or provider calls. `NO_COLOR=1` or `--no-color` preserves
non-color cues. Use `--help` for CLI options; real drafting requires explicit
`--shape-pi`, `--model` and `--seed-me` plus operator-authorized budget.

## Work intake

[Backlog](docs/design/backlog.md) contains eight **drafts**, not published issues.
They must be reviewed with the operator before anyone creates them. For an approved
issue, keep the slice bounded, follow its acceptance checks and don't-list, and
report observed Rust/Node test counts and PTY journeys/sizes separately from any
operator dogfood. Stub tests establish mechanics, not goal quality on real dumps.
