from __future__ import annotations

import importlib.util
import json
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
HOOK = ROOT / "templates" / "hooks" / "instruction_source_check.py"
SYNC_PATH = ROOT / "scripts" / "bridgeforge_codex_project_sync.py"

def load_hook():
    spec = importlib.util.spec_from_file_location("instruction_source_check", HOOK)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def load_sync():
    spec = importlib.util.spec_from_file_location("rule_runtime_sync", SYNC_PATH)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


class InstructionSourceCheckTests(unittest.TestCase):
    def setUp(self) -> None:
        self.hook = load_hook()

    def test_agent_routing_owner_chain_is_explicit_in_public_agents(self) -> None:
        expected = (
            "用户未明确要求、且适用的项目或 Skill 指令未显式委派的阶段，"
            "必须由主对话执行；Skill 一旦委派，必须点名已存在的 Agent 角色，"
            "禁止只写“独立 agent”“子 agent”等泛称。"
        )
        self.assertIn(
            expected,
            (ROOT / "templates" / "AGENTS.md").read_text(encoding="utf-8"),
        )
        self.assertIn(
            expected,
            (ROOT / "AGENTS.md").read_text(encoding="utf-8"),
        )

    def test_public_agents_route_to_distributed_document_owners(self) -> None:
        for path in (ROOT / "templates" / "AGENTS.md", ROOT / "AGENTS.md"):
            with self.subTest(path=path):
                text = path.read_text(encoding="utf-8")
                self.assertIn("各层职责与当前布局以 `doc/README.md` 为准", text)
                self.assertIn("按项目级“快速命令”恢复主语言依赖和 `.venv`", text)
                self.assertIn("调用 `$bridgeforge-codex` 核验骨架与 Hook", text)
                self.assertNotIn("git clone <repo_url>", text)
                self.assertNotIn("`INSTALL.md` 恢复", text)
                self.assertNotIn("doc/3_reference/codex-project-operating-guide.md", text)

    def test_public_agents_only_route_clarification_to_distributed_owner(self) -> None:
        ownership_examples = (
            ("每轮最多问一个问题", "并只问一个最关键问题"),
            ("累计每 3 个问题暂停总结", "每满 3 个问题必须暂停并总结"),
        )
        for path in (ROOT / "templates" / "AGENTS.md", ROOT / "AGENTS.md"):
            with self.subTest(path=path):
                text = path.read_text(encoding="utf-8")
                self.assertIn("完整响应流程见 `doc/3_reference/codex-hook-signals.md`", text)
                self.assertNotIn("[focus]", text)
                self.assertNotIn("任务防漂移", text)
                for root_duplicate, _ in ownership_examples:
                    self.assertNotIn(root_duplicate, text)

        owner = ROOT / "templates" / "doc" / "3_reference" / "codex-hook-signals.md"
        owner_text = owner.read_text(encoding="utf-8")
        for _, owner_rule in ownership_examples:
            self.assertIn(owner_rule, owner_text)
        self.assertNotIn("## `[focus]`", owner_text)
        self.assertIn("Clarify 与 Focus 自动 Hook 均已退役", owner_text)

        contract = json.loads(
            (ROOT / "templates" / "managed-skeleton.json").read_text(encoding="utf-8")
        )
        self.assertTrue(
            any(
                item.get("source") == "templates/doc/3_reference/codex-hook-signals.md"
                and item.get("target") == "doc/3_reference/codex-hook-signals.md"
                for item in contract["assets"]
            )
        )

    def test_public_agents_scope_gate_precedes_repository_access(self) -> None:
        for path in (ROOT / "templates" / "AGENTS.md", ROOT / "AGENTS.md"):
            with self.subTest(path=path):
                text = path.read_text(encoding="utf-8")
                self.assertIn(
                    "必须先只给当前理解并问一个最关键的范围问题，本轮停止",
                    text,
                )
                self.assertIn(
                    "此范围硬闸先于读取仓库、调用工具、运行测试、写盘和进入 "
                    "`$confirm` / `$plan` / `$develop`",
                    text,
                )
                self.assertIn("只有用户回答后才能继续", text)
                self.assertIn("禁止用“读上下文”跳过硬闸", text)

    def test_public_agents_retire_project_memory_and_bound_summary(self) -> None:
        retired_contract = (
            "memory 纳入项目 Git（`.codex/memory/`）",
            "检索必须先读 `MEMORY.md` 主索引",
            "生命周期只由 `$summary` 管理",
            "深度检索只由 `$find-memory` 管理",
            "进入 `MEMORY_COLD.md`",
        )
        for path in (ROOT / "templates" / "AGENTS.md", ROOT / "AGENTS.md"):
            with self.subTest(path=path):
                text = path.read_text(encoding="utf-8")
                for phrase in (
                    "Codex 原生 `~/.codex/memories/` 只由官方机制生成和注入",
                    "禁止新建或继续使用项目 `.codex/memory/`",
                    "必须作为 legacy 原样保留并报告待迁移",
                    "同步器不得直接删除",
                    "`$summary` 只做阶段总结和“同意验收”收口",
                ):
                    self.assertIn(phrase, text)
                for phrase in retired_contract:
                    self.assertNotIn(phrase, text)

        summary_root = ROOT / "skills" / "summary"
        summary = "\n".join(
            path.read_text(encoding="utf-8")
            for path in sorted(summary_root.rglob("*.md"))
        )
        for phrase in (
            "普通模式 | 零写入",
            "禁止创建、更新、移动或删除项目 `.codex/memory/`",
            "禁止直接写入 Codex 原生 `~/.codex/memories/`",
            "等待用户采纳；未写入；未实现",
            "其他开发方法单独确认范围、落盘和验证",
        ):
            self.assertIn(phrase, summary)
        for retired_runtime in (
            "project_memory_writer.py",
            "memory_rebuild_index.py",
            "MEMORY_COLD.md",
        ):
            self.assertNotIn(retired_runtime, summary)

    def test_public_agents_keep_doc_redlines_without_skill_catalog(self) -> None:
        hardcoded_catalog = (
            "常用入口：`$confirm`",
            "`$archive-scan` / `$escalate` / `$snapshot`",
        )
        for path in (ROOT / "templates" / "AGENTS.md", ROOT / "AGENTS.md"):
            with self.subTest(path=path):
                text = path.read_text(encoding="utf-8")
                self.assertIn("`doc/0_architecture`、`1_delivery`、`2_bugs`", text)
                self.assertIn("`doc/README.md` 是唯一索引", text)
                self.assertIn("`delivery_layout` 是交付路径单一事实源", text)
                self.assertIn("所有测试代码必须放在 `scripts/tests/**`", text)
                self.assertIn("可用 Skill 以当前会话原生发现结果为准", text)
                for phrase in hardcoded_catalog:
                    self.assertNotIn(phrase, text)

        doc_owner = (ROOT / "templates" / "doc" / "README.md").read_text(encoding="utf-8")
        for layer in ("0_architecture", "1_delivery", "2_bugs", "3_reference", "4_archive"):
            self.assertIn(layer, doc_owner)
        self.assertIn("delivery_layout:", doc_owner)
        self.assertIn("## 文档生命周期", doc_owner)
        self.assertIn("`lifecycle`", doc_owner)
        self.assertIn("`validation_status`", doc_owner)
        self.assertIn("缺少 `lifecycle` 的事项视为 `unclassified`", doc_owner)

    def test_public_agents_keep_compact_communication_and_evidence_redlines(self) -> None:
        required = (
            "默认先给结论，再给依据",
            "未验证 / 缺证据 / 只是推断",
            "代码审查必须先列问题、按严重度排序",
            "架构判断必须先给推荐结论",
            "执行类任务默认接管到结果",
            "已做什么 / 验证了什么 / 还剩什么风险",
            "禁止直接下结论或改盘，先用单命令二次验真",
            "实际命令或收据、具体验证断言和覆盖场景",
            "发现自己的结论或操作错误时必须立即承认、更正并重新验证",
            "项目自建的 CPython 3.11+ `.venv`",
            "其他场景禁止回退 PATH",
            "非 ASCII 正文禁止经 shell 字符串中转",
            "可验证的无可见控制台窗口入口",
        )
        for path in (ROOT / "templates" / "AGENTS.md", ROOT / "AGENTS.md"):
            with self.subTest(path=path):
                text = path.read_text(encoding="utf-8")
                for phrase in required:
                    self.assertIn(phrase, text)

    def test_public_agents_keep_compact_debug_and_audit_redlines(self) -> None:
        required = (
            "必须独立验证，禁止只靠自测",
            "任务跨两个以上陌生模块时必须先调研再动手",
            "同一 Bug 前 3 次修改失败后，第 4 次禁止继续写",
            "再次修改前必须取得量化证据",
            "列出已试方案和未验证假说",
            "进入 `$escalate` 或 `$debate`",
            "确认数据源、用户路径、边界条件和外部副作用",
            "根因未确认时必须标明置信度",
            "性能调优必须先用 timer、counter 或 log 建立基线",
            "必须启动独立 agent 二次审计",
        )
        for path in (ROOT / "templates" / "AGENTS.md", ROOT / "AGENTS.md"):
            with self.subTest(path=path):
                text = path.read_text(encoding="utf-8")
                for phrase in required:
                    self.assertIn(phrase, text)
                self.assertEqual(text.count("修改前必须追踪完整调用链"), 1)

    def _zoned_project(self, root: Path) -> Path:
        (root / "doc" / "3_reference").mkdir(parents=True)
        (root / "doc" / "README.md").write_text("# Docs\n", encoding="utf-8")
        (root / "doc" / "3_reference" / "codex-hook-signals.md").write_text(
            "# Hook signals\n",
            encoding="utf-8",
        )
        target = root / "AGENTS.md"
        target.write_text(
            (ROOT / "templates/AGENTS.md").read_text(encoding="utf-8").replace(
                "{{PROJECT_NAME}}", root.name
            ),
            encoding="utf-8",
        )
        contract = root / ".codex" / "managed-skeleton.json"
        contract.parent.mkdir(parents=True)
        shutil.copy2(ROOT / "templates/managed-skeleton.json", contract)
        return target

    def test_retired_runtime_asset_is_blocked_in_active_instructions(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            agents = self._zoned_project(root)
            agents.write_text(
                agents.read_text(encoding="utf-8").replace(
                    "<!-- BRIDGEFORGE:PROJECT:END -->",
                    "- 项目继续使用 skill-routing.json。\n"
                    "<!-- BRIDGEFORGE:PROJECT:END -->",
                ),
                encoding="utf-8",
            )

            issues = self.hook.instruction_source_issues(root)

            self.assertTrue(
                any("retired runtime asset skill-routing.json" in item for item in issues),
                issues,
            )

    def test_missing_managed_instruction_reference_is_blocked(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self._zoned_project(root)
            (root / "doc" / "3_reference" / "codex-hook-signals.md").unlink()

            issues = self.hook.instruction_source_issues(root)

            self.assertTrue(
                any("references missing managed instruction document" in item for item in issues),
                issues,
            )

    def test_project_zone_edit_is_allowed_but_public_or_marker_edit_is_blocked(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            agents = self._zoned_project(root)
            self.assertEqual(self.hook.instruction_source_issues(root), [])
            baseline = agents.read_text(encoding="utf-8")
            agents.write_text(
                baseline.replace(
                    "### 项目业务与安全红线",
                    "### 项目业务与安全红线\n\n- 项目订单必须经过本地风控。",
                ),
                encoding="utf-8",
            )
            self.assertEqual(self.hook.instruction_source_issues(root), [])
            agents.write_text(
                baseline.replace("默认先给结论", "默认最后给结论", 1),
                encoding="utf-8",
            )
            issues = self.hook.instruction_source_issues(root)
            self.assertTrue(any("public zone was modified" in item for item in issues))
            agents.write_text(
                baseline.replace("<!-- BRIDGEFORGE:PROJECT:END -->", "", 1),
                encoding="utf-8",
            )
            issues = self.hook.instruction_source_issues(root)
            self.assertTrue(any("exactly once" in item for item in issues))

    def test_project_zone_allows_fenced_heading_examples(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            agents = self._zoned_project(root)
            baseline = agents.read_text(encoding="utf-8")
            agents.write_text(
                baseline.replace(
                    "### 项目业务与安全红线",
                    "### 项目业务与安全红线\n\n"
                    "```markdown\n### 项目架构红线\n- 示例，不是结构标题。\n```",
                ),
                encoding="utf-8",
            )
            self.assertEqual(self.hook.instruction_source_issues(root), [])

    def test_zoned_agents_fail_closed_when_contract_is_missing_or_invalid(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self._zoned_project(root)
            contract = root / ".codex" / "managed-skeleton.json"
            contract.unlink()
            issues = self.hook.instruction_source_issues(root)
            self.assertTrue(any("cannot be verified" in item for item in issues))

            contract.write_text("{invalid", encoding="utf-8")
            issues = self.hook.instruction_source_issues(root)
            self.assertTrue(any("cannot be verified" in item for item in issues))

    def test_staged_public_edit_is_detected_after_worktree_restore(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            agents = self._zoned_project(root)
            subprocess.run(["git", "init"], cwd=root, check=True, capture_output=True)
            subprocess.run(["git", "config", "user.email", "test@example.com"], cwd=root, check=True)
            subprocess.run(["git", "config", "user.name", "Test"], cwd=root, check=True)
            subprocess.run(["git", "add", "AGENTS.md", ".codex/managed-skeleton.json"], cwd=root, check=True)
            subprocess.run(["git", "commit", "-m", "baseline"], cwd=root, check=True, capture_output=True)
            baseline = agents.read_bytes()
            agents.write_bytes(baseline.replace("默认先给结论".encode(), "默认最后给结论".encode()))
            subprocess.run(["git", "add", "AGENTS.md"], cwd=root, check=True)
            agents.write_bytes(baseline)
            staged = self.hook._git_agents(root, "INDEX")
            self.assertIsNotNone(staged)
            issues = self.hook._root_agents_issues(
                staged, root, label="staged AGENTS.md"
            )
            self.assertTrue(any("public zone was modified" in item for item in issues))

    def test_unchanged_staged_agents_does_not_block_unrelated_commit(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self._zoned_project(root)
            subprocess.run(["git", "init"], cwd=root, check=True, capture_output=True)
            subprocess.run(["git", "config", "user.email", "test@example.com"], cwd=root, check=True)
            subprocess.run(["git", "config", "user.name", "Test"], cwd=root, check=True)
            subprocess.run(["git", "add", "AGENTS.md"], cwd=root, check=True)
            subprocess.run(["git", "commit", "-m", "baseline"], cwd=root, check=True, capture_output=True)
            (root / "business.txt").write_text("changed\n", encoding="utf-8")
            subprocess.run(["git", "add", "business.txt"], cwd=root, check=True)
            self.assertEqual(self.hook._staged_agents_issues(root), [])

    def test_factory_rejects_markdown_rule_directory(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "templates" / "rules").mkdir(parents=True)
            (root / "templates" / "rules" / "legacy.md").write_text("---\npaths: ['src/**']\n---\n", encoding="utf-8")
            self._zoned_project(root)
            (root / "templates" / "AGENTS.md").write_bytes(
                (ROOT / "templates" / "AGENTS.md").read_bytes()
            )
            issues = self.hook.instruction_source_issues(root)
            self.assertTrue(any("must remain retired" in item for item in issues), issues)

    def test_negative_autoload_statement_and_runtime_fixture_are_ignored(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            agents = self._zoned_project(root)
            agents.write_text(
                agents.read_text(encoding="utf-8")
                .replace("<!-- BRIDGEFORGE:PROJECT:END -->", "Markdown 中的 paths: 不会被 Codex 自动加载。\n<!-- BRIDGEFORGE:PROJECT:END -->"),
                encoding="utf-8",
            )
            runtime_agents = root / ".runtime" / "historical" / "AGENTS.md"
            runtime_agents.parent.mkdir(parents=True)
            runtime_agents.write_text(
                "Markdown paths: 自动加载历史夹具。\n",
                encoding="utf-8",
            )
            self.assertEqual(self.hook.instruction_source_issues(root), [])

            normal_nested = root / "src" / "AGENTS.md"
            comma_nested = root / "lib" / "AGENTS.md"
            for nested, separator in (
                (normal_nested, "；"),
                (comma_nested, "，但"),
            ):
                nested.parent.mkdir(parents=True)
                nested.write_text(
                    f"Codex 不支持 Markdown paths{separator}BridgeForge 会自动加载 Markdown paths: rules。\n",
                    encoding="utf-8",
                )
            issues = self.hook.instruction_source_issues(root)
            self.assertTrue(any("src\\AGENTS.md" in item or "src/AGENTS.md" in item for item in issues))
            self.assertTrue(any("lib\\AGENTS.md" in item or "lib/AGENTS.md" in item for item in issues))

    def test_dispatcher_and_precommit_register_new_gate(self) -> None:
        for dispatcher in (
            ROOT / "templates" / "hooks" / "hook_dispatcher.py",
            ROOT / ".codex" / "hooks" / "hook_dispatcher.py",
        ):
            text = dispatcher.read_text(encoding="utf-8")
            self.assertIn('"hooks/instruction_source_check.py"', text)
        for precommit in (ROOT / "templates" / ".githooks" / "pre-commit", ROOT / ".githooks" / "pre-commit"):
            self.assertIn("instruction_source_check.py", precommit.read_text(encoding="utf-8"))

    def test_precommit_region_uses_only_the_current_ownership_rule(self) -> None:
        sync_source = SYNC_PATH.read_text(encoding="utf-8")
        builder_source = (
            ROOT / "scripts" / "rebuild_shared_skill_manifest.py"
        ).read_text(encoding="utf-8")
        release_source = (
            ROOT / "templates" / "scripts" / "version_release.py"
        ).read_text(encoding="utf-8")
        self.assertNotIn("BRIDGEFORGE_MANAGED_BEGIN", sync_source)
        self.assertNotIn("_merge_region_history", builder_source)
        self.assertNotIn("schema v1 managed region", release_source)

    def test_agents_contract_has_no_historical_ownership_path(self) -> None:
        sync = load_sync()
        contract = json.loads((ROOT / "templates" / "managed-skeleton.json").read_text(encoding="utf-8"))
        asset = next(item for item in contract["assets"] if item["id"] == "root.agents")
        self.assertEqual(set(asset).intersection({"managed_blocks", "section_layout"}), set())
        self.assertNotIn(
            "legacy_section_migrations",
            asset["agents_zones"]["project"],
        )
        self.assertFalse(hasattr(sync, "_legacy_agents_source"))


if __name__ == "__main__":
    unittest.main()
