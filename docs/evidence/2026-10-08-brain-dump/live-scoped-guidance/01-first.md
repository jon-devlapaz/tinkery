# Tinkery / provisional board

Nothing confirmed. Unsaved.

## I think this is about… / guess

I think this is about making Tinkery checks understandable and low-friction for the next person, including whether each check used a real model or a stub and which questions should not recur.

Supports: f1, f2, f3, f4

## Desired experience / proposed

Before dogfooding, the next person can quickly understand what was actually checked, whether real models or stubs were used, and avoid questions already judged unuseful, without repetitive data entry.

## Doesn't fit yet

None identified by the agent; this does not mean everything fits.

## One question in focus

Should the receipt summarize the whole PR or identify each individual check?

## Possible approaches / not accepted

### Conditional candidate: automatically add a compact PR-thread receipt after checks run / candidate

**Benefit:** Low manual effort while making model usage and checked scope visible, if check results and model type are available automatically.

**Cost:** May omit context or create misleading receipts when checks are partial or stale.

**Undo cost / provisional:** Provisional: changing or removing posted receipts may require PR edits or follow-up clarification.

### Conditional candidate: provide a lightweight review view that people consult before dogfooding / candidate

**Benefit:** Can show richer context and support marking questions as unuseful without adding a form to every edit.

**Cost:** Requires the next person to visit another view and may reduce visibility in the PR thread.

**Undo cost / provisional:** Provisional: reverting to the current workflow may leave prior review records distributed across places.

## Your fragments / exact excerpts

### f1 / original 1 bytes 0..89

I keep losing track of which Tinkery checks used a real model and which just used a stub.

### f2 / original 1 bytes 90..209

I want a small receipt in the PR thread so the next person can see what's actually been checked before they dogfood it.

### f3 / original 1 bytes 210..269

But I don't want to fill another form with every tiny edit.

### f4 / original 1 bytes 270..351

I also want to be able to say 'that question wasn't useful' and not get it again.

## Original 1 / dump

I keep losing track of which Tinkery checks used a real model and which just used a stub.
I want a small receipt in the PR thread so the next person can see what's actually been checked before they dogfood it.
But I don't want to fill another form with every tiny edit.
I also want to be able to say 'that question wasn't useful' and not get it again.
