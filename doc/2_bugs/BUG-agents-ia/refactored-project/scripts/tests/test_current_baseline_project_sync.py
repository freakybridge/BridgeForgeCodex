from __future__ import annotations

import importlib.util
import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock


ROOT = Path(__file__).resolve().parents[2]
CURRENT_VERSION = (ROOT / "VERSION").read_text(encoding="utf-8-sig").strip()
LEGACY_VERSION = "1.4.30"
CLEAN_BASELINE_VERSION = "1.4.31"


def load_module(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


SYNC = load_module(
    "bridgeforge_current_project_sync",
    ROOT / "scripts" / "bridgeforge_codex_project_sync.py",
)
BASELINE = load_module(
    "bridgeforge_current_baseline",
    ROOT / "templates" / "scripts" / "current_baseline.py",
)


class CurrentBaselineContractTests(unittest.TestCase):
    def _assert_isolated_gitattributes_validation(self, module) -> None:
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            sentinel = root / "sentinel"
            sentinel.mkdir()
            subprocess.run(["git", "init", "-q"], cwd=sentinel, check=True)
            git_dir = sentinel / ".git"
            quarantine = sentinel / "quarantine"
            quarantine.mkdir()
            config_before = (git_dir / "config").read_bytes()
            injected = {
                "GIT_DIR": str(git_dir),
                "GIT_WORK_TREE": str(sentinel),
                "GIT_INDEX_FILE": str(git_dir / "sentinel-index"),
                "GIT_COMMON_DIR": str(git_dir),
                "GIT_OBJECT_DIRECTORY": str(git_dir / "objects"),
                "GIT_ALTERNATE_OBJECT_DIRECTORIES": str(git_dir / "objects"),
                "GIT_QUARANTINE_PATH": str(quarantine),
                "GIT_CONFIG_COUNT": "1",
                "GIT_CONFIG_KEY_0": "core.bare",
                "GIT_CONFIG_VALUE_0": "true",
                "GIT_CONFIG_PARAMETERS": "invalid-dynamic-config",
            }

            with mock.patch.dict(os.environ, injected, clear=False):
                isolated = module._isolated_git_environment()
                isolated_keys = {key.upper() for key in isolated}
                local_names = subprocess.run(
                    ["git", "rev-parse", "--local-env-vars"],
                    capture_output=True,
                    text=True,
                    encoding="utf-8",
                    errors="replace",
                    check=True,
                ).stdout.splitlines()
                self.assertTrue(local_names)
                self.assertTrue(all(name not in isolated_keys for name in local_names))
                self.assertNotIn("GIT_QUARANTINE_PATH", isolated_keys)
                self.assertNotIn("GIT_CONFIG_COUNT", isolated_keys)
                self.assertFalse(any(
                    key.startswith(("GIT_CONFIG_KEY_", "GIT_CONFIG_VALUE_"))
                    for key in isolated_keys
                ))
                self.assertEqual(isolated["GIT_ATTR_NOSYSTEM"], "1")
                self.assertEqual(
                    module._gitattributes_default_state(b"* text=auto eol=lf\n"),
                    ("auto", "lf"),
                )

            self.assertEqual((git_dir / "config").read_bytes(), config_before)
            self.assertFalse((git_dir / "sentinel-index").exists())

    def test_current_baseline_gitattributes_git_environment_is_isolated(self) -> None:
        self._assert_isolated_gitattributes_validation(BASELINE)

    def test_project_sync_gitattributes_git_environment_is_isolated(self) -> None:
        self._assert_isolated_gitattributes_validation(SYNC)

    def test_real_precommit_preserves_common_config_in_checkout_and_worktree(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            project = root / "main"
            project.mkdir()
            subprocess.run(["git", "init", "-q", "-b", "main"], cwd=project, check=True)
            subprocess.run(
                ["git", "config", "user.name", "BridgeForge Test"],
                cwd=project,
                check=True,
            )
            subprocess.run(
                ["git", "config", "user.email", "bridgeforge@example.invalid"],
                cwd=project,
                check=True,
            )
            plan = SYNC.build_plan(project, ROOT, "init")
            self.assertFalse(plan.blockers)
            SYNC.apply_plan(plan, plan_fingerprint=plan.aggregate_fingerprint)
            subprocess.run(
                [sys.executable, "-m", "venv", "--without-pip", str(project / ".venv")],
                cwd=project,
                check=True,
            )
            hook = project / ".githooks" / "pre-commit"
            hook.write_text(
                hook.read_text(encoding="utf-8").replace(
                    "# >>> PROJECT_EXTENSION_BEGIN\n",
                    "# >>> PROJECT_EXTENSION_BEGIN\n"
                    "[ -n \"$GIT_DIR\" ] || { echo missing-GIT_DIR >&2; exit 2; }\n"
                    "mkdir -p .runtime\n"
                    "printf 'seen\\n' >> .runtime/precommit-env-seen.txt\n",
                    1,
                ),
                encoding="utf-8",
                newline="\n",
            )
            (project / ".gitignore").write_text(
                ".venv/\n.runtime/\n__pycache__/\n*.pyc\n", encoding="utf-8"
            )
            (project / "business.txt").write_text("one\n", encoding="utf-8")
            subprocess.run(
                ["git", "config", "core.hooksPath", ".githooks"],
                cwd=project,
                check=True,
            )
            common_config = project / ".git" / "config"
            config_before = common_config.read_bytes()
            subprocess.run(["git", "add", "-A"], cwd=project, check=True)
            commit_environment = {
                **os.environ,
                "GIT_DIR": str(project / ".git"),
                "GIT_COMMON_DIR": str(project / ".git"),
                "GIT_WORK_TREE": str(project),
                "GIT_INDEX_FILE": str(project / ".git" / "index"),
            }
            committed = subprocess.run(
                ["git", "commit", "-qm", "baseline"],
                cwd=project,
                env=commit_environment,
                text=True,
                encoding="utf-8",
                errors="replace",
                capture_output=True,
                check=False,
            )
            self.assertEqual(
                committed.returncode,
                0,
                committed.stdout + committed.stderr,
            )
            self.assertEqual(common_config.read_bytes(), config_before)
            self.assertEqual(
                (project / ".runtime" / "precommit-env-seen.txt").read_text(
                    encoding="utf-8"
                ),
                "seen\n",
            )

            subprocess.run(["git", "branch", "linked"], cwd=project, check=True)
            linked = root / "linked"
            subprocess.run(
                ["git", "worktree", "add", "-q", str(linked), "linked"],
                cwd=project,
                check=True,
            )
            subprocess.run(
                [sys.executable, "-m", "venv", "--without-pip", str(linked / ".venv")],
                cwd=linked,
                check=True,
            )
            linked_config_before = common_config.read_bytes()
            (linked / "business.txt").write_text("two\n", encoding="utf-8")
            subprocess.run(["git", "add", "business.txt"], cwd=linked, check=True)
            linked_git_dir = Path(
                subprocess.run(
                    ["git", "rev-parse", "--absolute-git-dir"],
                    cwd=linked,
                    text=True,
                    capture_output=True,
                    check=True,
                ).stdout.strip()
            )
            linked_commit_environment = {
                **os.environ,
                "GIT_DIR": str(linked_git_dir),
                "GIT_COMMON_DIR": str(project / ".git"),
                "GIT_WORK_TREE": str(linked),
                "GIT_INDEX_FILE": str(linked_git_dir / "index"),
            }
            linked_commit = subprocess.run(
                ["git", "commit", "-qm", "linked change"],
                cwd=linked,
                env=linked_commit_environment,
                text=True,
                encoding="utf-8",
                errors="replace",
                capture_output=True,
                check=False,
            )
            self.assertEqual(
                linked_commit.returncode,
                0,
                linked_commit.stdout + linked_commit.stderr,
            )
            self.assertEqual(common_config.read_bytes(), linked_config_before)
            self.assertEqual(
                subprocess.run(
                    ["git", "config", "--bool", "--get", "core.bare"],
                    cwd=linked,
                    text=True,
                    capture_output=True,
                    check=True,
                ).stdout.strip(),
                "false",
            )
            self.assertEqual(
                subprocess.run(
                    ["git", "status", "--porcelain=v1"],
                    cwd=project,
                    text=True,
                    capture_output=True,
                    check=True,
                ).stdout,
                "",
            )
            self.assertEqual(
                subprocess.run(
                    ["git", "status", "--porcelain=v1"],
                    cwd=linked,
                    text=True,
                    capture_output=True,
                    check=True,
                ).stdout,
                "",
            )
            self.assertEqual(
                (linked / ".runtime" / "precommit-env-seen.txt").read_text(
                    encoding="utf-8"
                ),
                "seen\n",
            )

    def test_gitattributes_merge_adds_default_lf_and_preserves_exceptions(self) -> None:
        source = b"* text=auto eol=lf\n"
        current = b"*.bat text eol=crlf\r\n.githooks/** text eol=lf\r\n"

        merged = SYNC._merge_gitattributes_default_lf(source, current)

        self.assertEqual(
            merged,
            b"* text=auto eol=lf\r\n" + current,
        )
        self.assertEqual(SYNC._gitattributes_default_state(merged), ("auto", "lf"))

    def test_gitattributes_merge_blocks_project_wide_conflict(self) -> None:
        conflicts = (
            b"* text eol=crlf\r\n",
            b"** text eol=crlf\r\n",
            b"[attr]windows text eol=crlf\r\n* windows\r\n",
        )
        for current in conflicts:
            with self.subTest(current=current), self.assertRaisesRegex(
                SYNC.SyncBlocked,
                "conflict",
            ):
                SYNC._merge_gitattributes_default_lf(
                    b"* text=auto eol=lf\n",
                    current,
                )
            with self.subTest(baseline=current):
                self.assertNotEqual(
                    BASELINE._gitattributes_default_state(
                        b"* text=auto eol=lf\n" + current
                    ),
                    ("auto", "lf"),
                )

    def test_transaction_identical_write_is_a_real_noop(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            target = root / "managed.txt"
            target.write_bytes(b"unchanged\n")
            transaction = SYNC._Transaction(root)

            with mock.patch.object(SYNC, "_atomic_write") as atomic_write:
                transaction.write(target, b"unchanged\n")

            atomic_write.assert_not_called()
            self.assertEqual(transaction.before, {})
            self.assertEqual(target.read_bytes(), b"unchanged\n")

    def test_contract_is_small_and_contains_no_history_model(self) -> None:
        path = ROOT / "templates" / "managed-skeleton.json"
        contract = json.loads(path.read_text(encoding="utf-8"))
        self.assertEqual(contract["schema_version"], 3)
        self.assertEqual(contract["release_version"], CURRENT_VERSION)
        self.assertEqual(contract["baseline_model"], "current-only")
        self.assertEqual(BASELINE.MINIMUM_CURRENT_BASELINE, (1, 4, 31))
        self.assertNotIn("minimum_supported_version", contract)
        text = path.read_text(encoding="utf-8")
        self.assertLessEqual(len(text.splitlines()), 2148)
        for token in (
            "historical_sha256",
            "trusted_legacy_sha256",
            "retired_sections",
            "retirement_guidance",
            "rule_index_check.py",
            "rule_size_check.py",
        ):
            self.assertNotIn(token, text)

    def test_core_sync_code_meets_reduction_gate(self) -> None:
        paths = (
            ROOT / "scripts" / "bridgeforge_codex_project_sync.py",
            ROOT / "templates" / "scripts" / "version_release.py",
        )
        lines = sum(
            len(path.read_text(encoding="utf-8").splitlines()) for path in paths
        )
        self.assertLessEqual(lines, 6912)

    def test_contract_rejects_unknown_fields_and_source_escape(self) -> None:
        contract = json.loads(
            (ROOT / "templates" / "managed-skeleton.json").read_text(
                encoding="utf-8"
            )
        )
        with tempfile.TemporaryDirectory() as raw:
            path = Path(raw) / "contract.json"
            contract["foo_by_version"] = {}
            path.write_text(json.dumps(contract), encoding="utf-8")
            with self.assertRaises(BASELINE.BaselineError):
                BASELINE.load_contract(path)
            contract.pop("foo_by_version")
            contract["assets"][0]["source"] = "../escape"
            path.write_text(json.dumps(contract), encoding="utf-8")
            with self.assertRaisesRegex(BASELINE.BaselineError, "escapes"):
                BASELINE.load_contract(path)


class CurrentProjectSyncTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.project = Path(self.temporary.name)
        self.template_temporary = tempfile.TemporaryDirectory()
        self.template_base = Path(self.template_temporary.name)

    def tearDown(self) -> None:
        self.temporary.cleanup()
        self.template_temporary.cleanup()

    def apply(self, plan, **kwargs):
        return SYNC.apply_plan(
            plan,
            plan_fingerprint=plan.aggregate_fingerprint,
            **kwargs,
        )

    def snapshot_tree(self) -> tuple[tuple[str, ...], dict[str, bytes]]:
        directories = tuple(sorted(
            path.relative_to(self.project).as_posix()
            for path in self.project.rglob("*")
            if path.is_dir()
        ))
        files = {
            path.relative_to(self.project).as_posix(): path.read_bytes()
            for path in self.project.rglob("*")
            if path.is_file()
        }
        return directories, files

    def write_project_hook_bundle(self, name: str = "project_risk") -> Path:
        bundle = self.project / ".codex" / "hooks" / name
        bundle.mkdir(parents=True)
        (bundle / "entrypoint.py").write_text(
            "from helper import run\nrun()\n",
            encoding="utf-8",
        )
        (bundle / "helper.py").write_text(
            "def run():\n    return None\n",
            encoding="utf-8",
        )
        hooks = self.project / ".codex" / "hooks.json"
        hooks.write_text(
            json.dumps({
                "hooks": {
                    "SessionStart": [{
                        "matcher": "",
                        "hooks": [{
                            "type": "command",
                            "command": (
                                f".venv/Scripts/python.exe "
                                f".codex/hooks/{name}/entrypoint.py"
                            ),
                        }],
                    }],
                },
            }),
            encoding="utf-8",
        )
        return bundle

    def template_with_removable_asset(
        self,
        name: str,
        strategy: str,
    ) -> tuple[Path, Path, str]:
        installed = self.template_base / f"{name}-installed"
        incoming = self.template_base / f"{name}-incoming"
        shutil.copytree(ROOT / "templates", installed / "templates")
        shutil.copy2(ROOT / "VERSION", installed / "VERSION")
        source = installed / "templates" / "hooks" / "phase2_removed.py"
        asset: dict[str, object] = {
            "id": f"codex.test.phase2-removed-{name}",
            "source": "templates/hooks/phase2_removed.py",
            "target": ".codex/hooks/phase2_removed.py",
            "strategy": strategy,
        }
        if strategy == "merge":
            payload = b'{"managed": true}\n'
            asset["merge_validation"] = {
                "format": "json-subset-current-v1",
                "required": {"managed": True},
            }
        elif strategy == "region":
            payload = b"# BEGIN PHASE2\nmanaged\n# END PHASE2\n"
            asset["region"] = {
                "begin": "# BEGIN PHASE2",
                "end": "# END PHASE2",
                "current_sha256": SYNC._sha256_bytes(payload),
            }
        else:
            payload = b"print('managed phase2 asset')\n"
        source.write_bytes(payload)
        asset["current_sha256"] = SYNC._sha256_bytes(payload)
        contract_path = installed / "templates" / "managed-skeleton.json"
        contract = json.loads(contract_path.read_text(encoding="utf-8"))
        contract["assets"].append(asset)
        contract_path.write_text(
            json.dumps(contract, ensure_ascii=False, indent=2) + "\n",
            encoding="utf-8",
        )
        shutil.copytree(installed, incoming)
        incoming_contract_path = incoming / "templates" / "managed-skeleton.json"
        incoming_contract = json.loads(
            incoming_contract_path.read_text(encoding="utf-8")
        )
        incoming_contract["assets"] = [
            item
            for item in incoming_contract["assets"]
            if item["id"] != asset["id"]
        ]
        incoming_contract_path.write_text(
            json.dumps(incoming_contract, ensure_ascii=False, indent=2) + "\n",
            encoding="utf-8",
        )
        return installed, incoming, str(asset["target"])

    def template_at_version(self, version: str) -> Path:
        template = self.template_base / f"baseline-{version}"
        shutil.copytree(ROOT / "templates", template / "templates")
        (template / "VERSION").write_text(version + "\n", encoding="utf-8")
        contract_path = template / "templates" / "managed-skeleton.json"
        contract = json.loads(contract_path.read_text(encoding="utf-8"))
        contract["release_version"] = version
        contract_path.write_text(
            json.dumps(contract, ensure_ascii=False, indent=2) + "\n",
            encoding="utf-8",
        )
        return template

    def template_without_asset(self, asset_id: str) -> Path:
        template = self.template_base / f"without-{asset_id.replace('.', '-')}"
        shutil.copytree(ROOT / "templates", template / "templates")
        shutil.copy2(ROOT / "VERSION", template / "VERSION")
        contract_path = template / "templates" / "managed-skeleton.json"
        contract = json.loads(contract_path.read_text(encoding="utf-8"))
        contract["assets"] = [
            item for item in contract["assets"] if item["id"] != asset_id
        ]
        contract_path.write_text(
            json.dumps(contract, ensure_ascii=False, indent=2) + "\n",
            encoding="utf-8",
        )
        return template

    def template_with_whole_strategy_transition(
        self,
        strategy: str,
    ) -> tuple[Path, Path, str]:
        installed = self.template_base / f"{strategy}-to-whole-installed"
        incoming = self.template_base / f"{strategy}-to-whole-incoming"
        shutil.copytree(ROOT / "templates", installed / "templates")
        shutil.copy2(ROOT / "VERSION", installed / "VERSION")
        relative = f".codex/hooks/{strategy}_to_whole.txt"
        source_relative = f"templates/hooks/{strategy}_to_whole.txt"
        source = installed / source_relative
        asset: dict[str, object] = {
            "id": f"codex.test.{strategy}-to-whole",
            "source": source_relative,
            "target": relative,
            "strategy": strategy,
        }
        if strategy == "merge":
            payload = b'{"managed": true}\n'
            asset["merge_validation"] = {
                "format": "json-subset-current-v1",
                "required": {"managed": True},
            }
        elif strategy == "region":
            payload = b"# BEGIN TRANSITION\nmanaged\n# END TRANSITION\n"
            asset["region"] = {
                "begin": "# BEGIN TRANSITION",
                "end": "# END TRANSITION",
                "current_sha256": SYNC._sha256_bytes(payload),
            }
        elif strategy == "seed":
            payload = b"managed seed\n"
        else:
            raise AssertionError(f"unsupported transition strategy: {strategy}")
        source.write_bytes(payload)
        asset["current_sha256"] = SYNC._sha256_bytes(payload)
        contract_path = installed / "templates" / "managed-skeleton.json"
        contract = json.loads(contract_path.read_text(encoding="utf-8"))
        contract["assets"].append(asset)
        contract_path.write_text(
            json.dumps(contract, ensure_ascii=False, indent=2) + "\n",
            encoding="utf-8",
        )

        shutil.copytree(installed, incoming)
        incoming_source = incoming / source_relative
        incoming_payload = b"official whole replacement\n"
        incoming_source.write_bytes(incoming_payload)
        incoming_contract_path = incoming / "templates" / "managed-skeleton.json"
        incoming_contract = json.loads(
            incoming_contract_path.read_text(encoding="utf-8")
        )
        incoming_asset = next(
            item for item in incoming_contract["assets"]
            if item["id"] == asset["id"]
        )
        incoming_asset["strategy"] = "whole"
        incoming_asset["current_sha256"] = SYNC._sha256_bytes(incoming_payload)
        incoming_asset.pop("merge_validation", None)
        incoming_asset.pop("region", None)
        incoming_contract_path.write_text(
            json.dumps(incoming_contract, ensure_ascii=False, indent=2) + "\n",
            encoding="utf-8",
        )
        return installed, incoming, relative

    def test_init_installs_a_verified_current_baseline(self) -> None:
        plan = SYNC.build_plan(self.project, ROOT, "init")
        self.assertFalse(plan.blockers)
        receipt = self.apply(plan)
        self.assertEqual(receipt.status, "completed")
        self.assertTrue(receipt.stamp_written_last)
        report = BASELINE.verify_current_baseline(
            self.project,
            expected_version=CURRENT_VERSION,
        )
        self.assertEqual(report.version, CURRENT_VERSION)
        self.assertEqual(
            (self.project / ".codex" / ".bridgeforge_codex_version")
            .read_text(encoding="utf-8")
            .strip(),
            CURRENT_VERSION,
        )

    def test_explicit_init_rejects_existing_unstamped_skeleton(self) -> None:
        (self.project / ".codex").mkdir()
        plan = SYNC.build_plan(self.project, ROOT, "init")
        self.assertIn("no existing skeleton identity", " ".join(plan.blockers))
        with self.assertRaisesRegex(SYNC.SyncBlocked, "blockers"):
            self.apply(plan)

    def test_old_stamp_routes_to_confirmed_rebuild_and_preserves_manifest(self) -> None:
        codex = self.project / ".codex"
        (codex / "hooks").mkdir(parents=True)
        (codex / "rules").mkdir()
        (codex / "skills" / "project-skill").mkdir(parents=True)
        old_stamp = codex / ".bridgeforge_version"
        old_stamp.write_text(LEGACY_VERSION + "\n", encoding="utf-8")
        project_hook = self.write_project_hook_bundle("project_only")
        project_rule = codex / "rules" / "project_only.md"
        project_skill = codex / "skills" / "project-skill" / "SKILL.md"
        project_rule.write_text("# project rule\n", encoding="utf-8")
        project_skill.write_text(
            "---\nname: project-skill\ndescription: project semantics\n---\n\n# Project Skill\n",
            encoding="utf-8",
        )
        memory = codex / "memory" / "engineering" / "project.md"
        memory.parent.mkdir(parents=True)
        memory.write_text(
            "---\ncategory: engineering\nstatus: active\n"
            "description: project memory semantics\n---\n\n# Project\n",
            encoding="utf-8",
        )
        agents = (ROOT / "templates" / "AGENTS.md").read_text(encoding="utf-8")
        agents = agents.replace(
            "> 本区由项目完全所有。",
            "> 本区由项目完全所有。\n\nPROJECT-ZONE-SENTINEL",
            1,
        )
        (self.project / "AGENTS.md").write_text(agents, encoding="utf-8")
        doc_readme = (ROOT / "templates" / "doc" / "README.md").read_text(
            encoding="utf-8"
        )
        managed_row = (
            "| [`M1/feature_x/`](1_delivery/M1/feature_x/) | "
            "milestone：按里程碑组织的需求包 |\n"
        )
        project_row = (
            "| [`project_topic/`](1_delivery/project_topic/) | "
            "PROJECT-DOC-INDEX-SENTINEL |\n"
        )
        self.assertIn(managed_row, doc_readme)
        legacy_managed_row = managed_row.replace(
            "milestone：按里程碑组织的需求包",
            "OLD-MANAGED-DESCRIPTION",
        )
        project_heading = (
            "\n## 项目文档入口\n\n"
            "PROJECT-DOC-HEADING-SENTINEL\n"
        )
        doc_readme = doc_readme.replace(
            managed_row,
            legacy_managed_row + project_row,
            1,
        ) + project_heading
        doc_readme_path = self.project / "doc" / "README.md"
        doc_readme_path.parent.mkdir()
        doc_readme_path.write_text(doc_readme, encoding="utf-8")
        skill_before = project_skill.read_bytes()
        memory_before = memory.read_bytes()

        plan = SYNC.build_plan(self.project, ROOT, "auto")
        self.assertEqual(plan.mode, "rebuild")
        self.assertEqual(plan.previous_version, LEGACY_VERSION)
        with self.assertRaisesRegex(
            SYNC.SyncBlocked,
            "confirmed-preservation-manifest",
        ):
            self.apply(plan)
        self.assertEqual(
            old_stamp.read_text(encoding="utf-8").strip(),
            LEGACY_VERSION,
        )

        preserve = tuple(
            item["id"]
            for item in plan.preservation_entries
            if item.get("target")
            in {
                "AGENTS.md",
                ".codex/hooks/project_only",
                ".codex/rules/project_only.md",
            }
        )
        delete = tuple(
            item["id"]
            for item in plan.preservation_entries
            if item.get("disposition") == "user-decision"
            and item["id"] not in preserve
        )
        checkpoints: list[str] = []

        def observe_stamp_order(label: str) -> None:
            checkpoints.append(label)
            if label == "after-preservation-manifest-clear":
                self.assertFalse(
                    (codex / ".bridgeforge_codex_version").exists()
                )

        receipt = self.apply(
            plan,
            confirmed_preservation_manifest=True,
            confirmed_risk=True,
            preserved_project_asset_ids=preserve,
            deleted_project_asset_ids=delete,
            checkpoint=observe_stamp_order,
        )
        self.assertEqual(receipt.mode, "rebuild")
        self.assertFalse(old_stamp.exists())
        self.assertTrue((project_hook / "entrypoint.py").is_file())
        self.assertTrue((project_hook / "helper.py").is_file())
        self.assertTrue(project_rule.is_file())
        self.assertEqual(project_skill.read_bytes(), skill_before)
        self.assertEqual(memory.read_bytes(), memory_before)
        self.assertIn("after-preservation-manifest-clear", checkpoints)
        self.assertIn(
            "PROJECT-ZONE-SENTINEL",
            (self.project / "AGENTS.md").read_text(encoding="utf-8"),
        )
        self.assertIn(
            project_row.strip(),
            doc_readme_path.read_text(encoding="utf-8"),
        )
        rebuilt_readme = doc_readme_path.read_text(encoding="utf-8")
        self.assertIn(project_heading.strip(), rebuilt_readme)
        self.assertIn(managed_row.strip(), rebuilt_readme)
        self.assertNotIn("OLD-MANAGED-DESCRIPTION", rebuilt_readme)
        BASELINE.verify_current_baseline(self.project)

    def test_old_rebuild_rejects_ambiguous_managed_markdown_without_writes(self) -> None:
        codex = self.project / ".codex"
        codex.mkdir()
        (codex / ".bridgeforge_version").write_text(
            LEGACY_VERSION + "\n",
            encoding="utf-8",
        )
        doc_readme = (ROOT / "templates" / "doc" / "README.md").read_text(
            encoding="utf-8"
        )
        doc_readme += "\n## 1_delivery/\n\n| duplicate | table |\n|---|---|\n"
        doc_readme_path = self.project / "doc" / "README.md"
        doc_readme_path.parent.mkdir()
        doc_readme_path.write_text(doc_readme, encoding="utf-8")
        before = self.snapshot_tree()

        with self.assertRaisesRegex(
            SYNC.SyncBlocked,
            "managed block ownership is ambiguous",
        ):
            SYNC.build_plan(self.project, ROOT, "auto")

        self.assertEqual(self.snapshot_tree(), before)

    def test_current_update_is_idempotent(self) -> None:
        self.apply(SYNC.build_plan(self.project, ROOT, "init"))
        plan = SYNC.build_plan(self.project, ROOT, "update")
        self.assertFalse(plan.blockers)
        self.assertEqual(plan.mode, "update")
        self.assertEqual(plan.previous_version, CURRENT_VERSION)
        self.assertEqual(plan.actions, [])
        receipt = self.apply(plan)
        self.assertEqual(receipt.applied, ())

    def test_new_whole_asset_collision_requires_confirmed_risk(self) -> None:
        asset_id = "codex.doc.hook-signals"
        baseline = self.template_without_asset(asset_id)
        self.apply(SYNC.build_plan(self.project, baseline, "init"))
        target = self.project / "doc" / "3_reference" / "codex-hook-signals.md"
        target.parent.mkdir(parents=True, exist_ok=True)
        original = b"project-owned hook notes\n"
        target.write_bytes(original)

        plan = SYNC.build_plan(self.project, ROOT, "update")
        self.assertFalse(plan.blockers)
        risks = [item for item in plan.risk_actions if item.asset_id == asset_id]
        self.assertEqual(len(risks), 1)
        self.assertEqual(risks[0].target, target.relative_to(self.project).as_posix())
        self.assertEqual(risks[0].before_sha256, SYNC._sha256_bytes(original))
        self.assertTrue(SYNC._plan_payload(plan)["confirmation_required"])
        before = self.snapshot_tree()

        with self.assertRaisesRegex(SYNC.SyncBlocked, "confirmed-risk"):
            self.apply(plan)
        self.assertEqual(self.snapshot_tree(), before)

        receipt = self.apply(plan, confirmed_risk=True)
        self.assertTrue(receipt.stamp_written_last)
        self.assertEqual(
            target.read_bytes(),
            (ROOT / "templates" / target.relative_to(self.project)).read_bytes(),
        )
        repeated = SYNC.build_plan(self.project, ROOT, "update")
        self.assertFalse(repeated.blockers)
        self.assertEqual(repeated.actions, [])

    def test_partial_ownership_to_whole_requires_confirmed_risk(self) -> None:
        for strategy in ("merge", "region", "seed"):
            with self.subTest(strategy=strategy), tempfile.TemporaryDirectory() as raw:
                project = Path(raw)
                installed, incoming, relative = (
                    self.template_with_whole_strategy_transition(strategy)
                )
                installed_plan = SYNC.build_plan(project, installed, "init")
                SYNC.apply_plan(
                    installed_plan,
                    plan_fingerprint=installed_plan.aggregate_fingerprint,
                )
                target = project / relative
                if strategy == "merge":
                    payload = json.loads(target.read_text(encoding="utf-8"))
                    payload["project_owned"] = True
                    target.write_text(
                        json.dumps(payload, ensure_ascii=False, indent=2) + "\n",
                        encoding="utf-8",
                    )
                elif strategy == "region":
                    target.write_bytes(
                        b"project prefix\n" + target.read_bytes() + b"project suffix\n"
                    )
                else:
                    target.write_bytes(target.read_bytes() + b"project-owned seed data\n")
                before = target.read_bytes()

                plan = SYNC.build_plan(project, incoming, "update")
                self.assertFalse(plan.blockers)
                risks = [
                    item for item in plan.risk_actions
                    if item.target == relative
                ]
                self.assertEqual(len(risks), 1)
                self.assertTrue(SYNC._plan_payload(plan)["confirmation_required"])
                with self.assertRaisesRegex(SYNC.SyncBlocked, "confirmed-risk"):
                    SYNC.apply_plan(
                        plan,
                        plan_fingerprint=plan.aggregate_fingerprint,
                    )
                self.assertEqual(target.read_bytes(), before)

                receipt = SYNC.apply_plan(
                    plan,
                    plan_fingerprint=plan.aggregate_fingerprint,
                    confirmed_risk=True,
                )
                self.assertTrue(receipt.stamp_written_last)
                self.assertEqual(
                    target.read_bytes(),
                    (incoming / "templates" / "hooks" / f"{strategy}_to_whole.txt")
                    .read_bytes(),
                )
                self.assertEqual(
                    SYNC.build_plan(project, incoming, "update").actions,
                    [],
                )

    def test_current_only_risk_cli_requires_confirmation(self) -> None:
        asset_id = "codex.doc.hook-signals"
        baseline = self.template_without_asset(asset_id)
        self.apply(SYNC.build_plan(self.project, baseline, "init"))
        target = self.project / "doc" / "3_reference" / "codex-hook-signals.md"
        target.parent.mkdir(parents=True, exist_ok=True)
        original = b"project-owned hook notes\n"
        target.write_bytes(original)
        subprocess.run(
            [
                sys.executable,
                "-m",
                "venv",
                "--without-pip",
                str(self.project / ".venv"),
            ],
            check=True,
        )
        executable = self.project / ".venv" / "Scripts" / "python.exe"
        command = [
            str(executable),
            "-B",
            str(ROOT / "scripts" / "bridgeforge_codex_project_sync.py"),
            "--project-root",
            str(self.project),
            "--template-root",
            str(ROOT),
            "--mode",
            "update",
        ]
        planned = subprocess.run(
            command,
            text=True,
            encoding="utf-8",
            capture_output=True,
            check=False,
        )
        self.assertEqual(planned.returncode, 0, planned.stdout + planned.stderr)
        plan_payload = json.loads(planned.stdout)
        self.assertTrue(plan_payload["confirmation_required"])
        self.assertEqual(len(plan_payload["risk"]), 1)
        fingerprint = plan_payload["aggregate_fingerprint"]

        human_plan = subprocess.run(
            command + ["--output-format", "human"],
            text=True,
            encoding="utf-8",
            capture_output=True,
            check=False,
        )
        self.assertEqual(human_plan.returncode, 0, human_plan.stderr)
        self.assertIn("结论：已生成升级计划，尚未修改文件。", human_plan.stdout)
        self.assertIn("下一步：确认上述事项后，才能执行升级。", human_plan.stdout)
        self.assertNotIn("aggregate_fingerprint", human_plan.stdout)
        self.assertFalse(human_plan.stderr)

        combined_plan = subprocess.run(
            command + ["--output-format", "combined"],
            text=True,
            encoding="utf-8",
            capture_output=True,
            check=False,
        )
        self.assertEqual(combined_plan.returncode, 0, combined_plan.stderr)
        combined_payload = json.loads(combined_plan.stdout)
        self.assertEqual(
            combined_payload["machine"]["aggregate_fingerprint"],
            fingerprint,
        )
        self.assertEqual(
            combined_payload["human"]["conclusion"],
            "已生成升级计划，尚未修改文件。",
        )

        rejected = subprocess.run(
            command + ["--apply", "--plan-fingerprint", fingerprint],
            text=True,
            encoding="utf-8",
            capture_output=True,
            check=False,
        )
        self.assertEqual(rejected.returncode, 2)
        rejected_payload = json.loads(rejected.stdout)
        self.assertIn("confirmed-risk", rejected_payload["error"])
        self.assertEqual(target.read_bytes(), original)

        combined_rejected = subprocess.run(
            command
            + [
                "--apply",
                "--plan-fingerprint",
                fingerprint,
                "--output-format",
                "combined",
            ],
            text=True,
            encoding="utf-8",
            capture_output=True,
            check=False,
        )
        self.assertEqual(combined_rejected.returncode, 2)
        combined_failure = json.loads(combined_rejected.stdout)
        self.assertEqual(combined_failure["machine"]["status"], "failed")
        self.assertEqual(
            combined_failure["human"]["conclusion"],
            "骨架升级未完成。",
        )
        self.assertIn("确认", combined_failure["human"]["next_step"])
        self.assertFalse(combined_rejected.stderr)
        self.assertEqual(target.read_bytes(), original)

        applied = subprocess.run(
            command
            + [
                "--apply",
                "--plan-fingerprint",
                fingerprint,
                "--confirmed-risk",
            ],
            text=True,
            encoding="utf-8",
            capture_output=True,
            check=False,
        )
        self.assertEqual(applied.returncode, 0, applied.stdout + applied.stderr)
        self.assertEqual(json.loads(applied.stdout)["status"], "completed")
        repeated = subprocess.run(
            command,
            text=True,
            encoding="utf-8",
            capture_output=True,
            check=False,
        )
        self.assertEqual(repeated.returncode, 0, repeated.stdout + repeated.stderr)
        self.assertFalse(json.loads(repeated.stdout)["safe"])
        self.assertFalse(json.loads(repeated.stdout)["risk"])

    def test_new_whole_asset_identical_content_is_adopted_without_risk(self) -> None:
        asset_id = "codex.doc.hook-signals"
        baseline = self.template_without_asset(asset_id)
        self.apply(SYNC.build_plan(self.project, baseline, "init"))
        target = self.project / "doc" / "3_reference" / "codex-hook-signals.md"
        target.parent.mkdir(parents=True, exist_ok=True)
        incoming = ROOT / "templates" / target.relative_to(self.project)
        target.write_bytes(incoming.read_bytes())

        plan = SYNC.build_plan(self.project, ROOT, "update")
        self.assertFalse(plan.blockers)
        self.assertFalse(plan.risk_actions)
        self.assertFalse(any(item.asset_id == asset_id for item in plan.actions))
        self.apply(plan)
        self.assertEqual(target.read_bytes(), incoming.read_bytes())
        self.assertEqual(SYNC.build_plan(self.project, ROOT, "update").actions, [])

    def test_new_whole_asset_risk_replan_and_rollback_fail_closed(self) -> None:
        asset_id = "codex.doc.hook-signals"
        baseline = self.template_without_asset(asset_id)
        self.apply(SYNC.build_plan(self.project, baseline, "init"))
        target = self.project / "doc" / "3_reference" / "codex-hook-signals.md"
        target.parent.mkdir(parents=True, exist_ok=True)
        original = b"project-owned hook notes\n"
        target.write_bytes(original)

        drift_plan = SYNC.build_plan(self.project, ROOT, "update")
        drifted = b"changed after planning\n"
        target.write_bytes(drifted)
        with self.assertRaisesRegex(SYNC.SyncBlocked, "fingerprint"):
            self.apply(drift_plan, confirmed_risk=True)
        self.assertEqual(target.read_bytes(), drifted)

        target.write_bytes(original)
        rollback_plan = SYNC.build_plan(self.project, ROOT, "update")

        def fail_after_risk(label: str) -> None:
            if label == f"after-action:{asset_id}":
                raise RuntimeError("injected risk failure")

        with self.assertRaisesRegex(SYNC.SyncBlocked, "rolled back"):
            self.apply(
                rollback_plan,
                confirmed_risk=True,
                checkpoint=fail_after_risk,
            )
        self.assertEqual(target.read_bytes(), original)

    def test_current_update_deletes_removed_unchanged_whole_asset(self) -> None:
        installed, incoming, relative = self.template_with_removable_asset(
            "whole-success",
            "whole",
        )
        self.apply(SYNC.build_plan(self.project, installed, "init"))
        target = self.project / relative
        self.assertTrue(target.is_file())

        plan = SYNC.build_plan(self.project, incoming, "update")
        self.assertFalse(plan.blockers)
        self.assertIn(
            relative,
            [action.target for action in plan.actions if action.action == "delete"],
        )
        receipt = self.apply(plan)

        self.assertIn("current.remove.codex.test.phase2-removed-whole-success", receipt.applied)
        self.assertFalse(target.exists())
        repeated = SYNC.build_plan(self.project, incoming, "update")
        self.assertFalse(repeated.blockers)
        self.assertEqual(repeated.actions, [])

    def test_removed_whole_asset_drift_blocks_without_writes(self) -> None:
        installed, incoming, relative = self.template_with_removable_asset(
            "whole-drift",
            "whole",
        )
        self.apply(SYNC.build_plan(self.project, installed, "init"))
        target = self.project / relative
        target.write_text("project drift\n", encoding="utf-8")
        before = self.snapshot_tree()

        plan = SYNC.build_plan(self.project, incoming, "update")

        self.assertIn("current baseline drifted", " ".join(plan.blockers))
        self.assertEqual(self.snapshot_tree(), before)

    def test_removed_non_whole_assets_block_without_writes(self) -> None:
        for strategy in ("merge", "region", "seed"):
            with self.subTest(strategy=strategy):
                with tempfile.TemporaryDirectory() as raw:
                    project = Path(raw)
                    installed, incoming, _relative = self.template_with_removable_asset(
                        f"non-whole-{strategy}",
                        strategy,
                    )
                    self.apply(SYNC.build_plan(project, installed, "init"))
                    before = {
                        path.relative_to(project).as_posix(): path.read_bytes()
                        for path in project.rglob("*")
                        if path.is_file()
                    }
                    plan = SYNC.build_plan(project, incoming, "update")
                    self.assertIn("not whole-owned", " ".join(plan.blockers))
                    after = {
                        path.relative_to(project).as_posix(): path.read_bytes()
                        for path in project.rglob("*")
                        if path.is_file()
                    }
                    self.assertEqual(after, before)

    def test_removed_asset_delete_failure_rolls_back(self) -> None:
        installed, incoming, relative = self.template_with_removable_asset(
            "whole-rollback",
            "whole",
        )
        self.apply(SYNC.build_plan(self.project, installed, "init"))
        plan = SYNC.build_plan(self.project, incoming, "update")
        before = self.snapshot_tree()

        def fail_after_removed_delete(label: str) -> None:
            if label.startswith("after-action:current.remove."):
                raise RuntimeError("injected removed-asset failure")

        with self.assertRaisesRegex(SYNC.SyncBlocked, "rolled back"):
            self.apply(plan, checkpoint=fail_after_removed_delete)
        self.assertEqual(self.snapshot_tree(), before)
        self.assertTrue((self.project / relative).is_file())

    def test_duplicate_normalized_target_and_damaged_contract_block(self) -> None:
        for damage in ("duplicate-target", "unknown-strategy"):
            with self.subTest(damage=damage):
                with tempfile.TemporaryDirectory() as raw:
                    project = Path(raw)
                    installed, incoming, relative = self.template_with_removable_asset(
                        f"damage-{damage}",
                        "whole",
                    )
                    self.apply(SYNC.build_plan(project, installed, "init"))
                    contract_path = project / ".codex" / "managed-skeleton.json"
                    contract = json.loads(contract_path.read_text(encoding="utf-8"))
                    asset = next(
                        item for item in contract["assets"]
                        if item["target"] == relative
                    )
                    if damage == "duplicate-target":
                        duplicate = dict(asset)
                        duplicate["id"] = "codex.test.duplicate-normalized-target"
                        duplicate["target"] = relative.replace("/", "\\")
                        contract["assets"].append(duplicate)
                    else:
                        asset["strategy"] = "unknown"
                    contract_path.write_text(
                        json.dumps(contract, ensure_ascii=False, indent=2) + "\n",
                        encoding="utf-8",
                    )
                    before = {
                        path.relative_to(project).as_posix(): path.read_bytes()
                        for path in project.rglob("*")
                        if path.is_file()
                    }
                    plan = SYNC.build_plan(project, incoming, "update")
                    self.assertTrue(plan.blockers)
                    if damage == "duplicate-target":
                        self.assertIn(
                            "duplicate",
                            " ".join(plan.blockers),
                        )
                    after = {
                        path.relative_to(project).as_posix(): path.read_bytes()
                        for path in project.rglob("*")
                        if path.is_file()
                    }
                    self.assertEqual(after, before)

    def test_downstream_merge_and_markdown_projections_fail_closed(self) -> None:
        self.apply(SYNC.build_plan(self.project, ROOT, "init"))
        settings_path = self.project / ".codex" / "settings.json"
        settings = json.loads(settings_path.read_text(encoding="utf-8"))
        settings["permissions"]["defaultMode"] = "bypassPermissions"
        settings_path.write_text(json.dumps(settings), encoding="utf-8")
        with self.assertRaisesRegex(BASELINE.BaselineError, "drifted"):
            BASELINE.verify_current_baseline(self.project)

        settings_path.write_bytes((ROOT / "templates" / "settings.json").read_bytes())
        readme = self.project / "doc" / "README.md"
        readme.write_text(
            readme.read_text(encoding="utf-8").replace(
                "系统当前架构、关键接口、数据流与 ADR",
                "drifted managed row",
                1,
            ),
            encoding="utf-8",
        )
        with self.assertRaisesRegex(BASELINE.BaselineError, "Markdown projection"):
            BASELINE.verify_current_baseline(self.project)

    def test_hooks_reject_duplicate_json_and_unknown_managed_id(self) -> None:
        self.apply(SYNC.build_plan(self.project, ROOT, "init"))
        hooks_path = self.project / ".codex" / "hooks.json"
        canonical = hooks_path.read_text(encoding="utf-8")
        hooks_path.write_text(
            '{"hooks": {}, "hooks": {}}\n',
            encoding="utf-8",
        )
        with self.assertRaisesRegex(BASELINE.BaselineError, "duplicate JSON key"):
            BASELINE.verify_current_baseline(self.project)

        hooks = json.loads(canonical)
        hooks["hooks"].setdefault("SessionStart", []).append(
            {
                "matcher": "",
                "hooks": [
                    {
                        "type": "command",
                        "command": "echo invalid",
                        "bridgeforgeCodexId": "bridgeforge-codex.project-hook.v1:unknown",
                    }
                ],
            }
        )
        hooks_path.write_text(json.dumps(hooks), encoding="utf-8")
        with self.assertRaisesRegex(BASELINE.BaselineError, "identity set"):
            BASELINE.verify_current_baseline(self.project)

    def test_rebuild_drops_every_unselected_project_surface(self) -> None:
        codex = self.project / ".codex"
        (codex / "hooks").mkdir(parents=True)
        (codex / ".bridgeforge_version").write_text(
            LEGACY_VERSION + "\n",
            encoding="utf-8",
        )
        project_hook = self.write_project_hook_bundle("project_only")
        (codex / "settings.json").write_text(
            '{"projectOnly": true}\n', encoding="utf-8"
        )
        agents = (ROOT / "templates" / "AGENTS.md").read_text(encoding="utf-8")
        (self.project / "AGENTS.md").write_text(
            agents.replace(
                "> 本区由项目完全所有。",
                "> 本区由项目完全所有。\n\nDROP-ME",
                1,
            ),
            encoding="utf-8",
        )
        precommit = self.project / ".githooks" / "pre-commit"
        precommit.parent.mkdir()
        precommit.write_text(
            (ROOT / "templates" / ".githooks" / "pre-commit")
            .read_text(encoding="utf-8")
            .replace(
                "# >>> PROJECT_EXTENSION_BEGIN\n",
                "# >>> PROJECT_EXTENSION_BEGIN\necho project-only\n",
            ),
            encoding="utf-8",
        )

        plan = SYNC.build_plan(self.project, ROOT, "auto")
        delete = tuple(
            item["id"]
            for item in plan.preservation_entries
            if item.get("disposition") == "user-decision"
        )
        receipt = self.apply(
            plan,
            confirmed_preservation_manifest=True,
            confirmed_risk=True,
            preserved_project_asset_ids=(),
            deleted_project_asset_ids=delete,
        )
        self.assertEqual(receipt.mode, "rebuild")
        self.assertFalse(project_hook.exists())
        self.assertNotIn("DROP-ME", (self.project / "AGENTS.md").read_text(encoding="utf-8"))
        self.assertNotIn(
            "project-only",
            (self.project / ".githooks" / "pre-commit").read_text(encoding="utf-8"),
        )
        settings = json.loads(
            (self.project / ".codex" / "settings.json").read_text(encoding="utf-8")
        )
        self.assertNotIn("projectOnly", settings)
        hooks = json.loads(
            (self.project / ".codex" / "hooks.json").read_text(encoding="utf-8")
        )
        self.assertNotIn("project_only/entrypoint.py", json.dumps(hooks))

    def test_project_skill_without_description_blocks_before_writes(self) -> None:
        skill = self.project / ".codex" / "skills" / "broken" / "SKILL.md"
        skill.parent.mkdir(parents=True)
        (self.project / SYNC.OBSOLETE_STAMP).write_text(
            LEGACY_VERSION + "\n",
            encoding="utf-8",
        )
        skill.write_text("---\nname: broken\n---\n", encoding="utf-8")
        plan = SYNC.build_plan(self.project, ROOT, "adopt")
        self.assertIn("compatibility check", " ".join(plan.blockers))
        self.assertEqual(skill.read_text(encoding="utf-8"), "---\nname: broken\n---\n")

    def test_project_skill_generic_agent_role_becomes_gap_without_writes(self) -> None:
        skill = self.project / ".codex" / "skills" / "generic" / "SKILL.md"
        skill.parent.mkdir(parents=True)
        (self.project / SYNC.OBSOLETE_STAMP).write_text(
            LEGACY_VERSION + "\n",
            encoding="utf-8",
        )
        skill.write_text(
            "---\nname: generic\ndescription: project semantics\n---\n\n"
            "旧项目必须先由独立 agent 审计。\n",
            encoding="utf-8",
        )
        before = skill.read_bytes()

        plan = SYNC.build_plan(self.project, ROOT, "adopt")

        self.assertEqual(plan.blockers, [])
        self.assertTrue(
            any(
                gap.asset_id == "project.skill-agent-routing"
                and "generic Agent label" in gap.reason
                for gap in plan.gaps
            ),
            plan.gaps,
        )
        with self.assertRaisesRegex(SYNC.SyncBlocked, "unresolved gaps"):
            self.apply(plan)
        self.assertEqual(skill.read_bytes(), before)
        self.assertFalse(
            (self.project / SYNC.CURRENT_STAMP).exists()
        )

    def test_project_skill_named_agent_role_is_accepted(self) -> None:
        skill = self.project / ".codex" / "skills" / "named" / "SKILL.md"
        skill.parent.mkdir(parents=True)
        (self.project / SYNC.OBSOLETE_STAMP).write_text(
            LEGACY_VERSION + "\n",
            encoding="utf-8",
        )
        skill.write_text(
            "---\nname: named\ndescription: project semantics\n---\n\n"
            "把独立审计显式分派给 `review-auditor`。\n",
            encoding="utf-8",
        )

        plan = SYNC.build_plan(self.project, ROOT, "adopt")

        self.assertEqual(plan.blockers, [])
        self.assertFalse(
            any(gap.asset_id == "project.skill-agent-routing" for gap in plan.gaps),
            plan.gaps,
        )

    def test_project_skill_unknown_agent_role_becomes_gap(self) -> None:
        skill = self.project / ".codex" / "skills" / "unknown" / "SKILL.md"
        skill.parent.mkdir(parents=True)
        (self.project / SYNC.OBSOLETE_STAMP).write_text(
            LEGACY_VERSION + "\n",
            encoding="utf-8",
        )
        skill.write_text(
            "---\nname: unknown\ndescription: project semantics\n---\n\n"
            "把调查显式分派给 `missing-agent`。\n",
            encoding="utf-8",
        )

        plan = SYNC.build_plan(self.project, ROOT, "adopt")

        self.assertEqual(plan.blockers, [])
        self.assertTrue(
            any(
                gap.asset_id == "project.skill-agent-routing"
                and "unknown Agent role 'missing-agent'" in gap.reason
                for gap in plan.gaps
            ),
            plan.gaps,
        )

    def test_fingerprint_and_preservation_ids_fail_closed(self) -> None:
        plan = SYNC.build_plan(self.project, ROOT, "init")
        with self.assertRaisesRegex(SYNC.SyncBlocked, "fingerprint"):
            SYNC.apply_plan(plan, plan_fingerprint="sha256:" + "0" * 64)
        self.assertFalse((self.project / "AGENTS.md").exists())

        stamp = self.project / ".codex" / ".bridgeforge_version"
        stamp.parent.mkdir(parents=True)
        stamp.write_text(LEGACY_VERSION + "\n", encoding="utf-8")
        rebuild = SYNC.build_plan(self.project, ROOT, "auto")
        with self.assertRaisesRegex(SYNC.SyncBlocked, "unknown or non-selectable"):
            self.apply(
                rebuild,
                confirmed_preservation_manifest=True,
                confirmed_risk=True,
                preserved_project_asset_ids=("P:hook:not-present",),
            )
        self.assertTrue(stamp.is_file())

    def test_current_drift_and_stamp_identity_failures_block_without_writes(self) -> None:
        self.apply(SYNC.build_plan(self.project, ROOT, "init"))
        target = self.project / ".codex" / "hooks" / "requirements_check.py"
        target.write_text("# drift\n", encoding="utf-8")
        before = target.read_bytes()
        drifted = SYNC.build_plan(self.project, ROOT, "update")
        self.assertTrue(drifted.blockers)
        self.assertEqual(target.read_bytes(), before)

        target.write_bytes(
            (ROOT / "templates" / "hooks" / "requirements_check.py").read_bytes()
        )
        stamp = self.project / ".codex" / ".bridgeforge_codex_version"
        stamp.unlink()
        missing = SYNC.build_plan(self.project, ROOT, "update")
        self.assertIn("no recognized version stamp", " ".join(missing.blockers))
        self.assertFalse(stamp.exists())

        stamp.write_text(CURRENT_VERSION + "\n", encoding="utf-8")
        obsolete = self.project / ".codex" / ".bridgeforge_version"
        obsolete.write_text(LEGACY_VERSION + "\n", encoding="utf-8")
        double = SYNC.build_plan(self.project, ROOT, "update")
        self.assertIn("both current and obsolete", " ".join(double.blockers))

        stamp.unlink()
        obsolete.write_text("not-a-version\n", encoding="utf-8")
        invalid = SYNC.build_plan(self.project, ROOT, "update")
        self.assertIn("not stable SemVer", " ".join(invalid.blockers))

    def test_identity_blocker_never_executes_downstream_memory_lint(self) -> None:
        codex = self.project / ".codex"
        hooks = codex / "hooks"
        memory = codex / "memory" / "engineering"
        hooks.mkdir(parents=True)
        memory.mkdir(parents=True)
        (codex / ".bridgeforge_version").write_text(
            LEGACY_VERSION + "\n",
            encoding="utf-8",
        )
        (codex / ".bridgeforge_codex_version").write_text(
            CURRENT_VERSION + "\n",
            encoding="utf-8",
        )
        (hooks / "memory_lint.py").write_text(
            "from pathlib import Path\nPath('MALICIOUS-WRITE').write_text('ran')\n",
            encoding="utf-8",
        )
        (memory / "note.md").write_text(
            "---\ncategory: engineering\nstatus: active\n"
            "description: valid note\n---\n",
            encoding="utf-8",
        )
        before = self.snapshot_tree()

        plan = SYNC.build_plan(self.project, ROOT, "auto")

        self.assertIn("both current and obsolete", " ".join(plan.blockers))
        self.assertEqual(self.snapshot_tree(), before)
        self.assertFalse((self.project / "MALICIOUS-WRITE").exists())

    def test_both_stamp_names_route_legacy_version_to_rebuild(self) -> None:
        for stamp_name in (SYNC.OBSOLETE_STAMP, SYNC.CURRENT_STAMP):
            with self.subTest(stamp=stamp_name):
                with tempfile.TemporaryDirectory() as raw:
                    project = Path(raw)
                    stamp = project / stamp_name
                    stamp.parent.mkdir(parents=True)
                    stamp.write_text(LEGACY_VERSION + "\n", encoding="utf-8")

                    plan = SYNC.build_plan(project, ROOT, "auto")

                    self.assertFalse(plan.blockers)
                    self.assertEqual(plan.mode, "rebuild")
                    self.assertEqual(plan.previous_version, LEGACY_VERSION)

    def test_obsolete_clean_stamp_migrates_to_current_and_replans_noop(self) -> None:
        baseline_template = self.template_at_version(CLEAN_BASELINE_VERSION)
        self.apply(SYNC.build_plan(self.project, baseline_template, "init"))
        current = self.project / SYNC.CURRENT_STAMP
        obsolete = self.project / SYNC.OBSOLETE_STAMP
        current_plan = SYNC.build_plan(
            self.project,
            baseline_template,
            "update",
        )
        self.assertFalse(current_plan.blockers)
        self.assertEqual(current_plan.mode, "update")
        self.assertEqual(current_plan.previous_version, CLEAN_BASELINE_VERSION)
        current.replace(obsolete)
        self.assertEqual(
            obsolete.read_text(encoding="utf-8").strip(),
            CLEAN_BASELINE_VERSION,
        )

        plan = SYNC.build_plan(self.project, baseline_template, "auto")

        self.assertFalse(plan.blockers)
        self.assertEqual(plan.mode, "update")
        self.assertIn(
            "stamp.remove-obsolete",
            [item.asset_id for item in plan.actions],
        )
        receipt = self.apply(plan)
        self.assertEqual(receipt.mode, "update")
        self.assertFalse(obsolete.exists())
        self.assertEqual(
            current.read_text(encoding="utf-8").strip(),
            CLEAN_BASELINE_VERSION,
        )
        repeated = SYNC.build_plan(
            self.project,
            baseline_template,
            "update",
        )
        self.assertFalse(repeated.blockers)
        self.assertEqual(repeated.actions, [])

    def test_legacy_rebuild_ignores_missing_or_damaged_old_contract(self) -> None:
        for damage in ("missing", "damaged"):
            with self.subTest(damage=damage):
                with tempfile.TemporaryDirectory() as raw:
                    project = Path(raw)
                    stamp = project / SYNC.CURRENT_STAMP
                    stamp.parent.mkdir(parents=True)
                    stamp.write_text(LEGACY_VERSION + "\n", encoding="utf-8")
                    if damage == "damaged":
                        (project / ".codex" / "managed-skeleton.json").write_text(
                            "{not-json}\n",
                            encoding="utf-8",
                        )

                    plan = SYNC.build_plan(project, ROOT, "auto")

                    self.assertFalse(plan.blockers)
                    self.assertEqual(plan.mode, "rebuild")
                    receipt = SYNC.apply_plan(
                        plan,
                        plan_fingerprint=plan.aggregate_fingerprint,
                        confirmed_preservation_manifest=True,
                        confirmed_risk=True,
                    )
                    self.assertEqual(receipt.mode, "rebuild")
                    BASELINE.verify_current_baseline(project)

    def test_legacy_rebuild_ignores_head_contract_without_release(self) -> None:
        codex = self.project / ".codex"
        codex.mkdir()
        legacy_contract = json.loads(
            (ROOT / "templates" / "managed-skeleton.json").read_text(
                encoding="utf-8"
            )
        )
        legacy_contract["schema_version"] = 2
        legacy_contract.pop("release_version")
        legacy_contract["stamp"] = ".codex/.bridgeforge_version"
        contract_path = codex / "managed-skeleton.json"
        contract_path.write_text(
            json.dumps(legacy_contract, ensure_ascii=False, indent=2) + "\n",
            encoding="utf-8",
        )
        legacy_stamp = codex / ".bridgeforge_version"
        legacy_stamp.write_text("0.94.2\n", encoding="utf-8")
        subprocess.run(["git", "init", "-q"], cwd=self.project, check=True)
        subprocess.run(
            [
                "git",
                "add",
                ".codex/managed-skeleton.json",
                ".codex/.bridgeforge_version",
            ],
            cwd=self.project,
            check=True,
        )
        subprocess.run(
            [
                "git",
                "-c",
                "user.name=BridgeForge Test",
                "-c",
                "user.email=test@example.invalid",
                "commit",
                "-qm",
                "legacy contract",
            ],
            cwd=self.project,
            check=True,
        )

        plan = SYNC.build_plan(self.project, ROOT, "update")
        self.assertFalse(plan.blockers)
        self.assertEqual(plan.mode, "rebuild")
        receipt = self.apply(
            plan,
            confirmed_preservation_manifest=True,
            confirmed_risk=True,
        )

        self.assertEqual(receipt.mode, "rebuild")
        self.assertTrue(receipt.stamp_written_last)
        BASELINE.verify_current_baseline(self.project)

    def test_unknown_codex_structure_blocks_rebuild_without_writes(self) -> None:
        stamp = self.project / SYNC.OBSOLETE_STAMP
        stamp.parent.mkdir(parents=True)
        stamp.write_text(LEGACY_VERSION + "\n", encoding="utf-8")
        unknown = self.project / ".codex" / "unclassified" / "payload.txt"
        unknown.parent.mkdir()
        unknown.write_text("project data\n", encoding="utf-8")
        before = self.snapshot_tree()

        plan = SYNC.build_plan(self.project, ROOT, "auto")

        self.assertIn("unknown .codex structure", " ".join(plan.blockers))
        self.assertEqual(self.snapshot_tree(), before)

    def test_required_project_maps_survive_rebuild_byte_identically(self) -> None:
        stamp = self.project / SYNC.OBSOLETE_STAMP
        stamp.parent.mkdir(parents=True)
        stamp.write_text(LEGACY_VERSION + "\n", encoding="utf-8")
        payloads = {
            ".codex/find-doc.map.md": b"# find-doc project map\n",
            ".codex/sync-docs.map.md": b"# sync-docs project map\n",
        }
        for relative, payload in payloads.items():
            target = self.project / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(payload)

        plan = SYNC.build_plan(self.project, ROOT, "auto")

        self.assertFalse(plan.blockers)
        entries = {
            str(item["target"]): item
            for item in plan.preservation_entries
            if item.get("kind") == "project-map"
        }
        self.assertEqual(set(entries), set(payloads))
        self.assertTrue(all(
            item.get("disposition") == "required-preserve"
            for item in entries.values()
        ))
        self.assertFalse(any(
            action.action == "delete" and action.target in payloads
            for action in plan.actions
        ))

        receipt = self.apply(
            plan,
            confirmed_preservation_manifest=True,
            confirmed_risk=True,
        )

        self.assertEqual(receipt.mode, "rebuild")
        for relative, payload in payloads.items():
            self.assertEqual((self.project / relative).read_bytes(), payload)
        BASELINE.verify_current_baseline(self.project)
        repeated = SYNC.build_plan(self.project, ROOT, "update")
        self.assertFalse(repeated.blockers)
        self.assertEqual(repeated.actions, [])

    def test_unsafe_required_project_map_blocks_without_writes(self) -> None:
        stamp = self.project / SYNC.OBSOLETE_STAMP
        stamp.parent.mkdir(parents=True)
        stamp.write_text(LEGACY_VERSION + "\n", encoding="utf-8")
        target = self.project / ".codex" / "find-doc.map.md"
        target.write_text("# project map\n", encoding="utf-8")
        before = self.snapshot_tree()
        original = SYNC._is_reparse

        with mock.patch.object(
            SYNC,
            "_is_reparse",
            side_effect=lambda path: path == target or original(path),
        ):
            plan = SYNC.build_plan(self.project, ROOT, "auto")

        self.assertIn("required project mapping", " ".join(plan.blockers))
        self.assertEqual(self.snapshot_tree(), before)

    def test_required_project_map_drift_rolls_back_without_stamping(self) -> None:
        obsolete_stamp = self.project / SYNC.OBSOLETE_STAMP
        obsolete_stamp.parent.mkdir(parents=True)
        obsolete_stamp.write_text(LEGACY_VERSION + "\n", encoding="utf-8")
        target = self.project / ".codex" / "find-doc.map.md"
        target.write_bytes(b"original\n")
        plan = SYNC.build_plan(self.project, ROOT, "auto")
        changed = False

        def drift_before_first_action(label: str) -> None:
            nonlocal changed
            if not changed and label.startswith("before-action:"):
                target.write_bytes(b"drifted\n")
                changed = True

        with self.assertRaisesRegex(
            SYNC.SyncBlocked,
            "required-preserve project file drifted",
        ):
            self.apply(
                plan,
                confirmed_preservation_manifest=True,
                confirmed_risk=True,
                checkpoint=drift_before_first_action,
            )

        self.assertTrue(changed)
        self.assertEqual(target.read_bytes(), b"drifted\n")
        self.assertEqual(
            obsolete_stamp.read_text(encoding="utf-8").strip(),
            LEGACY_VERSION,
        )
        self.assertFalse((self.project / SYNC.CURRENT_STAMP).exists())

    def test_rebuild_preserves_legacy_memory_and_reports_gap(self) -> None:
        stamp = self.project / SYNC.OBSOLETE_STAMP
        stamp.parent.mkdir(parents=True)
        stamp.write_text(LEGACY_VERSION + "\n", encoding="utf-8")
        note = self.project / ".codex" / "memory" / "note.md"
        note.parent.mkdir(parents=True)
        note.write_text(
            "---\ncategory: engineering\nstatus: active\n"
            "description: project note\n---\n\n# Note\n",
            encoding="utf-8",
        )
        expected = note.read_bytes()

        plan = SYNC.build_plan(self.project, ROOT, "auto")

        self.assertFalse(plan.blockers)
        entry = next(
            item
            for item in plan.preservation_entries
            if item.get("id") == "R:legacy-project-memory"
        )
        self.assertEqual(entry["status"], "legacy-gap")
        self.assertEqual(entry["disposition"], "required-preserve")
        self.assertEqual(entry["file_count"], 1)
        self.assertIn("pending per-project migration", entry["reason"])
        self.assertEqual(len(entry["files"]), 1)
        scanned = entry["files"][0]
        self.assertEqual(scanned["source_path"], "note.md")
        self.assertEqual(scanned["size_bytes"], len(expected))
        self.assertEqual(scanned["sha256"], SYNC._sha256_bytes(expected))
        self.assertEqual(scanned["information_type"], "body")
        self.assertEqual(scanned["ledger_status"], "discovered")
        self.assertEqual(scanned["disposition"], "hold")
        self.assertEqual(scanned["metadata"]["category"], "engineering")
        machine = SYNC._plan_payload(plan)
        self.assertEqual(machine["status"], "planned_with_gaps")
        self.assertEqual(machine["readiness"], "action_required")
        human = SYNC._plan_human_result(machine)
        self.assertIn("尚未迁移", human["conclusion"])
        self.assertTrue(any(
            "1 个 legacy" in item
            for item in human["pending_items"]
        ))

        receipt = self.apply(
            plan,
            confirmed_preservation_manifest=True,
            confirmed_risk=True,
        )

        self.assertEqual(note.read_bytes(), expected)
        self.assertEqual(receipt.status, "completed_with_gaps")
        self.assertEqual(receipt.readiness, "action_required")
        receipt_human = SYNC._receipt_human_result(SYNC.asdict(receipt))
        self.assertIn("legacy Memory 缺口", receipt_human["conclusion"])
        self.assertIn("未迁移、未删除", receipt_human["pending_items"][-1])
        repeated = SYNC.build_plan(self.project, ROOT, "update")
        self.assertFalse(repeated.blockers)
        self.assertTrue(any(
            item.get("status") == "legacy-gap"
            for item in repeated.preservation_entries
        ))

    def test_retired_memory_runtime_removal_plan_excludes_legacy_tree(self) -> None:
        runtime = self.project / ".codex" / "scripts" / "memory_router.py"
        runtime.parent.mkdir(parents=True)
        runtime.write_bytes(b"retired runtime\n")
        legacy = self.project / ".codex" / "memory" / "legacy.md"
        legacy.parent.mkdir(parents=True)
        legacy.write_bytes(b"legacy project memory\n")
        installed = {
            "assets": [{
                "id": "codex.script.memory-router",
                "source": "templates/scripts/memory_router.py",
                "target": ".codex/scripts/memory_router.py",
                "strategy": "whole",
                "current_sha256": SYNC._sha256_bytes(runtime.read_bytes()),
            }],
        }
        incoming = {"assets": []}

        actions, blockers, removed = SYNC._current_contract_removals(
            self.project,
            installed,
            incoming,
        )

        self.assertEqual(blockers, [])
        self.assertEqual(removed, {".codex/scripts/memory_router.py"})
        self.assertEqual(len(actions), 1)
        self.assertEqual(actions[0].action, "delete")
        self.assertEqual(actions[0].classification, "safe")
        self.assertEqual(legacy.read_bytes(), b"legacy project memory\n")

    def test_legacy_ledger_progress_and_damage_are_reported_read_only(self) -> None:
        stamp = self.project / SYNC.OBSOLETE_STAMP
        stamp.parent.mkdir(parents=True)
        stamp.write_text(LEGACY_VERSION + "\n", encoding="utf-8")
        note = self.project / ".codex" / "memory" / "topic.md"
        note.parent.mkdir(parents=True)
        note.write_text(
            "---\nstatus: active\n---\nlegacy body\n",
            encoding="utf-8",
        )
        expected = note.read_bytes()

        initial = SYNC.build_plan(self.project, ROOT, "auto")
        entry = next(
            item for item in initial.preservation_entries
            if item.get("id") == "R:legacy-project-memory"
        )
        self.assertEqual(entry["ledger"]["status"], "absent")
        scanned = entry["files"][0]
        ledger_path = self.project / SYNC.LEGACY_MEMORY_LEDGER
        ledger_path.parent.mkdir(parents=True)
        ledger_path.write_text(
            json.dumps({
                "schema_version": 1,
                "scan_fingerprint": entry["scan_fingerprint"],
                "records": {
                    scanned["asset_id"]: {
                        "source_path": scanned["source_path"],
                        "source_sha256": scanned["sha256"],
                        "migration_status": "verified",
                        "proposed_target": "doc/0_architecture/topic.md",
                        "disposition": "migrate",
                        "user_decision": "approve",
                        "cleanup_decision": None,
                    },
                },
            }, ensure_ascii=False, indent=2) + "\n",
            encoding="utf-8",
        )

        current = SYNC.build_plan(self.project, ROOT, "auto")
        current_entry = next(
            item for item in current.preservation_entries
            if item.get("id") == "R:legacy-project-memory"
        )
        self.assertFalse(current.blockers)
        self.assertEqual(current_entry["ledger"]["status"], "current")
        self.assertEqual(current_entry["ledger"]["progress"], {"verified": 1})
        self.assertEqual(current_entry["files"][0]["user_decision"], "approve")
        self.assertEqual(note.read_bytes(), expected)

        ledger = json.loads(ledger_path.read_text(encoding="utf-8"))
        ledger["records"][scanned["asset_id"]]["source_sha256"] = "sha256:" + "0" * 64
        ledger_path.write_text(
            json.dumps(ledger, ensure_ascii=False, indent=2) + "\n",
            encoding="utf-8",
        )
        source_drift = SYNC.build_plan(self.project, ROOT, "auto")
        source_drift_entry = next(
            item for item in source_drift.preservation_entries
            if item.get("id") == "R:legacy-project-memory"
        )
        self.assertEqual(source_drift_entry["ledger"]["status"], "drifted")
        self.assertEqual(source_drift_entry["files"][0]["ledger_status"], "drifted")
        self.assertTrue(any(
            "ledger source drift" in error
            for error in source_drift_entry["ledger"]["errors"]
        ))
        self.assertEqual(note.read_bytes(), expected)

        ledger["records"][scanned["asset_id"]]["source_sha256"] = scanned["sha256"]
        ledger["scan_fingerprint"] = "sha256:" + "f" * 64
        ledger_path.write_text(
            json.dumps(ledger, ensure_ascii=False, indent=2) + "\n",
            encoding="utf-8",
        )
        fingerprint_drift = SYNC.build_plan(self.project, ROOT, "auto")
        fingerprint_entry = next(
            item for item in fingerprint_drift.preservation_entries
            if item.get("id") == "R:legacy-project-memory"
        )
        self.assertEqual(fingerprint_entry["ledger"]["status"], "drifted")
        self.assertTrue(any(
            "scan_fingerprint" in error
            for error in fingerprint_entry["ledger"]["errors"]
        ))
        self.assertEqual(note.read_bytes(), expected)

        ledger_path.write_text("{damaged\n", encoding="utf-8")
        damaged = SYNC.build_plan(self.project, ROOT, "auto")
        damaged_entry = next(
            item for item in damaged.preservation_entries
            if item.get("id") == "R:legacy-project-memory"
        )
        self.assertFalse(damaged.blockers)
        self.assertEqual(damaged_entry["ledger"]["status"], "invalid")
        self.assertTrue(damaged_entry["ledger"]["errors"])
        self.assertEqual(note.read_bytes(), expected)

    def test_reparse_codex_directory_blocks_rebuild_without_writes(self) -> None:
        stamp = self.project / SYNC.OBSOLETE_STAMP
        stamp.parent.mkdir(parents=True)
        stamp.write_text(LEGACY_VERSION + "\n", encoding="utf-8")
        linked = self.project / ".codex" / "rules" / "linked"
        linked.mkdir(parents=True)
        before = self.snapshot_tree()
        original = SYNC._is_reparse

        with mock.patch.object(
            SYNC,
            "_is_reparse",
            side_effect=lambda path: path == linked or original(path),
        ):
            plan = SYNC.build_plan(self.project, ROOT, "auto")

        self.assertIn("unsafe .codex structure", " ".join(plan.blockers))
        self.assertEqual(self.snapshot_tree(), before)

    def test_reparse_project_asset_leaf_blocks_rebuild_without_writes(self) -> None:
        sources = {
            "AGENTS.md": ROOT / "templates" / "AGENTS.md",
            ".githooks/pre-commit": (
                ROOT / "templates" / ".githooks" / "pre-commit"
            ),
        }
        original = SYNC._is_reparse
        for relative, source in sources.items():
            with self.subTest(relative=relative):
                with tempfile.TemporaryDirectory() as raw:
                    project = Path(raw)
                    stamp = project / SYNC.OBSOLETE_STAMP
                    stamp.parent.mkdir(parents=True)
                    stamp.write_text(LEGACY_VERSION + "\n", encoding="utf-8")
                    leaf = project / relative
                    leaf.parent.mkdir(parents=True, exist_ok=True)
                    shutil.copy2(source, leaf)
                    before = leaf.read_bytes()

                    with mock.patch.object(
                        SYNC,
                        "_is_reparse",
                        side_effect=(
                            lambda path, target=leaf: (
                                path == target or original(path)
                            )
                        ),
                    ):
                        plan = SYNC.build_plan(project, ROOT, "auto")

                    self.assertIn("not a plain file", " ".join(plan.blockers))
                    self.assertEqual(leaf.read_bytes(), before)

    def test_scattered_hook_blocks_until_normalized_bundle_is_confirmed(self) -> None:
        codex = self.project / ".codex"
        hooks_root = codex / "hooks"
        hooks_root.mkdir(parents=True)
        (codex / ".bridgeforge_version").write_text(
            LEGACY_VERSION + "\n",
            encoding="utf-8",
        )
        scattered = hooks_root / "legacy_hook.py"
        scattered.write_text("print('legacy')\n", encoding="utf-8")
        (codex / "hooks.json").write_text(
            json.dumps({
                "hooks": {
                    "SessionStart": [{
                        "hooks": [{
                            "type": "command",
                            "command": (
                                ".venv/Scripts/python.exe "
                                ".codex/hooks/legacy_hook.py"
                            ),
                        }],
                    }],
                },
            }),
            encoding="utf-8",
        )
        before = self.snapshot_tree()

        blocked = SYNC.build_plan(self.project, ROOT, "auto")

        self.assertIn("must be normalized first", " ".join(blocked.blockers))
        self.assertEqual(self.snapshot_tree(), before)

        scattered.unlink()
        bundle = self.write_project_hook_bundle("project_legacy")
        plan = SYNC.build_plan(self.project, ROOT, "auto")
        self.assertFalse(plan.blockers)
        bundle_id = next(
            item["id"]
            for item in plan.preservation_entries
            if item.get("target") == ".codex/hooks/project_legacy"
        )
        receipt = self.apply(
            plan,
            confirmed_preservation_manifest=True,
            confirmed_risk=True,
            preserved_project_asset_ids=(bundle_id,),
        )
        self.assertEqual(receipt.mode, "rebuild")
        self.assertTrue((bundle / "entrypoint.py").is_file())
        repeated = SYNC.build_plan(self.project, ROOT, "update")
        self.assertFalse(repeated.blockers)
        self.assertEqual(repeated.actions, [])

    def test_obsolete_stamp_migration_failure_restores_old_identity(self) -> None:
        baseline_template = self.template_at_version(CLEAN_BASELINE_VERSION)
        self.apply(SYNC.build_plan(self.project, baseline_template, "init"))
        current = self.project / SYNC.CURRENT_STAMP
        obsolete = self.project / SYNC.OBSOLETE_STAMP
        current.replace(obsolete)
        plan = SYNC.build_plan(self.project, baseline_template, "update")
        before = self.snapshot_tree()

        def fail_after_obsolete_delete(label: str) -> None:
            if label == "after-action:stamp.remove-obsolete":
                raise RuntimeError("injected obsolete-stamp failure")

        with self.assertRaisesRegex(SYNC.SyncBlocked, "rolled back"):
            self.apply(plan, checkpoint=fail_after_obsolete_delete)
        self.assertEqual(self.snapshot_tree(), before)
        self.assertFalse(current.exists())
        self.assertEqual(
            obsolete.read_text(encoding="utf-8").strip(),
            CLEAN_BASELINE_VERSION,
        )

    def test_reparse_obsolete_stamp_blocks_current_migration_without_writes(
        self,
    ) -> None:
        baseline_template = self.template_at_version(CLEAN_BASELINE_VERSION)
        self.apply(SYNC.build_plan(self.project, baseline_template, "init"))
        current = self.project / SYNC.CURRENT_STAMP
        obsolete = self.project / SYNC.OBSOLETE_STAMP
        current.replace(obsolete)
        before = self.snapshot_tree()
        original = SYNC._is_reparse

        with mock.patch.object(
            SYNC,
            "_is_reparse",
            side_effect=lambda path: path == obsolete or original(path),
        ):
            plan = SYNC.build_plan(
                self.project,
                baseline_template,
                "update",
            )

        self.assertIn("not a plain file", " ".join(plan.blockers))
        self.assertEqual(plan.actions, [])
        self.assertEqual(self.snapshot_tree(), before)

    def test_stamp_identity_errors_block_without_writes(self) -> None:
        cases = (
            (SYNC.CURRENT_STAMP, "999.0.0", False, "newer than"),
            (SYNC.OBSOLETE_STAMP, "invalid", False, "not stable SemVer"),
            (SYNC.CURRENT_STAMP, "", True, "not a plain file"),
        )
        for stamp_name, version, directory, expected in cases:
            with self.subTest(stamp=stamp_name, version=version):
                with tempfile.TemporaryDirectory() as raw:
                    project = Path(raw)
                    stamp = project / stamp_name
                    stamp.parent.mkdir(parents=True)
                    if directory:
                        stamp.mkdir()
                    else:
                        stamp.write_text(version + "\n", encoding="utf-8")
                    before = tuple(sorted(
                        path.relative_to(project).as_posix()
                        for path in project.rglob("*")
                    ))

                    plan = SYNC.build_plan(project, ROOT, "auto")

                    self.assertIn(expected, " ".join(plan.blockers))
                    after = tuple(sorted(
                        path.relative_to(project).as_posix()
                        for path in project.rglob("*")
                    ))
                    self.assertEqual(after, before)

    def test_unknown_agents_and_precommit_markers_block_rebuild(self) -> None:
        stamp = self.project / SYNC.OBSOLETE_STAMP
        stamp.parent.mkdir(parents=True)
        stamp.write_text(LEGACY_VERSION + "\n", encoding="utf-8")
        (self.project / "AGENTS.md").write_text(
            "# project without canonical ownership markers\n",
            encoding="utf-8",
        )
        precommit = self.project / ".githooks" / "pre-commit"
        precommit.parent.mkdir()
        precommit.write_text(
            "# <<< PROJECT_EXTENSION_END\necho project\n"
            "# >>> PROJECT_EXTENSION_BEGIN\n",
            encoding="utf-8",
        )
        before = self.snapshot_tree()

        plan = SYNC.build_plan(self.project, ROOT, "auto")

        joined = " ".join(plan.blockers)
        self.assertIn("AGENTS project markers are invalid", joined)
        self.assertIn("pre-commit project-extension markers are invalid", joined)
        self.assertEqual(self.snapshot_tree(), before)

    def test_project_hook_bundle_and_registration_must_be_a_closed_pair(self) -> None:
        stamp = self.project / SYNC.OBSOLETE_STAMP
        stamp.parent.mkdir(parents=True)
        stamp.write_text(LEGACY_VERSION + "\n", encoding="utf-8")
        hooks = self.project / ".codex" / "hooks.json"
        hooks.write_text(
            json.dumps({
                "hooks": {
                    "SessionStart": [{
                        "hooks": [{
                            "type": "command",
                            "command": (
                                ".venv/Scripts/python.exe "
                                ".codex/hooks/project_missing/entrypoint.py"
                            ),
                        }],
                    }],
                },
            }),
            encoding="utf-8",
        )
        plan = SYNC.build_plan(self.project, ROOT, "auto")
        self.assertIn("has no canonical bundle", " ".join(plan.blockers))

    def test_noncanonical_project_hook_commands_are_not_reparsed(self) -> None:
        for command in (
            r".venv\Scripts\python.exe .codex\hooks\project_risk\entrypoint.py",
            (
                "powershell -NoProfile -Command "
                "'.venv/Scripts/python.exe .codex/hooks/project_risk/entrypoint.py'"
            ),
            r"C:\project\.venv\Scripts\python.exe C:\project\hook.py",
        ):
            with self.subTest(command=command):
                payload = json.dumps({
                    "hooks": {
                        "SessionStart": [{
                            "hooks": [{"type": "command", "command": command}],
                        }],
                    },
                }).encode("utf-8")
                with self.assertRaisesRegex(
                    SYNC.SyncBlocked,
                    "not a canonical Python command",
                ):
                    SYNC._project_hook_projection(payload)

    def test_rebuild_requires_explicit_disposition_for_every_project_asset(self) -> None:
        stamp = self.project / SYNC.OBSOLETE_STAMP
        stamp.parent.mkdir(parents=True)
        stamp.write_text(LEGACY_VERSION + "\n", encoding="utf-8")
        self.write_project_hook_bundle()
        plan = SYNC.build_plan(self.project, ROOT, "auto")
        before = self.snapshot_tree()
        with self.assertRaisesRegex(SYNC.SyncBlocked, "explicit preserve/delete"):
            self.apply(
                plan,
            confirmed_preservation_manifest=True,
                confirmed_risk=True,
            )
        self.assertEqual(self.snapshot_tree(), before)

    def test_bundle_delete_failure_restores_the_complete_directory(self) -> None:
        stamp = self.project / SYNC.OBSOLETE_STAMP
        stamp.parent.mkdir(parents=True)
        stamp.write_text(LEGACY_VERSION + "\n", encoding="utf-8")
        self.write_project_hook_bundle()
        plan = SYNC.build_plan(self.project, ROOT, "auto")
        delete = tuple(
            item["id"]
            for item in plan.preservation_entries
            if item.get("disposition") == "user-decision"
        )
        before = self.snapshot_tree()

        def fail_after_bundle_delete(label: str) -> None:
            if label == "after-project-hook-bundle-deletions":
                raise RuntimeError("injected bundle failure")

        with self.assertRaisesRegex(SYNC.SyncBlocked, "rolled back"):
            self.apply(
                plan,
                confirmed_preservation_manifest=True,
                confirmed_risk=True,
                deleted_project_asset_ids=delete,
                checkpoint=fail_after_bundle_delete,
            )
        self.assertEqual(self.snapshot_tree(), before)

    def test_transaction_failure_rolls_back_every_write(self) -> None:
        plan = SYNC.build_plan(self.project, ROOT, "init")

        def fail_after_first_write(label: str) -> None:
            if label.startswith("after-action:"):
                raise RuntimeError("injected failure")

        with self.assertRaisesRegex(SYNC.SyncBlocked, "rolled back"):
            SYNC.apply_plan(
                plan,
                plan_fingerprint=plan.aggregate_fingerprint,
                checkpoint=fail_after_first_write,
            )
        self.assertFalse((self.project / "AGENTS.md").exists())
        self.assertFalse((self.project / ".codex").exists())

    def test_current_config_health_failure_blocks_apply(self) -> None:
        self.apply(SYNC.build_plan(self.project, ROOT, "init"))
        local = self.project / ".codex" / "settings.local.json"
        local.write_text('{"hooks": {"SessionStart": []}}\n', encoding="utf-8")
        plan = SYNC.build_plan(self.project, ROOT, "update")
        with self.assertRaisesRegex(SYNC.SyncBlocked, "config health"):
            self.apply(plan)
        self.assertTrue(local.is_file())

    def test_post_legacy_preserve_checkpoint_failure_rolls_back(self) -> None:
        plan = SYNC.build_plan(self.project, ROOT, "init")
        def fail_after_preserve(label: str) -> None:
            if label == "after-legacy-memory-preserve":
                raise RuntimeError("post-preserve failure")

        with self.assertRaisesRegex(SYNC.SyncBlocked, "rolled back"):
            SYNC.apply_plan(
                plan,
                plan_fingerprint=plan.aggregate_fingerprint,
                checkpoint=fail_after_preserve,
            )

        self.assertFalse((self.project / "AGENTS.md").exists())
        self.assertFalse((self.project / ".codex" / "memory").exists())

    def test_legacy_memory_drift_during_apply_is_rolled_back(self) -> None:
        note = self.project / ".codex" / "memory" / "legacy.md"
        note.parent.mkdir(parents=True)
        note.write_bytes(b"original legacy memory\n")
        (self.project / SYNC.OBSOLETE_STAMP).write_text(
            LEGACY_VERSION + "\n",
            encoding="utf-8",
        )
        plan = SYNC.build_plan(self.project, ROOT, "auto")
        self.assertFalse(plan.blockers)

        def drift_memory_after_preserve(label: str) -> None:
            if label == "after-legacy-memory-preserve":
                note.write_bytes(b"drifted legacy memory\n")

        with self.assertRaisesRegex(SYNC.SyncBlocked, "rolled back"):
            self.apply(
                plan,
                confirmed_preservation_manifest=True,
                confirmed_risk=True,
                checkpoint=drift_memory_after_preserve,
            )

        self.assertEqual(note.read_bytes(), b"original legacy memory\n")

    def test_human_result_covers_noop_gap_blocker_and_failure(self) -> None:
        common = {
            "safe": [],
            "risk": [],
            "gaps": [],
            "blockers": [],
            "preservation_manifest": [],
            "confirmation_required": False,
        }
        noop = SYNC._plan_human_result(dict(common))
        self.assertEqual(noop["conclusion"], "当前骨架已是最新状态。")
        self.assertEqual(noop["pending_items"], [])

        gap_payload = dict(common)
        gap_payload["gaps"] = [{"reason": "unresolved gap in project skill"}]
        gap = SYNC._plan_human_result(gap_payload)
        self.assertIn("未解决缺口", gap["pending_items"][0])
        self.assertNotIn("project skill", SYNC._render_human_result(gap))

        blocker_payload = dict(common)
        blocker_payload["blockers"] = [
            "project runtime contract rejected: missing interpreter"
        ]
        blocker = SYNC._plan_human_result(blocker_payload)
        self.assertEqual(blocker["conclusion"], "骨架升级未完成。")
        self.assertIn("项目 .venv", blocker["next_step"])

        failure = SYNC._failure_human_result(
            {
                "error": "transaction failed and was rolled back: disk error",
                "rollback_performed": True,
            }
        )
        rendered = SYNC._render_human_result(failure)
        self.assertIn("结论：骨架升级未完成。", rendered)
        self.assertIn("本次写入已回滚", rendered)
        self.assertNotIn("traceback", rendered.casefold())


if __name__ == "__main__":
    unittest.main()
