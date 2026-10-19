"""Named adversarial inputs supplement (not replace) the upstream harvested calls."""
from copy import deepcopy
import json
import random
import struct


def iso_fuzz():
    rng = random.Random(272)
    seeds = ["2026-10-19T09:15:00+01:00", "20261019T091500Z",
             "2026-W43-1T09:15:00.5-05:30", "2024-02-29T23:59:59,999999+23:59"]
    alphabet = "0123456789-:TWZ+.,  "
    samples = [''.join(rng.choices(alphabet, k=rng.randint(6, 32))) for _ in range(1500)]
    for _ in range(2500):
        value = rng.choice(seeds)
        for _ in range(rng.randint(1, 3)):
            position = rng.randrange(len(value))
            operation = rng.choice(("replace", "insert", "delete"))
            if operation == "delete":
                value = value[:position] + value[position + 1:]
            else:
                char = rng.choice(alphabet)
                value = value[:position] + char + value[position + (operation == "replace"):]
        samples.append(value)
    return samples


def cases(session, stamp):
    base = {"schema_version": 1, "session_id": "TEST-session", "version": 0, "revision": 0,
            "created_at": stamp, "updated_at": stamp, "status": "active",
            "draft": {"goal": "", "outcome": "", "options": []}, "goal": None, "origin": None,
            "current_question": None, "frontier": [], "nodes": [], "assumed": [], "operator": "human"}
    draft = session.editable(base)

    def node(id, kind="decision", settled=False):
        n = {"id": id, "kind": kind, "status": "settled" if settled else "unresolved",
             "prerequisites": [], "evidence": []}
        if kind == "decision":
            n.update(owner="TEST owner", gate="TEST gate")
        if settled:
            n.update(answer="TEST goal", authority="user" if kind == "decision" else "evidence",
                     authority_source="TEST source")
            if kind == "fact":
                n["evidence"] = ["TEST observed"]
        return n

    confirmed = deepcopy(draft)
    confirmed.update(goal="TEST goal", origin="goal", nodes=[node("goal", settled=True)])
    saved = session.transition(base, confirmed, 0, "TEST confirm")
    with_fact = deepcopy(confirmed)
    with_fact["nodes"].append(node("f", "fact", True))
    fact_saved = session.transition(base, with_fact, 0, "TEST confirm")
    saved_edit = session.editable(saved)
    out = []

    def add(name, op="validate", **args):
        out.append({"name": "edge:" + name, "op": op, **deepcopy(args)})

    def validation(name, field, value, state=draft):
        s = deepcopy(state)
        s[field] = value
        add(name, state=s)

    def node_bad(name, field, value):
        s = deepcopy(confirmed)
        s["nodes"].append(node("d"))
        s["nodes"][-1][field] = value
        add(name, state=s)

    def load_bad(name, mutate, ledger=saved):
        l = deepcopy(ledger)
        mutate(l)
        add(name, "load", raw=json.dumps(l, ensure_ascii=False))

    def tx(name, state, ledger=saved, expected=1, reason="TEST update", revalidated=None):
        add(name, "transition", ledger=ledger, state=state, expected_version=expected,
            reason=reason, revalidated=revalidated)

    for name, field, value in [
        ("unknown-state-field", "extra", 1), ("operator", "operator", "robot"),
        ("status", "status", "bad"), ("draft-shape", "draft", {}),
        ("draft-text", "draft", {"goal": 1, "outcome": "", "options": []}),
        ("nodes-not-list", "nodes", {}), ("node-not-object", "nodes", [None]),
        ("invalid-goal", "origin", "absent"), ("completed-no-origin", "status", "completed"),
    ]:
        validation(name, field, value)
    for name, field, value in [
        ("id", "id", ""), ("kind", "kind", "bad"), ("node-status", "status", "bad"),
        ("parents", "prerequisites", None), ("duplicate-parent", "prerequisites", ["goal", "goal"]),
        ("predicate-not-bool", "predicate", 1), ("decision-gate", "gate", " "),
        ("active-answer", "answer", "not settled"), ("deferral", "status", "deferred"),
        ("settled-answer", "status", "settled"),
    ]:
        node_bad(name, field, value)
    validation("duplicate-id", "nodes", [node("goal", settled=True)] * 2, confirmed)
    s = deepcopy(confirmed)
    s["nodes"][0]["answer"] = "other"
    add("origin-explicit", state=s)
    s = deepcopy(confirmed)
    s["nodes"][0]["predicate"] = False
    add("settled-false", state=s)
    s = deepcopy(confirmed)
    s["nodes"].append(node("d"))
    s.update(status="stopped", current_question="d")
    add("ended-current-question", state=s)
    # The DFS must reach a cycle before a later missing parent.
    s = deepcopy(confirmed)
    s["nodes"] += [node("a"), node("b")]
    s["nodes"][1]["prerequisites"] = ["b", "missing"]
    s["nodes"][2]["prerequisites"] = ["a"]
    add("cycle-before-missing", state=s)
    # Each Python strip-only control must be blank; non-ASCII spaces too.
    for i in [*range(0x1c, 0x20), 0x85, 0xa0, 0x2007, 0x202f, 0x3000]:
        node_bad(f"python-whitespace-{i:x}", "id", chr(i))
    # Receipts exercise CPython's extended ISO grammar, including week-date ambiguity.
    times = ["20261019T091500Z", "2026-W43-1T09:15:00Z", "2026W431T091500+013015.5",
             "2026-W43T09Z", "2026W43T09+01", "2026-10-19🐍09:15:00,123456789-00:00:01.25",
             "2026-10-19T09.5+01.5", "2026-10-19T09:15.5+00:60", "2026-10-19T09:15:60Z",
             "2026-02-29T09Z", "2024-02-29T09Z", "2026-W53-1T09Z", "2020-W53-7T09Z",
             "9999-W52-7T09Z", "2026-10-19T09+24", "2026-10-19T09+23:59:59.999999",
             "2026-10-19T09+23:99", "2026-10-19T09+00:00:00.0000001", "2026-10-19T09+01:00:99",
             "2026-10-19T09+00:00:", "2026-10-19T09+0", "2026-10-19T09+00:00junk",
             "2026W431091500Z", "2026-W43-1091500Z", "2026-W43-09Z", "2026-W43-1Z",
             "2026-10-19T09", "2026-10-19", "0000-01-01T09Z", "2026-13-01T09Z",
             "2026-10-19T09:15:00.Z", "2026-10-19T09:15:00+01:00.5", "\x1c2026-10-19T09Z\x1f"]
    # C cursor boundaries distinguish a single consumed byte, fractional tails,
    # and the zone's stricter end check, including UTF-8 and embedded NUL.
    times += ["20261019T09150000Z", "20261019T09150099Z",
              "2026-10-19T09:15:00X+01:00", "2026-10-19T09:15:00🐍+01:00",
              "2026-10-19T09:15:00:123456+01:00", "20261019T091500.123456XYZ+01:00",
              "20261019T091500.12345X+01:00", "20261019T091500+01000000",
              "20261019T091500+0100000", "20261019T091500+01:00:00X",
              "20261019T091500+236000", "2026-10-19T09:15:00\x00+01:00",
              "20261019T091500+01:00:00.123456\x00junk", "20261019T091500+00:00:00.999999"]
    # Cross product catches component length and separator behavior, not just examples.
    for date in ["20261019", "2026-10-19", "2026W43", "2026W431", "2026-W43", "2026-W43-1"]:
        for clock in ["09", "0915", "09:15", "091500", "09:15:00", "09.1", "09:15,1234567",
                      "09:15:00.", "09:15.", "091500,"]:
            for zone in ["Z", "+01", "+0130", "+01:30:15.1234567", "-00:99", "+00:00:99", "+00:",
                         "+01:30:15."]:
                times.append(date + "T" + clock + zone)
    for i, time in enumerate(times):
        s = deepcopy(with_fact)
        s["nodes"][-1]["evidence"] = [{"checked": "TEST read", "at": time, "observed": "TEST seen"}]
        add(f"iso-{i}-{time!r}", state=s)
    regressions = ["20261019T09150Z", "20261019T0915400Z", "20261019T06191500Z",
                   "20261019T01500Z", "2026-10-19T09:15:080+0170"]
    for i, time in enumerate(regressions):
        s = deepcopy(with_fact)
        receipt = {"checked": "TEST read", "at": time, "observed": "TEST seen"}
        s["nodes"][-1]["evidence"] = [receipt]
        add(f"iso-c-regression-{i}-{time!r}", state=s)
        l = deepcopy(fact_saved)
        l["nodes"][-1]["evidence"] = [receipt]
        add(f"iso-c-stored-regression-{i}-{time!r}", "load", raw=json.dumps(l, ensure_ascii=False))
    for i, time in enumerate(iso_fuzz()):
        s = deepcopy(with_fact)
        s["nodes"][-1]["evidence"] = [{"checked": "TEST read", "at": time, "observed": "TEST seen"}]
        add(f"iso-fuzz-{i}-{time!r}", state=s)
    # Unicode IGNORECASE and Python word boundaries, including non-word combining marks
    # and alphanumeric numeric characters that Rust regex's default \\w does not share.
    secrets = ["ghp_" + "a" * 20, "sk-" + "x" * 20, "xoxb-" + "a" * 10,
               "AKIA" + "A" * 16, "password=abcdef", "Bearer " + "a" * 16,
               "-----BEGIN RSA PRIVATE KEY-----", "https://user:pass@host"]
    for i, raw in enumerate(secrets):
        for j, value in enumerate([raw, "²" + raw, "\u0301" + raw, raw + "²", raw + "\u0301",
                                  raw.replace("I", "İ").replace("i", "ı"), raw.replace("s", "ſ").replace("k", "K")]):
            s = deepcopy(with_fact)
            s["nodes"][-1]["evidence"] = [{"checked": value, "at": stamp, "observed": "TEST seen"}]
            add(f"secret-{i}-{j}", state=s)
    # This named order exception covers equally failing settlements:
    # Python's affected set visits them in hash-seed order.
    s = deepcopy(with_fact)
    for id in ("alpha", "beta", "gamma"):
        d = node(id, settled=True)
        d.update(prerequisites=["f"], evidence=["TEST revalidation evidence"])
        s["nodes"].append(d)
    l = session.transition(base, s, 0, "TEST graph")
    s = session.editable(l)
    s["nodes"][1]["answer"] = "TEST premise changed"
    for revalidated in (None, {"alpha": "TEST reviewed"}):
        tx("unordered-affected-" + ("all" if revalidated is None else "partial"), s, l, revalidated=revalidated)
        out[-1]["unordered_affected"] = True
    # Python int/float/bool equality affects both exact repeats and per-node history.
    for before, after in [(1, 1.0), (1.0, True), (True, 1), (0, False), (9007199254740993, 9007199254740992.0)]:
        l = deepcopy(fact_saved)
        l["nodes"][-1]["label"] = before
        l["nodes"][-1]["question"] = {"unordered": before, "other": []}
        s = session.editable(l)
        s["nodes"][-1]["label"] = after
        s["nodes"][-1]["question"] = {"other": [], "unordered": after}
        tx(f"python-equality-noop-{before!r}-{after!r}", s, l)
        s["draft"]["goal"] = "TEST force publication"
        tx(f"python-equality-history-{before!r}-{after!r}", s, l)
    rng = random.Random(272)
    numbers = [1e16, 1e23, 1e-4, 1e-5, -0.0, 5e-324, 1.7976931348623157e308,
               10**100, 9007199254740993, 1.0000000000000002,
               1187762618703838.25, -1804983053029684.25]
    numbers += [struct.unpack('!d', rng.randbytes(8))[0] for _ in range(2000)]
    for i, number in enumerate(numbers):
        if isinstance(number, float) and not __import__('math').isfinite(number):
            continue
        s = deepcopy(with_fact)
        s["nodes"][-1]["label"] = number
        tx(f"number-{i}", s, base, 0)
    l = deepcopy(fact_saved)
    l["nodes"][-1]["question"] = float("nan")
    s = session.editable(l)
    tx("nan-container-identity-noop", s, l)
    s["draft"]["goal"] = "TEST force publication"
    tx("nan-container-identity-history", s, l)
    for value in [True, 1.0, -1, None, "1"]:
        tx(f"bad-expected-version-{value!r}", saved_edit, expected=value)
    changed = deepcopy(saved_edit)
    changed["draft"]["goal"] = "TEST changed"
    tx("blank-reason", changed, reason="\x1c")
    changed = deepcopy(saved_edit)
    changed["nodes"][0].update(kind="fact", authority="evidence", evidence=["TEST seen"])
    # Origin validation would refuse a fact root first. Change a non-origin fact instead.
    changed = session.editable(fact_saved)
    changed["nodes"][-1].update(kind="decision", authority="user", owner="TEST", gate="TEST")
    tx("immutable-kind", changed, fact_saved)
    tx("invalid-revalidation-shape", changed, fact_saved, revalidated=[])
    changed = deepcopy(saved_edit)
    changed["draft"]["goal"] = "TEST changed"
    tx("invalid-revalidation-shape", changed, revalidated=[])
    for name, mutate in [
        ("schema", lambda l: l.update(schema_version=True)),
        ("stored-fields", lambda l: l.update(extra=1)),
        ("identity", lambda l: l.update(session_id="\x1c")),
        ("version", lambda l: l.update(version=True)),
        ("revision", lambda l: l.update(revision=-1)),
        ("history", lambda l: l["nodes"][0].pop("history")),
        ("node-revision", lambda l: l["nodes"][0].update(revision=1)),
        ("premise", lambda l: l["nodes"][0].update(premise_version=4)),
        ("reviewed", lambda l: l["nodes"][0].update(reviewed_version=True)),
        ("history-entry", lambda l: l["nodes"][0].update(history=[{}])),
        ("historical-node", lambda l: l["nodes"][0].update(history=[{"state": {}, "superseded_at": stamp, "reason": "TEST"}])),
        ("frontier", lambda l: l.update(frontier=["goal"])),
    ]:
        load_bad(name, mutate)
    raw = json.dumps(saved, ensure_ascii=False)
    add("duplicate-key-last-wins-first-position", "load", raw=raw.replace('"version": 1', '"version": 99, "version": 1'))
    huge = deepcopy(base)
    huge["version"] = 10**4300 - 1
    s = session.editable(huge)
    s["draft"]["goal"] = "TEST changed"
    tx("counter-exceeds-serialization-digit-limit", s, huge, huge["version"])
    for size in (4300, 4301):
        add(f"large-integer-{size}", "load", raw=raw.replace('"owner": "TEST owner"', '"owner": "TEST owner", "label": ' + '9' * size))
    add("unpaired-surrogate", "load", raw=raw.replace('"TEST owner"', '"\\ud800"'))
    for number in [float("nan"), float("inf"), -float("inf")]:
        load_bad(f"nonfinite-unchecked-{number}", lambda l, n=number: l["nodes"][0].update(label=n))
    # Serialization must name the first non-finite value in insertion/array order, not sorted keys.
    for first, later in [(float("nan"), float("inf")), (float("inf"), -float("inf")),
                         (-float("inf"), float("nan"))]:
        load_bad(f"nonfinite-traversal-{first}", lambda l, a=first, b=later:
                 l["nodes"][0].update(label={"z": ["TEST finite", {"nested": a}], "a": b}))
    # These are upstream crashes, not ordinary ValueError refusals; Rust must refuse.
    for name, value in [("null", None), ("number", 7), ("true", True), ("false", False)]:
        load_bad("python-crash-nodes-" + name, lambda l, v=value: l.update(nodes=v))
    for name, value in [("empty-object", {}), ("object-members", {"a": 1}), ("empty-string", ""), ("string-members", "abc")]:
        load_bad("nodes-" + name, lambda l, v=value: l.update(nodes=v))
    load_bad("empty-object-preserves-operator-refusal-order", lambda l: l.update(nodes={}, operator="robot"))
    load_bad("empty-string-preserves-draft-refusal-order", lambda l: l.update(nodes="", draft={}))
    for name, text in [("truncated-object", '{"TEST": 1'), ("truncated-string", '"TEST'),
                       ("trailing-comma-object", '{"TEST": 1,}'), ("trailing-comma-array", '[1,]'),
                       ("bom", '\ufeff' + raw), ("unicode-truncated", '{"🐍": 1'),
                       ("multiline-truncated", '{\n  "TEST": 1\n'), ("extra-data", raw + '[]')]:
        add(name, "load", raw=text)
    add("invalid-utf8", "load", raw_hex="ff7b7d")
    load_bad("python-crash-missing-history-id", lambda l: (l["nodes"][0].pop("id"), l["nodes"][0].update(
        history=[{"state": {}, "superseded_at": stamp, "reason": "TEST"}])))
    # Direct transition callers also get refusals, never the helper's container/type crashes.
    for name, mutate in [
        ("python-crash-transition-missing-field", lambda l: l.pop("draft")),
        ("python-crash-transition-node-null", lambda l: l.update(nodes=[None])),
        ("python-crash-transition-version-string", lambda l: l.update(version="1")),
        ("python-crash-transition-history-null", lambda l: l["nodes"][0].update(history=None)),
    ]:
        l = deepcopy(saved)
        mutate(l)
        s = deepcopy(saved_edit)
        if name.endswith("history-null"):
            s["nodes"][0]["label"] = "TEST changed"
        tx(name, s, l)
    # No-op ordering: a repeat may precede stale-version, reason, or ended checks.
    tx("stale-noop-ignores-reason", saved_edit, expected=0, reason=None)
    ended = deepcopy(saved)
    ended["status"] = "stopped"
    tx("ended-noop", session.editable(ended), ended, reason=None)
    # Legacy helper counters are optional; their insertion order must follow the pin.
    legacy = deepcopy(fact_saved)
    for n in legacy["nodes"]:
        n.pop("premise_version", None)
        n.pop("reviewed_version", None)
    s = session.editable(legacy)
    s["draft"]["goal"] = "TEST legacy edit"
    tx("legacy-counters-absent", s, legacy)
    return out
