from pathlib import Path
import hashlib
import re
import unittest

ROOT = Path(__file__).resolve().parents[1]
RELEASE = {
    "## Starting point": "2b8c0d43c96e50f01242027a5d39418d294d76d05e1e519e52fa0ea3a1950708",
    "## Finish line": "eea5bd71b2be43a26b1502d7e18b9a3ef5c8bad19a13589137c4b1b074885d70",
    "## Milestones and dependencies": "5ec0b234038b981f1e93f70ff2f6fc5c3e16f18077213dd72cb66af76029c29b",
    "## How to measure the benefit": "cf87d09453c50f53449199b105b97e95b511ac3c5bd697b52d09ceb7ddb5bc1f"
}
PROPOSALS = {
    "### Context menu and send aliases (C4–C6, C9)": "c6e1492e229317cef4081de5dcdea416a58eb0dea80aaedb59f5a0c3481b4fd5",
    "### Evidence on demand (C13)": "3d69bd3b847475c34e28291d15bf66b3a353dbee256838b8b0341ce79103aeb6",
    "### Multiple-choice answers": "1e47e1d7372b78b06f3651bb479b6ffcf8d56321cabe2bf50c4d6827128c3552",
    "### Scratch-space spike — design only": "2b4f5bc10f2c02e613988a61c546a73cb741e36b0ea6846cba70d0c5cf2af584",
    "### A calm dark theme": "b283b75afcf28b3209d4dd442526bd65b4392361f4c74ef08e378d75988dff99",
    "### Absorb substrate: keep runs and learn from them": "1996982b57faf5a9e80918ea3b6418542019535f9278d2728065be6698de67db"
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
                      "operator data", "sessions, installs, settings", "terminal/macOS/launcher preferences",
                      "do not approve any proposal"):
            self.assertIn(guard, safeguards)
        for heading, digest in PROPOSALS.items():
            section = next(s for s in re.split(r"(?=^### )", source, flags=re.M)
                           if s.startswith(heading + "\n"))
            contract = section.split("**Read:**")[0].strip()
            self.assertEqual(digest, hashlib.sha256(contract.encode()).hexdigest(), heading)
            self.assertIn("**Read:** [safeguards](#shared-safeguards)", section)
        for boundary in ("or make copy/send confirm anything", "imply unused words are rejected",
                         "Force choices", "mix guesses with verbatim cards", "persist canvases",
                         "invent operator dumps", "break NO_COLOR", "Don't add features to substrate"):
            self.assertIn(boundary, source)


if __name__ == "__main__":
    unittest.main()
