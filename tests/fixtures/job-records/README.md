# Synthetic job records

Companion to the [historical proposal](../../../docs/research/shared/job-records-proposal.md). Every name, word,
approval, check and result here is test data. Never execute the fake PR URL or
treat `operator: human`, `authority: user` or `decision: approved` as human action.
The temporary helpers accepted those fields; that does not authenticate them.

`current.json` is a **fixture container, not an upstream format or proposed job
schema**. Its values use these source shapes at the design's pinned revisions:

| Key / file | Original source location or response | How this example was made |
| --- | --- | --- |
| `think` | Tinkery think record, schema 1 | Hand-authored from `record.rs::write`; simulated origin, synthetic words; not generated through the UI |
| `seed_ledger` | Seed Me `<session>/ledger.json`, schema 1 | Official `transition`, `confirm_seed`, `end` and `load` in a temporary TEST root; two separate settled decisions |
| `seed-contract.md` | Seed Me `<session>/seed-contract.md` and SDLC's byte-identical run copy | Minimal structural fixture, like upstream `tests/seed_fixture.py`; helper confirmed it. Intentionally not a full seed specification |
| `sdlc_run` | `runs/test-job/run.json`, schema 1 | Created in a temporary test checkout; copied-seed fields follow the stage writer's shape, without invoking the launcher |
| `sdlc_decision` | `runs/test-job/decisions/3-1791806400000000000-TEST.json` | Body from official `decide` with TEST reviewer/source/reason; representative valid filename supplies stage 3 |
| `sdlc_verification` | `runs/test-job/04-test/output/verification.json` | Official `verify` ran `python3 -c 'assert 1 + 1 == 2'` for configured and checklist checks; official status was `current` |
| `github_pr` | Selected fields from `gh pr view --json` | Hand-authored with synthetic checks and the temporary candidate HEAD; `ci` is absent because Substrate derives it |
| `work.md` | Substrate TOML-front-matter work record, schema 1 | Hand-authored using `sources.read_record`'s format |

Only the source path is replaced with `/TEST/...`. Candidate and evidence hashes
come from the disposable test checkout; the full checkout is not saved here.
These snapshots are not enough to re-establish a current verification on another
machine. A real reader must ask that run's helper for current state. The goal and
Seed Me ID agree in this example; no source record connects them to the work
record. No tools-package receipt or real GitHub outcome is claimed.

`cases.json` is a **scenario description with proposed display expectations**,
not a source response. Each case starts from the current example. Its
`source_change` describes the independent alteration; it is not an implemented
patch language. `expected_source_state` is an observation/classification label,
not a common upstream enum. `unreadable-ledger.json` is deliberately truncated.

| Case | Source fact or condition |
| --- | --- |
| current | The temporary run's helper reported current; synthetic PR remains open without review approval |
| missing | Remove verification while preserving approved gates: helper reports `not-run` |
| unreadable | Truncated ledger cannot parse; its authority is unknown |
| outdated | Change the tracked counter after verification: helper reports `stale` |
| failed | Replace receipt with the source-shaped failed record: helper reports `failed` |
| interrupted | A source-shaped running receipt with **no active writer**: helper reports `interrupted` |
| conflict | Replace PR head; Substrate warns that local HEAD and PR checks differ |

Generation exercised current/missing/outdated/failed/interrupted through the
pinned SDLC runtime, in a disposable TEST checkout with fake decisions. JSON
parsing covers unreadable; the pinned Substrate source tests cover head mismatch.
No provider, microphone, installed launcher or operator data was used. This is
fixture consistency evidence, not Tinkery end-to-end testing.

Validation at the pinned revisions: all 4 selected SDLC protocol tests passed
(read-only status, full-profile gate staleness, changed candidate/mark/log, and
failed/interrupted checks); all 3 selected Substrate tests passed (head mismatch,
nonpassing CI states, and conflicting change-ID/PR links). The saved-shape check
below and document link checks passed. Rust, Node and PTY tests were not run.

The aggregation dependency is retired. To check the saved JSON and direct links
without upstream checkouts or GitHub, run from the Tinkery root. This does not
rerun the historical upstream validation above:

```sh
python3 -B - <<'PY'
import hashlib, json, pathlib
root = pathlib.Path('tests/fixtures/job-records')
value = json.loads((root / 'current.json').read_text())
json.loads((root / 'cases.json').read_text())
ledger = value['seed_ledger']
assert value['think']['goal'] == ledger['goal']
assert value['think']['seed_me_session'] == ledger['session_id']
assert hashlib.sha256((root / 'seed-contract.md').read_bytes()).hexdigest() == value['sdlc_run']['seed_contract']['sha256']
assert value['sdlc_decision']['inputs'] == value['sdlc_verification']['inputs']
try:
    json.loads((root / 'unreadable-ledger.json').read_text())
except ValueError:
    pass
else:
    raise AssertionError('unreadable example parsed')
print('Saved JSON and direct links checked; no upstream validation or real outcome asserted.')
PY
```
