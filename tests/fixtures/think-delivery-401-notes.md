# Delivery constraints omitted in Think (#401)

`think-delivery-401.json` retains the exact initial words, submitted correction and
two decoded candidates from the agent-operated Room to Choose trial on October 25,
2026. The source is `docs/trial-evidence/think-record.json` in the private
[Room to Choose repository](https://github.com/jon-devlapaz/room-to-choose/pull/2);
the fixture records the source file's SHA-256. These are decoded candidates, not
raw provider replies. No credentials or provider logs are included.

The first reading omitted requirements to use Tinkery, record failures and manual
handoffs, preserve original words and use isolated TEST roots from the main goal.
It put them together in `other_goals`, shown as “also in your dump,” and still
satisfied structural readiness. Full review caught the omission; a refinement
recovered the requirements. The fixture keeps both readings unchanged.

`python3 tests/think_delivery_terminal.py` drives the compiled binary through
typing, F2, copy, help, review, correction and typed confirmation at 80×24 and
140×42, with and without color. A stub returns these candidates. The test checks
an honest review invitation, exact originals, no calls from navigation, and the
corrected displayed goal reaching the saved record and ledger unchanged.

This is a mechanical replay, not evidence that the live model now retains every
requirement. Prompt changes and a separately budgeted live comparison remain in
[#401](https://github.com/jon-devlapaz/tinkery/issues/401).
