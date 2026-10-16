from pathlib import Path
import hashlib
import re
import unittest

ROOT = Path(__file__).resolve().parents[1]
SOURCE = {
    "What was inspected": "a112f48208c60ee65df11dba9928da2d2811797c16932e651d7d3cd39788da50",
    "Record owners and limits": "0c6f9277c7c1d9e74a6ff1ee3d4269c7b2bd4d628a7b9ecded6f41fa2acc4700",
    "Trace one synthetic job": "6b0883c017582e1d0c2ede8be44e41cb802ff1a7579d04641cc83de21de91e99",
    "Attention and cost": "2dcb0184eb635869774112ffc16e156fa5179b3c0345b1d341d8176529be45e2"
}
PROPOSAL = {
    "Goal-to-run identity prerequisite": "1ae773db465de0ab2e7e77f73b18d18841cde91c610e38a85f0cfdad698daec9",
    "Changes, conflicts and interruption": "a81c58ee42848744fe40f180dc77e4d13c2f51442200f25fb600e85b3713f720",
    "Proposed R5: one explicit read": "9176f2eafee686208966692e6e701384d21b2894a95271edf065a4762d79ba75",
    "Proposed R6: external approval handoff": "8bab68c0ab263c24ee4252ae48c51ce6ad63c924ffcd6da5df0da5e81c06a0ad",
    "Evidence and checks": "7da90f7549f4a2a172cc5612558e19f75914dda9d9720d2ccc7df13a578faaba"
}
CITATIONS = "76d64a8e045bc169c9db8d5cb57c9adc109a460ef893c51a302f767104d9d0d9"


def sections(source):
    body = re.split(r"^\[[^\]]+\]:", source, maxsplit=1, flags=re.M)[0]
    return {s.splitlines()[0][3:]: s.strip()
            for s in re.split(r"(?=^## )", body, flags=re.M) if s.startswith("## ")}


class JobRecordDocumentationTests(unittest.TestCase):
    def test_source_evidence_has_one_home_and_legacy_anchors(self):
        proposal = (ROOT / "docs/design/job-records.md").read_text()
        survey = (ROOT / "docs/research/shared/job-record-survey.md").read_text()
        self.assertIn("not fresh capability verification or implementation", survey)
        self.assertIn("(../../design/job-records.md)", survey)
        current, historical = sections(proposal), sections(survey)
        for title, digest in SOURCE.items():
            self.assertEqual(digest, hashlib.sha256(historical[title].encode()).hexdigest(), title)
            anchor = title.lower().replace(" ", "-")
            self.assertEqual(f"## {title}\n\nSee [historical source evidence]"
                             f"(../research/shared/job-record-survey.md#{anchor}).", current[title])

    def test_proposal_contract_and_permission_status_are_unchanged(self):
        source = (ROOT / "docs/design/job-records.md").read_text()
        current = sections(source)
        for title, digest in PROPOSAL.items():
            self.assertEqual(digest, hashlib.sha256(current[title].encode()).hexdigest(), title)
        self.assertIn("**Revised design proposal; not accepted or implemented.**", source)
        self.assertIn("both issues remain `needs-approval`", source)
        self.assertIn("not the binding writer or R5", source)
        self.assertIn("(../research/shared/job-record-survey.md)", source)

    def test_all_pinned_citations_survive_and_resolve_locally(self):
        combined = []
        for path in ("docs/design/job-records.md", "docs/research/shared/job-record-survey.md"):
            source = (ROOT / path).read_text()
            definitions = re.findall(r"^\[([^\]]+)\]: (.+)$", source, re.M)
            used = set(re.findall(r"\]\[([^\]]+)\]", source))
            self.assertEqual(used, {name for name, _ in definitions}, path)
            combined.extend(f"[{name}]: {url}" for name, url in definitions)
        digest = hashlib.sha256("\n".join(sorted(combined)).encode()).hexdigest()
        self.assertEqual(CITATIONS, digest)


if __name__ == "__main__":
    unittest.main()
