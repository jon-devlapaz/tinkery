# Dogfood run 2 candidate / evidence and limits

## Promises assessed

- **Keep the requested result.** The final real bookmarks drafts retain both ranking by value using jev/Jev **and seeing the result in a spreadsheet**. CSV export remains an unaccepted candidate mechanism, not the outcome. Nothing implements the bookmarks idea.
- **Make uncertainty actionable.** Final drafts ask about value, Jev's role/capabilities, and bookmark inclusion boundaries. The graph draft asks what decisions/work and their connections mean. Your turn surfaces the first actual question, not just keep/cut/reshape.
- **Do not manufacture supplied facts.** The model's summary is now labelled **Model's reading / check this**, not asserted to be literal supplied context. Successful feedback is included **verbatim by the application**, independently of model paraphrasing; original notes remain unchanged. Assumptions are still model judgments, not machine-verified semantic classifications.
- **Keep selection and recover safely.** F2 and gesture cancellation retain note selection. Shift-click toggles notes; explicit Ctrl-A is a host-independent keyboard fallback. Failed/cancelled requests retain the previous paper and sources. No implicit all-note selection or canned fallback was added.
- **Read and copy the document, not the terminal frame.** Headings/labels/emphasis are styled. Paper drag selects logical lines, including wrapped rows; explicit y/button copy exports raw Markdown through OSC 52, without UI boxes or display wraps. The host's actual clipboard permission remains a manual check.

These are assistant assessments of particular evidence, **not operator recognition**, broad model reliability, Claude desktop E2E, or permission to advance the factory.

## Final real-model papers

`after/` is the final source's three-request run through the real `openai-codex/gpt-5.6-luna` model, **low** reasoning:

| Paper | Input / result |
| --- | --- |
| [02-plan.md](after/02-plan.md) | Two graph notes, selected using actual component Shift-click. Outcome preserves the requested graph view; implementation remains a candidate. Two definition/connection questions, no recorded assumptions. |
| [04-bookmarks.md](after/04-bookmarks.md) | Exact two dogfood notes. Outcome: “You can see your bookmarks in a spreadsheet and sort them by value using jev.” Three questions, no recorded assumptions. |
| [05-bookmarks-feedback.md](after/05-bookmarks-feedback.md) | Exact feedback: “Keep Jev; ranking by value is the point; the spreadsheet is just the place to see them.” Outcome: “You can see your Chrome bookmarks in a spreadsheet and rank them by value using Jev.” Feedback and original notes are preserved verbatim. |

Corresponding JSON files contain dimensioned, padded paper views. Bookmarks were exercised at **160×40**; the graph at **80×24**. Supported-size component and PTY checks are reported separately below. This real-component run uses a TestBackend, **not a real-model PTY/desktop journey**.

Remaining model limitations in the final run:

- The second bookmarks candidate still phrases the unchecked existing export too specifically as something leading to CSV. Its benefit is conditional, but feasibility and conversion are not checked. **Do not treat it as Chrome/Jev advice or an approved plan.**
- The initial bookmark scope question has a stray trailing quote. Strict JSON is valid, but prose polishing is not guaranteed.
- Empty assumptions are legitimate here because consequential missing facts became questions; they do **not** prove that no assumptions exist.

The operator clarified the semantic oracle during this work: explicitly requested results/viewing media belong in outcome; export/transport/implementation belong in candidates. The earlier blanket prohibition on interfaces/artifacts was wrong. That correction—not a hidden relaxation to pass a sample—explains the restored spreadsheet and final graph view.

## Preserved failures and intermediate judgments

No failed paper was overwritten with a later pass, parser tolerance, automatic repair, or canned draft.

| Directory | Why retained |
| --- | --- |
| `after-initial/` | Questions/context returned, but implementation/viewing-media distinctions still needed correction. |
| `after-incomplete/` | Graph completed, then bookmarks failed validation. Original failed stdout was not captured; its cause cannot be asserted. |
| `after-second/` | Another unsuccessful semantic iteration before the operator's clarification. |
| `after-format-failure/provider-stdout.txt` | Actual stdout was JSON followed by ` দেখা`; strict parsing rejected it. A regression explicitly rejects this suffix. |
| `after-over-tightened/` | Earlier low-reasoning run erased the requested spreadsheet from outcome. Kept as evidence of over-correction. |
| `after-pre-ui/` | Operator's media/result clarification applied; three valid real requests before the final gesture/formatting changes. |
| `after-ui-initial/` | Fresh UI/live rerun reclassified the stated spreadsheet use as an assumption. This is a real semantic failure, despite valid JSON and passing mechanical checks. |
| `after/` | One bounded final rerun after an assumption self-check, conditional-capability instruction, honest summary label, and application-owned verbatim feedback. No more sampling until a perfect-looking answer. |

Reasoning changed from `off` to configurable **low** after exploratory semantic/format failures. The model was not changed. This changes potential cost/latency and does not establish causation or eliminate variability. Normal application requests still run once, with explicit failure and no automatic retries. Raw-diagnostic capture used only a temporary forwarding wrapper around genuine Pi.

## Mechanical validation

Final-source checks:

```sh
cargo fmt --all --check
cargo test --locked --all-targets
cargo test --locked -p pinstar --no-default-features
cargo build --locked
python3 -B tests/terminal_smoke.py
cargo +1.95.0 clippy --locked --all-targets -- -D warnings
cargo +1.95.0 clippy --locked -p pinstar --all-targets --no-default-features -- -D warnings
git diff --check
```

The seven existing journey families remain: capture, selection, correction, deletion/recovery, reading, terminal hosts, and safe exit. Relevant evidence now includes:

- Differential buffer equality while opening/closing/scrolling paper at **80×24, 100×30, 160×40, 240×40, 320×40**.
- Running-binary POSIX PTY journeys with a **stub provider**, including Shift mouse decoding, F2 selection retention, feedback revision, clean decoded OSC 52 payload, terminal-mode restoration, no application persistence, Ctrl-L, and 320-column `--full-redraw`. An intentionally lost header cell is restored by Ctrl-L; removal of one-shot repaint fails that running-binary test in an isolated copy. This validates recovery, not reproduction of the desktop fault.
- Bold/italic modifiers and reverse-highlighted selection with exact raw-copy fidelity; oversize copy is rejected rather than truncated.
- Baseline-verified removal probes in an isolated copy reject loss of blank filtering, no-session flags, unaccepted labels, single provisional prefix, requested-medium/intent/question instructions, gesture retention, Shift-click, clean Markdown copy, assumption/candidate self-check instructions, verbatim feedback, and bold/italic styling. Instruction probes prove **delivery**, not semantic obedience.
- Font-corner projection initially also changed authored arrow characters. A new regression **failed first**, then passed after restricting substitution to the UI's accent/bold selection marker style, in both color and monochrome. Original note text and displayed authored arrows are preserved.

The first full-redraw PTY attempt failed. Redrawing was changed from terminal clearing/cursor queries to forced cell updates. The smoke test's accidentally literal `\\x0c` was also corrected to the actual Ctrl-L byte. The final supported/wide journeys pass; their VT reader has not been taught to silently accept arbitrary unknown escapes.

Transcripts: [final checks](validation.log), [strengthened repaint journeys](repaint-validation.log), [sixteen baseline-verified removals](mutations.log), and [rejected repaint removal](repaint-removal.json) (escaped transcript preserves screen padding).

These tests cannot diagnose the exact missing-ASCII corruption reported in Claude's desktop panel. Synchronized frames, **Ctrl-L**, and optional **--full-redraw** are mitigations awaiting that host's dogfood check, not a confirmed fix.

## Font / host boundary

See [terminal setup](../../terminal-setup.md) for Paper Mono installation, host configuration, typography proof, copying, and launch controls. No font was installed and no host settings were modified.

Inspected official Regular/Bold font files have uniform 606-unit advances for basic ASCII/UI box glyphs, but lack `⇘⇙⇖⇗`. Only Tinkery's UI selection markers receive supported box-corner substitution. Native italic files were not present in the inspected release; host synthesis must be visually checked. Font hashes:

- Regular: `130e1a09b64b4f150d34c93a6b7bd52d6c5c3513761d1fbc7be42d68398b8c10`
- Bold: `6f7f5849f5e115b538cf2ef40e33bca1bdb4fcf8664553cb4e801aa97332d26c`

Glyph metrics are not proof of desktop rendering. A documented Claude desktop terminal-specific font control could not be established. Optional external-terminal instructions are not a claim that Ghostty was installed or used.

## Reproduce the bounded real cases

Requires existing authorized Pi authentication, the selected model, and the actual skill. Makes three provider requests and can incur charges. Use a **new, nonexistent evidence directory**; the helper refuses overwrite.

```sh
TINKERY_LIVE_MODEL=openai-codex/gpt-5.6-luna \
TINKERY_SEED_ME=/path/to/tink-skills/skills/seed-me/SKILL.md \
TINKERY_LIVE_DOGFOOD=1 TINKERY_LIVE_SKIP_FEEDBACK=1 \
TINKERY_LIVE_THINKING=low \
cargo run --locked --example live_drafting_check -- /new/evidence/directory
```

`SKIP_FEEDBACK` skips the graph's optional feedback journey; the exact bookmarks correction still runs. Pi **1.1.0**, Seed Me **2.2.0**, skill repository revision `86066b175b53e13b4180ce4cc9929a626b57a2f6`, skill SHA-256 `8e8f4f07143b0bfa08c69134d7ff9e9dd009eafe28b32cb0c25f6d67e4792102`.

No seed confirmation, canonical seed/ledger persistence, plan approval, execution, or implicit downstream handoff was added. PR #1 remains review-only, not merged. Dogfood run 2 must still establish operator recognition and actual desktop typography/mouse/clipboard behavior.
