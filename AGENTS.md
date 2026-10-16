# Working on Tinkery

**Read [docs/yohaku.md](docs/yohaku.md) first:** the constitution wins over guides,
designs and briefs. Only the operator amends it; report discrepancies, never edit it.
[README.md](README.md) describes shipped behavior.

## Guardrails

- Subtract first; say each thing once, plainly.
- Check changeability: name the [architecture diagnostics](docs/design/architecture.md#design-score)
  moved. Module and `pub` comments state promises and refusals, not implementation;
  the build requires them.
- Preserve operator originals verbatim; readings are guesses. Keep the confirmed
  goal unchanged through saving and handoff.
- Navigation/copying never launch agents. Goal confirmation is neither seed
  confirmation nor implementation permission.
- Use an issue, branch and PR. Merge approved/operator-requested work only with green
  CI and no conflicts. Drafts-only requests permit neither creation nor merging.
- Model/provider calls require an explicit budget; default zero. `live_*` examples
  and eval commands are outside zero-call verification.
- Never record real audio; voice tests use fake engines/recorders, never native addons.
- Use temporary TEST roots. Never write to `~/.local/share/seed-me` or
  `~/.local/share/tinkery`, or change operator sessions, installs, launcher, Pi,
  Pi Voice or the operator's Seed Me checkout.
- Read relevant tests/design before implementation; update changed decisions.
  Add regressions, prove important protections fail when reverted, then restore them.
  Backlog proposals confer neither shipped behavior nor build permission.
- For agent-facing docs, use `writing-for-agents`: one rule per authoritative home,
  task detail behind pointers. Preserve safeguards and runnable commands.
  `tests/test_documentation.py` checks links, headings, paths, options and reachability.
- At rest: fetched `main` matching `origin/main`, clean, no stashes, extra worktrees
  or leftover branches. Report open PRs instead of cleaning them up. Delete only
  merged branches (`git branch -d`); never force-delete branches, drop stashes or
  remove worktrees for cleanliness.

## Layout and design entry points

Read only the task's design; [architecture](docs/design/architecture.md#modules-today)
maps modules to responsibilities.

| Work | Read first |
| --- | --- |
| CLI, rendering, graph, input | [principles](docs/design/principles.md), [Pinstar API](vendor/pinstar/API.md), [Graf API](vendor/graf/API.md) |
| Module boundaries | [architecture](docs/design/architecture.md) |
| Menu, discovery, Measure | [menu](docs/design/menu.md) |
| Think | [think](docs/design/think.md) |
| Think evaluations | [think evaluation](docs/design/think-evaluation.md) |
| Goal composition/validation/rendering | [goal block](docs/design/goal-block.md) |
| Saving, Tend, Shape, continuation | [tend and handoff](docs/design/shape-and-handoff.md) |
| Voice/setup | [voice](docs/design/voice.md) |

## Cold setup

Fresh Debian/Ubuntu clone with Bash and Git:

```bash
./scripts/cloud-setup.sh
```

Installs missing prerequisites, stable Rust/rustfmt/Clippy and Node; checks out CI's
Seed Me ref in `.ci-seed-me` and prints exports. Reruns refuse dirty/unrelated fixtures;
Git-local excludes keep bytecode untracked without fixture edits. Downloads only
packages/Git, not providers, microphones, native voice engines or speech models.
Apt may need root/sudo.

CI uses stable Rust on Ubuntu/macOS; use that updated stable for Clippy.
Never alter operator toolchains without approval. On macOS, provision Rust, Python 3
and Node separately (bootstrap refuses macOS), then clone Seed Me into `.ci-seed-me`
at `.github/workflows/ci.yml`'s ref. No other developer's files are needed.

## Full zero-call verification

Run this exact block from the repository root after setup; cold-clone CI runs it
when setup may change. [CI routing](scripts/ci-scope.py) and its
[regressions](tests/test_ci_scope.py) define prose-only exceptions and fail-closed gates;
code changes require the full block.

Node is only for `voice/*.test.mjs`. Keep the invalid `TINKERY_NODE` for voice PTYs;
Python journeys use the real binary, stub providers and temporary roots.

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

[Try it](README.md#try-it) without the installed launcher. `--snapshot` prints
100×30 without terminal mode/providers; `NO_COLOR=1`/`--no-color` retain non-color
cues. [Terminal setup](docs/terminal-setup.md#launch) has isolated real-drafting instructions.

## Work intake

[Issues-board](.agents/skills/issues-board/SKILL.md) owns filing, triage, board operations
and parallel-work rules. Concrete tasks live in issues; unpublished proposals in the
[parked backlog](docs/research/shared/backlog.md), grouped by
[area](docs/design/architecture.md#phases-and-the-dependency-rule).

Unapproved work stays `needs-approval`. `approved` records scoped operator permission,
never grants it; material scope changes return to `needs-approval`. Publication, design
acceptance, implementation, data use and budget are separate decisions.

One branch/PR per approved slice; link the issue and stay within its scope. Close with
completion evidence or an explicit reason. Report Rust/Node/Python counts, PTY
journeys/sizes, failures and reversion probes separately from operator dogfood:
stubs prove mechanics, not real-dump goal quality.
