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
CITATIONS = "d527a72f7f9ac0e000832a86a9ffc05b983d7c72c9f954a5791fbb69e043678d"


def sections(source):
    body = re.split(r"^\[[^\]]+\]:", source, maxsplit=1, flags=re.M)[0]
    return {s.splitlines()[0][3:]: s.strip()
            for s in re.split(r"(?=^## )", body, flags=re.M) if s.startswith("## ")}


class JobRecordDocumentationTests(unittest.TestCase):
    def test_source_evidence_has_one_home_and_legacy_anchors(self):
        proposal = (ROOT / "docs/research/shared/job-records-proposal.md").read_text()
        survey = (ROOT / "docs/research/shared/job-record-survey.md").read_text()
        self.assertIn("not fresh capability verification or implementation", survey)
        self.assertIn("(job-records-proposal.md)", survey)
        current, historical = sections(proposal), sections(survey)
        for title, digest in SOURCE.items():
            self.assertEqual(digest, hashlib.sha256(historical[title].encode()).hexdigest(), title)
            anchor = title.lower().replace(" ", "-")
            self.assertEqual(f"## {title}\n\nSee [historical source evidence]"
                             f"(job-record-survey.md#{anchor}).", current[title])

    def test_retired_proposal_is_clearly_historical(self):
        source = (ROOT / "docs/research/shared/job-records-proposal.md").read_text()
        self.assertIn("superseded proposal is retained for its dated design history", source)
        self.assertIn("guidance or implementation permission.", source)
        self.assertIn("https://github.com/jon-devlapaz/tinkery/issues/258", source)

    def test_all_pinned_citations_survive_and_resolve_locally(self):
        combined = []
        for path in ("docs/research/shared/job-records-proposal.md",
                     "docs/research/shared/job-record-survey.md"):
            source = (ROOT / path).read_text()
            definitions = re.findall(r"^\[([^\]]+)\]: (.+)$", source, re.M)
            used = set(re.findall(r"\]\[([^\]]+)\]", source))
            self.assertEqual(used, {name for name, _ in definitions}, path)
            combined.extend(f"[{name}]: {url}" for name, url in definitions)
        digest = hashlib.sha256("\n".join(sorted(combined)).encode()).hexdigest()
        self.assertEqual(CITATIONS, digest)


if __name__ == "__main__":
    unittest.main()
