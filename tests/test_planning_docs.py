from pathlib import Path
import hashlib
import re
import unittest

ROOT = Path(__file__).resolve().parents[1]
RELEASE = {
    "## Starting point": "019bbfbc2b2ccc0399342bd55d5b13fe3c71b7f58b1604866dff20b20f7f0cd3",
    "## Finish line": "567bc71d6c78c65e41a2f46011b48db1a74813ba5e0f4127ad8864c81a93a46f",
    "## Milestones and dependencies": "db47e232c84d3b61042d0c7e41437cbe84d8907acab0dc4e9d6279253a772b77",
    "## How to measure the benefit": "02169dfc426f72fb0e25b3a29209ddf5507a5db78f81ba2e532611fec7e19a54"
}
PROPOSALS = {
    "### Context menu and send aliases (C4–C6, C9)": "3cac3e0ea7fa2c754e23be7735446da9016b00b6a0ff0f4460fa13b0c0622bd0",
    "### Evidence on demand (C13)": "8a211c583f7d1691c250df21dd7b6619f7687dfdd4914880d8bd162829916547",
    "### A calm dark theme": "815b024ed6a9264b23cff0f39b798936b10aab5667520f3c8b1e660add88182b",
    "### Keep runs and learn from them": "384ff0d85ae36f44f540d1a9b1cd886f0f0b7af8268e104e05f9c20143fe72fa"
}


class PlanningDocumentationTests(unittest.TestCase):
    def test_release_keeps_boundaries_and_links_status_to_issues(self):
        source = (ROOT / "docs/design/first-release.md").read_text()
        for heading, digest in RELEASE.items():
            section = next(s for s in re.split(r"(?=^## )", source, flags=re.M)
                           if s.startswith(heading + "\n"))
            self.assertEqual(digest, hashlib.sha256(section.strip().encode()).hexdigest(), heading)
        self.assertIn("links the authoritative issues for scope, status, approval and completion evidence.", source)
        self.assertIn("R2's revised design is not accepted", source)
        self.assertIn("does not authorize the next", source)
        self.assertNotIn("R1, R2 and R3 are done", source)
        self.assertNotIn("published as unassigned", source)
        self.assertNotIn("needs-approval", source)

    def test_shared_safeguards_apply_without_changing_proposal_contracts(self):
        source = (ROOT / "docs/research/shared/backlog.md").read_text()
        safeguards = source.split("## Shared safeguards\n", 1)[1].split("\n## ", 1)[0]
        for guard in ("../../../AGENTS.md#guardrails", "explicit budget", "default is zero",
                      "Never record\nreal audio", "original words and answers verbatim",
                      "operator data", "sessions, installs, settings", "terminal, macOS and launcher preferences",
                      "do not approve any proposal"):
            self.assertIn(guard, safeguards)
        for heading, digest in PROPOSALS.items():
            section = next(s for s in re.split(r"(?=^### )", source, flags=re.M)
                           if s.startswith(heading + "\n"))
            contract = section.split("**Read:**")[0].strip()
            self.assertEqual(digest, hashlib.sha256(contract.encode()).hexdigest(), heading)
            self.assertIn("**Read:** [safeguards](#shared-safeguards)", section)
        for boundary in ("or make copying or sending confirm anything", "imply unused words are rejected",
                         "break NO_COLOR",
                         "Don't restore the retired aggregation dependency"):
            self.assertIn(boundary, source)


if __name__ == "__main__":
    unittest.main()
