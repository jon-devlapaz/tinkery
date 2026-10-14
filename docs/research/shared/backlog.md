# Shared backlog

Drafts, not permission. [Work intake](../../../AGENTS.md#work-intake) says how a draft becomes work.

## A calm dark theme

**Context:** The operator wants a calm, not generic-black terminal. The shipped
palette is light. Jade must still mean location; rust must still mean consequence.

**Build:** Propose a restrained dark palette for operator review; follow macOS dark
mode automatically, including changes while running. Define an honest non-macOS
fallback without treating unavailable appearance as known. Keep NO_COLOR working.

**Acceptance:** Review light/dark text mock-ups with the operator. Measure actual
foreground/background contrast for ordinary text (at least 4.5:1), including muted
hints and semantic jade/rust. Fake appearance-change tests cover automatic switching;
snapshot/PTY tests cover selection, consequence prompts and NO_COLOR's non-color cues.
Changing appearance must preserve input, navigation and the confirmed goal.

**Read:** [principles](../../design/principles.md), [think](../../design/think.md), [menu](../../design/menu.md).

**Don't:** Substitute generic black, add category colors or a settings maze,
change terminal/macOS preferences, break NO_COLOR, record audio, spend provider
budget without approval, or touch operator installs/data.

## Absorb substrate: keep runs and learn from them

**Context:** The operator decided on 2026-10-14 that tink-substrate's jobs fold into
Tinkery and substrate is deprecated once they are absorbed. Its Seed Me handoff
belongs to think and shape, its dashboard to watch, its retro to taste and its
archive to measure. A live tink-sdlc run that night left its only record in an
untracked worktree folder, and no step turned what it learned into anything lasting.
Watch itself is already planned as [R7](../../design/first-release-issues.md#r7);
this entry covers what R1 to R10 leave out.

**Build:** Two pieces, each its own slice. First, a run's record stays local,
outside Tinkery's history, and is kept only by an explicit operator action. Its
identity and revisions follow R2 and [R5](../../design/first-release-issues.md#r5);
R5 does not depend on this draft. Second, after Taste, the operator may ask for one
proposed lesson from the run. Before any model call, Tinkery shows what run text
would be sent and where, and sends it only on an explicit action within a budget. It
is shown as Tinkery's guess, with the change it would make (a test or a rule), where
that change would go, and the run it came from. When an accepted lesson doesn't
prevent a repeat, Tinkery suggests removing it; only the operator removes it.

**Acceptance:** Nothing is kept, drafted or learned without a keypress that says
what it does. Missing, unreadable, outdated, failed and interrupted records stay
distinct, and none reads as current success. A proposed lesson is written only when
the operator accepts that exact change; leaving writes nothing and says what comes
next. No run text leaves the machine except by that shown, explicit send. An accepted lesson names its source run.
Reversion probes protect the keypress gate and the distinct record states.

**Evidence:** Claude Code's [agent view](https://code.claude.com/docs/en/agent-view)
puts sessions that need input or review first and links to their transcripts.
Anthropic's [agent eval guide](https://www.anthropic.com/engineering/demystifying-evals-for-ai-agents)
and Hamel Husain's [error analysis](https://hamel.dev/blog/posts/evals-faq/why-is-error-analysis-so-important-in-llm-evals-and-how-is-it-performed.html)
turn reviewed failures into tests. Research on self-written agent memory
([Reflexion](https://arxiv.org/abs/2303.11366), [ExpeL](https://arxiv.org/abs/2308.10144))
shows methods, not that unreviewed lessons are safe.

**Read:** [first release](../../design/first-release.md), [the constitution](../../yohaku.md).

**Don't:** Add a multi-agent dashboard, notifications, restart controls, a hosted
trace service, or agent-written memory. Don't add features to substrate.
