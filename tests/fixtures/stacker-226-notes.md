# Stacker retry (#226)

`stacker-226.json` is a verbatim copy of the TEST record from the Stacker retry run:

- Source: `/tmp/tinkery-TEST-stacker2-dgATpt/think/2026-10-18T15-08-28.771Z_cc1413f8-5b39-4816-b3f1-b5db51895cef.json`
- SHA-256: `88f07b7a75f1676d88e5bc6318cb367459bc9af43bf9a3c39cebc5ce332a7c2e`

The record contains decoded candidates, not raw model replies. The frozen stub
replays those candidates through the production decoder and board verification,
using the exact ordered words and answers. No provider is started. This checks
mechanics, not model quality or operator recognition.

The second reading offered a binary Done-when choice. The third stopped after
`stacker available to tink-arcade`. The fourth includes the operator's later
restatement of style, sounds and mechanics; these must not be invented earlier.
The regressions check both earlier failures and that the fourth reading can settle.

Run with `cargo +stable test --locked frozen_stacker_226 -- --nocapture`.
Before changing production code: two failures (binary question and premature
readiness), one pass (explicit criteria). With the structural guard: three passes.
The reversion probe restored the original `board.rs` while retaining the frozen
fixture and tests: the same two failures returned; restoring the guard passed all
three again. No prompt was changed and no live quality evaluation was run.

The later operator correction during #264 keeps these observations and the frozen
record unchanged, but removes the answer-length follow-up policy. The replay now
checks that a selected success question is open-ended, a short answer does not
manufacture another question or invent the later criteria, and explicitly supplied
criteria still settle. Mechanical readiness is not a completeness judgment; see
[Think's question contract](../../docs/design/think.md#questions).
