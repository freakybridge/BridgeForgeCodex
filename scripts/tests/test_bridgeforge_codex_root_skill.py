from __future__ import annotations

import json
import re
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SKILL = ROOT / "skills/bridgeforge-codex/SKILL.md"
REFERENCES = ROOT / "skills/bridgeforge-codex/references"
OPENAI_YAML = ROOT / "skills/bridgeforge-codex/agents/openai.yaml"
SLASH_COMMAND = re.compile(r"(?<![A-Za-z0-9_.~-])/bridgeforge(?:-codex)?\b")

USER_COMMAND_SURFACES = (
    ROOT / "README.md",
    ROOT / "INSTALL.md",
    ROOT / "skills/bridgeforge-codex/SKILL.md",
    ROOT / "scripts/install-shared-skills.ps1",
    ROOT / "skills/summary/SKILL.md",
    ROOT / "templates/hooks/config_health_check.py",
    ROOT / ".codex/hooks/config_health_check.py",
    ROOT / "templates/hooks/skill_sync_check.py",
    ROOT / ".codex/hooks/skill_sync_check.py",
    ROOT / "templates/scripts/version_release.py",
    ROOT / ".codex/scripts/version_release.py",
)

ACTIVE_TEXT_SUFFIXES = {".json", ".md", ".ps1", ".py", ".toml", ".yaml", ".yml"}
PASCAL_CASE_ALLOWED_SNIPPETS = (
    "https://github.com/freakybridge/BridgeForgeCodex.git",
    "freakybridge/BridgeForgeCodex",
    r"D:\tools\BridgeForgeCodex",
    "BridgeForgeCodex/",
    "git clone <repo_url> BridgeForgeCodex && cd BridgeForgeCodex",
    r"Local\BridgeForgeCodex.SharedSkillUpdate",
    '"heading": "## 2 BridgeForgeCodex 协作骨架"',
    'replace("{{PROJECT_NAME}}", "BridgeForgeCodex")',
)
HISTORICAL_HOOK_DESCRIPTION = (
    "BridgeForgeCodex project lifecycle hooks. "
    "This is the only managed Codex hook registration source."
)


class BridgeForgeCodexRootSkillTests(unittest.TestCase):
    def test_active_user_commands_use_dollar_skill_invocation(self) -> None:
        for path in USER_COMMAND_SURFACES:
            with self.subTest(path=path):
                text = path.read_text(encoding="utf-8-sig")
                self.assertIsNone(SLASH_COMMAND.search(text))

    def test_pascal_case_name_is_confined_to_technical_allowlist(self) -> None:
        candidates = [
            ROOT / "README.md",
            ROOT / "INSTALL.md",
            ROOT / "AGENTS.md",
            ROOT / "bridgeforge-codex-manifest.json",
        ]
        for base in (ROOT / "skills", ROOT / "scripts", ROOT / "templates", ROOT / ".codex"):
            candidates.extend(
                path
                for path in base.rglob("*")
                if path.is_file()
                and path.suffix.lower() in ACTIVE_TEXT_SUFFIXES
                and not {"tests", "compat", "memory", "__pycache__"}.intersection(
                    path.relative_to(ROOT).parts
                )
            )

        for path in sorted(set(candidates)):
            text = path.read_text(encoding="utf-8-sig", errors="replace")
            for line_number, line in enumerate(text.splitlines(), start=1):
                if "BridgeForge Codex" not in line and "BridgeForgeCodex" not in line:
                    continue
                with self.subTest(path=path, line=line_number):
                    self.assertNotIn("BridgeForge Codex", line)
                    historical_contract_value = (
                        path.name == "managed-skeleton.json"
                        and line.strip().rstrip(",")
                        == json.dumps(HISTORICAL_HOOK_DESCRIPTION)
                    )
                    self.assertTrue(
                        historical_contract_value
                        or any(
                            snippet in line
                            for snippet in PASCAL_CASE_ALLOWED_SNIPPETS
                        ),
                        f"unexpected active PascalCase product name at {path}:{line_number}",
                    )

    def test_menu_display_name_is_exact_lowercase_slug(self) -> None:
        metadata = OPENAI_YAML.read_text(encoding="utf-8")
        self.assertIn('display_name: "bridgeforge-codex"', metadata)
        self.assertIn("$bridgeforge-codex", metadata)

    def test_user_visible_product_name_is_lowercase_kebab_case(self) -> None:
        display_surfaces = (
            ROOT / "templates/AGENTS.md",
            ROOT / "skills/bridgeforge-codex/SKILL.md",
            ROOT / "doc/0_architecture/design/codex-project-sync.md",
            ROOT / "doc/3_reference/codex-project-operating-guide.md",
        )
        for path in display_surfaces:
            with self.subTest(path=path):
                text = path.read_text(encoding="utf-8")
                self.assertIn("bridgeforge-codex", text)
                self.assertNotIn("BridgeForgeCodex", text)

        readme = (ROOT / "README.md").read_text(encoding="utf-8")
        install = (ROOT / "INSTALL.md").read_text(encoding="utf-8")
        self.assertTrue(readme.startswith("# bridgeforge-codex\n"))
        self.assertTrue(install.startswith("# bridgeforge-codex 安装与迁移\n"))
        self.assertIn("freakybridge/BridgeForgeCodex.git", readme)
        self.assertIn("freakybridge/BridgeForgeCodex.git", install)

    def test_only_new_codex_entry_is_active(self) -> None:
        self.assertTrue(SKILL.is_file())
        self.assertFalse((ROOT / "skills/bridgeforge").exists())
        self.assertFalse((ROOT / "templates/claude").exists())
        self.assertFalse((ROOT / "CLAUDE.md").exists())
        text = SKILL.read_text(encoding="utf-8")
        self.assertIn("name: bridgeforge-codex", text)
        self.assertIn("Codex-only", text)
        self.assertIn('".bridgeforge-codex"', text)
        self.assertIn("薄入口只是 Codex 可发现入口", text)
        self.assertIn("scripts/bridgeforge_codex_project_sync.py", text)

    def test_legacy_user_migration_surfaces_are_retired(self) -> None:
        for relative in (
            "shared-skill-manifest.json",
            "scripts/bridgeforge_codex_legacy_entry.SKILL.md",
            "scripts/bridgeforge_codex_user_migrate.py",
        ):
            self.assertFalse((ROOT / relative).exists(), relative)
        compatibility_root = ROOT / "scripts/compat/legacy-shared-skills"
        self.assertFalse(
            compatibility_root.exists()
            and any(path.is_file() for path in compatibility_root.rglob("*"))
        )

    def test_current_only_risk_and_project_sync_contract_are_explicit(self) -> None:
        text = "\n".join(
            [SKILL.read_text(encoding="utf-8")]
            + [
                path.read_text(encoding="utf-8")
                for path in sorted(REFERENCES.glob("*.md"))
            ]
        )
        for marker in (
            "destructive rebuild 计划",
            "`review-auditor`",
            "`implementation-worker`",
            "逐项确认可以组成本轮唯一确认",
            "plan-fingerprint",
            "--confirmed-preservation-manifest",
            "--confirmed-risk",
            "current baseline",
            "`repair-hook` 只能修改用户 hooks",
            "项目骨架更新禁止顺手执行完整 `reconcile`",
            "本地较新自动上传",
            "远端较新自动恢复",
            "日常同步和 hook 修复不得重复询问",
            "user_native_memory_readiness",
            "remote_reconcile=applied/declined/not_requested",
            ".codex/.bridgeforge_codex_version",
            ".codex/.bridgeforge_version",
        ):
            self.assertIn(marker, text)

    def test_version_domain_and_transaction_have_distinct_owners(self) -> None:
        skill = SKILL.read_text(encoding="utf-8")
        transaction = (REFERENCES / "transaction.md").read_text(encoding="utf-8")
        guide = (
            ROOT / "doc/3_reference/codex-project-operating-guide.md"
        ).read_text(encoding="utf-8")
        agents_text = (
            (ROOT / "AGENTS.md").read_text(encoding="utf-8"),
            (ROOT / "templates/AGENTS.md").read_text(encoding="utf-8"),
        )

        self.assertIn("版本戳写入顺序的唯一操作 owner", transaction)
        self.assertIn("最后写 `.codex/.bridgeforge_codex_version`", transaction)
        self.assertIn("任一失败回滚本事务全部写入", transaction)
        self.assertNotIn("版本戳写入顺序的唯一操作 owner", skill)
        self.assertNotIn("最后写 `.codex/.bridgeforge_codex_version`", skill)
        for agents in agents_text:
            self.assertIn("仅允许统一项目同步器修改", agents)
            self.assertNotIn("在全部验证通过后最后写入", agents)
            self.assertNotIn("版本戳只允许在 ready 时最后写", agents)

        self.assertIn("唯一归 `$bridgeforge-codex`", guide)
        for operational_detail in (
            "release preflight",
            "ownership classifier",
            "逐文件 `G*` 清单",
        ):
            self.assertNotIn(operational_detail, guide)

    def test_default_result_is_conclusion_first_and_hides_raw_receipt(self) -> None:
        text = SKILL.read_text(encoding="utf-8")
        result_contract = text[text.index("## 5. Apply 与用户结果") :]
        receipt_contract = (REFERENCES / "technical-receipts.md").read_text(
            encoding="utf-8"
        )
        for marker in (
            "结论、待处理事项、下一步",
            "`human` 区",
            "禁止自行改写结论",
            "--output-format combined",
            "本次操作已结束，无需继续处理",
            "说明当前骨架版本",
            "`$git-sync`",
            "同步器确定的停止原因",
            "只有影响当前结果、需要用户操作或会改变后续行为时才展示",
            "用户未追问时不得补发整份技术清单",
        ):
            self.assertIn(marker, result_contract)

        self.assertIn("`machine` 保持旧 JSON 自动化合同", receipt_contract)
        self.assertIn("不得用临场解释覆盖 `human` 区结论", receipt_contract)

        for marker in (
            "`execution_status`",
            "preserved project asset IDs",
            "rollback 字段",
            "`user_native_memory_readiness`",
        ):
            self.assertNotIn(marker, result_contract)
            self.assertIn(marker, receipt_contract)

    def test_preservation_manifest_is_the_only_old_project_decision_term(self) -> None:
        skill = SKILL.read_text(encoding="utf-8")
        adopt = (REFERENCES / "adopt.md").read_text(encoding="utf-8")
        sync = (
            ROOT / "scripts" / "bridgeforge_codex_project_sync.py"
        ).read_text(encoding="utf-8")
        self.assertIn("PreservationManifest", skill)
        self.assertNotIn("--confirmed-preservation-manifest", skill)
        self.assertIn("--confirmed-preservation-manifest", adopt)
        obsolete_term = "white" + "list"
        self.assertNotIn(obsolete_term, skill.casefold())
        self.assertNotIn(obsolete_term, sync.casefold())
        self.assertNotIn("project_asset_" + obsolete_term, sync)
        self.assertNotIn("--confirmed-" + obsolete_term, sync)

    def test_python_preflight_native_memory_and_project_sync_are_in_one_orchestration(self) -> None:
        text = SKILL.read_text(encoding="utf-8")
        runtime = (REFERENCES / "runtime-preflight.md").read_text(encoding="utf-8")
        native = (REFERENCES / "native-memory.md").read_text(encoding="utf-8")
        preflight = text.index("## 2. 判断模式并锁定项目 Python")
        mode = text.index("只读检查版本戳并锁定唯一 `$MODE`", preflight)
        validate = text.index("project_runtime.py", mode)
        native_memory = text.index("codex_memory_sync.py", validate)
        project_sync = text.index("bridgeforge_codex_project_sync.py", native_memory)
        self.assertLess(preflight, mode)
        self.assertLess(mode, validate)
        self.assertLess(validate, native_memory)
        self.assertLess(native_memory, project_sync)
        self.assertNotIn("\npython ", text)
        self.assertIn("status --project-root .", text)
        self.assertIn("bootstrap --project-root . --mode $MODE", runtime)
        self.assertIn("禁止持久化任一项目的绝对 Python 路径", native)
        self.assertIn("唯一 accumulator", text)

    def test_references_are_codex_only_and_have_no_switch(self) -> None:
        expected = {
            "user-skill-maintenance.md",
            "runtime-preflight.md",
            "native-memory.md",
            "init.md",
            "adopt.md",
            "update.md",
            "transaction.md",
            "technical-receipts.md",
        }
        self.assertEqual({path.name for path in REFERENCES.glob("*.md")}, expected)
        entry = SKILL.read_text(encoding="utf-8")
        for name in expected:
            self.assertIn(f"references/{name}", entry)
        combined = "\n".join(
            path.read_text(encoding="utf-8") for path in sorted(REFERENCES.glob("*.md"))
        )
        self.assertIn("bridgeforge_codex_project_sync.py", combined)
        self.assertNotIn("bridgeforge_switch.py", combined)
        self.assertNotIn("project_finalize", combined)
        self.assertNotIn("bridgeforge_codex_user_maintenance.ps1", combined)

    def test_bridgeforge_skill_manifest_covers_every_packaged_file(self) -> None:
        manifest = json.loads(
            (ROOT / "bridgeforge-codex-manifest.json").read_text(encoding="utf-8")
        )
        skill = next(
            item
            for item in manifest["platforms"]["codex"]["skills"]
            if item["name"] == "bridgeforge-codex"
        )
        prefix = "skills/bridgeforge-codex/"
        actual = {
            path.relative_to(ROOT).as_posix()
            for path in (ROOT / "skills" / "bridgeforge-codex").rglob("*")
            if path.is_file() and "__pycache__" not in path.parts
        }
        declared = {
            item["source"]
            for item in skill["files"]
            if item["source"].startswith(prefix)
        }
        self.assertEqual(declared, actual)

    def test_entry_routes_conditional_details_to_single_reference_owners(self) -> None:
        entry = SKILL.read_text(encoding="utf-8")
        owners = {
            "runtime-preflight.md": (
                "bootstrap --project-root . --mode $MODE",
                "update` 禁止创建或重建 `.venv`",
            ),
            "native-memory.md": (
                "`consent=null + disabled`",
                "本地较新自动上传",
                "`repair-hook` 只能修改用户 hooks",
            ),
            "adopt.md": (
                "`--confirmed-preservation-manifest`",
                "显式分派给 `review-auditor`",
                "显式分派给 `implementation-worker`",
            ),
            "transaction.md": (
                "版本戳写入顺序的唯一操作 owner",
                "最后写 `.codex/.bridgeforge_codex_version`",
                "任一失败回滚本事务全部写入",
            ),
            "technical-receipts.md": (
                "`execution_status`",
                "`user_native_memory_readiness`",
                "rollback 字段",
            ),
        }
        reference_text = {
            path.name: path.read_text(encoding="utf-8")
            for path in REFERENCES.glob("*.md")
        }
        for owner, markers in owners.items():
            with self.subTest(owner=owner):
                for marker in markers:
                    self.assertNotIn(marker, entry)
                    self.assertIn(marker, reference_text[owner])
                    self.assertEqual(
                        [
                            name
                            for name, text in reference_text.items()
                            if marker in text
                        ],
                        [owner],
                    )

        self.assertNotIn("<1.4.31", reference_text["update.md"])
        self.assertIn("<1.4.31", reference_text["adopt.md"])

    def test_shared_skills_inherit_session_model(self) -> None:
        skill_files = sorted((ROOT / "skills").glob("*/SKILL.md"))
        self.assertGreaterEqual(len(skill_files), 15)
        for path in skill_files:
            with self.subTest(path=path):
                frontmatter = path.read_text(encoding="utf-8").split("---", 2)[1]
                self.assertNotRegex(frontmatter, r"(?m)^model\s*:")


if __name__ == "__main__":
    unittest.main()
