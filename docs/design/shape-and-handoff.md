# Shape and handoff

## Receipt

Successful explicit goal saving shows `Saved.`, the exact frozen goal block and
one next step: `Next: shape it into a seed with Seed Me.`

- `c copy prompt` sends OSC 52 with quiet write success/failure feedback and can
  be retried. This does not verify the host clipboard; see
  [clipboard support](../terminal-setup.md#clipboard-osc-52).
- `s open in shape` opens **this session's detail**, not the list.
- F10 returns to the menu. Standalone think also transitions into the menu.

Nothing launches, opens a browser, pastes into a harness or starts implementation.
Storage paths live in technical help (F1, then `h`), not the ordinary receipt or
detail. Help shows the [think record](think.md#think-record)'s path under Seed Me's.
If the record wasn't written, help and the receipt's muted notice say so; the goal
stays saved. A notice wraps whole rather than clipping, in the receipt and shape's
detail. The plain-text terminal exit receipt may include recovery locations.

## Shape list and detail

One row per session, newest first: **only the first line of its saved goal**, then
status and relative time. Truncate the headline with an ellipsis, not the status.
Old unstructured goals use their first line too. No preview lines, divider or count.

| Stored state | Label | Detail next step |
| --- | --- | --- |
| Confirmed goal | goal saved | Shape it into a seed with Seed Me |
| Draft seed contract | seed drafted | Confirm the seed draft with Seed Me |
| Seed confirmed for intake | seed ready | The seed is ready for intake |
| Ended session | ended | None; copying is unavailable |

Unknown and practice states are distinct, never mislabeled ready. Unreadable
ledger records are skipped rather than shown as ready, and the list says how many
couldn't be read. Sessions come from the menu's [background read](menu.md#session-discovery):
until it lands, shape says it is reading and Esc/F10 still leave; it never says
“no sessions” for an unknown or failed read.
Enter opens the stored block line by line using the receipt's shared component,
without `Saved.`. `c` copies the same continuation prompt again. Arrows / `j k`
move in the list; Esc goes detail → list → menu; F10 goes to menu. F1 helps.

## Continuation contract

One formatter owns the receipt/detail prompt. It names the skill and **existing**
session, requests `session.py read` and `status` before mutation, preserves the
confirmed goal and decisions/context, and stops if the session is inactive.
It asks Seed Me to draft `seed-contract.md` in that session and obtain explicit
confirmation of the displayed seed revision. It does not initialize a replacement
session or grant implementation permission. An operator manually pastes it into
an authorized harness. The prompt needs paths, even though the ordinary UI does not.

Confirmation uses the official Seed Me helpers, not a second schema. Recovery
errors remain explicit; tests exercise helpers only under temporary roots. The
Seed Me fixture is external and pinned by CI, not vendored or edited here.

## Rationale and verification

Compact title/state/recency, context-sensitive keys and repeatable copying follow
patterns in [lazygit](https://lazygit.dev/keybindings/),
[gh-dash](https://gh-dash.dev/getting-started/usage/) and
[fzf](https://github.com/junegunn/fzf/blob/master/README.md). These are design
inferences, not comparative user-study results.
Regression coverage includes exact multiline preservation, status/order/truncation,
repeatable prompt bytes, no ordinary-view paths, ended-session guards and receipt
`s` navigation at four PTY sizes.
