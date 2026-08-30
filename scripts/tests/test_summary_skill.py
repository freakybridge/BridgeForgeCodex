#!/usr/bin/env python3
from __future__ import annotations

import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SKILL = ROOT / "skills" / "summary" / "SKILL.md"
REFERENCES = ROOT / "skills" / "summary" / "references"


class SummarySkillContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.skill = SKILL.read_text(encoding="utf-8")
        cls.references = {
            path.name: path.read_text(encoding="utf-8")
            for path in sorted(REFERENCES.glob("*.md"))
        }
        cls.ordinary = cls.references["ordinary-mode.md"]
        cls.acceptance = cls.references["acceptance-mode.md"]
        cls.deep_steps = cls.references["deep-steps.md"]
        cls.all_text = cls.skill + "\n" + "\n".join(cls.references.values())

    def test_two_modes_have_distinct_bounded_write_surfaces(self) -> None:
        for marker in (
            "`$summary` | 普通模式 | 零写入",
            "`$summary 同意验收` | 验收模式",
            "当前交付既有需求卡或 Bug 的 `lifecycle`",
        ):
            self.assertIn(marker, self.skill)
        self.assertIn("普通模式只汇总阶段进展", self.ordinary)
        self.assertIn("零写入", self.ordinary)
        self.assertIn("只更新当前交付已经存在", self.acceptance)
        self.assertIn("禁止为了收口新建文档", self.acceptance)
        self.assertIn("`lifecycle: completed`", self.acceptance)
        self.assertIn("`validation_status: verified`", self.acceptance)

    def test_summary_forbids_project_and_native_memory_writes(self) -> None:
        for marker in (
            "禁止创建、更新、移动或删除项目 `.codex/memory/`",
            "禁止直接写入 Codex 原生 `~/.codex/memories/`",
            "禁止调用其 writer、rebuild、lint、检索或统计链",
        ):
            self.assertIn(marker, self.skill)
        for retired_reference in ("writer-routing.md", "memory-targets.md"):
            self.assertNotIn(retired_reference, self.references)
            self.assertFalse((REFERENCES / retired_reference).exists())

    def test_rule_and_hook_candidates_are_suggestions_only(self) -> None:
        for marker in (
            "完整可见对话",
            "等待用户采纳；未写入；未实现",
            "用户在本次调用中同意验收，不等于采纳建议",
            "其他开发方法单独确认范围、落盘和验证",
        ):
            self.assertIn(marker, self.skill)
        for marker in (
            "稳定触发条件",
            "建议承载者",
            "事实源关系",
            "误伤风险与验证",
            "本次建议不是实现授权",
        ):
            self.assertIn(marker, self.deep_steps)

    def test_acceptance_stays_with_current_delivery(self) -> None:
        for marker in (
            "任一必要条件未满足或收据冲突",
            "其他 topic、Bug 和项目级 TODO 保持不变",
            "请另行调用 `$archive-scan`",
            "禁止执行",
            "`git mv`",
        ):
            self.assertIn(marker, self.acceptance)
        self.assertIn("Rule、AGENTS、Hook、配置和测试始终零写入", self.acceptance)

    def test_evidence_and_git_boundaries_are_preserved(self) -> None:
        for marker in (
            "禁止为了总结重新运行测试、build、审计或 smoke",
            "缺少收据时标记“未验证”",
            "git status",
            "未暂存、未 commit、未 push",
            "$git-sync",
        ):
            self.assertIn(marker, self.skill)

    def test_entry_is_small_and_routes_every_reference(self) -> None:
        self.assertLessEqual(len(self.skill.splitlines()), 90)
        self.assertEqual(
            set(self.references),
            {"ordinary-mode.md", "acceptance-mode.md", "deep-steps.md"},
        )
        for name in self.references:
            self.assertIn(f"references/{name}", self.skill)

        manifest = json.loads(
            (ROOT / "bridgeforge-codex-manifest.json").read_text(encoding="utf-8")
        )
        summary = next(
            item
            for item in manifest["platforms"]["codex"]["skills"]
            if item["name"] == "summary"
        )
        actual = {
            path.relative_to(ROOT).as_posix()
            for path in SKILL.parent.rglob("*")
            if path.is_file() and "__pycache__" not in path.parts
        }
        declared = {item["source"] for item in summary["files"]}
        self.assertEqual(declared, actual)

    def test_harvest_alias_is_not_reintroduced(self) -> None:
        self.assertNotIn("harvest", self.all_text.lower())


if __name__ == "__main__":
    unittest.main()
