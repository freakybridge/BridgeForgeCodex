from __future__ import annotations

import hashlib
import importlib.util
import json
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]


def load_module(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


MIGRATION = load_module(
    "bridgeforge_project_asset_migration_test",
    ROOT / "scripts" / "project_asset_migration.py",
)
SYNC = load_module(
    "bridgeforge_project_sync_migration_test",
    ROOT / "scripts" / "bridgeforge_codex_project_sync.py",
)
REBUILD = load_module(
    "bridgeforge_manifest_rebuild_migration_test",
    ROOT / "scripts" / "rebuild_shared_skill_manifest.py",
)


def sha(payload: bytes) -> str:
    return "sha256:" + hashlib.sha256(payload).hexdigest()


def snapshot(root: Path) -> dict[str, bytes]:
    return {
        path.relative_to(root).as_posix(): path.read_bytes()
        for path in sorted(item for item in root.rglob("*") if item.is_file())
    }


def source_record(
    source: dict[str, object],
    *,
    target: str | None = None,
    content: str = "",
    asset_type: str = "documentation",
) -> dict[str, object]:
    fixed = bool(source["fixed_retirement"])
    decisions: list[dict[str, object]] = []
    discarded: list[dict[str, str]] = []
    if target is not None:
        decisions.append({
            "target": target,
            "asset_type": asset_type,
            "reason": "用户确认迁往唯一职责载体",
            "target_before_sha256": None,
            "content_utf8": content,
        })
    elif not fixed:
        discarded.append({
            "summary": "重复或失效内容",
            "reason": "用户明确确认删除",
        })
    return {
        "asset_id": source["asset_id"],
        "source_path": source["source_path"],
        "source_sha256": source["source_sha256"],
        "kind": source["kind"],
        "confirmed": True,
        "retire_source": True,
        "summary": "旧项目资产完整迁移包",
        "retirement_reason": (
            MIGRATION.FIXED_DERIVED_RETIREMENT
            if fixed
            else "用户确认新资产与旧源在同一事务切换"
        ),
        "decisions": decisions,
        "discarded": discarded,
    }


class ManifestValidationTests(unittest.TestCase):
    def setUp(self) -> None:
        self.raw = tempfile.TemporaryDirectory()
        self.root = Path(self.raw.name)
        rule = self.root / ".codex" / "rules" / "risk.md"
        memory = self.root / ".codex" / "memory"
        rule.parent.mkdir(parents=True)
        memory.mkdir(parents=True)
        rule.write_text("# old rule\n", encoding="utf-8")
        (memory / "note.md").write_text("# old note\n", encoding="utf-8")
        (memory / "MEMORY.md").write_text("# derived\n", encoding="utf-8")

    def tearDown(self) -> None:
        self.raw.cleanup()

    def manifest(self) -> dict[str, object]:
        inventory = MIGRATION.inventory(self.root)
        records = []
        for source in inventory["sources"]:
            if source["kind"] == "legacy-rule":
                records.append(source_record(
                    source,
                    target="src/AGENTS.md",
                    content="# project red line\n",
                    asset_type="agents",
                ))
            elif source["kind"] == "legacy-memory":
                records.append(source_record(
                    source,
                    target="doc/3_reference/legacy-note.md",
                    content="# retained rationale\n",
                ))
            else:
                records.append(source_record(source))
        return {"schema_version": 1, "sources": records}

    def test_inventory_is_per_source_and_marks_fixed_derived_files(self) -> None:
        result = MIGRATION.inventory(self.root)
        self.assertEqual(result["status"], "awaiting-confirmation")
        self.assertEqual(result["source_count"], 3)
        derived = [item for item in result["sources"] if item["fixed_retirement"]]
        self.assertEqual([item["source_path"] for item in derived], [
            ".codex/memory/MEMORY.md",
        ])

    def test_valid_manifest_covers_every_source_without_writing(self) -> None:
        before = snapshot(self.root)
        result = MIGRATION.validate_manifest(self.root, self.manifest())
        self.assertEqual(len(result.sources), 3)
        self.assertEqual(len(result.targets), 2)
        self.assertEqual(snapshot(self.root), before)

    def test_missing_source_conflicting_shared_target_and_hash_drift_fail_closed(self) -> None:
        missing = self.manifest()
        missing["sources"].pop()
        with self.assertRaisesRegex(MIGRATION.MigrationBlocked, "does not cover"):
            MIGRATION.validate_manifest(self.root, missing)

        duplicate = self.manifest()
        nonderived = [
            item for item in duplicate["sources"]
            if item["kind"] != "derived-memory"
        ]
        shared = nonderived[0]["decisions"][0]
        nonderived[1]["decisions"][0]["target"] = shared["target"]
        nonderived[1]["decisions"][0]["asset_type"] = shared["asset_type"]
        nonderived[1]["decisions"][0]["target_before_sha256"] = shared["target_before_sha256"]
        with self.assertRaisesRegex(MIGRATION.MigrationBlocked, "identical final payload"):
            MIGRATION.validate_manifest(self.root, duplicate)

        drift = self.manifest()
        (self.root / ".codex" / "memory" / "note.md").write_text(
            "changed\n",
            encoding="utf-8",
        )
        with self.assertRaisesRegex(MIGRATION.MigrationBlocked, "hash drifted"):
            MIGRATION.validate_manifest(self.root, drift)

    def test_multiple_sources_can_share_one_identical_final_target(self) -> None:
        manifest = self.manifest()
        nonderived = [
            item for item in manifest["sources"]
            if item["kind"] != "derived-memory"
        ]
        shared = nonderived[0]["decisions"][0]
        nonderived[1]["decisions"] = [dict(shared)]
        result = MIGRATION.validate_manifest(self.root, manifest)
        self.assertEqual(len(result.targets), 1)
        self.assertEqual(len(result.targets[0].source_asset_ids), 2)

    def test_source_hash_detects_crlf_to_lf_byte_drift(self) -> None:
        source = self.root / ".codex" / "rules" / "risk.md"
        source.write_bytes(b"# old rule\r\n")
        manifest = self.manifest()
        source.write_bytes(b"# old rule\n")
        with self.assertRaisesRegex(MIGRATION.MigrationBlocked, "hash drifted"):
            MIGRATION.validate_manifest(self.root, manifest)

    def test_two_hook_sources_share_one_combined_hooks_registration(self) -> None:
        root = self.root
        second = root / ".codex" / "rules" / "second.md"
        second.write_text("# second\n", encoding="utf-8")
        sources = [
            item for item in MIGRATION.inventory(root)["sources"]
            if item["kind"] != "derived-memory"
        ]
        commands = {
            source["source_path"]: (
                ".codex/hooks/project_" + str(index) + "/entrypoint.py"
            )
            for index, source in enumerate(sources)
        }
        registration = json.dumps({
            "hooks": {
                "SessionStart": [{
                    "matcher": "",
                    "hooks": [
                        {"type": "command", "command": ".venv/Scripts/python.exe " + target}
                        for target in commands.values()
                    ],
                }],
            },
        }) + "\n"
        records = []
        for source in MIGRATION.inventory(root)["sources"]:
            record = source_record(source)
            if source["kind"] != "derived-memory":
                target = commands[source["source_path"]]
                record["discarded"] = []
                record["decisions"] = [
                    {
                        "target": target,
                        "asset_type": "hook",
                        "reason": "用户确认迁移 Hook",
                        "target_before_sha256": None,
                        "content_utf8": "print('ok')\n",
                    },
                    {
                        "target": ".codex/hooks.json",
                        "asset_type": "hook-registration",
                        "reason": "用户确认合并注册",
                        "target_before_sha256": None,
                        "content_utf8": registration,
                    },
                ]
            records.append(record)
        result = MIGRATION.validate_manifest(
            root,
            {"schema_version": 1, "sources": records},
        )
        registrations = [item for item in result.targets if item.target == ".codex/hooks.json"]
        self.assertEqual(len(registrations), 1)
        self.assertEqual(len(registrations[0].source_asset_ids), 3)

    def test_derived_memory_cannot_be_semantically_migrated(self) -> None:
        manifest = self.manifest()
        derived = next(
            item for item in manifest["sources"]
            if item["kind"] == "derived-memory"
        )
        derived["decisions"] = [{
            "target": "doc/3_reference/derived.md",
            "asset_type": "documentation",
            "reason": "not allowed",
            "target_before_sha256": None,
            "content_utf8": "# no\n",
        }]
        with self.assertRaisesRegex(MIGRATION.MigrationBlocked, "fixed retirement"):
            MIGRATION.validate_manifest(self.root, manifest)


class ProjectSyncMigrationTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.template_temporary = tempfile.TemporaryDirectory()
        cls.template_root = Path(cls.template_temporary.name)
        shutil.copytree(ROOT / "templates", cls.template_root / "templates")
        shutil.copy2(ROOT / "VERSION", cls.template_root / "VERSION")
        (cls.template_root / "templates" / "managed-skeleton.json").write_bytes(
            REBUILD.render_managed_contract(
                ROOT / "templates" / "managed-skeleton.json"
            )
        )

    @classmethod
    def tearDownClass(cls) -> None:
        cls.template_temporary.cleanup()

    def setUp(self) -> None:
        self.raw = tempfile.TemporaryDirectory()
        self.project = Path(self.raw.name)
        initial = SYNC.build_plan(self.project, self.template_root, "init")
        self.assertFalse(initial.blockers)
        SYNC.apply_plan(
            initial,
            plan_fingerprint=initial.aggregate_fingerprint,
        )

    def tearDown(self) -> None:
        self.raw.cleanup()

    def write_legacy_assets(self) -> None:
        rule = self.project / ".codex" / "rules" / "legacy.md"
        memory = self.project / ".codex" / "memory"
        rule.parent.mkdir(parents=True, exist_ok=True)
        memory.mkdir(parents=True, exist_ok=True)
        rule.write_text("# legacy rule\n", encoding="utf-8")
        (memory / "note.md").write_text("# legacy note\n", encoding="utf-8")
        (memory / "MEMORY.md").write_text("# derived\n", encoding="utf-8")
        (memory / "MEMORY_COLD.md").write_text("# derived\n", encoding="utf-8")
        (memory / "_stats.json").write_text("{}\n", encoding="utf-8")

    def manifest(self) -> dict[str, object]:
        records = []
        for source in MIGRATION.inventory(self.project)["sources"]:
            if source["kind"] == "legacy-rule":
                records.append(source_record(
                    source,
                    target="src/AGENTS.md",
                    content="# migrated rule\n",
                    asset_type="agents",
                ))
            elif source["kind"] == "legacy-memory":
                records.append(source_record(
                    source,
                    target="doc/3_reference/migrated-memory.md",
                    content="# migrated memory\n",
                ))
            else:
                records.append(source_record(source))
        return {"schema_version": 1, "sources": records}

    def one_source_manifest(
        self,
        decisions: list[dict[str, object]],
    ) -> dict[str, object]:
        sources = MIGRATION.inventory(self.project)["sources"]
        self.assertEqual(len(sources), 1)
        record = source_record(sources[0])
        record["decisions"] = decisions
        record["discarded"] = []
        return {"schema_version": 1, "sources": [record]}

    def test_unconfirmed_inventory_and_apply_are_zero_write(self) -> None:
        self.write_legacy_assets()
        plan = SYNC.build_plan(self.project, self.template_root, "update")
        before = snapshot(self.project)
        self.assertEqual(plan.asset_migration["status"], "awaiting-confirmation")
        self.assertEqual(plan.asset_migration["source_count"], 5)
        with self.assertRaisesRegex(SYNC.SyncBlocked, "requires one confirmed"):
            SYNC.apply_plan(
                plan,
                plan_fingerprint=plan.aggregate_fingerprint,
                confirmed_risk=True,
            )
        self.assertEqual(snapshot(self.project), before)

    def test_confirmed_manifest_applies_and_retires_sources_atomically(self) -> None:
        self.write_legacy_assets()
        plan = SYNC.build_plan(
            self.project,
            self.template_root,
            "update",
            migration_manifest=self.manifest(),
        )
        receipt = SYNC.apply_plan(
            plan,
            plan_fingerprint=plan.aggregate_fingerprint,
            confirmed_risk=True,
            confirmed_asset_migration=True,
        )
        self.assertEqual(receipt.status, "completed")
        self.assertFalse((self.project / ".codex" / "memory").exists())
        self.assertFalse((self.project / ".codex" / "rules" / "legacy.md").exists())
        self.assertEqual(
            (self.project / "src" / "AGENTS.md").read_text(encoding="utf-8"),
            "# migrated rule\n",
        )
        self.assertEqual(
            (self.project / "doc" / "3_reference" / "migrated-memory.md").read_text(
                encoding="utf-8"
            ),
            "# migrated memory\n",
        )

    def test_existing_crlf_target_uses_raw_before_hash_during_apply(self) -> None:
        self.write_legacy_assets()
        target = self.project / "doc" / "3_reference" / "migrated-memory.md"
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(b"# existing\r\n")
        manifest = self.manifest()
        memory = next(
            item for item in manifest["sources"]
            if item["kind"] == "legacy-memory"
        )
        memory["decisions"][0]["target_before_sha256"] = (
            "sha256:" + hashlib.sha256(target.read_bytes()).hexdigest()
        )
        plan = SYNC.build_plan(
            self.project,
            self.template_root,
            "update",
            migration_manifest=manifest,
        )
        receipt = SYNC.apply_plan(
            plan,
            plan_fingerprint=plan.aggregate_fingerprint,
            confirmed_risk=True,
            confirmed_asset_migration=True,
        )
        self.assertEqual(receipt.status, "completed")
        self.assertEqual(target.read_bytes(), b"# migrated memory\n")

    def test_failure_after_cleanup_restores_every_source_and_target(self) -> None:
        self.write_legacy_assets()
        plan = SYNC.build_plan(
            self.project,
            self.template_root,
            "update",
            migration_manifest=self.manifest(),
        )
        before = snapshot(self.project)

        def fail(label: str) -> None:
            if label == "after-project-asset-migration":
                raise RuntimeError("injected migration failure")

        with self.assertRaisesRegex(SYNC.SyncBlocked, "rolled back"):
            SYNC.apply_plan(
                plan,
                plan_fingerprint=plan.aggregate_fingerprint,
                confirmed_risk=True,
                confirmed_asset_migration=True,
                checkpoint=fail,
            )
        self.assertEqual(snapshot(self.project), before)

    def test_source_drift_between_plan_and_apply_stops_before_writes(self) -> None:
        self.write_legacy_assets()
        plan = SYNC.build_plan(
            self.project,
            self.template_root,
            "update",
            migration_manifest=self.manifest(),
        )
        source = self.project / ".codex" / "memory" / "note.md"
        source.write_text("changed after plan\n", encoding="utf-8")
        before = snapshot(self.project)
        with self.assertRaisesRegex(SYNC.SyncBlocked, "fingerprint drifted"):
            SYNC.apply_plan(
                plan,
                plan_fingerprint=plan.aggregate_fingerprint,
                confirmed_risk=True,
                confirmed_asset_migration=True,
            )
        self.assertEqual(snapshot(self.project), before)

    def test_every_older_current_stamp_routes_to_full_rebuild(self) -> None:
        older = "0.0.0"
        stamp = self.project / SYNC.CURRENT_STAMP
        stamp.write_text(older + "\n", encoding="utf-8")
        contract = self.project / ".codex" / "managed-skeleton.json"
        contract.write_text("damaged old schema\n", encoding="utf-8")
        plan = SYNC.build_plan(self.project, self.template_root, "auto")
        self.assertFalse(plan.blockers)
        self.assertEqual(plan.mode, "rebuild")
        self.assertEqual(plan.previous_version, older)

    def test_unstamped_existing_skeleton_routes_to_current_rebuild(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            project = Path(raw)
            rule = project / ".codex" / "rules" / "legacy.md"
            rule.parent.mkdir(parents=True)
            rule.write_text("# unstamped legacy rule\n", encoding="utf-8")
            plan = SYNC.build_plan(project, self.template_root, "auto")
            self.assertFalse(plan.blockers)
            self.assertEqual(plan.mode, "rebuild")
            self.assertIsNone(plan.previous_version)
            self.assertEqual(
                plan.asset_migration["status"],
                "awaiting-confirmation",
            )

    def test_cli_accepts_ephemeral_manifest_from_stdin(self) -> None:
        self.write_legacy_assets()
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
            str(self.template_root),
            "--mode",
            "update",
            "--asset-migration-manifest",
            "-",
        ]
        manifest_text = json.dumps(self.manifest(), ensure_ascii=False)
        planned = subprocess.run(
            command,
            input=manifest_text,
            text=True,
            encoding="utf-8",
            capture_output=True,
            check=False,
        )
        self.assertEqual(planned.returncode, 0, planned.stdout + planned.stderr)
        payload = json.loads(planned.stdout)
        self.assertEqual(payload["asset_migration"]["status"], "confirmed")
        self.assertFalse((self.project / "asset-migration-manifest.json").exists())

        applied = subprocess.run(
            command
            + [
                "--apply",
                "--plan-fingerprint",
                payload["aggregate_fingerprint"],
                "--confirmed-risk",
                "--confirmed-asset-migration",
            ],
            input=manifest_text,
            text=True,
            encoding="utf-8",
            capture_output=True,
            check=False,
        )
        self.assertEqual(applied.returncode, 0, applied.stdout + applied.stderr)
        self.assertFalse((self.project / ".codex" / "memory").exists())
        self.assertFalse((self.project / ".codex" / "rules" / "legacy.md").exists())

    def test_composes_agents_project_zone_and_doc_readme_nonmanaged_index(self) -> None:
        note = self.project / ".codex" / "memory" / "note.md"
        note.parent.mkdir(parents=True)
        note.write_text("legacy knowledge\n", encoding="utf-8")
        agents = self.project / "AGENTS.md"
        readme = self.project / "doc" / "README.md"
        proposed_agents = agents.read_text(encoding="utf-8")
        proposed_agents = proposed_agents.replace(
            "## BridgeForge 公共区",
            "## CORRUPTED PUBLIC AREA",
            1,
        ).replace(
            "<!-- BRIDGEFORGE:PROJECT:END -->",
            "PROJECT-MIGRATION-SENTINEL\n<!-- BRIDGEFORGE:PROJECT:END -->",
            1,
        )
        proposed_readme = (
            readme.read_text(encoding="utf-8")
            + "\n## 项目迁移索引\n\n- [迁移说明](3_reference/migrated.md)\n"
        )
        manifest = self.one_source_manifest([
            {
                "target": "AGENTS.md",
                "asset_type": "agents",
                "reason": "项目红线进入项目区",
                "target_before_sha256": SYNC._sha256_path(agents),
                "content_utf8": proposed_agents,
            },
            {
                "target": "doc/README.md",
                "asset_type": "documentation",
                "reason": "登记项目迁移文档索引",
                "target_before_sha256": SYNC._sha256_path(readme),
                "content_utf8": proposed_readme,
            },
        ])
        plan = SYNC.build_plan(
            self.project,
            self.template_root,
            "update",
            migration_manifest=manifest,
        )
        self.assertFalse(plan.blockers)
        composed_targets = {
            action.target
            for action in plan.actions
            if action.asset_id.startswith("migration.compose.")
        }
        self.assertEqual(composed_targets, {"AGENTS.md", "doc/README.md"})
        self.assertEqual(
            sum(action.target == "AGENTS.md" for action in plan.actions),
            1,
        )
        self.assertEqual(
            sum(action.target == "doc/README.md" for action in plan.actions),
            1,
        )
        SYNC.apply_plan(
            plan,
            plan_fingerprint=plan.aggregate_fingerprint,
            confirmed_risk=True,
            confirmed_asset_migration=True,
        )
        result_agents = agents.read_text(encoding="utf-8")
        self.assertIn("## BridgeForge 公共区", result_agents)
        self.assertNotIn("CORRUPTED PUBLIC AREA", result_agents)
        self.assertIn("PROJECT-MIGRATION-SENTINEL", result_agents)
        result_readme = readme.read_text(encoding="utf-8")
        self.assertIn("## 文档生命周期", result_readme)
        self.assertIn("## 项目迁移索引", result_readme)

    def test_hook_migration_requires_entrypoint_and_registration_and_composes_hooks(self) -> None:
        note = self.project / ".codex" / "memory" / "hook.md"
        note.parent.mkdir(parents=True)
        note.write_text("legacy hook knowledge\n", encoding="utf-8")
        hooks_path = self.project / ".codex" / "hooks.json"
        hooks = json.loads(hooks_path.read_text(encoding="utf-8"))
        command = (
            ".venv/Scripts/python.exe "
            ".codex/hooks/project_memory_guard/entrypoint.py"
        )
        hooks["hooks"].setdefault("SessionStart", []).append({
            "matcher": "",
            "hooks": [{"type": "command", "command": command}],
        })
        manifest = self.one_source_manifest([
            {
                "target": ".codex/hooks/project_memory_guard/entrypoint.py",
                "asset_type": "hook",
                "reason": "把可自动判定约束实现为项目 Hook",
                "target_before_sha256": None,
                "content_utf8": "def main():\n    return 0\n",
            },
            {
                "target": ".codex/hooks.json",
                "asset_type": "hook-registration",
                "reason": "注册同一项目 Hook 入口",
                "target_before_sha256": SYNC._sha256_path(hooks_path),
                "content_utf8": json.dumps(hooks, ensure_ascii=False, indent=2) + "\n",
            },
        ])
        plan = SYNC.build_plan(
            self.project,
            self.template_root,
            "update",
            migration_manifest=manifest,
        )
        self.assertFalse(plan.blockers)
        self.assertEqual(
            sum(action.target == ".codex/hooks.json" for action in plan.actions),
            1,
        )
        SYNC.apply_plan(
            plan,
            plan_fingerprint=plan.aggregate_fingerprint,
            confirmed_risk=True,
            confirmed_asset_migration=True,
        )
        self.assertTrue(
            (self.project / ".codex" / "hooks" / "project_memory_guard" / "entrypoint.py").is_file()
        )
        self.assertIn(command, hooks_path.read_text(encoding="utf-8"))
        SYNC._trusted_current_baseline_module(self.template_root).verify_current_baseline(
            self.project,
            expected_version=(self.template_root / "VERSION").read_text(
                encoding="utf-8"
            ).strip(),
        )

    def test_hook_entrypoint_without_registration_is_blocked(self) -> None:
        note = self.project / ".codex" / "memory" / "hook.md"
        note.parent.mkdir(parents=True)
        note.write_text("legacy hook knowledge\n", encoding="utf-8")
        manifest = self.one_source_manifest([{
            "target": ".codex/hooks/project_memory_guard/entrypoint.py",
            "asset_type": "hook",
            "reason": "incomplete hook package",
            "target_before_sha256": None,
            "content_utf8": "def main():\n    return 0\n",
        }])
        before = snapshot(self.project)
        plan = SYNC.build_plan(
            self.project,
            self.template_root,
            "update",
            migration_manifest=manifest,
        )
        self.assertTrue(any(
            "no hooks.json registration" in blocker
            for blocker in plan.blockers
        ))
        self.assertEqual(snapshot(self.project), before)


if __name__ == "__main__":
    unittest.main()
