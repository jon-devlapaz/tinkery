"""Publish harvested transitions into paired TEST folders with the pinned writer and clock."""
import copy
import importlib.util
import json
from pathlib import Path
import sys
import subprocess

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
    edges = edge_cases(session, STAMP)
    cases += [c for c in edges if c["op"] == "transition"
              and any(c["name"].startswith("edge:" + name) for name in (
                  "stale-noop", "ended-noop", "legacy-counters", "bad-expected-version", "blank-reason"))]
    for edge in edges:
        if not edge["name"].startswith(("edge:nonfinite-unchecked-", "edge:nonfinite-traversal-")):
            continue
        ledger = json.loads(edge["raw"])
        state = session.editable(ledger)
        # A finite edit moves the unchecked old value into history, where saving must still refuse.
        state["nodes"][0]["label"] = "TEST finite replacement"
        cases.append(dict(name=edge["name"], ledger=ledger, state=state, expected_version=ledger["version"],
                          reason="TEST replace unchecked label", revalidated=None, nonfinite=True))
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
            if case.get("nonfinite"):
                path = directory / "ledger.json"
                path.write_text(json.dumps(case["ledger"], ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
                path.chmod(0o600)
            else:
                session.atomic_write(directory / "ledger.json", case["ledger"])
        answer = {"case": case, "pair": str(pair), "real_path": real_path}
        if case.get("nonfinite"):
            read = subprocess.run([sys.executable, str(script), "read", str(pair / "python"), "--editable"],
                                  capture_output=True, text=True, check=False)
            assert read.returncode != 0 and not read.stdout, read
            answer["read_message"] = read.stderr.splitlines()[0]
        else:
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
