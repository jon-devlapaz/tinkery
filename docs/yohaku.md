# Yohaku: the constitution of Tinkery

Tinkery is the operator's front door to agentic work: think, shape, approve, watch, taste, measure. This document
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
an explicit human action. Copying is not running. Readiness is not consent.
**Why:** the operator decides; Tinkery prepares.
**The test:** could this action happen without the operator pressing a key that says what it does?

**5. One at a time.** One reading, one question, one next step.
**Why:** choices compete for attention; one clear thing invites an answer.
**The test:** is there exactly one thing to read, one to answer, and one to do next?

**6. Never stranded.** Every screen says what comes next, in plain words, and how to leave.
**Why:** a dead end after real effort is the most expensive moment in the tool.
**The test:** on any screen, can the operator say what to do next without help?

**7. Honest over reassuring.** Unknown is shown as unknown. Failures are said plainly, with what to do.
Nothing is invented to look complete.
**Why:** a confident wrong answer costs more than an honest gap.
**The test:** is anything on screen claiming more than Tinkery actually knows?

**8. Local and private by default.** Voice stays on the operator's machine. Nothing leaves without the operator
knowing it will.
**Why:** a front door for thinking has to be safe to think in.
**The test:** does the operator know exactly what leaves their machine, and when?

## How yohaku shows up today

- **Think opens on one empty box,** with no suggestions while the operator writes.
- **One voice on the right:** "I think you mean…", then the goal block, then "One thing I'm not sure about", then
  "your answer".
- **The goal block** (see [design/goal-block.md](design/goal-block.md)): a goal line naming who, then `Why:`,
  `Done when:`, `Must:`, `Must not:`. A missing line becomes the next question, never an invention.
- **"Nothing I'm unsure about"** is the stopping rule. Questions stop when the goal is whole.
- **After an answer,** changed lines stay in full ink and unchanged lines go quiet. Nothing is added to show the
  change.
- **The key bar shows only what is possible now,** muted. Help has two layers: what to do now, then every key.
- **The operator's original stays plain ink.** No highlighting by default.
- **The receipt and shape show the confirmed goal exactly as confirmed,** with one next step and a prompt to copy.
  Paths live in "How it works", not on screen.

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
- **Ask the operator only what only they can decide.** If running something could answer it, run it.
- **Prove behaviour, not effort.** Test what the operator sees and does. Add a reversion probe for anything that
  protects an article above.
- **Keep the design true.** When a decision changes, update `docs/design/` in the same PR.
- **Never merge your own work.** Open a PR; the operator decides.

When unsure, choose the version of the change that leaves more empty space and the operator more in control.
