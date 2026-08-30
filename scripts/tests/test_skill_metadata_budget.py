#!/usr/bin/env python3
from __future__ import annotations

import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / ".codex" / "hooks" / "skill_metadata_check.py"
ALL_METADATA_HOOKS = (
    (".codex", ROOT / ".codex" / "hooks" / "skill_metadata_check.py"),
    (".codex", ROOT / "templates" / "hooks" / "skill_metadata_check.py"),
)


def skill_text(
    name: str,
    description: str = "compact discovery",
    body: str = "# Body\n",
    invocation: bool = True,
) -> str:
    invocation_metadata = "user_invocable: true\nargument: 无\n" if invocation else ""
    return (
        "---\n"
        f"name: {name}\n"
        f"description: {description}\n"
        f"{invocation_metadata}"
        "---\n\n"
        f"{body}"
    )


class SkillMetadataBudgetTests(unittest.TestCase):
    def make_repo(self) -> Path:
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        repo = Path(self.temp.name)
        hooks = repo / ".codex" / "hooks"
        hooks.mkdir(parents=True)
        shutil.copy2(SCRIPT, hooks / SCRIPT.name)
        agents = repo / "templates" / "agents"
        agents.mkdir(parents=True)
        for name in ("light-explorer", "implementation-worker", "review-auditor"):
            (agents / f"{name}.toml").write_text(
                f'name = "{name}"\n',
                encoding="utf-8",
            )
        return repo

    def run_hook(self, repo: Path) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [sys.executable, "-B", str(repo / ".codex" / "hooks" / SCRIPT.name)],
            cwd=repo,
            capture_output=True,
            text=True,
            encoding="utf-8",
            check=False,
        )

    def write_skill(self, repo: Path, name: str, text: str) -> None:
        folder = repo / "skills" / name
        folder.mkdir(parents=True, exist_ok=True)
        (folder / "SKILL.md").write_text(text, encoding="utf-8")

    def test_good_skill_passes(self) -> None:
        repo = self.make_repo()
        self.write_skill(repo, "demo", skill_text("demo"))
        self.assertEqual(self.run_hook(repo).returncode, 0)

    def test_openai_standard_frontmatter_passes(self) -> None:
        repo = self.make_repo()
        self.write_skill(repo, "demo", skill_text("demo", invocation=False))
        self.assertEqual(self.run_hook(repo).returncode, 0)

    def test_incomplete_invocation_metadata_fails(self) -> None:
        repo = self.make_repo()
        text = (
            "---\n"
            "name: demo\n"
            "description: compact discovery\n"
            "user_invocable: true\n"
            "---\n\n"
            "# Body\n"
        )
        self.write_skill(repo, "demo", text)
        result = self.run_hook(repo)
        self.assertEqual(result.returncode, 2)
        self.assertIn("invocation metadata requires argument", result.stderr)

    def test_orphan_skill_directory_and_root_file_fail(self) -> None:
        repo = self.make_repo()
        orphan = repo / "skills" / "orphan"
        orphan.mkdir(parents=True)
        (repo / "skills" / "README.md").write_text("not a skill\n", encoding="utf-8")
        result = self.run_hook(repo)
        self.assertEqual(result.returncode, 2)
        self.assertIn("skills/orphan: missing SKILL.md", result.stderr)
        self.assertIn("skills/README.md: skill root may contain directories only", result.stderr)

    def test_all_hook_copies_share_the_current_contract(self) -> None:
        for host_dir, source_hook in ALL_METADATA_HOOKS:
            with self.subTest(source_hook=source_hook):
                repo = self.make_repo()
                hook_dir = repo / host_dir / "hooks"
                hook_dir.mkdir(parents=True, exist_ok=True)
                hook = hook_dir / "skill_metadata_check.py"
                shutil.copy2(source_hook, hook)
                self.write_skill(repo, "demo", skill_text("demo", invocation=False))
                standard = subprocess.run(
                    [sys.executable, "-B", str(hook)],
                    cwd=repo,
                    capture_output=True,
                    text=True,
                    encoding="utf-8",
                    check=False,
                )
                self.assertEqual(standard.returncode, 0, standard.stderr)

                incomplete = skill_text("demo").replace("argument: 无\n", "")
                self.write_skill(repo, "demo", incomplete)
                incomplete_result = subprocess.run(
                    [sys.executable, "-B", str(hook)],
                    cwd=repo,
                    capture_output=True,
                    text=True,
                    encoding="utf-8",
                    check=False,
                )
                self.assertEqual(incomplete_result.returncode, 2, incomplete_result.stderr)
                self.assertIn("invocation metadata requires argument", incomplete_result.stderr)

    def test_long_description_entry_and_dead_reference_fail(self) -> None:
        repo = self.make_repo()
        body = "[missing](references/missing.md)\n" + "line\n" * 121
        self.write_skill(repo, "demo", skill_text("demo", "x" * 501, body))
        result = self.run_hook(repo)
        self.assertEqual(result.returncode, 2)
        self.assertIn("description exceeds", result.stderr)
        self.assertIn("entry exceeds 120 lines", result.stderr)
        self.assertIn("dead markdown reference", result.stderr)

    def test_entry_responsibility_budget_blocks_ninth_h2_section(self) -> None:
        repo = self.make_repo()
        body = "\n".join(f"## Section {index}\nstep" for index in range(9))
        self.write_skill(repo, "demo", skill_text("demo", body=body))

        result = self.run_hook(repo)

        self.assertEqual(result.returncode, 2)
        self.assertIn("exceeds 8 H2 sections", result.stderr)

    def test_retired_runtime_asset_is_blocked_in_entry_and_reference(self) -> None:
        repo = self.make_repo()
        folder = repo / "skills" / "demo"
        references = folder / "references"
        references.mkdir(parents=True)
        (folder / "SKILL.md").write_text(
            skill_text(
                "demo",
                body=(
                    "运行 focus_reminder.py。\n\n"
                    "命中迁移时读取 [迁移](references/adopt.md)。\n"
                ),
            ),
            encoding="utf-8",
        )
        (references / "adopt.md").write_text(
            "运行 skill-routing.json。\n",
            encoding="utf-8",
        )

        result = self.run_hook(repo)

        self.assertEqual(result.returncode, 2)
        self.assertIn("focus_reminder.py", result.stderr)
        self.assertIn("references/adopt.md", result.stderr)
        self.assertIn("skill-routing.json", result.stderr)

    def test_entry_must_not_duplicate_long_reference_paragraph(self) -> None:
        repo = self.make_repo()
        folder = repo / "skills" / "demo"
        references = folder / "references"
        references.mkdir(parents=True)
        paragraph = (
            "这是一个足够长的规则段落，用于证明入口和按需参考文件不能同时复制同一事实源。"
            "入口只负责说明何时读取参考文件，详细规则必须只保留在唯一 owner 中。"
            "任何需要补充的异常处理都应继续写入该 owner，禁止回到入口再次手写一份。"
        )
        (folder / "SKILL.md").write_text(
            skill_text(
                "demo",
                body=f"[详细规则](references/deep.md)\n\n{paragraph}\n",
            ),
            encoding="utf-8",
        )
        (references / "deep.md").write_text(paragraph + "\n", encoding="utf-8")

        result = self.run_hook(repo)

        self.assertEqual(result.returncode, 2)
        self.assertIn("entry duplicates 1 long paragraph", result.stderr)

    def test_project_links_and_placeholders_are_not_packaged_references(self) -> None:
        repo = self.make_repo()
        body = (
            "[TODO](doc/0_architecture/TODO-INDEX.md)\n"
            "[memory](<agent-dir>/memory/MEMORY.md)\n"
        )
        self.write_skill(repo, "demo", skill_text("demo", body=body))
        self.assertEqual(self.run_hook(repo).returncode, 0)

    def test_orphan_reference_fails_until_entry_routes_to_it(self) -> None:
        repo = self.make_repo()
        folder = repo / "skills" / "demo"
        references = folder / "references"
        references.mkdir(parents=True)
        (folder / "SKILL.md").write_text(skill_text("demo"), encoding="utf-8")
        (references / "deep.md").write_text("低频步骤。\n", encoding="utf-8")

        result = self.run_hook(repo)
        self.assertEqual(result.returncode, 2)
        self.assertIn("orphan markdown reference: references/deep.md", result.stderr)

        (folder / "SKILL.md").write_text(
            skill_text(
                "demo",
                body="命中低频分支时读取 [深档](references/deep.md)。\n",
            ),
            encoding="utf-8",
        )
        self.assertEqual(self.run_hook(repo).returncode, 0)

    def test_agent_role_contract_accepts_default_main_and_named_delegate(self) -> None:
        repo = self.make_repo()
        self.write_skill(repo, "main-only", skill_text("main-only"))
        self.write_skill(
            repo,
            "delegated",
            skill_text(
                "delegated",
                body="把只读调查显式分派给 `light-explorer`。\n",
            ),
        )
        self.assertEqual(self.run_hook(repo).returncode, 0)

    def test_agent_role_contract_rejects_unknown_and_generic_delegate(self) -> None:
        repo = self.make_repo()
        self.write_skill(
            repo,
            "unknown",
            skill_text(
                "unknown",
                body="把实现显式分派给 `missing-worker`。\n",
            ),
        )
        self.write_skill(
            repo,
            "generic",
            skill_text(
                "generic",
                body="旧项目必须先由独立 agent 审计。\n",
            ),
        )
        result = self.run_hook(repo)
        self.assertEqual(result.returncode, 2)
        self.assertIn("unknown Agent role 'missing-worker'", result.stderr)
        self.assertIn("generic Agent label", result.stderr)

    def test_agent_role_contract_ignores_prohibited_delegation(self) -> None:
        repo = self.make_repo()
        self.write_skill(
            repo,
            "main-only",
            skill_text(
                "main-only",
                body="禁止把脚本执行委派给其他 agent。\n",
            ),
        )
        self.assertEqual(self.run_hook(repo).returncode, 0)

    def test_agent_role_contract_scans_linked_references(self) -> None:
        repo = self.make_repo()
        folder = repo / "skills" / "delegated"
        references = folder / "references"
        references.mkdir(parents=True)
        (folder / "SKILL.md").write_text(
            skill_text(
                "delegated",
                body="命中迁移时读取 [迁移](references/adopt.md)。\n",
            ),
            encoding="utf-8",
        )
        reference = references / "adopt.md"
        reference.write_text(
            "把只读审计显式分派给 `light-explorer`。\n",
            encoding="utf-8",
        )
        self.assertEqual(self.run_hook(repo).returncode, 0)

        reference.write_text(
            "把只读审计显式分派给 `missing-worker`。\n",
            encoding="utf-8",
        )
        result = self.run_hook(repo)
        self.assertEqual(result.returncode, 2)
        self.assertIn("references/adopt.md", result.stderr)
        self.assertIn("unknown Agent role 'missing-worker'", result.stderr)

    def test_catalog_description_budget_fails(self) -> None:
        repo = self.make_repo()
        for index in range(9):
            name = f"demo-{index}"
            self.write_skill(repo, name, skill_text(name, "x" * 450))
        result = self.run_hook(repo)
        self.assertEqual(result.returncode, 2)
        self.assertIn("skill catalog descriptions exceed 4000", result.stderr)

    def test_public_agents_define_progressive_skill_disclosure(self) -> None:
        required = (
            "简单 Skill 必须保持单文件",
            "入口只保留共同目标、主路径、选择点、停止条件",
            "明确的 reference 读取条件",
            "入口与 reference 禁止手写复制同一规则",
        )
        for path in (ROOT / "templates" / "AGENTS.md", ROOT / "AGENTS.md"):
            with self.subTest(path=path):
                text = path.read_text(encoding="utf-8")
                for phrase in required:
                    self.assertIn(phrase, text)

if __name__ == "__main__":
    unittest.main()
