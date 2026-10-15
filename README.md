# Tinkery

## The big idea

Tinkery is a calm substrate for agentic work: the place where a messy thought
becomes useful, checked work done by agents—without losing what you meant or
taking control away from you.

Agents do the repeatable work here, but none starts without you. It isn’t meant
to be another busy dashboard. It should show what matters now, one next step,
and how to leave.

## The journey

Imagine you say: “I want customers to find their invoices more easily.”

Tinkery’s intended journey is:

**Think → Tend → Shape → Approve → work happens → Watch → Taste → Measure**

| Place | What it helps you do |
| --- | --- |
| **Think** | Work out what you actually want. It offers a reading and asks one question at a time. |
| **Tend** | Choose a saved goal in a quiet garden and open its detail. |
| **Shape** | Turn a confirmed goal into a seed by running Seed Me’s interview inside Tinkery. Planned for the first release. |
| **Approve** | Understand exactly what decision is needed and what you’re authorizing. |
| **Watch** | See what’s happening, what’s blocked and what needs your attention. |
| **Taste** | Compare the result with your original goal. You decide whether it’s useful or needs correction. |
| **Measure** | See whether the process produced accepted, checked results with less effort from you. |

Once you confirm the goal, it stays unchanged through the handoffs. Suggestions
and guesses must never masquerade as your words. Your original words remain
intact.

This is the direction, not a claim that every place is built.

## What Tinkery builds on

The work happens in Tinkery, but some records belong to the tools it builds on.

- **Seed Me** keeps the saved goal, working brief and separate decisions.
- **SDLC** organizes the external implementation, checks and review process.
- **GitHub** holds issues, proposed code changes, checks and reviews.
- **You** supply permission and judge whether the result meets your intent.

Tinkery should connect those records and explain them—not invent a competing set
of approvals.

Confirming a goal, confirming a brief and approving implementation are different
decisions. Copying a prompt approves none of them.

## What works today

Today, Tinkery can:

- Help you think through a dump and explicitly save a confirmed goal. Confirming
  saves that goal, unchanged, as a Seed Me session in
  `~/.local/share/seed-me/sessions/`, and writes a think record to
  `~/.local/share/tinkery/think/`. Started inside a Git repository, Tinkery names it
  on the menu and the record keeps where the goal was made.
- Show active saved goals as rings in Tend's graph, with lines for what they need.
  Link or unlink goals, open their detail and copy a continuation prompt to Claude,
  Codex or Pi. In a goal's detail, press `r` to read its
  [Think history](docs/design/shape-and-handoff.md#think-history), when its saved record can be verified.
  Seed Me sessions stay unchanged.
- Show an existing ledger report in Measure, or say it’s unavailable.

**Shape, Approve, Watch and Taste aren’t built yet.** The complete journey and its
claimed benefit still need proving.

Nothing runs by itself. Copying a prompt starts no agent. Voice is dictation that
stays on your machine.

## Where we’re going next

The foundation is trustworthy connections.

If two jobs have similar names, Tinkery must not guess which one belongs to your
goal. So we’re proposing:

1. A permanent serial number for each external run.
2. An explicitly confirmed connection between your saved goal and that exact run.
3. A shared reader that checks those identities and brings back current evidence.

First check whether SDLC already provides permanent run identity; recreating a
run under the same name must not reuse it. The first binding version creates one
immutable connection and detects duplicates; changing or relocating it is deferred.

That would let Approve, Watch and Taste all talk about the same work, rather than
loosely related files. The [binding design](docs/design/job-records.md#goal-to-run-identity-prerequisite)
remains an unaccepted proposal, not implemented behavior.

We also intend to bring useful Substrate responsibilities into Tinkery, rather
than permanently requiring a separate Substrate aggregation service. That
migration is planned, not finished. Today’s Measure still uses its ledger report.

The [first-release plan](docs/design/first-release.md) and
[task index](docs/design/first-release-issues.md) explain the path. The twelve
published tasks organize that direction; publication does not approve their
implementation. Speculative ideas stay in the [parked backlog](docs/research/shared/backlog.md).

## What success means

Not “more agents running” or “more code generated.”

**Success means useful, verified changes per hour of your attention.**

You should explain yourself less, recover after interruptions, understand
failures, and retain the final judgment. Missing evidence should say unknown,
not pretend everything is green.

Active effort is different from elapsed session time. Passing checks is different
from your acceptance. We still need a separately permitted baseline and trial to
show that the complete workflow helps; improvement is a goal, not a measured
claim today.

## Try it

On a fresh Debian/Ubuntu clone:

```bash
./scripts/cloud-setup.sh
export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PWD/.cloud-tools/node/bin:$PATH"
cargo +stable run --locked -- --home
```

Think is practice: no model calls, no goals saved. Tend saves needs only when you
explicitly link or unlink. F2 sends, Enter adds a line,
F1 helps, F10 returns to the menu. Add `--snapshot` to print the screen as text.

Real drafting needs `--shape-pi --model provider/model-id --seed-me PATH` and an
explicitly authorized budget; it can incur provider charges. `--help` lists every
option. See [terminal setup](docs/terminal-setup.md#launch) for an isolated trial.

## Read next

- [docs/yohaku.md](docs/yohaku.md): the constitution and the meaning of calm.
- [AGENTS.md](AGENTS.md): macOS setup, the full test run, and rules for changing Tinkery.
- [First-release plan](docs/design/first-release.md): milestones and the finish line.
- [Task index](docs/design/first-release-issues.md): authoritative issue links and dependencies.
- [docs/design/](docs/design/principles.md): current decisions and clearly marked proposals.
- [Parked ideas](docs/research/shared/backlog.md): one home for unpublished proposals.

GPL-3.0-only; see [LICENSE](LICENSE).
