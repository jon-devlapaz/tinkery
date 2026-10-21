# Approve

Approve prepares the manual start of a shaped goal in tink-sdlc. It creates no run,
records no approval and starts no agent. Seed confirmation, definition approval and
implementation permission remain separate.

## Source and verification

The menu supplies its existing session discovery result, without Tend's repository
filter. Approve reads across that configured session root, oldest update first.
No skill or model configuration is needed. Only Tinkery-verified ready seeds not already started appear, including
completed sessions whose seed remains verified. Drafts, stopped sessions and unverifiable confirmations do not.
Failed discovery and unreadable seeds stay unavailable;
an incomplete scan cannot claim there are no waiting seeds.

`seed_me::ready` owns the handoff reading: exact confirmed goal, revision and the
seed's Acceptance criteria section, including original whitespace. It uses the
same ledger verification and canonical, shell-quoted intake formatter as Shape.
Seed and ledger bytes are checked before and after verification; observed changes
refuse the reading. This is a snapshot, not a live freshness claim. Return to the
menu and reopen to read again. Unsafe terminal controls are escaped only for display.

One menu-owned verifier survives visits. Reads run off input and paint paths through
disk reads. Leaving stops publication and further session
reads; the current file read finishes before the verifier is reused. Reopening while it
finishes refuses a replacement worker and asks the operator to reopen later. Late
results never reopen a view or populate a different visit.

## One seed at a time

The goal and acceptance checks precede the manual handoff explanation and commands:

```text
cd '<named checkout>' && python3 _system/scripts/sdlc.py new seed-title && python3 _system/scripts/sdlc.py stage seed-title 1 --seed-contract '<seed path>' --tinkery '<Tinkery executable path>'
```

One shell line changes to the explicitly named `--sdlc-root`, creates a run and
imports its seed and checks for definition review. The default slug comes from the
seed's H1 title (without `Seed:`): lowercase ASCII words joined by hyphens, capped
at tink-sdlc's 80 characters without a trailing hyphen. Paths use the shared canonical
shell quoting; no placeholder needs replacing. Existing run names still refuse creation.
Copying runs nothing and grants no approval. Missing checkout, an empty slug or
unsafe paths leave the verified seed readable with a reason and no copy action.
Copy uses the existing OSC 52 transport and feedback. For
[agent-confirmed seeds](think.md#confirmation-attribution-185), show why the external
human-only intake command is withheld, not a missing-checkout excuse.

| Key | Action |
| --- | --- |
| `n` / `p`, Right / Left | Next / previous seed, clamped. |
| Up / Down, `k` / `j`, PgUp / PgDn, Home / End | Read wrapped content. |
| `c` | Copy the combined command, only when available. |
| F1 / `?` | Help and bounded unreadable-session details; no underlying actions. |
| Esc | Close help or return to the menu. |
| F10 | Return to the menu. |

The footer always retains navigation. Empty state points to Shape or Watch.
[SDLC handoff](shape-and-handoff.md#sdlc-handoff) owns the exact join: matching
started seeds are hidden. Changed or unavailable originals get an unknown-start
notice and a Shape recovery step, not a waiting claim. Without `--sdlc-root`, or
if the runs cannot be read, say `Can't tell which seeds have started; none are hidden.`
and how to check. Opening reads only that checkout's status; it discovers no checkout.
Help retains the bounded reasons. Unknown start classification never conceals a
verified seed or justifies a complete empty-waiting claim.

## Verification

Shared-boundary tests prove verified-only selection, exact bytes, shared intake
quoting and refusal on changed sources. View tests cover copying, help isolation,
scrolling and abandoned reads. [PTY journeys](../../tests/approve_terminal.py)
use the actual Rust ledger and temporary TEST roots at 80×24 and 100×30,
with and without colour. They cover ordering, excluded drafts/stale seeds, the
combined command, empty/incomplete/unreadable roots, matched/edited/moved seeds,
no checkout, unreadable runs and unchanged source trees. The
[real handoff journey](../../tests/shape_sdlc_handoff.py) proves the move to Watch.
These prove mechanics, not operator acceptance or real-work benefit.
