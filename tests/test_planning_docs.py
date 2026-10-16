from pathlib import Path
import hashlib
import re
import unittest

ROOT = Path(__file__).resolve().parents[1]
RELEASE = {
    "## Starting point": "615e9097e5b1b6760d1981683a377151943242e1a110295a5c6d7e6e96649d1c",
    "## Finish line": "de6423ca3f9415290ffb59f3139fb0663407885bbdd814b38630eab07d07a040",
    "## Milestones and dependencies": "65181baf4aec0af7f08a9f35067750e15b059aaff7fc1fa97ada6f7398152ec2",
    "## How to measure the benefit": "be2dd6a3e8aa7f9730efaeec4cb270680a4dd3ec417c209232fd77468e94fe5c"
}
PROPOSALS = {
    "### Context menu and send aliases (C4–C6, C9)": "199481341352a47adbf97c63bc8854e69f525fdc3454832acd31f63829cc0bd8",
    "### Evidence on demand (C13)": "519068b07737d26f852c68dc33ef466209f4b4f0d6133868e6064fc4d54f64e9",
    "### Multiple-choice answers": "c202b31904276cacb4481d046aa9c9fd2027719d44438479dab41e6ca261c7d0",
    "### Scratch-space spike — design only": "d292219249213ef242741b6756cda4dc3f69dd180879919ffc3929babbd86780",
    "### A calm dark theme": "95dc883b2cf66fc39c806d107cde8abfe56bf054e87cf16bbb3a4b673b2fee49",
    "### Absorb substrate: keep runs and learn from them": "65e57e40b33d9389af884bc7006dbfc8dc9aabee586573469beeaa7f03a65788"
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
