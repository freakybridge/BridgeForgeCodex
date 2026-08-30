from __future__ import annotations

import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
DELIVERY_DIR = ROOT / "doc" / "1_delivery"
LIFECYCLES = {"active", "completed", "superseded", "archived"}
VALIDATION_STATUSES = {
    "not_started",
    "in_progress",
    "awaiting_validation",
    "awaiting_user_acceptance",
    "verified",
}


def frontmatter(path: Path) -> dict[str, str]:
    text = path.read_text(encoding="utf-8")
    if not text.startswith("---\n"):
        return {}
    end = text.find("\n---\n", 4)
    if end < 0:
        return {}
    fields: dict[str, str] = {}
    for line in text[4:end].splitlines():
        key, separator, value = line.partition(":")
        if separator:
            fields[key.strip()] = value.strip()
    return fields


class DocumentLifecycleContractTests(unittest.TestCase):
    def test_factory_requirements_are_fully_classified(self) -> None:
        cards = sorted(DELIVERY_DIR.rglob("requirements_*.md"))
        self.assertTrue(cards)

        for card in cards:
            with self.subTest(card=card.relative_to(ROOT)):
                fields = frontmatter(card)
                self.assertIn(fields.get("lifecycle"), LIFECYCLES)
                self.assertIn(
                    fields.get("validation_status"), VALIDATION_STATUSES
                )
                self.assertNotIn("status", fields)
                self.assertNotEqual(fields["lifecycle"], "archived")
                if fields["lifecycle"] == "completed":
                    self.assertEqual(fields["validation_status"], "verified")
                if fields["lifecycle"] == "superseded":
                    self.assertTrue(fields.get("superseded_by"))
                    target = (card.parent / fields["superseded_by"]).resolve()
                    self.assertTrue(target.is_relative_to(ROOT / "doc"))
                    self.assertTrue(target.is_file())


if __name__ == "__main__":
    unittest.main()
