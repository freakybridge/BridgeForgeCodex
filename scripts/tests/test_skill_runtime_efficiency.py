#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
import sys
import types
import unittest
from pathlib import Path
from unittest import mock


ROOT = Path(__file__).resolve().parents[2]
SHOW_STATES = (
    ROOT / ".codex/hooks/show_state.py",
    ROOT / "templates/hooks/show_state.py",
)
ARCHIVE_SCANS = (
    ROOT / ".codex/scripts/archive_scan.py",
    ROOT / "templates/scripts/archive_scan.py",
)
def load(path: Path, name: str) -> types.ModuleType:
    spec = importlib.util.spec_from_file_location(name, path)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


class SkillRuntimeEfficiencyTests(unittest.TestCase):
    def test_show_state_uses_one_git_process_and_preserves_fields(self) -> None:
        receipt = "\n".join((
            "# branch.oid abc123",
            "# branch.head codex/fast-path",
            "# branch.upstream origin/codex/fast-path",
            "# branch.ab +2 -3",
            "1 .M N... 100644 100644 100644 abc abc work.py",
            "? new.txt",
        ))
        for index, path in enumerate(SHOW_STATES):
            with self.subTest(path=path):
                module = load(path, f"show_state_efficiency_{index}")
                completed = types.SimpleNamespace(returncode=0, stdout=receipt)
                with mock.patch.object(module.subprocess, "run", return_value=completed) as run:
                    self.assertEqual(module._git_state(), ("codex/fast-path", 2, "2/3"))
                run.assert_called_once()
                self.assertEqual(
                    run.call_args.args[0],
                    ["git", "status", "--porcelain=v2", "--branch"],
                )

    def test_show_state_preserves_detached_and_no_upstream_fallbacks(self) -> None:
        module = load(SHOW_STATES[0], "show_state_fallbacks")
        self.assertEqual(
            module._parse_git_status("# branch.oid abc\n# branch.head (detached)\n"),
            ("?", 0, "no-upstream"),
        )
        self.assertEqual(module._parse_git_status(""), ("?", 0, "no-upstream"))

    def test_archive_scan_batches_git_history_and_keeps_untracked_none(self) -> None:
        module = load(ARCHIVE_SCANS[0], "archive_scan_efficiency")
        module.REPO_ROOT = Path("C:/repo")
        tracked = module.REPO_ROOT / "doc/2_bugs/BUG-001-中文.md"
        untracked = module.REPO_ROOT / "doc/2_bugs/BUG-002.md"
        now = 50 * 86400
        receipt = (
            "\0@@bridgeforge-commit-time:864000\0\0\n"
            "doc/2_bugs/BUG-001-中文.md\0"
        )
        completed = types.SimpleNamespace(returncode=0, stdout=receipt)
        with (
            mock.patch.object(module.subprocess, "run", return_value=completed) as run,
            mock.patch.object(module.time, "time", return_value=now),
        ):
            days = module._days_by_path([tracked, untracked])
        self.assertEqual(days, {tracked: 40, untracked: None})
        run.assert_called_once()
        command = run.call_args.args[0]
        self.assertEqual(command[:11], [
            "git", "-c", "core.quotepath=false", "log", "--no-renames",
            "--format=%x00@@bridgeforge-commit-time:%at%x00", "--name-only",
            "-z", "--", "doc/2_bugs/BUG-001-中文.md",
            "doc/2_bugs/BUG-002.md",
        ])
        self.assertEqual(command[11:], [])

    def test_runtime_scripts_are_mirrored_for_codex(self) -> None:
        self.assertEqual(SHOW_STATES[0].read_bytes(), SHOW_STATES[1].read_bytes())
        archive = ARCHIVE_SCANS[0].read_bytes()
        for path in ARCHIVE_SCANS[1:]:
            self.assertEqual(archive, path.read_bytes())

    def test_summary_and_todo_are_native_memory_read_write_free(self) -> None:
        todo = (ROOT / "skills/todo/SKILL.md").read_text(encoding="utf-8")
        summary_files = [ROOT / "skills/summary/SKILL.md"]
        summary_files.extend(sorted((ROOT / "skills/summary/references").glob("*.md")))
        summary = "\n".join(path.read_text(encoding="utf-8") for path in summary_files)
        combined = summary + "\n" + todo

        self.assertIn("禁止创建、更新、移动或删除项目 `.codex/memory/`", summary)
        self.assertIn("禁止直接写入 Codex 原生 `~/.codex/memories/`", summary)
        self.assertIn("禁止创建、读取、更新、移动或删除项目 `.codex/memory/`", todo)
        self.assertIn("禁止直接写入或把内容路由到 Codex 原生", todo)
        for retired_runtime in (
            "project_memory_writer.py",
            "memory_rebuild_index.py",
            "memory_lint.py",
            "MEMORY_COLD.md",
        ):
            self.assertNotIn(retired_runtime, combined)

    def test_find_doc_does_not_route_to_project_or_native_memory(self) -> None:
        find_doc_root = ROOT / "skills/find-doc"
        files = [find_doc_root / "SKILL.md"]
        files.extend(sorted((find_doc_root / "references").glob("*.md")))
        content = "\n".join(path.read_text(encoding="utf-8") for path in files)

        self.assertIn("不扫描源代码、项目 Memory 或原生 Memory", content)
        self.assertIn("禁止扫描源代码、项目 `.codex/memory/`", content)
        for retired_route in (
            "相关 memory",
            "memory 索引",
            "entries from Path D",
            "Path E",
        ):
            self.assertNotIn(retired_route, content)

    def test_confirm_is_the_only_scale_budget_contract_owner(self) -> None:
        skill_text = {
            path.parent.name: path.read_text(encoding="utf-8")
            for path in (ROOT / "skills").glob("*/SKILL.md")
        }
        for marker in (
            "20 分钟 / 8k 新增 token",
            "45 分钟 / 20k 新增 token",
            "平台没有可靠 token 计量器",
            "验证轮次统一口径",
        ):
            owners = [name for name, text in skill_text.items() if marker in text]
            self.assertEqual(owners, ["confirm"], marker)

        develop = skill_text["develop"]
        self.assertIn("读取并应用 `confirm` 的“规模与预算硬闸”", develop)
        for path_name in ("S 级直接路径", "M 级精简路径", "L 级完整路径"):
            self.assertIn(path_name, develop)

    def test_collab_is_the_only_parallel_execution_contract_owner(self) -> None:
        collab = (ROOT / "skills/collab/SKILL.md").read_text(encoding="utf-8")
        develop_entry = (ROOT / "skills/develop/SKILL.md").read_text(encoding="utf-8")
        agent_execution = (
            ROOT / "skills/develop/references/agent-execution.md"
        ).read_text(encoding="utf-8")
        develop = develop_entry + "\n" + agent_execution

        self.assertIn("并行研读、拆分确认、执行分派、串联和独立验证机制的唯一 owner", collab)
        for parallel_rule in (
            "单任务控制在 3-5 个文件",
            "同一并行组同时启动多个实例",
            "禁止同一并行组的 agent 修改同一文件",
        ):
            self.assertIn(parallel_rule, collab)
            self.assertNotIn(parallel_rule, develop)

        self.assertIn("完整交给 `collab`", agent_execution)
        self.assertIn("直接复用 `collab` 的独立 review 收据", agent_execution)
        self.assertNotIn("不得重新走其用户确认闸", develop)

    def test_plan_and_collab_guard_runtime_usability_failures(self) -> None:
        plan = (ROOT / "skills/plan/SKILL.md").read_text(encoding="utf-8")
        collab = (ROOT / "skills/collab/SKILL.md").read_text(encoding="utf-8")

        self.assertLess(plan.index("范围充分性硬闸"), plan.index("读取相关代码"))
        self.assertIn("只读评估也不得跳过此闸", plan)
        self.assertIn("禁止在计划阶段运行完整回归、下游 fixture 或性能测试", plan)

        self.assertIn("都不能代替有效确认卡，也不构成例外", collab)
        self.assertIn("禁止降级为无确认卡的临时协作流程", collab)
        self.assertIn("禁止以只读、时间紧或目标看似明确为由绕过", collab)



if __name__ == "__main__":
    unittest.main()
