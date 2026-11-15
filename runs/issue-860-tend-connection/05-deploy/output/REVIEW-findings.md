Round: 1
Reviewer session: /root/review_860_code

## Verdict

Changes required. One Important finding blocks review completion.

Reviewed detached candidate `ea6242bec05cfb80b1bdd1bdae13eb58f7388467`, with product code at `6765119`, against `origin/main` (`2c0c8f1`). The later PR receipt commit is outside this review and does not change the product code.

## Important

### 1. Complete publication syncs when resuming an existing destination

Location: `src/seed_me/connection_save.rs:151–152`.

`Pending::resume` returns immediately when `inspect` finds the published destination. The first attempt performs the directory syncs only after `publish`, at lines 214–216. If the process stops after rename, or either sync returns an error, the next explicit resume takes the early return. It never retries the outstanding directory syncs. `Context::save` can then persist `saved` and open Shape although publication durability remains unconfirmed.

This breaks the brief's requirement to persist parent-directory updates before reporting success. File content checks establish that the destination is readable now. They do not establish that its renamed directory entry is durable. A later machine failure can lose an entry that the application has reported as saved.

Minimal fix: after verifying an existing destination, explicitly sync both publication parent directories before returning the receipt. Propagate sync failures and retain the pending receipt. Keep `inspect` read-only. Continue to bypass source-membership checks for an already-published goal.

Add focused failure coverage for publication followed by a failed or interrupted directory sync. Explicit resume must complete both syncs before success. A remaining sync failure must keep the operation pending. No power-loss claim is required.

## Checked scope and evidence

The implementation remains within one explicit connection pass and one shared goal-save path. It does not add a wiki, background discovery, phase imports, or automatic drafting.

I inspected automatic source selection, shared freshness checks, request and consent handling, proposal review, editor retention, private file modes, fixed-destination recovery, typed origin validation, Shape and Approve refusals, and existing retry evidence copying. No further Important finding was established.

The SDLC status command reported current verification for this candidate. The receipt records passed verification after all 16 checklist attestations. Its source inputs remain unchanged in the review checkout. The recorded full check reports 1120 Rust passes and one ignored test, 39 Node passes, 84 Python files, formatting, and Clippy. I inspected the receipt, test log, tests, and handoff. I did not rerun the full checks or mutate their marks.

The terminal evidence uses fake providers and TEST roots. It includes real process restarts and reconstructed interruption states. It does not prove real power-loss behavior or human discovery usefulness. Linux execution and current forge checks remain the owner's responsibility. No real provider call occurred during this review.

The duplicate-handoff reversion removes the published-destination fast path. Its observed failure proves recovery despite changed membership. Exclusive publication and fixed identity provide separate protection against duplicate destinations. Keep those evidence claims distinct.

## Review method

`principle-prove-it-works` directed inspection of the actual candidate, verification receipt, test code, and saved output rather than relying on the worker's summary. `writing-for-agents` kept the repair instruction in this report without changing project rules.

The user has not authorized a merge. This review does not grant human acceptance or merge permission.
