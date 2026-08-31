from __future__ import annotations

import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
DELIVERY_DIR = ROOT / "doc" / "1_delivery"
BUGS_DIR = ROOT / "doc" / "2_bugs"
LIFECYCLES = {"active", "completed", "superseded", "archived"}
VALIDATION_STATUSES = {
    "not_started",
    "in_progress",
    "awaiting_validation",
    "awaiting_user_acceptance",
    "verified",
}
BUG_ACCEPTANCE_RECEIPTS = (
    "用户已执行 `$summary 同意验收`",
    "用户明确执行 `$summary 同意验收`",
    "用户显式调用 `$summary 同意验收`",
)


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


def has_bug_acceptance_receipt(text: str) -> bool:
    return any(receipt in text for receipt in BUG_ACCEPTANCE_RECEIPTS)


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

    def test_factory_bug_records_are_classified_and_navigation_is_current(self) -> None:
        records = [(path, path) for path in sorted(BUGS_DIR.glob("BUG-*.md"))]
        records.extend(
            (package, package / "README.md")
            for package in sorted(BUGS_DIR.glob("BUG-*"))
            if package.is_dir() and (package / "README.md").is_file()
        )
        self.assertTrue(records)

        index = (ROOT / "doc" / "README.md").read_text(encoding="utf-8")
        current_section = index.split("## 当前 Bug records\n", 1)[1].split(
            "\n## ", 1
        )[0]

        for source, evidence in records:
            with self.subTest(record=source.relative_to(ROOT)):
                fields = frontmatter(evidence)
                self.assertIn(fields.get("lifecycle"), LIFECYCLES)
                self.assertIn(
                    fields.get("validation_status"), VALIDATION_STATUSES
                )
                self.assertNotIn("status", fields)
                self.assertNotEqual(fields["lifecycle"], "archived")
                if fields["lifecycle"] == "completed":
                    self.assertEqual(fields["validation_status"], "verified")
                    body = evidence.read_text(encoding="utf-8")
                    self.assertTrue(has_bug_acceptance_receipt(body))
                if fields["lifecycle"] == "superseded":
                    self.assertTrue(fields.get("superseded_by"))
                    target = (evidence.parent / fields["superseded_by"]).resolve()
                    self.assertTrue(target.is_relative_to(ROOT / "doc"))
                    self.assertTrue(target.is_file())

                if fields["lifecycle"] == "active":
                    self.assertIn(source.name, current_section)
                else:
                    self.assertNotIn(source.name, current_section)

    def test_waiting_for_user_acceptance_is_not_an_acceptance_receipt(self) -> None:
        self.assertFalse(
            has_bug_acceptance_receipt("修复已完成，等待用户验收；本 Bug 关闭。")
        )
        self.assertTrue(
            has_bug_acceptance_receipt("用户已执行 `$summary 同意验收`。")
        )


if __name__ == "__main__":
    unittest.main()
