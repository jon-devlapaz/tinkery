# Intact dump / exact-span direction

Operator-approved Ctrl-O screenshot transcription in `original-dump.txt`: **745 UTF-8 bytes**, no surrounding quotes or added newline. Typos are deliberately unchanged. SHA-256: `2aab458a45d27a2ca828a3c69e5c6e7b093099b7143da86e247ef94fd90dc5ce`. **No answer was supplied or submitted.** This is the approved transcription, not a claim of recovering bytes from the previous running application.

## Change

- A submitted dump/answer remains one intact viewer card. No automatic sentence splitting, no mandatory extracted cards. Other intact sources are available with Ctrl-PgUp/PgDn; Ctrl-O opens originals.
- Supporting quotes are highlighted in place; unresolved quotes are underlined. Authored glyphs and carbon foreground remain unchanged. Unmarked words are neutral, not implicitly rejected.
- The model supplies source ID, literal quote and zero-based occurrence. The app resolves byte ranges, validates exact spelling/whitespace/punctuation and whole grapheme boundaries, and rejects unsupported or overlapping support/unresolved annotations. No normalization or parser repair.
- Deliberate extraction only: mouse drag or Shift-navigation selects source text; Ctrl-E creates an exact linked card. Board `e` switches to the spatial extracted-card view; select a card and Enter returns to its exact original range. Titles use the person's words; a clipping cue never replaces them. Existing extracted-card positions remain owned by the operator. Agent-proposed split acceptance is not implemented in this slice.
- One truthful `Ctrl-O originals` header control, only after a source exists. No duplicate clipping labels. The brief now says never reword the person's text and never cut it apart for them.

## Actual model evidence: two calls, both dump-only

`openai-codex/gpt-5.6-luna`, low reasoning; Pi 1.1.0. Scoped Seed Me skill SHA-256 `8e8f4f07143b0bfa08c69134d7ff9e9dd009eafe28b32cb0c25f6d67e4792102`. Existing disabled-resource/unsaved transport boundaries remain; this is not an OS sandbox.

1. **Rejected**, `raw-provider/00-stdout.json`, `live/`, `first-live.log`: valid JSON syntax, wrong response shape. Two readings preserved the vision, but `misfits` contained an inferred relationship sentence instead of an exact-source anchor. The application rejected it, retained the intact original and showed no invented substitute. The question also imported “investigation” language and the first reading narrowed compounding toward codebase improvement. Kept as failed evidence.
2. **Accepted**, `raw-provider/01-stdout.json`, `contract-rerun/`, `contract-rerun.log`: one explicit manual rerun after clarifying the typed misfit contract and preserving the referent of “compounds itself.” No relaxed validation or automatic retry. Two readings: the human's coherent intuition-to-review experience, and the self-compounding coordination harness. **Zero misfits; zero automatic extractions.** Forest/restaurant wording stays intact and neutral rather than classified as unrelated leftovers.

Application-resolved supports in `01-first-anchors.json`:
- Reading 1: source 1, bytes **514..723**, the full intuition-to-PR/health/next-change sentence.
- Reading 2: source 1, bytes **38..214**, including “a system that compounds itself” and the human-first board/lifecycle wording, with its typo preserved.

Both anchors exactly match the approved source bytes. `01-first-guess.json` matches the raw accepted reply. Exact Markdown is retained in `*-paper.json`; `.md` only trims presentation whitespace. Frames cover **80×24, 100×30, 160×40, 320×40**, including the one-action originals view. At 80×24 the long intact source scrolls; `[ / ]` explicitly switches evidence and reveals the selected reading's first support without recording an answer/request/confirmation.

**Semantic caveat:** the accepted question still asks for a primary success measure (“or both equally”), which may unnecessarily force emphasis between compatible aims. Alternatives are provisional, not verified feasibility. The metaphors were not individually cited by the accepted readings. This is not operator recognition, native-host/font/clipboard proof, or evidence that the board beats chat. Operator assessment remains necessary.

## Mechanical validation and retained intermediate failures

`final-validation.log`: formatting, all app targets (**52 library tests**), upstream Pinstar (**40 tests**), build, both PTY suites through 320×40, strict app/upstream Clippy and whitespace checks passed.

All eleven historical component tests remain, adapted from the superseded automatic-fragment contract to intact sources/deliberate extraction. Five new tests cover exact occurrences/graphemes, exact rendered source-cell styles, explicit linked extraction without provider side effects, bad-span recovery, and truthful originals/neutral unmarked text. The boundary test also rejects narrative misfits.

`removal-proofs.log` / `removal-proof.py`: passing baseline, then **ten independently removed protections** caught by their regression tests in an isolated copy: automatic card creation, wrong quote occurrence, unmatched quote, split annotation grapheme, split extraction grapheme, marking all words as supports, omitted unresolved styling, extraction calling the provider, title replaced by a clipping cue, and originals displayed before a source exists. The script does not modify the working tree.

Earlier artifacts are kept, not substituted:
- `earlier-pty-contract.log`: the old fragment-ID stub correctly failed the new anchor contract; updated fixture/oracle, not relaxed validation. The exact padded output is in `earlier-pty-contract.json`; the readable log trims presentation whitespace so the staged whitespace check remains strict.
- `earlier-unicode-oracle.log`: the new test initially called byte 3 in “Café” a UTF-8 cut; that is the valid end of “Caf”. Corrected the oracle to byte 4, which really cuts “é”.
- `earlier-removal-harness.log`: baseline passed; the probe finder then mismatched rustfmt's single-line `text.get` spelling. Fixed harness only.
- `earlier-validation-window.log`: regular command's 60-second window expired after the legacy PTY suite; the complete background run is `final-validation.log`.

Reproduce the dump-only actual run with the approved dump file and `TINKERY_LIVE_FIRST_ONLY=1`; select the real model/skill explicitly. Do not supply an answer file or use the example's default answer path for this case. Such calls may incur charges. No persistence, canonical confirmation, execution, handoff, voice, answer-choice UI or merge was added.
