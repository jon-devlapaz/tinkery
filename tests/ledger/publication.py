"""Publish harvested transitions into paired TEST folders with the pinned writer and clock."""
import copy
import importlib.util
import json
from pathlib import Path
import sys

sys.dont_write_bytecode = True
from harvest import harvest, helper, STAMP
from edges import cases as edge_cases


def main():
    script = helper()
    harvested, _ = harvest()
    spec = importlib.util.spec_from_file_location("publication_oracle", script)
    session = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(session)
    session.now = lambda: STAMP
    cases = [c for c in harvested if c["op"] == "transition"]
    cases += [c for c in edge_cases(session, STAMP) if c["op"] == "transition"
              and any(c["name"].startswith("edge:" + name) for name in (
                  "stale-noop", "ended-noop", "legacy-counters", "bad-expected-version", "blank-reason"))]
    root = Path(sys.argv[1])
    for i, original in enumerate(cases):
        case = copy.deepcopy(original)
        # The old answering boundary sorted state recursively before the helper read it.
        real_path = (type(case["expected_version"]) is int and 0 <= case["expected_version"] < 2**64
                     and isinstance(case["reason"], str) and case["revalidated"] in (None, {}))
        if real_path:
            case["state"] = json.loads(json.dumps(case["state"], sort_keys=True))
        pair = root / str(i)
        for name in ("python", "rust"):
            directory = pair / name
            directory.mkdir(parents=True)
            session.atomic_write(directory / "ledger.json", case["ledger"])
        answer = {"case": case, "pair": str(pair), "real_path": real_path}
        try:
            answer["read"] = session.editable_payload(pair / "python")
        except (ValueError, OSError, KeyError, TypeError):
            pass  # Invalid stored ledgers exercise publish's load refusal instead.
        try:
            result = session.publish(pair / "python", case["state"], case["expected_version"],
                                     case["reason"], case["revalidated"])
            answer.update(verdict="accept", version=result["version"])
        except (ValueError, OSError, KeyError, TypeError) as error:
            answer.update(verdict="refuse", message=str(error))
        print(json.dumps(answer, ensure_ascii=True))


if __name__ == "__main__":
    main()
