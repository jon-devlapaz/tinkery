# Working on Tinkery

**Read [docs/yohaku.md](docs/yohaku.md) first:** the constitution wins over guides,
designs and briefs. Only the operator amends it; report discrepancies, never edit it.
[README.md](README.md) describes shipped behavior.

## Guardrails

- Subtract first; say each thing once, plainly.
- Validate external data at its boundary and preserve those guarantees in types.
  Justify extra checks by the failure they prevent. Split functions when doing so
  makes the code easier to understand or change.
- Check changeability: name the [architecture diagnostics](docs/design/architecture.md#design-score)
  moved. Module and `pub` comments state promises and refusals, not implementation;
  the build requires them.
- Preserve operator originals verbatim; readings are guesses.
  Keep the displayed, confirmed goal unchanged through saving and handoff.
- Navigation and copying never launch agents. Goal confirmation is neither seed
  confirmation nor implementation permission.
- Work from an issue on a branch and open a PR. Merge it yourself once the issue is approved
  (or the operator asked for the change), CI is green and there are no conflicts.
  If the operator asks for drafts only, create and merge nothing until they say so.
- Model and provider calls require an explicit budget. The default is zero.
  Eval commands are outside zero-call verification.
- Agent operators (Pi via herdr) always launch Tinkery with `--operator agent`.
- Never record real audio; voice tests use fake engines and recorders, never native addons.
- Use temporary TEST roots. Never write to `~/.local/share/seed-me` or
  `~/.local/share/tinkery`, or change operator sessions, installs, launcher, Pi,
  Pi Voice or the operator's Seed Me checkout.
- Read relevant tests and design before implementation; update changed decisions.
  Add regressions, prove important protections fail when reverted, then restore them.
  Tests assert what users observe through public interfaces; never add test-only access
  to private state. Drive Think through its input, not `Conversation`'s capped test accessors.
  Backlog proposals confer neither shipped behavior nor build permission.
- For agent-facing docs, use `writing-for-agents`: one rule per authoritative home,
  task detail behind pointers.
  Preserve safeguards and runnable commands; brevity is not permission to weaken them.
  `tests/test_documentation.py` checks links, headings, paths, options and reachability.
- Create each task's worktree at `~/dev/worktrees/<repo>/<branch>`, never inside a checkout
  or beside one.
- At rest, keep a clean `main` matching `origin/main` after a fetch, with no stashes,
  extra worktrees or leftover branches. Report open PRs instead of cleaning them up. Delete only
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
| Goal composition, validation and rendering | [goal block](docs/design/goal-block.md) |
| Saving, Tend, Shape, continuation | [tend and handoff](docs/design/shape-and-handoff.md) |
| Voice/setup | [voice](docs/design/voice.md) |

## Cold setup

Fresh Debian or Ubuntu clone with Bash and Git:

```bash
./scripts/cloud-setup.sh
```

The script installs missing prerequisites, stable Rust with rustfmt and Clippy, and Node.
It prints the verification exports and can be rerun safely. It downloads only
prerequisite packages, not providers, microphones, native voice engines or speech models.
Apt may need root or sudo.

CI uses stable Rust: Ubuntu on pull requests, Ubuntu and macOS on `main`; use that updated stable for Clippy.
Never alter operator toolchains without approval. On macOS, provision Rust, Python 3
and Node separately (bootstrap refuses macOS). No external skill checkout or other
developer's files are needed.

## Full zero-call verification

Run `python3 scripts/full-check.py` from the repository root after setup. Its shared
launch guard gives the exact block below a temporary TEST HOME; do not run binary
verification against the operator's HOME. Cold-clone CI runs the block when setup may change. [CI routing](scripts/ci-scope.py) and its
[regressions](tests/test_ci_scope.py) define prose-only exceptions and fail-closed gates;
code changes require the full block. [The Python runner](scripts/python-tests.py) runs files
in parallel; with `TINKERY_TEST_TIER=pr`, as on pull requests, it skips only the slow journeys it
lists, which still run on every push to `main`.

Node is only for `voice/*.test.mjs` and `pi/*.test.mjs`. Keep the invalid `TINKERY_NODE` for voice PTYs;
Python journeys use the real binary, stub providers and temporary roots.

<!-- full-check:start -->
```bash
export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PWD/.cloud-tools/node/bin:$PATH"
export TINKERY_NODE=/tinkery-TEST-no-node
rustc +stable --version
node --version
python3 --version
cargo +stable build --locked
cargo +stable run --locked -- --operator agent --home --snapshot
cargo +stable run --locked -- --operator agent --snapshot --no-color
cargo +stable test --workspace --locked
node --test voice/*.test.mjs pi/*.test.mjs
python3 scripts/python-tests.py
cargo +stable fmt --all -- --check
cargo +stable clippy --workspace --all-targets --locked -- -D warnings
```
<!-- full-check:end -->

[Try it](README.md#try-it) without the installed launcher. `--snapshot` prints
100×30 without terminal mode or provider calls; `NO_COLOR=1` or `--no-color` retain non-color
cues. [Terminal setup](docs/terminal-setup.md#launch) has isolated real-drafting instructions.

## Work intake

[Issues-board](.agents/skills/issues-board/SKILL.md) owns filing, triage, board operations
and parallel-work rules. Concrete tasks live in issues; unpublished proposals in the
[parked backlog](docs/research/shared/backlog.md), grouped by
[area](docs/design/architecture.md#phases-and-the-dependency-rule).

Unapproved work stays `needs-approval`. `approved` records scoped operator permission,
never grants it; material scope changes return to `needs-approval`. Publication, design
acceptance, implementation, data use and budget are separate decisions.

Use one branch and PR per approved slice; link the issue and stay within its scope. Close with
completion evidence or an explicit reason. Report Rust, Node and Python counts, PTY
journeys and sizes, failures and reversion probes separately from operator dogfood:
stubs prove mechanics, not real-dump goal quality.
