# Think voice — initial build report

Historical borrowed-engine slice. The [ownership update](owned/README.md) supersedes its runtime dependencies, settings, messages and setup behavior; initial evidence remains intact.

Authority: the operator-approved voice brief and four decisions in the voice plan review (2026-10-10). Built from `main` at `945cb39` on `think-voice`; **PR only, no merge or installation**.

## Design as built

- `voice/helper.mjs` is a goal-blind adapter to installed Pi Voice settings, recorder and transcribe-cpp/model. Rust embeds it and starts `node --input-type=module --eval <source>`; no runtime helper file, downloads, settings writes, audio writes or transcript logging. One model load per think visit; recording starts only on `start`.
- `src/voice/{protocol,process,mod}.rs`: typed version-1 JSONL, monotonic take IDs, a fakeable `VoiceHost`, bounded transport and take lifecycle. Node discovery uses explicit `TINKERY_NODE` (invalid override never falls back), PATH, then a bounded login-shell query, cached per run. Installed Pi's entry point uses `/usr/bin/env node`; its resolved runtime major is passed to the helper and checked before imports. Here that is 26. Native-ABI load errors are also `engine-load`; no claim of independent addon build-major metadata. If Pi's default runtime cannot be found, an explicit executable is checked by its version and actual native loading.
- `brain_dump/voice_input.rs`: render-only muted partial **at the current cursor**, final inserted through ordinary `Note` sanitation **at the cursor on arrival**. No captured anchor, edit deltas or persistent preview state in the editor. Ghost text never enters originals, extraction, clipboard or a shaping request. Color and NO_COLOR cells are tested.
- F6 toggles; Ctrl+Alt+Z/Ω are consumed before undo/text handling. Plain Ctrl+Z still undoes a skip. Esc cancels the take first. Ordinary final never sends; explicit F2 during a take stops, waits up to 30 seconds, then submits once only after a nonempty, non-filler final fits. Errors/cancel/overflow/timeout invalidate pending send and late events. F10/Drop close stdin, stop/reap the helper, with a 200 ms grace before killing its owned process group. The existing unsaved guard stays intact; after choosing n, F6 can reload voice without starting a take.
- Voice is unavailable in review/receipt and cannot type `confirm` there. Active voice blocks readiness/review and defers arriving shaping results even with an empty draft. Original/history/goal-only Seed Me contracts are unchanged. Only interactive think attaches voice, not snapshots, workbench or scratchpad. No banner/waveform/animation.
- Limits: five-minute take, immutable half-second PCM chunks, eight queued feeds, 32 KiB protocol lines, 32 commands/64 events, 4096-byte editor insertion. Overflow is an explicit failure, not truncation. Streaming models produce partials; non-streaming models retain bounded in-memory PCM for final transcription.

## Failure paths

Every failure clears ghost/pending-send state, preserves typed text and displays one muted actionable line; native exceptions are not silently reclassified as permission denial.

| Kind | Handling |
| --- | --- |
| `no-node` | Executable/version/discovery failure; set `TINKERY_NODE` to Pi's Node. Invalid explicit overrides do not select another executable. |
| `no-pi-voice` | Missing install/settings or unsupported settings; run `/voice-settings` in Pi. |
| `no-model` | Missing configured local model; select it in Pi. No download/fallback model. |
| `engine-load` | Module/native/model/language load failure. Major mismatch uses exactly `Voice needs the Node version Pi uses (N). Set TINKERY_NODE to it.` |
| `mic-permission` | Minimal macOS JXA query before first take; denied/restricted/check failure gives privacy guidance. Undetermined may request access only at explicit start. Whole-take RMS below 0.00001 gives near-zero guidance, **not proof of denial** or an empty insertion. |
| `mic-unavailable` | No device or missing selected device/occurrence; connect/select a microphone. |
| `mic-busy` | Explicit recorder busy/already-initialized failure; close the other recording app and retry. Unknown native capture exceptions remain `capture`, with their message. |
| `protocol` / `capture` / `transcription` / `timeout` | Malformed/version/queue/resource/native failures remain explicit; timeout kills the helper. Stale take events do nothing. |
| Empty / exact normalized silence filler | Empty, `thank you`, `you`, `.` (case/terminal punctuation normalized) insert nothing: `Didn't catch that.` Other text is not semantically rewritten. This can discard a genuinely intended standalone filler; the operator remains the authority. |

## Verification

- Rust **1.99.0**: **162 Tinkery unit tests** (31 new voice/controller/process/editor tests), **41 Pinstar tests**, **31 integration tests**, **2 compile-fail documentation tests**; fmt, strict workspace/all-target Clippy, build. All passed.
- **20 `node --test` helper tests**, all fake-only/no native addons or microphone access, including isolated import-only module resolution, recorder lifecycle, permission-cancel races, immutable borrowed buffers, PCM backlog, EOF during load/listen/finalization, error kinds and version mismatch. CI uses existing runner Node: no setup-node/downloads; absence fails loudly.
- All seven Python test scripts passed, including five census tests and official Seed Me journeys in isolated TEST roots. Eight voice PTYs cover four sizes in color/NO_COLOR; final repeat deliberately provides **no Node runtime on PATH**, only the scripted Python transport. Terminal restoration, clipboard exclusion, exactly-once F2, review aliases, EOF and PID reaping are checked.
- **10 fresh-copy reversion probes**: pristine source recompiles/pass, protection removed/fails. Current cursor, F2, stale IDs, 30 s deadline, fillers, frozen review, actual muted cells, borrowed PCM, stdin EOF and Node major. The EOF mutant fails the test's own bounded 5 s deadline (Node reports cancellation, exit 1), not an unexplained harness timeout.
- Final helper SHA-256 `3ac5d42a87dfbb3ec9c3165f0202c8caeb90136c57f2bf36fb0834d7931a4791` ran through installed local Parakeet under Node 26.8.1: generated one-second 440 Hz WAV → ready/listening/two partials/final, empty recognition. **Recorder module was not imported.** The same generated fixture succeeded at the earlier slice boundary; final `generated-smoke.json` records exact helper provenance without logging transcript text. This verifies ASR glue, not speech recognition or microphone permissions.
- **0 Pi/provider calls, 0 real microphone recordings.** Hash checks confirm all **32 protected paths** (launcher, installed binary, Pi Voice settings, operator ledgers) unchanged. Pi Voice/Seed Me source was not modified; canonical Seed Me remains `1a2f31a136d53bceff51ae8c2e0e787c1a8a90bb`. C16 shaping prompts/model budgets were not touched.

Reproduce local checks with `cargo +1.99.0 test --workspace --locked`, `cargo +1.99.0 clippy --workspace --all-targets --locked -- -D warnings`, `node --test voice/helper.test.mjs`, and the Python loop from CI with the pinned official TEST skill. `generated-smoke.py` is **manual/local only** and generates its own temporary tone; never add it to CI or use a microphone. `reversion-probes.py` retains fresh-copy artifacts and requires the installed 1.99 toolchain.

## Retained failures / limits

- First smoke input failed **before native import**: `ENOENT: no such file or directory, open ''`. Empty fake-mode env wrongly hid the smoke WAV; fixed explicit nonempty selection and added a regression.
- Initial native start exposed a fake-facade mismatch: `invalid feature "streaming"; expected one of initial_prompt, temperature_fallback, long_form, cancellation, pnc, itn, diarization`. Correct API is `model.capabilities.supportsStreaming`, not `model.supports('streaming')`; fake facade now matches the declarations. ESM eval imports themselves worked; no file-writing fallback.
- First Unicode test compared physical wide-character buffer cells with an unpadded logical string; corrected to assert logical preview plus rendered presence. Initial strict Clippy caught `field_reassign_with_default` in test setup; initializer corrected. Original failed logs retained.
- First probe run reused a previous mutant through shared Cargo artifacts with preserved copy mtimes. Discarded those results; now refresh copied sources and require baseline recompilation. Next harness rejected Node's bounded EOF-test cancellation despite exit 1; accounting corrected and all ten rerun successfully. Both failed harness runs retained.
- Real microphone capture/permission/busy behavior and human recognition remain **operator-only, unverified**. Busy mapping uses the recorder's reported class/message, not a general diagnosis. Existing Pi Voice English settings were used; this adapter preserves raw recognized text rather than importing Pi Voice's optional Chinese script-conversion service. Fake cells/PTYs do not prove desktop rendering, clipboard permission or every terminal's modifier encoding. Long native guidance can clip at narrow widths. No installation or rollout performed.

See `validation/`, `probes/results.json`, `generated-smoke.json`, and `verification.json`. Stop with the unmerged PR for review.
