---
name: software-design
description: Use when designing, changing or reviewing a Tinkery module, `pub` interface or phase boundary, or naming the design-score rows a change moves.
---

Design for the next reader: an agent with a context window and grep. Complexity is
whatever makes Tinkery hard to understand or change. It arrives in small pieces, as
change amplification, cognitive load and unknown unknowns, caused by dependencies and
obscurity. The vocabulary is John Ousterhout's *A Philosophy of Software Design*;
[architecture](../../../docs/design/architecture.md) owns the module map, limits and
design score.

An agent's default is _tactical_: the smallest diff that makes the check pass. Agent
adoption has been measured raising cognitive complexity by about 39%
([Agarwal et al., 2026](https://arxiv.org/abs/2601.13597)). Work _strategically_:
each change leaves the design it touches simpler, or at least no worse.

## Steps

1. **Locate the decision.** Name the decision the change touches (a format, rule,
   lifetime or policy) and the one module that owns it in the
   [module map](../../../docs/design/architecture.md#modules-today). Done when the
   decision has exactly one home and every touched module still has a one-sentence
   description. If the change would teach the decision to a second module, move it first.
2. **Design it twice.** For a new or changed `pub` interface, sketch two materially
   different shapes as the caller's code, not variations of one. Choose the one whose
   common call is simplest; implementation ease gets no vote. Done when the PR names
   the losing shape and why it lost.
3. **Write the interface comment first.** State the promise and the refusal a caller
   needs, before the body. A comment that is long, must explain how it works, or
   resists a precise name means the abstraction is wrong: return to step 2.
4. **Pull complexity down.** The owner absorbs its special cases, defaults and
   failure handling so callers stay simple. Prefer a default to a parameter, and a
   general case that covers the situation to an `if` for it. _Define errors out of
   existence_ by changing semantics, never by hiding failure (see below).
5. **Subtract.** Review the diff against every red flag below and delete before
   adding. Done when each flag present is fixed or named in the PR as a deliberate cost.
6. **Record.** Name the [design-score rows](../../../docs/design/architecture.md#design-score)
   the change moves and update the module map when an owner changes. Design work
   beyond the slice becomes a `needs-approval` issue, not part of this PR.

## Red flags

| Flag | Looks like | Instead |
| --- | --- | --- |
| Shallow module | Interface nearly as large as the body; a new file or type for a few lines | Fold it into a deeper owner |
| Pass-through | A layer forwarding the same arguments to a similar signature | Remove it, or give it its own abstraction |
| Information leakage | One format, field name, ordering or constant known by two modules | One owner; others call it |
| Temporal decomposition | Modules split by when things happen: read, then parse, then save | Split by the knowledge each hides |
| Special case at the caller | An `if` or flag added upstream for one situation | A general case inside the owner |
| Overexposure | Common use requires knowing rare options | Default the rare ones |
| Conjoined code | One function can't be read without another | Merge them, or cut where they share nothing |
| Micro-split | One-use helpers the reader must hop between | Keep it inline, with a comment if needed |
| Speculative generality | Parameters, traits or hooks no current caller uses | Generality for today's callers only |
| Comment repeats code | The comment says what the line says, or how the body works | Units, invariants, reasons, refusals |
| Vague or split name | `data`, `info`, `handle`; two names for one concept | One precise name everywhere, so grep finds every use |
| Private reach | A test needs internals to check behavior | The interface is missing something; fix the interface |

## Where Tinkery bends the book

- **Errors:** an error is defined out of existence when the operation's meaning covers
  the case, such as removing an absent link succeeding. A fallback that turns unknown
  into a value is not that: Yohaku's _honest over reassuring_ wins, unknown stays unknown
  and a refusal is said plainly.
- **Length:** depth beats shortness. Split a function when its parts hide separate
  decisions, never to reach a line count. Ousterhout and Robert Martin
  [disagree here](https://github.com/johnousterhout/aposd-vs-clean-code); Tinkery sides with depth.
- **Investment:** the book's continual design investment is spent inside the approved
  slice, on code the slice touches. The operator's attention is the scarce resource,
  so a drive-by refactor costs more review than it saves.
