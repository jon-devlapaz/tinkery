#!/usr/bin/env python3
"""Offline board-rejection census; no subprocesses, models, or ledger writes."""
import argparse
import collections
import json
from pathlib import Path
import re

FAILURE = re.compile(r"Reshape failed:\s*([^\n]+)")
SYNTHETIC = re.compile(r"pty|terminal|validation|removal|mutation|format-failure|tests\d", re.I)


def classify(reason):
    r = reason.lower()
    if "meaning check rejected" in r:
        return "F", "legacy-meaning-fidelity-veto", "model allegation; not a proven fact violation"
    if "lost authored" in r:
        return "S", "lexical-presence-veto", "literal output-term requirement, not source-reference integrity"
    if "precondition" in r or "unknown field" in r:
        return "S", "exact-object-keys", "valid JSON rejected for extra fields"
    if "omitted fragments" in r:
        return "S", "exhaustive-fragment-annotation", "forced support-or-misfit allocation"
    if "unsupported or overlong" in r:
        return "S", "reading-length-or-support-shape", "compound message; subrule needs raw-response evidence"
    if "expected one/two" in r or "two uncertain" in r:
        return "S", "reading-or-candidate-count-coupling", "compound count gate; subrule needs raw response"
    if any(s in r for s in ("anchor", "grapheme", "verbatim", "source reference", "source span", "source occurrence", "exact source substring", "application-owned", "invented authority", "settled decision")):
        return "F", "source-history-authority-integrity", "inspect cited evidence; classification is not proof of harm"
    if any(s in r for s in ("unreadable board response", "no readable interpretation", "no readable board object")):
        return "P", "unreadable-core-response", "invalid JSON or no readable core; optional omissions and extra fields are not parse failures"
    if "invalid brain-dump json" in r or "invalid board json" in r:
        return "P", "opaque-json-decoder-error", "provisional: decoder mixes syntax and schema; inspect raw response"
    return None, "unclassified", "requires human classification"


def normalize(reason):
    return reason.strip().split(" Original and previous board retained")[0].rstrip(". ")


def collect(root):
    root = root.resolve()
    events = []
    excluded = []
    replays = []
    advisory = []
    notices = []
    scanned = 0
    for p in sorted(root.rglob("*")):
        if not p.is_file() or p.suffix not in (".json", ".jsonl", ".log"):
            continue
        scanned += 1
        rel = p.relative_to(root).as_posix()
        if p.suffix == ".log":
            matches = list(FAILURE.finditer(p.read_text(errors="replace")))
            for match in matches:
                ref = f"{rel}:{p.read_text(errors='replace')[:match.start()].count(chr(10)) + 1}"
                if SYNTHETIC.search(p.name):
                    excluded.append({"artifact": ref, "reason": match[1], "scope": "synthetic/test failure, not live rejection"})
                else:
                    events.append({"artifact": ref, "reason": normalize(match[1]), "kind": "live-log", "aliases": []})
            continue
        try:
            values = [json.loads(line) for line in p.read_text().splitlines() if line.strip()] if p.suffix == ".jsonl" else [json.loads(p.read_text())]
        except (ValueError, UnicodeError) as e:
            excluded.append({"artifact": rel, "scope": "artifact decoding failure; not evidence of provider rejection", "reason": str(e)})
            continue
        def walk(value, pointer="$"):
            if isinstance(value, dict):
                if "notice" in value and isinstance(value["notice"], str):
                    match = FAILURE.search(value["notice"])
                    if match:
                        notices.append({"artifact": f"{rel}:{pointer}.notice", "reason": normalize(match[1]), "kind": "state-notice", "aliases": []})
                if value.get("decision") in ("reject", "flag", "error"):
                    item = {"artifact": f"{rel}:{pointer}", "reason": value.get("reason", ""), "kind": "check-decision", "aliases": []}
                    if "schema_replay" in pointer or "boundary_replay" in pointer:
                        replays.append(item)
                    elif value.get("mode") == "log-only" or value.get("decision") == "flag":
                        advisory.append(item)
                    elif value.get("decision") == "reject":
                        events.append(item)
                if "output" in value and isinstance(value["output"], str):
                    for match in FAILURE.finditer(value["output"]):
                        excluded.append({"artifact": f"{rel}:{pointer}.output", "reason": match[1], "scope": "embedded test/PTY transcript; not independent live event"})
                for key, child in value.items():
                    if isinstance(child, (dict, list)):
                        walk(child, pointer + "." + key)
            elif isinstance(value, list):
                for i, child in enumerate(value):
                    walk(child, pointer + f"[{i}]")
        for value in values:
            walk(value)
    # Pair structured state/check notices with their live log, never count them again.
    for notice in notices:
        chapter = notice["artifact"].split("/")[0]
        peers = [e for e in events if e["artifact"].split("/")[0] == chapter and normalize(e["reason"]) == notice["reason"]]
        if len(peers) == 1:
            peers[0]["aliases"].append(notice["artifact"])
        elif not peers:
            events.append(notice)
        else:
            excluded.append({**notice, "scope": "ambiguous duplicate notice; independent log events retained separately"})
    # An explicit decision is more precise than its enclosing notice.
    for event in events:
        event["class"], event["rule"], event["qualification"] = classify(event["reason"])
    return {"events": events, "excluded": excluded, "offline_replays": replays, "advisory_non_rejections": advisory, "scanned_files": scanned}


def refine(root, data):
    # Explicit provenance links for historical opaque/compound messages. Never infer
    # that a decoder error is syntax failure merely from the word 'invalid'.
    links = {
        "2026-10-08-intact-dump/first-live.log": ("2026-10-08-intact-dump/raw-provider/00-stdout.json", "misfit-item-type", "misfits are narrative strings, not anchor objects"),
        "2026-10-08-goal-handoff/medium-checkpoints.log": ("2026-10-08-goal-handoff/raw-medium/05-stdout.json", "reading-word-limit", "47-word combined reading exceeded the historical 45-word gate"),
    }
    for event in data["events"]:
        path = event["artifact"].split(":")[0]
        if path in links:
            raw_path, rule, reason = links[path]
            raw = json.loads((root / raw_path).read_text())
            if rule == "misfit-item-type":
                assert any(isinstance(m, str) for m in raw["misfits"])
            else:
                assert len(raw["framings"][0]["text"].split()) == 47
            event.update({"class": "S", "rule": rule, "qualification": reason, "raw_evidence": raw_path})
        if path == "2026-10-09-log-only/initial-eval/onboarding-emails/decisions.json":
            raw = json.loads(json.loads((root / path).read_text())["decisions"][0]["raw"])
            assert any("precondition" in c for c in raw["alternatives"])
            event.update({"class": "S", "rule": "exact-object-keys", "qualification": "raw is valid JSON; extra candidate precondition fields; offline replay is the SAME attempt, not another live failure"})
        if path == "2026-10-09-log-only/initial-eval/trap/decisions.json":
            raw = json.loads(json.loads((root / path).read_text())["decisions"][0]["raw"])
            assert raw["uncertain"] and len(raw["framings"]) == 1 and len(raw["alternatives"]) == 3
            event.update({"class": "S", "rule": "uncertainty-reading-count-coupling", "qualification": "one uncertain reading, three candidates; unchanged raw passes v2 offline"})
    return data


def summarize(data):
    counts = collections.Counter({"F": 0, "S": 0, "P": 0})
    counts.update(e["class"] or "unclassified" for e in data["events"])
    rules = collections.Counter((e["class"] or "unclassified", e["rule"]) for e in data["events"])
    data["counts"] = dict(sorted(counts.items()))
    data["counts_per_rule"] = [{"class": cls, "rule": rule, "count": n} for (cls, rule), n in sorted(rules.items())]
    data["scope"] = "Persisted live board rejections only; snapshots deduplicated, synthetic/validation logs and offline replays separated. F is a check's target, NOT proof that a fact was violated. Six legacy semantic vetoes are unverified model allegations, not strict deterministic F failures. Narrative reports and screenshots are not independent attempts."
    return data


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--evidence", type=Path, default=Path(__file__).resolve().parents[1] / "docs/evidence")
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()
    data = summarize(refine(args.evidence, collect(args.evidence)))
    if args.json:
        print(json.dumps(data, indent=2, ensure_ascii=False))
    else:
        print(data["scope"])
        print(f"Live rejection events: {len(data['events'])}; classes: {data['counts']}")
        for row in data["counts_per_rule"]:
            print(f"{row['class']} {row['count']:3} {row['rule']}")
        print(f"Offline replay rejects (not new live calls): {sum(e['reason'] != '' for e in data['offline_replays'])}; advisory flag/error records: {len(data['advisory_non_rejections'])}")
        for event in data["events"]:
            print(f"  {event['class']} {event['artifact']} — {event['qualification']}")
    return 1 if any(e["class"] is None for e in data["events"]) else 0


if __name__ == "__main__":
    raise SystemExit(main())
