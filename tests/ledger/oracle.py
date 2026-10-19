"""Live pinned answers, with only named malformed-input and hash-order exceptions."""
import ast
import copy
import importlib.util
import inspect
import json
from pathlib import Path
import sys
import tempfile

sys.dont_write_bytecode = True
from harvest import harvest, helper, STAMP
from edges import cases as edge_cases

PORTS = {"load", "validate", "validate_receipt", "validate_claim", "validate_support", "validate_rationale",
         "transition", "visit"}
CRASHES = {
    "edge:python-crash-nodes-null": "TypeError",
    "edge:python-crash-nodes-number": "TypeError",
    "edge:python-crash-nodes-true": "TypeError",
    "edge:python-crash-nodes-false": "TypeError",
    "edge:python-crash-missing-history-id": "KeyError",
    "edge:python-crash-transition-missing-field": "KeyError",
    "edge:python-crash-transition-node-null": "AttributeError",
    "edge:python-crash-transition-version-string": "TypeError",
    "edge:python-crash-transition-history-null": "AttributeError",
}
# Operator-approved malformed-input boundaries. Each must still refuse in Rust.
# These are not misreported as Python validation refusals or as non-ValueError crashes.
DIVERGENCES = {
    "edge:unpaired-surrogate": ("UnicodeEncodeError", "Python loads lone surrogates but cannot write UTF-8; Rust refuses to decode them"),
    "edge:invalid-utf8": ("UnicodeDecodeError", "Both refuse; Rust names invalid UTF-8, Python also names its byte and offset"),
}
# The port follows 3.13's parser; cold Ubuntu may still run 3.12's older error wording.
if sys.version_info < (3, 13):
    for kind in ("object", "array"):
        DIVERGENCES["edge:trailing-comma-" + kind] = ("JSONDecodeError", "Pre-3.13 Python has different trailing-comma error wording")


def alternatives(session, case):
    # Reviewing each live reported target exposes the next legitimate exact message
    # without copying Python's affected-set rule into the oracle.
    prefix = "affected settlement requires explicit revalidation with evidence: "
    reviews = dict(case["revalidated"] or {})
    messages = []
    while True:
        try:
            session.transition(copy.deepcopy(case["ledger"]), copy.deepcopy(case["state"]),
                               case["expected_version"], case["reason"], reviews)
            return messages
        except ValueError as error:
            message = str(error)
            if not message.startswith(prefix) or message in messages:
                raise AssertionError("unexpected unordered refusal: " + message)
            messages.append(message)
            reviews[message[len(prefix):]] = "TEST reviewed to enumerate hash-order alternatives"


def main():
    script = helper()
    harvested, upstream_tests = harvest()
    if len(harvested) < 1974 or upstream_tests < 125:
        raise AssertionError(f"incomplete upstream harvest: {len(harvested)} calls, {upstream_tests} tests")
    spec = importlib.util.spec_from_file_location("ledger_oracle", script)
    session = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(session)
    session.now = lambda: STAMP
    edges = edge_cases(session, STAMP)
    cases = harvested + edges
    required = set()
    for function in ast.parse(script.read_text()).body:
        if isinstance(function, ast.FunctionDef) and function.name in PORTS:
            for call in ast.walk(function):
                if isinstance(call, ast.Call) and isinstance(call.func, ast.Name) and call.func.id == "require":
                    required.add(call.lineno)
    refused = set()
    original = session.require

    def require(condition, message):
        caller = inspect.currentframe().f_back
        if not condition and caller.f_code.co_name in PORTS:
            refused.add(caller.f_lineno)
        original(condition, message)

    session.require = require
    seen_crashes, seen_divergences = {}, {}
    with tempfile.TemporaryDirectory(prefix="TEST-ledger-oracle-") as root:
        path = Path(root) / "ledger.json"
        for case in cases:
            answer = {"case": case}
            try:
                op = case["op"]
                if op == "load":
                    raw = (bytes.fromhex(case["raw_hex"]) if "raw_hex" in case else
                           case.get("raw", json.dumps(case.get("ledger"), ensure_ascii=False)).encode("utf-8"))
                    path.write_bytes(raw)
                    ledger = session.load(root)
                    if path.read_bytes() != raw:
                        raise AssertionError("load rewrote its input")
                elif op == "validate":
                    answer["ready"] = session.validate(copy.deepcopy(case["state"]))
                    ledger = None
                elif op == "transition":
                    ledger = session.transition(copy.deepcopy(case["ledger"]), copy.deepcopy(case["state"]),
                                                case["expected_version"], case["reason"], case["revalidated"])
                else:
                    raise AssertionError(op)
                answer["verdict"] = "accept"
                if ledger is not None:
                    answer["flags"] = session.review_flags(ledger["nodes"])
                    answer["editable"] = session.editable(ledger)
                    try:
                        answer["bytes"] = json.dumps(ledger, ensure_ascii=False, indent=2, allow_nan=False) + "\n"
                        answer["bytes"].encode("utf-8")
                    except UnicodeEncodeError:
                        raise
                    except ValueError:
                        answer["serialization_refuses"] = True
            except ValueError as error:
                name = case["name"]
                if name in DIVERGENCES:
                    expected, explanation = DIVERGENCES[name]
                    if type(error).__name__ != expected:
                        raise AssertionError("malformed-input exception changed: " + name)
                    seen_divergences[name] = explanation
                    answer = {"case": case, "verdict": "divergence", "exception": expected, "explanation": explanation}
                else:
                    answer.update(verdict="refuse", message=str(error))
                    if case.get("unordered_affected"):
                        messages = alternatives(session, case)
                        if str(error) not in messages or len(messages) < 2:
                            raise AssertionError("unordered exception no longer applies")
                        answer["allowed_messages"] = messages
            except Exception as error:
                name = case["name"]
                if CRASHES.get(name) != type(error).__name__:
                    raise RuntimeError(f"unnamed Python crash in {name}: {type(error).__name__}: {error}") from error
                seen_crashes[name] = type(error).__name__
                answer.update(verdict="crash", exception=type(error).__name__)
            print(json.dumps(answer, ensure_ascii=True))
    if seen_crashes != CRASHES or set(seen_divergences) != set(DIVERGENCES):
        raise AssertionError(f"exception expectations changed: {seen_crashes}, {seen_divergences}")
    missing = required - refused
    if missing:
        lines = script.read_text().splitlines()
        raise AssertionError("uncased refusal sites: " + repr([(n, lines[n-1].strip()) for n in sorted(missing)]))
    print(json.dumps({"summary": {"cases": len(cases), "harvested": len(harvested), "edges": len(edges),
                                 "upstream_tests": upstream_tests, "refusal_sites": len(required),
                                 "crashes": seen_crashes, "divergences": seen_divergences}}))


if __name__ == "__main__":
    main()
