from __future__ import annotations

import json
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SKILL = ROOT / "skills" / "develop" / "SKILL.md"
REFERENCES = ROOT / "skills" / "develop" / "references"


class DevelopSkillContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.skill = SKILL.read_text(encoding="utf-8")
        cls.references = {
            path.name: path.read_text(encoding="utf-8")
            for path in sorted(REFERENCES.glob("*.md"))
        }
        cls.ml_delivery = cls.references["ml-delivery.md"]
        cls.agent_execution = cls.references["agent-execution.md"]

    def test_entry_is_bounded_and_routes_each_mode_conditionally(self) -> None:
        self.assertLessEqual(len(self.skill.splitlines()), 85)
        self.assertIn("S 级直接路径", self.skill)
        self.assertIn("M 级精简路径", self.skill)
        self.assertIn("L 级完整路径", self.skill)
        self.assertIn("需要 Agent 时再读", self.skill)
        for name in self.references:
            self.assertIn(f"references/{name}", self.skill)

    def test_mode_details_have_single_reference_owners(self) -> None:
        for marker in (
            "项目级长期约束更新 `doc/0_architecture/`",
            "新增、删除或重命名 `doc/**.md`",
            "同一症状修复失败两次",
        ):
            self.assertIn(marker, self.ml_delivery)
            self.assertNotIn(marker, self.skill)
            self.assertNotIn(marker, self.agent_execution)

        for marker in (
            "显式分派给 `light-explorer`",
            "显式分派给 `implementation-worker`",
            "显式分派给 `review-auditor`",
            "完整交给 `collab`",
        ):
            self.assertIn(marker, self.agent_execution)
            self.assertNotIn(marker, self.skill)
            self.assertNotIn(marker, self.ml_delivery)

    def test_manifest_covers_every_packaged_develop_file(self) -> None:
        manifest = json.loads(
            (ROOT / "bridgeforge-codex-manifest.json").read_text(encoding="utf-8")
        )
        skill = next(
            item
            for item in manifest["platforms"]["codex"]["skills"]
            if item["name"] == "develop"
        )
        actual = {
            path.relative_to(ROOT).as_posix()
            for path in SKILL.parent.rglob("*")
            if path.is_file() and "__pycache__" not in path.parts
        }
        declared = {item["source"] for item in skill["files"]}
        self.assertEqual(declared, actual)


if __name__ == "__main__":
    unittest.main()
