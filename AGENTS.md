# Working on Tinkery

**Read [docs/yohaku.md](docs/yohaku.md) first.** It is Tinkery's constitution and
wins in conflicts with this file, design docs or briefs. Only the operator amends
it; report factual discrepancies rather than editing it.

This is the working guide, subordinate to the constitution. Read
[README.md](README.md) for what works today.

## Guardrails

- Yohaku: show only what is needed now. Say each thing once, in plain words.
- Structure: does this make the code simpler to change? State which design diagnostics in
  [architecture](docs/design/architecture.md) it moves. Interface comments say what a module
  or `pub` item promises and refuses, not how it works; the build refuses a `pub` item without one.
- The operator's words are authoritative. Preserve originals verbatim; a reading
  is a guess. The displayed, confirmed goal stays unchanged through saving and handoff.
- Nothing runs by itself: navigation and copying never launch agents. Explicit goal
  confirmation is not seed confirmation or implementation permission.
- Work from a GitHub issue on a branch; open a PR, **never merge**. If the operator
  requests draft-only issues, leave them uncreated and identify the approved task instead.
- No model/provider calls without an explicit budget. Default budget is zero.
  Examples named `live_*` and eval commands are not part of the zero-call suite.
- Never record real audio. Voice tests use fake engines/recorders, not native addons.
- Tests use temporary TEST roots. Never write to `~/.local/share/seed-me` or
  `~/.local/share/tinkery`, or change operator sessions, installs, launcher, Pi,
  Pi Voice, or the operator's Seed Me checkout.
- Read relevant tests before changing behavior. Add regression tests and verify
  important protections go red when reverted. Restore the protection afterward.
- Read the relevant design below before implementation; update it when decisions
  change. Backlog proposals are not shipped behavior or build permission.
- When refining agent-facing docs, use `writing-for-agents` if available: subtract
  first, keep one authoritative home per rule, and link task-specific detail.
  Preserve safeguards and runnable commands; brevity is not permission to weaken them.
  `tests/test_documentation.py` fails when a link, heading, repository path or
  documented option no longer exists, or a page can't be reached from `AGENTS.md`.
- Leave the repository at rest ("git equanimity"): on `main`, matching `origin/main`
  after a fetch, clean, with no stashes, extra worktrees or leftover branches.
  While a PR is open, say so instead of cleaning it up. Delete only merged
  branches (`git branch -d`); never force-delete, drop stashes or remove worktrees
  to look clean.

## Layout and design entry points

Read the design for the task, not every page. [Architecture](docs/design/architecture.md#modules-today)
maps individual modules to their responsibilities.

| Work | Code | Read first |
| --- | --- | --- |
| CLI, terminal loop, snapshots | `src/main.rs` | [principles](docs/design/principles.md) |
| Module boundaries, design limits | `tests/architecture.rs` | [architecture](docs/design/architecture.md) |
| Menu, session discovery, measure | `src/menu.rs`, `src/tend/discovery.rs`, `src/measure.rs` | [menu](docs/design/menu.md) |
| Think: conversation, prompts, views, drafting, records, cards | `src/think/` | [think](docs/design/think.md) |
| Think evaluation runner | `examples/think_eval.rs` | [think evaluation](docs/design/think-evaluation.md) |
| Goal composition, validation, shared rendering | `src/goal/`, `src/think/brain_dump/board.rs` | [goal block](docs/design/goal-block.md) |
| Review, saving, tend, continuation | `src/think/brain_dump/goal_view.rs`, `src/tend/`, `src/seed_me/` | [tend and handoff](docs/design/shape-and-handoff.md) |
| Local voice and setup | `src/voice/`, `voice/setup.mjs`, `voice/helper.mjs` | [voice](docs/design/voice.md) |
| Rendering, graph and input primitives | `vendor/pinstar/`, `vendor/graf/`, `src/lib.rs` | [principles](docs/design/principles.md), [Pinstar API](vendor/pinstar/API.md), [Graf API](vendor/graf/API.md) |
| Tests | inline Rust tests, `tests/`, `voice/*.test.mjs` | relevant tests; `tests/vt_screen.py` is a helper |

## Cold setup

From the clone's root, on a fresh Debian/Ubuntu Linux box with Bash and Git:

```bash
./scripts/cloud-setup.sh
```

The script installs missing build/Python prerequisites, stable Rust with rustfmt
and Clippy, and Node if missing. It checks out the **same Seed Me commit as CI**
into ignored `.ci-seed-me/` and prints shell exports. It refuses dirty or unrelated
fixture checkouts rather than resetting them; rerunning is safe. A local Git exclude
ignores generated Python bytecode without changing tracked Seed Me files.
It fetches only packages and Git repositories: no providers, microphones, native
voice engines or speech models. Missing apt packages need root or sudo.

CI uses **stable Rust** on Ubuntu and macOS. Local Clippy must use the same updated
stable toolchain, not an older default. The setup script updates stable in the
fresh cloud box. On another development machine, arrange that toolchain explicitly
before checking; do not alter an operator installation without approval.

On macOS, provision Rust (stable, rustfmt, Clippy), Python 3 and Node separately;
the Linux bootstrap deliberately refuses to install there. Clone Seed Me into
`.ci-seed-me`, check out the fixture `ref` in `.github/workflows/ci.yml`, then use
the exports below. No access to another developer's files is needed.

## Full zero-call verification

CI runs documentation contracts instead of runtime checks only for modified files
on [the classifier's prose allowlist](scripts/ci-scope.py). Fenced, indented or
recognized inline command examples in either revision, mixed paths, unknown paths, setup,
instruction, and CI changes retain full checks. Routing failures block the
always-running `CI ready` gate, which checks the exact expected job results.
[Regression tests](tests/test_ci_scope.py) cover routing and failure handling.
The full verification block below remains authoritative for code changes.

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

For interactive practice, see [README](README.md#try-it); the installed launcher
is not needed. `--snapshot` prints a 100×30 view without terminal mode or provider
calls. `NO_COLOR=1` or `--no-color` preserves non-color cues. Real drafting requires
explicit `--shape-pi`, `--model` and `--seed-me` plus operator-authorized budget;
[terminal setup](docs/terminal-setup.md#launch) has an isolated trial example.

## Work intake

Concrete tasks live in GitHub Issues; unpublished ideas have one home in the
[parked backlog](docs/research/shared/backlog.md), grouped by
[area](docs/design/architecture.md#phases-and-the-dependency-rule). Review proposals
with the operator before publication; replace published local specifications with
issue links while retaining design contracts and historical findings.

Use the Bounded task issue form. Triage applies exactly one `area:` label and one
kind (`bug`, `enhancement`, `documentation`, `research`), a release milestone where
relevant and native blocking links. Its dropdowns do not apply labels automatically;
priority is a separate triage decision. Unapproved work stays `needs-approval`.
`approved` summarizes an explicit scoped operator decision, never creates permission;
material scope changes return to `needs-approval`. Publication, design acceptance,
implementation, data use, provider budget and merge permission remain separate.

Each approved slice gets one branch and PR, inherits the guardrails above and stays
bounded by its issue. Closing requires completion evidence or an explicit reason
such as superseded/not planned. Link the issue and report observed Rust/Node/Python
test counts, PTY journeys/sizes, failures and reversion probes separately from
operator dogfood. Stub tests establish mechanics, not goal quality on real dumps.
