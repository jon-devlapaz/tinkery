"""Harvest live ledger-rule calls from the pinned suites without altering that checkout."""
import copy
import importlib
import json
import os
from pathlib import Path
import re
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.dont_write_bytecode = True
PIN = "47b6e13c545d5016f62eb16ea5ff75689ec13ae6435b99894069c72cde5a5451"
STAMP = "2026-10-19T00:00:00+00:00"


def helper():
    import hashlib
    skill = os.environ.get("SEED_ME_TEST_SKILL")
    if not skill:
        raise RuntimeError("SEED_ME_TEST_SKILL is required (including outside CI)")
    script = Path(skill).parent / "scripts/session.py"
    if hashlib.sha256(script.read_bytes()).hexdigest() != PIN:
        raise RuntimeError("session.py is not the c9d7667 pin")
    return script


def harvest():
    script = helper()
    sys.path.insert(0, str(script.parents[3] / "tests"))
    modules = [importlib.import_module("test_seed_me_" + name)
               for name in ("session", "epistemics", "delegated_origin")]
    session = modules[0].session
    cases = []
    original_transition, original_load, original_validate = session.transition, session.load, session.validate
    current_test = ["upstream"]
    nested = 0

    def record(op, **args):
        cases.append({"name": current_test[0] + ":" + op, "op": op, **copy.deepcopy(args)})

    def transition(ledger, state, expected_version, reason, revalidated=None):
        nonlocal nested
        record("transition", ledger=ledger, state=state, expected_version=expected_version,
               reason=reason, revalidated=revalidated)
        nested += 1
        try:
            return original_transition(ledger, state, expected_version, reason, revalidated)
        finally:
            nested -= 1

    def load(directory):
        nonlocal nested
        record("load", ledger=json.loads((Path(directory) / "ledger.json").read_text()))
        nested += 1
        try:
            return original_load(directory)
        finally:
            nested -= 1

    def validate(state):
        if not nested:
            record("validate", state=state)
        return original_validate(state)

    class Result(unittest.TextTestResult):
        def startTest(self, test):
            current_test[0] = test.id()
            super().startTest(test)

    original_temp = tempfile.TemporaryDirectory

    def temp(*args, **kwargs):
        kwargs["prefix"] = "TEST-ledger-harvest-"
        return original_temp(*args, **kwargs)

    with patch.object(session, "transition", transition), patch.object(session, "load", load), \
         patch.object(session, "validate", validate), patch.object(session, "now", lambda: STAMP), \
         patch.object(tempfile, "TemporaryDirectory", temp):
        suite = unittest.TestSuite(unittest.defaultTestLoader.loadTestsFromModule(m) for m in modules)
        result = unittest.TextTestRunner(resultclass=Result).run(suite)
    if not result.wasSuccessful():
        raise RuntimeError("upstream suites failed while harvesting")

    def normalize(v):
        if isinstance(v, str):
            v = re.sub(r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}", "TEST-session", v)
            return re.sub(r"(?:/private)?/(?:var/folders/[^\s\"']+?/T|tmp)/TEST-ledger-harvest-[a-z0-9_]+", "/TEST-fixture", v)
        if isinstance(v, list):
            return [normalize(x) for x in v]
        if isinstance(v, dict):
            return {k: (STAMP if k in {"created_at", "updated_at", "superseded_at"} and isinstance(x, str) else normalize(x))
                    for k, x in v.items()}
        return v

    unique = {}
    for case in cases:
        case = normalize(case)
        fingerprint = json.dumps({k: v for k, v in case.items() if k != "name"})
        unique.setdefault(fingerprint, case)
    return list(unique.values()), result.testsRun
