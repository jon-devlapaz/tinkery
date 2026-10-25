# Yohaku: the constitution of Tinkery

Tinkery is the substrate for the operator’s agentic work: think, tend, shape, approve, watch, taste, measure.
Agents do the repeatable work; the operator decides what runs and judges the result.

This constitution wins over guides, designs, briefs and instincts. Only the operator amends it.

## Purpose

**Useful, verified changes per operator hour.**

Generating code is cheap; the operator’s attention is not. Questions, explanations and reviews spend it.
Judge changes by useful outcomes relative to that attention—not by activity, code volume or agent count.
Improvement requires evidence; this purpose grants no permission to act.

*Yohaku no bi* describes deliberately composed space; *ma* includes the intervals between things and actions.
Their value lies in the relationships they make possible, not in emptiness alone
([Seattle Japanese Garden](https://www.seattlejapanesegarden.org/blog/2016/03/15/yohaku-no-bi-the-beauty-of-empty-space),
[Deeper Japan](https://www.deeperjapan.com/deeper-views/ma-and-modern-minimalism)).

For Tinkery: **leave room for judgment, not gaps in evidence or consent.**

## The articles

### 1. Intent remains sovereign.

Begin Think with the operator’s words, not suggestions. Keep what they type or say exactly.
Never rewrite, negate or reinterpret their words as fact. Preserve the displayed, confirmed goal unchanged
through saving and handoff.

Visibly distinguish Tinkery’s interpretations as guesses and its proposals as provisional until explicitly
confirmed; distinguish both from operator words. Missing intent becomes a question, not an invention.

**The test:** are originals intact, confirmed goals unchanged, and Tinkery-authored interpretations and
proposals visibly distinguishable from operator words at a glance?

### 2. Action stays within explicit authority.

Tinkery starts no agent, harness or command, and writes no durable record, without an explicit action in its
interface by the operator or an agent acting within the operator’s explicit authorization.

Copying is not running. Readiness is not consent. Goal confirmation, seed confirmation and implementation
permission are separate.

Ask the operator only what only they can decide. Investigate discoverable facts within the work and budget
they authorized; ask before going beyond them.

Keep work local and private by default. Voice stays on the operator’s machine. Nothing leaves without the
operator knowing exactly what will leave and when.

**The test:** could this action happen without the operator or an explicitly authorized agent pressing a key
that says what it does? Does the operator know exactly what leaves their machine, and when?

### 3. Claims stop where evidence stops.

Distinguish proposed, attempted, observed, checked and accepted. Interpretations are not observations.
Present evidence-backed status as status, not as a guess. Passing checks establishes neither operator
acceptance nor improved outcomes.

Show unknowns as unknown. Say failures plainly, with what to do next. Never invent information to make a
result look complete.

**The test:** does each claim have matching evidence, with its limits and remaining unknowns visible?

### 4. Make room for informed judgment.

Show only what is needed now.

Focus on one reading, at most one active question, and one clear next step. Keep the originals, alternatives
and evidence needed to judge that step available.

Every screen says what comes next, in plain words, and how to leave. Preserve meaningful decision points
without adding needless acknowledgments.

Intent, authority and truthful evidence constrain simplification. Keep necessary consequences, uncertainty
and exits legible, even when they require more space or attention.

Among choices that preserve these protections, favor the one that leaves more room for judgment, keeps
the operator more in control, and makes useful work easier to verify.

**The test:** does the screen focus on one reading, ask at most one active question, provide the context
needed to judge the next step, and make that step and the way out clear without help?

## For anyone changing Tinkery, human or agent

- **Subtract first.** Before adding a field, key, check, structural mechanism or screen text, ask what can be
  removed while preserving the protected behavior. Prefer the smallest coherent, reversible change;
  compare added complexity with a simpler approach.
- **Prove behaviour, not effort.** Test what the operator sees and does. Add a reversion probe for anything
  protecting an article above. Preserve verification’s ability to reject faulty work; never weaken checks
  to manufacture success.
- **Move each correction left.** When the operator corrects an agent, make the mistake impossible in code,
  or else a check that fails. Write a rule only when neither can hold it; review is the last resort.
  Preserve the correction’s underlying protection without turning a contextual preference into a universal
  prohibition.
- **Keep the next change clear.** Give responsibilities clear boundaries and rules one authoritative home.
  Hide implementation decisions behind interfaces so changing them need not spread through callers.
- **Keep the design true.** For design updates, verification and approved-work issue, branch and PR rules,
  follow the [working guide](../AGENTS.md#guardrails).

## Colour has one job each

| Colour | Job |
|---|---|
| Ink | the operator’s words, and the goal as it stands |
| Muted grey | hints, the key bar, and lines that didn’t change |
| Italic | Tinkery’s interpretation of intent—a guess |
| Jade | where you are: the question, the current place |
| Rust | consequence only, always with `!` and words saying what is lost |

Colour is never the only signal. With `NO_COLOR`, the same meanings carry through bold, dim and underline.

## Design contracts

- **Think interactions:** readings, questions, review and stopping → [Think](design/think.md).
- **Goal structure:** composition and rendering → [goal block](design/goal-block.md).
- **Persistence and handoff:** saving, receipts, Shape and continuation → [Tend and handoff](design/shape-and-handoff.md).
- **Presentation:** visual conventions and controls → [principles](design/principles.md).

*Amended by the operator on 2026-10-15: Tinkery is the substrate where agentic work happens, not only its front door.*

*Amended by the operator on 2026-10-16: agents may merge approved work once its checks pass.*
