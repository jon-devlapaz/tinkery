# Tinkery

## The big idea

A calm substrate where a messy thought becomes useful, checked agent work without
losing your words or control. Nothing runs without you: one next step, with a way out.

## The journey

**Think → Tend → Shape → Approve → work happens → Watch → Taste → Measure**

| Place | Purpose |
| --- | --- |
| **Think** | Clarify what you want: one reading, one question at a time. |
| **Tend** | Choose a saved goal and open its detail. |
| **Shape** | Run Seed Me's interview to turn the goal into a seed. |
| **Approve** | Understand the decision and exactly what you're authorizing. |
| **Watch** | See progress, blockers and decisions needing your attention. |
| **Taste** | Compare the result with your original goal; accept or correct it. |
| **Measure** | Judge accepted, checked results against your effort. |

This is the intended journey, not all shipped behavior. Originals remain verbatim;
confirmed goals stay unchanged through handoffs. Readings and suggestions are guesses.

## What Tinkery builds on

Seed Me owns saved goals, working briefs and separate decisions; external SDLC owns
implementation/check/review orchestration; GitHub holds issues, PRs, checks and reviews.
You authorize work and judge results. Tinkery connects these records, not competing approvals.

Goal confirmation, brief confirmation and implementation approval are separate.
Copying a prompt authorizes none of them and starts no agent. Voice is local dictation.

## What works today

- **Think:** clarify a dump and explicitly save the confirmed goal, unchanged, to
  `~/.local/share/seed-me/sessions/` with a record in `~/.local/share/tinkery/think/`.
  When started in Git, the menu names the repository and the record keeps its origin.
- **Tend:** active goals form a needs graph. Link/unlink goals, open detail, or copy
  continuation prompts for Claude, Codex or Pi without changing Seed Me sessions.
  Press `r` in detail for [Think history](docs/design/shape-and-handoff.md#think-history)
  when its saved record verifies.
- **Shape:** press `s` in goal detail or choose menu place 3 to see the current question,
  waiting draft or verified ready seed. Real drafting requires `--shape-budget CALLS`
  (default zero). `F2` saves an answer and lets an agent investigate the next with
  read-only tools; `F3` confirms the seed in your words. `F5` shows the whole goal and decisions.
- **Measure:** reads an existing ledger report or says unavailable.

**Approve, Watch and Taste aren't built.** The complete journey and its benefit remain unproven.

## Where we’re going next

[Goal-to-run binding](docs/design/job-records.md#goal-to-run-identity-prerequisite)
is an **unaccepted, unimplemented proposal**: verify permanent SDLC run identity first,
then explicitly bind a goal to one exact run. Never infer matches from names; a shared
reader verifies identities and current evidence. Recreating a name must not reuse
identity; the first binding is immutable, detects duplicates and defers changes/relocation.

Absorbing useful Substrate responsibilities is planned, not complete; Measure still
uses its ledger. See the [first-release plan](docs/design/first-release.md) and
[task index](docs/design/first-release-issues.md). Published tasks are not implementation
permission; unpublished ideas belong in the [parked backlog](docs/research/shared/backlog.md).

## What success means

**Useful, verified changes per hour of your attention**, not agent/code volume.
Active effort is not elapsed time; passing checks is not your acceptance. Unknown
evidence stays unknown. Improvement needs a separately permitted baseline and trial;
it is not a measured claim.

## Try it

Fresh Debian/Ubuntu clone:

```bash
./scripts/cloud-setup.sh
export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PWD/.cloud-tools/node/bin:$PATH"
cargo +stable run --locked -- --home
```

Think is practice: no model calls or saved goals. Tend writes needs only on explicit
link/unlink. F2 sends, Enter adds a line, F1 helps, F10 returns to the menu.
Add `--snapshot` for text output.

Real drafting requires `--shape-pi --model provider/model-id --seed-me PATH` and an
explicitly authorized budget; provider charges may apply. `--help` lists options.
[Terminal setup](docs/terminal-setup.md#launch) gives an isolated trial.

## Read next

- [Constitution](docs/yohaku.md): governing principles.
- [Working guide](AGENTS.md): macOS setup, verification and contribution rules.
- [Design](docs/design/principles.md): decisions and marked proposals.

GPL-3.0-only; see [LICENSE](LICENSE).
