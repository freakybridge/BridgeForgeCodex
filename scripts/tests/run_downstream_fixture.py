#!/usr/bin/env python3
"""Executable current-only downstream regression fixture for bridgeforge-codex."""
from __future__ import annotations

import importlib.util
import json
import subprocess
import sys
import tempfile
import venv
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
CURRENT_VERSION = (ROOT / "VERSION").read_text(encoding="utf-8").strip()


def load(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


BASELINE = load(
    "bridgeforge_current_downstream_baseline",
    ROOT / "templates" / "scripts" / "current_baseline.py",
)


def project_python(project: Path) -> Path:
    venv.EnvBuilder(with_pip=False).create(project / ".venv")
    python = project / ".venv" / "Scripts" / "python.exe"
    if not python.is_file():
        python = project / ".venv" / "bin" / "python"
    if not python.is_file():
        raise RuntimeError("fixture project venv has no interpreter")
    return python


def cli(
    python: Path,
    project: Path,
    mode: str,
    *,
    apply_fingerprint: str | None = None,
    preserve: tuple[str, ...] = (),
    delete: tuple[str, ...] = (),
    migration_manifest: dict[str, object] | None = None,
    confirmed_asset_migration: bool = False,
    allow_blocked: bool = False,
) -> dict[str, object]:
    command = [
        str(python),
        "-B",
        str(ROOT / "scripts" / "bridgeforge_codex_project_sync.py"),
        "--project-root",
        str(project),
        "--template-root",
        str(ROOT),
        "--mode",
        mode,
    ]
    if apply_fingerprint is not None:
        command.extend(["--apply", "--plan-fingerprint", apply_fingerprint])
        if preserve or delete:
            command.extend([
                "--confirmed-preservation-manifest",
                "--confirmed-risk",
            ])
            for item in preserve:
                command.extend(["--preserve-project-asset", item])
            for item in delete:
                command.extend(["--delete-project-asset", item])
        if confirmed_asset_migration:
            command.extend([
                "--confirmed-risk",
                "--confirmed-asset-migration",
            ])
    if migration_manifest is not None:
        command.extend(["--asset-migration-manifest", "-"])
    result = subprocess.run(
        command,
        cwd=project,
        input=(
            json.dumps(migration_manifest, ensure_ascii=False)
            if migration_manifest is not None
            else None
        ),
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        timeout=180,
        check=False,
    )
    if result.returncode != 0:
        if allow_blocked and result.stdout:
            payload = json.loads(result.stdout)
            if payload.get("status") == "blocked":
                return payload
        if allow_blocked and result.stderr.startswith("BLOCKED:"):
            return {
                "status": "blocked",
                "blockers": [result.stderr.strip()],
            }
        raise RuntimeError(result.stderr or result.stdout)
    return json.loads(result.stdout)


def init_check(base: Path) -> dict[str, object]:
    project = base / "init"
    project.mkdir()
    python = project_python(project)
    plan = cli(python, project, "init")
    receipt = cli(
        python,
        project,
        "init",
        apply_fingerprint=str(plan["aggregate_fingerprint"]),
    )
    report = BASELINE.verify_current_baseline(project)
    repeated = cli(python, project, "update")
    human = subprocess.run(
        [
            str(python),
            "-B",
            str(ROOT / "scripts" / "bridgeforge_codex_project_sync.py"),
            "--project-root",
            str(project),
            "--template-root",
            str(ROOT),
            "--mode",
            "update",
            "--output-format",
            "human",
        ],
        cwd=project,
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        timeout=180,
        check=False,
    )
    return {
        "name": "current-init-idempotent",
        "ok": (
            receipt["stamp_written_last"]
            and report.version == CURRENT_VERSION
            and not (project / ".codex" / "skill-routing.json").exists()
            and not repeated["blockers"]
            and not repeated["safe"]
            and not repeated["risk"]
            and human.returncode == 0
            and "结论：无需处理。" in human.stdout
            and "aggregate_fingerprint" not in human.stdout
            and not human.stderr
        ),
    }


def rebuild_check(base: Path) -> dict[str, object]:
    project = base / "rebuild"
    hook = project / ".codex" / "hooks" / "project_only"
    scattered = project / ".codex" / "hooks" / "legacy_hook.py"
    skill = project / ".codex" / "skills" / "project" / "SKILL.md"
    scattered.parent.mkdir(parents=True)
    skill.parent.mkdir(parents=True)
    python = project_python(project)
    (project / ".codex" / ".bridgeforge_version").write_text(
        "1.4.30\n",
        encoding="utf-8",
    )
    project_maps = {
        project / ".codex" / "find-doc.map.md": b"# find-doc project map\n",
        project / ".codex" / "sync-docs.map.md": b"# sync-docs project map\n",
    }
    for path, payload in project_maps.items():
        path.write_bytes(payload)
    scattered.write_text("print('legacy')\n", encoding="utf-8")
    hooks_json = project / ".codex" / "hooks.json"
    hooks_json.write_text(
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
    skill.write_text(
        "---\nname: project\ndescription: project skill\n---\n\n# Project\n",
        encoding="utf-8",
    )
    blocked = cli(python, project, "auto", allow_blocked=True)

    scattered.unlink()
    hook.mkdir()
    (hook / "entrypoint.py").write_text("print('project')\n", encoding="utf-8")
    (hook / "config.json").write_text('{"project": true}\n', encoding="utf-8")
    hooks_json.write_text(
        json.dumps({
            "hooks": {
                "SessionStart": [{
                    "hooks": [{
                        "type": "command",
                        "command": (
                            ".venv/Scripts/python.exe "
                            ".codex/hooks/project_only/entrypoint.py"
                        ),
                    }],
                }],
            },
        }),
        encoding="utf-8",
    )
    skill_before = skill.read_bytes()
    plan = cli(python, project, "auto")
    required_map_targets = {
        str(item["target"])
        for item in plan["preservation_manifest"]
        if item.get("kind") == "project-map"
        and item.get("disposition") == "required-preserve"
    }
    hook_id = next(
        item["id"]
        for item in plan["preservation_manifest"]
        if item.get("target") == ".codex/hooks/project_only"
    )
    delete = tuple(
        str(item["id"])
        for item in plan["preservation_manifest"]
        if item.get("disposition") == "user-decision"
        and item["id"] != hook_id
    )
    receipt = cli(
        python,
        project,
        "auto",
        apply_fingerprint=str(plan["aggregate_fingerprint"]),
        preserve=(str(hook_id),),
        delete=delete,
    )
    BASELINE.verify_current_baseline(project)
    repeated = cli(python, project, "update")
    return {
        "name": "old-project-confirmed-rebuild",
        "ok": (
            bool(blocked["blockers"])
            and "must be normalized first" in " ".join(blocked["blockers"])
            and plan["mode"] == "rebuild"
            and receipt["mode"] == "rebuild"
            and (hook / "entrypoint.py").is_file()
            and (hook / "config.json").is_file()
            and skill.read_bytes() == skill_before
            and required_map_targets == {
                ".codex/find-doc.map.md",
                ".codex/sync-docs.map.md",
            }
            and all(
                path.read_bytes() == payload
                for path, payload in project_maps.items()
            )
            and not (project / ".codex" / ".bridgeforge_version").exists()
            and not (project / ".codex" / "skill-routing.json").exists()
            and not repeated["blockers"]
            and not repeated["safe"]
            and not repeated["risk"]
        ),
    }


def drift_check(base: Path) -> dict[str, object]:
    project = base / "drift"
    project.mkdir()
    python = project_python(project)
    plan = cli(python, project, "init")
    cli(
        python,
        project,
        "init",
        apply_fingerprint=str(plan["aggregate_fingerprint"]),
    )
    target = project / ".codex" / "hooks" / "requirements_check.py"
    target.write_text("# drift\n", encoding="utf-8")
    before = target.read_bytes()
    result = subprocess.run(
        [
            str(python),
            "-B",
            str(ROOT / "scripts" / "bridgeforge_codex_project_sync.py"),
            "--project-root",
            str(project),
            "--template-root",
            str(ROOT),
            "--mode",
            "update",
        ],
        cwd=project,
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        timeout=180,
        check=False,
    )
    blocked = json.loads(result.stdout)
    return {
        "name": "current-drift-zero-write-block",
        "ok": (
            result.returncode == 2
            and bool(blocked["blockers"])
            and target.read_bytes() == before
        ),
    }


def project_skill_agent_routing_gap_check(base: Path) -> dict[str, object]:
    project = base / "project-skill-agent-routing-gap"
    skill = project / ".codex" / "skills" / "project" / "SKILL.md"
    skill.parent.mkdir(parents=True)
    python = project_python(project)
    obsolete_stamp = project / ".codex" / ".bridgeforge_version"
    obsolete_stamp.write_text("1.4.30\n", encoding="utf-8")
    skill.write_text(
        "---\nname: project\ndescription: project skill\n---\n\n"
        "旧项目必须先由独立 agent 审计。\n",
        encoding="utf-8",
    )
    before = skill.read_bytes()
    plan = cli(python, project, "adopt")
    blocked = cli(
        python,
        project,
        "adopt",
        apply_fingerprint=str(plan["aggregate_fingerprint"]),
        allow_blocked=True,
    )
    routing_gaps = [
        item
        for item in plan["gaps"]
        if item.get("asset_id") == "project.skill-agent-routing"
    ]
    return {
        "name": "project-skill-agent-routing-gap-zero-write",
        "ok": (
            bool(routing_gaps)
            and blocked["status"] == "blocked"
            and "unresolved gaps" in " ".join(blocked["blockers"])
            and skill.read_bytes() == before
            and obsolete_stamp.read_text(encoding="utf-8") == "1.4.30\n"
            and not (project / ".codex" / ".bridgeforge_codex_version").exists()
            and not (project / "AGENTS.md").exists()
        ),
    }


def project_asset_migration_check(base: Path) -> dict[str, object]:
    project = base / "project-asset-migration"
    project.mkdir()
    python = project_python(project)
    initial = cli(python, project, "init")
    cli(
        python,
        project,
        "init",
        apply_fingerprint=str(initial["aggregate_fingerprint"]),
    )
    rule = project / ".codex" / "rules" / "legacy.md"
    memory = project / ".codex" / "memory"
    rule.parent.mkdir(parents=True, exist_ok=True)
    memory.mkdir(parents=True)
    rule.write_text("# legacy rule\n", encoding="utf-8")
    (memory / "note.md").write_text("# legacy note\n", encoding="utf-8")
    (memory / "MEMORY.md").write_text("# derived\n", encoding="utf-8")
    awaiting = cli(python, project, "update")
    before = {
        path.relative_to(project).as_posix(): path.read_bytes()
        for path in project.rglob("*")
        if path.is_file()
    }
    records = []
    for source in awaiting["asset_migration"]["sources"]:
        fixed = bool(source["fixed_retirement"])
        decisions = []
        if source["kind"] == "legacy-rule":
            decisions.append({
                "target": "src/AGENTS.md",
                "asset_type": "agents",
                "reason": "fixture confirmed project red line",
                "target_before_sha256": None,
                "content_utf8": "# migrated project rule\n",
            })
        elif source["kind"] == "legacy-memory":
            decisions.append({
                "target": "doc/3_reference/migrated-memory.md",
                "asset_type": "documentation",
                "reason": "fixture confirmed retained rationale",
                "target_before_sha256": None,
                "content_utf8": "# migrated project memory\n",
            })
        records.append({
            "asset_id": source["asset_id"],
            "source_path": source["source_path"],
            "source_sha256": source["source_sha256"],
            "kind": source["kind"],
            "confirmed": True,
            "retire_source": True,
            "summary": "fixture complete package",
            "retirement_reason": (
                "fixed-derived-retirement"
                if fixed
                else "fixture user confirmed transactional retirement"
            ),
            "decisions": decisions,
            "discarded": [],
        })
    manifest = {"schema_version": 1, "sources": records}
    planned = cli(
        python,
        project,
        "update",
        migration_manifest=manifest,
    )
    unchanged_after_plans = all(
        path.is_file() and path.read_bytes() == payload
        for relative, payload in before.items()
        for path in (project / relative,)
    )
    receipt = cli(
        python,
        project,
        "update",
        apply_fingerprint=str(planned["aggregate_fingerprint"]),
        migration_manifest=manifest,
        confirmed_asset_migration=True,
    )
    return {
        "name": "project-asset-migration-transaction",
        "ok": (
            awaiting["asset_migration"]["status"] == "awaiting-confirmation"
            and awaiting["asset_migration"]["source_count"] == 3
            and unchanged_after_plans
            and receipt["status"] == "completed"
            and not (project / ".codex" / "memory").exists()
            and not rule.exists()
            and (project / "src" / "AGENTS.md").is_file()
            and (project / "doc" / "3_reference" / "migrated-memory.md").is_file()
        ),
    }


def main() -> int:
    with tempfile.TemporaryDirectory() as raw:
        base = Path(raw)
        checks = [
            init_check(base),
            rebuild_check(base),
            drift_check(base),
            project_skill_agent_routing_gap_check(base),
            project_asset_migration_check(base),
        ]
    status = "passed" if all(bool(item["ok"]) for item in checks) else "failed"
    print(json.dumps({"status": status, "checks": checks}, ensure_ascii=False, indent=2))
    return 0 if status == "passed" else 1


if __name__ == "__main__":
    raise SystemExit(main())
