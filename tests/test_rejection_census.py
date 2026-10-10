"""Offline census regressions; fixtures cannot launch providers or helpers."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location("census", Path(__file__).resolve().parents[1] / "scripts/rejection_census.py")
census = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(census)


class CensusTests(unittest.TestCase):
    def test_rules_distinguish_schema_from_syntax_and_facts(self):
        for reason, expected in [
            ("unknown field precondition", "S"),
            ("Invalid board: expected one/two readings", "S"),
            ("Invalid unsupported or overlong reading", "S"),
            ("Board omitted fragments", "S"),
            ("lost authored acronym AI", "S"),
            ("Source reference missing; quote not verbatim", "F"),
            ("Pi returned invalid brain-dump JSON", "P"),
            ("Unreadable board response: expected value", "P"),
            ("No readable interpretation in the response", "P"),
            ("Invalid application-owned question identity/history", "F"),
            ("Invalid claimed source span", "F"),
            ("Not an exact source substring", "F"),
        ]:
            self.assertEqual(census.classify(reason)[0], expected)
        self.assertIsNone(census.classify("unrecognized new rejection")[0])

    def test_legacy_semantic_veto_is_not_proof_of_a_fact_violation(self):
        cls, _, qualification = census.classify("Meaning check rejected lost concrete terms")
        self.assertEqual(cls, "F")
        self.assertIn("not a proven fact", qualification)

    def test_duplicate_notices_and_replays_do_not_inflate_live_events(self):
        with tempfile.TemporaryDirectory(prefix="tinkery-TEST-census-") as d:
            root = Path(d); chapter = root / "chapter"; chapter.mkdir()
            reason = "Invalid unsupported or overlong reading"
            (chapter / "live.log").write_text("Reshape failed: " + reason + " Original and previous board retained; F2 retries.\n")
            (chapter / "state.json").write_text(json.dumps({"notice": "Reshape failed: " + reason + " Original and previous board retained; F2 retries."}))
            (chapter / "replay.json").write_text(json.dumps({"boundary_replay_v3": {"decision": "reject", "reason": reason}, "advisory": {"decision": "flag", "mode": "log-only", "reason": "Possible loss"}}))
            (chapter / "earlier-pty.log").write_text("Reshape failed: " + reason)
            (chapter / "validation.log").write_text("test long_readings_are_rejected ... ok")
            result = census.summarize(census.collect(root))
            self.assertEqual(len(result["events"]), 1)
            self.assertEqual(len(result["events"][0]["aliases"]), 1)
            self.assertEqual(result["counts"], {"F": 0, "P": 0, "S": 1})
            self.assertEqual(len(result["offline_replays"]), 1)
            self.assertEqual(len(result["advisory_non_rejections"]), 1)
            self.assertEqual(len(result["excluded"]), 1)

    def test_independent_identical_log_failures_remain_independent(self):
        with tempfile.TemporaryDirectory(prefix="tinkery-TEST-census-") as d:
            root = Path(d)
            for name in ("first.log", "second.log"):
                (root / name).write_text("Reshape failed: lost authored acronym AI\n")
            self.assertEqual(len(census.collect(root)["events"]), 2)

    def test_real_corpus_refinement_does_not_treat_readable_json_as_parse_failure(self):
        root = Path(__file__).resolve().parents[1] / "docs/evidence"
        result = census.summarize(census.refine(root, census.collect(root)))
        self.assertFalse(any(e["class"] is None for e in result["events"]))
        latest = [e for e in result["events"] if e["artifact"].startswith("2026-10-09-log-only/initial-eval/")]
        self.assertEqual(len(latest), 2)
        self.assertTrue(all(e["class"] == "S" for e in latest))
        historical = [e for e in result["events"] if e["artifact"].startswith(("2026-10-08-", "2026-10-09-log-only/", "2026-10-09-subtraction/"))]
        self.assertFalse(any(e["class"] == "P" for e in historical))


if __name__ == "__main__":
    unittest.main()
