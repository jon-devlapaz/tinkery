# Yohaku: the constitution of Tinkery

Tinkery is the substrate for the operator's agentic work: think, tend, shape, approve, watch, taste, measure.
Agents do the repeatable work here; the operator decides what runs and judges the result. This document
is its constitution. When a design choice, a brief or an instinct conflicts with it, this document wins. Only the
operator amends it.

*Yohaku no bi* is the Japanese idea that the empty space in a painting or a garden is not absence but what lets
the essential stand ([Seattle Japanese Garden](https://www.seattlejapanesegarden.org/blog/2016/03/15/yohaku-no-bi-the-beauty-of-empty-space)).
A stone stands out because of the raked gravel around it, not because it is marked. Tinkery works the same way.

## The articles

**1. Attention is the scarce resource.** Generating code is cheap; the operator's attention is not.
**Why:** every line on screen, every question and every review costs the one thing that doesn't scale.
**The test:** does this change raise useful, verified changes per operator hour, or does it spend attention?

**2. Empty space is the default.** Show only what is needed now. Every element earns its place, and removing
something comes before adding something.
**Why:** clutter hides the one thing that matters, and every extra element is something to learn.
**The test:** can anything on this screen go without losing what the operator needs right now?

**3. The operator's words are sovereign.** What they type or say is kept exactly. Tinkery never rewrites, negates
or reinterprets it as fact. Tinkery's own words are visibly a guess.
**Why:** a goal that drifted from the person's words is someone else's goal.
**The test:** is every original intact, and can the operator tell their words from Tinkery's at a glance?

**4. Nothing runs by itself.** Tinkery starts no agent, harness or command, and writes no durable record, without
an explicit action in its interface by the operator or an agent acting within the operator's explicit
authorization. Copying is not running. Readiness is not consent.
**Why:** the operator decides; Tinkery prepares.
**The test:** could this action happen without the operator or an explicitly authorized agent pressing a key
that says what it does?

**5. One at a time.** One reading, at most one active question, one next step.
**Why:** choices compete for attention; one clear thing invites an answer.
**The test:** does the screen focus on one reading, ask at most one question, and make the next step clear?

**6. Never stranded.** Every screen says what comes next, in plain words, and how to leave.
**Why:** a dead end after real effort is the most expensive moment in the tool.
**The test:** on any screen, can the operator say what to do next without help?

**7. Honest over reassuring.** Unknown is shown as unknown. Failures are said plainly, with what to do.
Nothing is invented to look complete.
**Why:** a confident wrong answer costs more than an honest gap.
**The test:** is anything on screen claiming more than Tinkery actually knows?

**8. Local and private by default.** Voice stays on the operator's machine. Nothing leaves without the operator
knowing it will.
**Why:** a place to think has to be safe to think in.
**The test:** does the operator know exactly what leaves their machine, and when?

## How yohaku shows up today

- **Think starts with the operator's words,** not suggestions. For readings, questions, stopping and help,
  read [Think](design/think.md).
- **Missing intent becomes a question, not an invention.** For goal structure and rendering,
  read the [goal-block contract](design/goal-block.md).
- **Changes speak without extra decoration.** For changed-line emphasis and controls,
  read [principles](design/principles.md).
- **The confirmed goal stays unchanged through saving and handoff.** For receipts, copying and Shape,
  read [Tend and handoff](design/shape-and-handoff.md).

## Colour has one job each

| Colour | Job |
|---|---|
| Ink | the operator's words, and the goal as it stands |
| Muted grey | hints, the key bar, and lines that didn't change |
| Italic | Tinkery's reading (a guess) |
| Jade | where you are: the question, the current place |
| Rust | consequence only, always with `!` and words saying what is lost |

Colour is never the only signal. With `NO_COLOR`, the same meanings carry through bold, dim and underline.

## For anyone changing Tinkery, human or agent

- **Subtract first.** Before adding a field, a key, a check or a line on screen, ask what can be removed.
- **Ask the operator only what only they can decide.** Investigate discoverable facts within the work and
  budget they authorized; ask before going beyond them.
- **Prove behaviour, not effort.** Test what the operator sees and does. Add a reversion probe for anything that
  protects an article above.
- **Move each correction left.** When the operator corrects an agent, make the mistake impossible in the code,
  or else a check that fails. Write a rule only when neither can hold it; review is the last resort.
- **Keep the design true and the operator in control.** For design updates, verification and approved-work
  issue, branch and PR rules, follow the [working guide](../AGENTS.md#guardrails).

When unsure, choose the version of the change that leaves more empty space and the operator more in control.

*Amended by the operator on 2026-10-15: Tinkery is the substrate where agentic work happens, not only its front door.*
*Amended by the operator on 2026-10-16: agents may merge approved work once its checks pass.*
