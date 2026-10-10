# Tinkery owns voice — PR #9 update

Authority: operator-approved `Tinkery voice ownership brief.md` (2026-10-10). Pulled `think-voice` first; retained Claude's `cacbcabc203b079a5985d8550c37014fe2c7b7fb` stop fix. No merge, installation of Tinkery, real microphone recording or provider calls.

## Ownership / first use

- `voice/setup.mjs` owns installation, package paths, optional config, model discovery and native module loading. It knows nothing about goals or the editor. Rust's separate `src/voice/setup.rs` owns consent/session state and platform labels; the take controller keeps its existing IDs, streaming/stop/finalization and cleanup contracts.
- Direct pins: **`transcribe-cpp@0.2.2`**, **`@picovoice/pvrecorder-node@1.2.9`**. Install into `~/.local/share/tinkery/voice/` with the resolved Node's `process.execPath` executing its adjacent npm CLI, never an unrelated PATH npm or a shell. `--ignore-scripts --no-audit --no-fund --save-exact`; own package lock retains the transitive resolution. Native imports are verified before the atomic completion marker records the Node major. A failed verification does not mark setup complete.
- The compiled helper remains embedded ESM `--eval`: setup source + helper source, no runtime helper file. Native imports are absolute within the owned tree; inherited `NODE_PATH`, `NODE_OPTIONS`, and `TRANSCRIBE_LIBRARY` are removed from the helper environment. No former-install path/message/read or fallback remains. The source test scans the complete voice-owned Rust/helper/setup code and voice-integration Rust string literals; it intentionally does not remove Tinkery's unrelated shaping-provider integration.
- The idle voice key is visible before installation. Pressing it shows one muted `Voice needs a one-time local install (about 20 MB). Install now? y / n` on macOS (90 MB estimate on Linux, 85 MB elsewhere). Only **y** starts background installation; the same row says `installing voice…`. Success makes voice ready without restarting **and without recording**. A subsequent explicit voice key starts a take.
- **n/Esc** declines that action for the whole application session, including leaving/reopening think. An install error says `Voice install failed: [short reason]. Press the voice key to retry.` A retry requires that key, not an automatic attempt; unchanged prior consent is not re-asked. Changed Node-major reinstall wording requires fresh y/n consent: `Voice used Node 26; found 28. Reinstall (~20 MB)? y / n`. Model download consent is separate. Setup has a ten-minute bound; native model loading and finalization retain 30-second bounds. F10/Drop stop the helper and its inherited installer process group.
- Defaults: system microphone, English. Optional `~/.config/tinkery/voice.json` accepts nonempty string fields, e.g. `{"microphone":"USB microphone","language":"en","model":"/absolute/model.gguf"}`. Device names select the first exact match. Unknown keys/types/unreadable JSON are errors, not silently coerced settings. No config is created or rewritten.
- `TINKERY_VOICE_MODEL` overrides config/cache; an invalid explicit override never falls back. Otherwise choose the newest shared Hugging Face snapshot directory's Parakeet Q8_0 GGUF. If absent, ask `Voice model is missing (732 MB). Download now? y / n`. Only y can download. Downloaded models live in the **owned** `voice/models/` directory, never overwrite the shared cache, and are used when the shared cache is absent. Streaming download is size-bounded and atomically renamed; regular failures remove the partial. A killed process may leave its bounded `.part` file, reused on the next explicit attempt. `TINKERY_VOICE_ROOT` is an explicit root override used by isolated tests/manual temporary smoke, not a legacy fallback.
- macOS advertises **⌃⌥Z speak / ⌃⌥Z stop**; other platforms **6 speak / 6 stop**. Both aliases still work everywhere. Every key lists `F6 / Ctrl+Alt+Z (⌃⌥Z)` on one line. Existing current-cursor preview/final, ordinary insertion, explicit F2, frozen-confirmation exclusion and stop-rejected-read handling are preserved.

Plain microphone messages are exactly:

- `Allow microphone access for your terminal in System Settings → Privacy & Security → Microphone.`
- `Microphone is in use by another app.`
- `No microphone found.`
- `Didn't catch that.`

Unknown native exceptions remain explicit capture/load failures, not guessed permission/busy diagnoses. Long guidance can still clip in narrow terminals; no banner, setup screen, wizard, spinner or automatic recording was added.

## What the packages actually needed

Registry metadata and package source were inspected, not inferred from a successful npm exit:

| Installed package (macOS arm64) | Bytes | Native delivery / scripts |
| --- | ---: | --- |
| transcribe-cpp 0.2.2 | 158,531 | JS binding; exact-version optional `@transcribe-cpp/<platform>` registry bundle. Its loader checks the native header/version contract. No install script. `build:native` is an explicit source-build command, not required here. |
| @transcribe-cpp/darwin-arm64-metal 0.2.2 | 3,936,511 | Prepacked Metal/CPU dylib and contract; no install script or extra binary download. |
| koffi 3.3.3 | 1,733,911 | Transitive `^3.0.2` resolves to this version in the retained lock. Declares a cnoke install script, **skipped**. |
| @koromix/koffi-darwin-arm64 3.3.3 | 1,227,121 | Its prepacked platform addon makes the skipped koffi script unnecessary on this host. |
| @picovoice/pvrecorder-node 1.2.9 | 8,998,478 | Registry tarball already contains `lib/<platform>/<arch>/pv_recorder.node`; `prepare: node copy.js`/`prepack` are publishing/build scripts, not needed for this registry installation. Static addon import works without constructing a recorder. |

**Clean `--ignore-scripts` installation worked**, including native import verification. No compiler, install lifecycle script, source build or secondary binary fetch was needed on this macOS arm64 host. npm's optional packages are filtered by platform; a zero npm exit alone is not accepted as native verification. npm docs were fetched through Context7 (`/npm/cli`) and pinned package metadata/source plus the real install are the decisive evidence.

Final production-installer temporary run: **16,073,538 bytes (16.1 MB / 15.3 MiB), 1.980 seconds**, Node **26.8.1**, npm **12.0.2**, with the already-populated local npm cache. This is not a cold-download time or a promise for other machines. The earlier independent command-line install took 0.940 s / 16,077,224 bytes; initial owned-installer smoke before native-verification consolidation took 1.576 s. These are distinct retained measurements, not averaged.

The existing shared model is **731,357,568 bytes**, separate from engine size. No model was downloaded or modified. Production estimates for other platforms reflect registry native bundle sizes (Linux x64 bundle 71,857,647 bytes; Windows x64 63,425,150 bytes plus binding/recorder/FFI); real non-macOS native installs remain unverified.

## Verification / retained failures

- Rust **1.99.0**: **171 Tinkery unit + 41 Pinstar + 31 integration + 2 compile-fail doc tests = 245**, including declined/session consent and changed-major reauthorization; fmt, strict workspace/all-target Clippy and build.
- **36 fake-only Node tests**: success/failure/native-verification failure, command arguments, cached/newest/missing/overridden models, Node-major mismatch, exact messages, absolute owned imports, no former dependency strings, EOF, stop rejection and existing PCM/take guards. No network or native addons in CI.
- All seven Python scripts, including official Seed Me journeys in isolated TEST roots and **eight voice PTYs** across four sizes × color/NO_COLOR, with only the scripted Python “node” on PATH (no actual Node runtime).
- **15 fresh-copy reversion probes**, every baseline recompiles/passes and each removed protection fails: existing ten adapted for owned major checking, plus stop-rejected-read, setup consent, platform label, newest model and override. Final probes use a separate build directory, not the application's artifacts. `probes/results.json` is current-source evidence.
- Final embedded-source SHA-256 **`274ac074bd5040bc16379283dec6fc8603fac5ac32a8a9c47289dba1d474eb1c`** through the real temporary pinned install and cached model: generated one-second tone → loading/ready/listening/two partials/final, empty recognition. Installation imports both native packages to verify them; no recorder is constructed, enumerated or started. The helper itself skips recorder import for the generated fixture. An earlier generated-tone smoke also passed before native-verification consolidation; both results are retained. No speech recorded and no claim of recognition quality.
- **0 provider calls, 0 real microphone recordings. All 77 protected files unchanged**: launcher, installed Tinkery binary, settings/operator ledgers and the installed former voice package's files. The permanent owned-engine directory and optional config remain absent; only temporary installs were made. Existing goal shaping, confirmation, Seed Me and operator sessions were not changed.

Retained initial failures: the no-former-dependency-string test caught an obsolete `no-pi-voice` fixture; PTYs first expected the old help wording, then assumed a visible key meant the helper had loaded (now intentionally false), then failed their allowed-fake-file census after a readiness marker was added. These fixture migrations are retained, not hidden by weakening behavior checks. The final check initially found one test formatting diff; corrected before validation. Native-verification and transport retry gaps were caught during review and have regression coverage. Final suites passed before publishing.

Reproduce fake checks with `cargo +1.99.0 test --workspace --locked`, strict Clippy/fmt/build, `node --test voice/*.test.mjs`, and the CI Python loop/pinned official TEST skill. The manual `generated-smoke.py` here performs a **real temporary network-capable npm install** and generated-WAV-only native verification; never run it in CI or with a microphone.

Differences/limits: downloads use owned storage rather than hand-writing the shared Hugging Face cache; advanced config microphone is a name string/first exact match; size/time are host/cache-specific. Real microphone use with the new owned install, cold-network installation, actual model download, other OS native delivery, desktop rendering and clipboard permissions remain unverified. No merge/rollout performed. Stop at the unmerged PR for operator review.
