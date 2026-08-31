#!/usr/bin/env python3
"""Validate deterministic migration manifests for retired project assets.

This module deliberately does not infer semantics.  A caller supplies one
complete, user-confirmed package per legacy source file; this module only
checks coverage, hashes, target ownership, and deterministic payloads.
"""
from __future__ import annotations

import hashlib
import json
import os
import re
from dataclasses import dataclass, replace
from pathlib import Path, PurePosixPath
from typing import Any, Iterable


SCHEMA_VERSION = 1
DERIVED_MEMORY_NAMES = frozenset({"MEMORY.md", "MEMORY_COLD.md", "_stats.json"})
FIXED_DERIVED_RETIREMENT = "fixed-derived-retirement"
SOURCE_KINDS = frozenset({"legacy-rule", "legacy-memory", "derived-memory"})
TARGET_TYPES = frozenset({
    "agents",
    "command-rule",
    "skill",
    "hook",
    "hook-registration",
    "test",
    "delivery",
    "bug",
    "todo",
    "documentation",
})


class MigrationBlocked(RuntimeError):
    """The manifest cannot be applied without guessing or losing data."""


@dataclass(frozen=True)
class SourceAsset:
    asset_id: str
    source_path: str
    source_sha256: str
    kind: str
    fixed_retirement: bool

    def public(self) -> dict[str, Any]:
        return {
            "asset_id": self.asset_id,
            "source_path": self.source_path,
            "source_sha256": self.source_sha256,
            "kind": self.kind,
            "fixed_retirement": self.fixed_retirement,
        }


@dataclass(frozen=True)
class TargetWrite:
    source_asset_ids: tuple[str, ...]
    target: str
    asset_type: str
    reason: str
    before_sha256: str | None
    payload: bytes

    @property
    def after_sha256(self) -> str:
        return _sha256_bytes(self.payload)


@dataclass(frozen=True)
class ValidatedMigration:
    sources: tuple[SourceAsset, ...]
    targets: tuple[TargetWrite, ...]
    manifest_sha256: str

    @property
    def source_paths(self) -> tuple[str, ...]:
        return tuple(item.source_path for item in self.sources)

    def public(self) -> dict[str, Any]:
        return {
            "status": "confirmed",
            "schema_version": SCHEMA_VERSION,
            "manifest_sha256": self.manifest_sha256,
            "source_count": len(self.sources),
            "target_count": len(self.targets),
            "sources": [item.public() for item in self.sources],
            "targets": [
                {
                    "source_asset_ids": list(item.source_asset_ids),
                    "target": item.target,
                    "asset_type": item.asset_type,
                    "reason": item.reason,
                    "before_sha256": item.before_sha256,
                    "after_sha256": item.after_sha256,
                }
                for item in self.targets
            ],
        }


def _sha256_bytes(payload: bytes) -> str:
    return "sha256:" + hashlib.sha256(payload).hexdigest()


def _sha256_path(path: Path) -> str:
    return _sha256_bytes(path.read_bytes())


def _is_reparse(path: Path) -> bool:
    try:
        return bool(os.lstat(path).st_file_attributes & 0x400)
    except AttributeError:
        return path.is_symlink()


def _plain_relative(value: Any, label: str) -> str:
    if not isinstance(value, str) or not value or "\\" in value:
        raise MigrationBlocked(f"{label} must be a non-empty POSIX relative path")
    path = PurePosixPath(value)
    if path.is_absolute() or value != path.as_posix():
        raise MigrationBlocked(f"{label} is not a canonical relative path: {value!r}")
    if any(part in {"", ".", ".."} or ":" in part for part in path.parts):
        raise MigrationBlocked(f"{label} escapes or aliases the project root: {value!r}")
    return value


def _inside(root: Path, relative: str, label: str) -> Path:
    canonical = _plain_relative(relative, label)
    target = root.joinpath(*PurePosixPath(canonical).parts)
    current = target
    while current != root:
        if current.exists() and _is_reparse(current):
            raise MigrationBlocked(f"{label} traverses a linked path: {relative}")
        current = current.parent
    return target


def _walk_plain_files(root: Path, relative_root: str) -> Iterable[Path]:
    folder = _inside(root, relative_root, f"source root {relative_root}")
    if not folder.exists():
        return ()
    if not folder.is_dir() or _is_reparse(folder):
        raise MigrationBlocked(f"source root is not a plain directory: {relative_root}")
    result: list[Path] = []
    for current, dirnames, filenames in os.walk(folder, followlinks=False):
        current_path = Path(current)
        if _is_reparse(current_path):
            raise MigrationBlocked(f"source tree contains a linked directory: {current_path}")
        for name in tuple(dirnames):
            candidate = current_path / name
            if _is_reparse(candidate):
                raise MigrationBlocked(f"source tree contains a linked directory: {candidate}")
        for name in filenames:
            candidate = current_path / name
            if not candidate.is_file() or _is_reparse(candidate):
                raise MigrationBlocked(f"source tree contains a non-plain file: {candidate}")
            result.append(candidate)
    return tuple(sorted(result, key=lambda item: item.relative_to(root).as_posix().casefold()))


def scan_sources(project_root: Path) -> tuple[SourceAsset, ...]:
    root = project_root.resolve()
    sources: list[SourceAsset] = []
    for path in _walk_plain_files(root, ".codex/rules"):
        if path.suffix.casefold() != ".md":
            continue
        relative = path.relative_to(root).as_posix()
        sources.append(SourceAsset(
            asset_id="legacy-rule:" + relative,
            source_path=relative,
            source_sha256=_sha256_path(path),
            kind="legacy-rule",
            fixed_retirement=False,
        ))
    for path in _walk_plain_files(root, ".codex/memory"):
        relative = path.relative_to(root).as_posix()
        derived = path.name in DERIVED_MEMORY_NAMES
        sources.append(SourceAsset(
            asset_id="legacy-memory:" + relative,
            source_path=relative,
            source_sha256=_sha256_path(path),
            kind="derived-memory" if derived else "legacy-memory",
            fixed_retirement=derived,
        ))
    folded: dict[str, str] = {}
    for source in sources:
        key = source.source_path.casefold()
        if key in folded:
            raise MigrationBlocked(
                "legacy source paths collide case-insensitively: "
                f"{folded[key]}, {source.source_path}"
            )
        folded[key] = source.source_path
    return tuple(sorted(sources, key=lambda item: item.source_path.casefold()))


def inventory(project_root: Path) -> dict[str, Any]:
    sources = scan_sources(project_root)
    return {
        "status": "awaiting-confirmation" if sources else "not-required",
        "schema_version": SCHEMA_VERSION,
        "source_count": len(sources),
        "target_count": 0,
        "sources": [item.public() for item in sources],
        "targets": [],
    }


def _expect_keys(value: Any, expected: set[str], label: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise MigrationBlocked(f"{label} must be an object")
    actual = set(value)
    if actual != expected:
        missing = sorted(expected - actual)
        unknown = sorted(actual - expected)
        raise MigrationBlocked(
            f"{label} keys do not match schema; missing={missing}, unknown={unknown}"
        )
    return value


def _nonempty_text(value: Any, label: str) -> str:
    if not isinstance(value, str) or not value.strip():
        raise MigrationBlocked(f"{label} must be non-empty text")
    return value


def _valid_target_type(asset_type: str, target: str) -> bool:
    path = PurePosixPath(target)
    if asset_type == "agents":
        return path.name == "AGENTS.md"
    if asset_type == "command-rule":
        return target.startswith(".codex/rules/") and path.suffix == ".rules"
    if asset_type == "skill":
        return target.startswith(".codex/skills/") and path.name == "SKILL.md"
    if asset_type == "hook":
        parts = path.parts
        return (
            len(parts) == 4
            and parts[:2] == (".codex", "hooks")
            and re.fullmatch(r"project_[A-Za-z0-9][A-Za-z0-9_-]*", parts[2])
            is not None
            and parts[3] == "entrypoint.py"
        )
    if asset_type == "hook-registration":
        return target == ".codex/hooks.json"
    if asset_type == "test":
        return target.startswith("scripts/tests/")
    if asset_type in {"delivery", "todo"}:
        return target.startswith("doc/1_delivery/")
    if asset_type == "bug":
        return target.startswith("doc/2_bugs/")
    if asset_type == "documentation":
        return target == "doc/README.md" or target.startswith((
            "doc/0_architecture/",
            "doc/3_reference/",
            "doc/4_archive/",
        ))
    return False


def _canonical_manifest_payload(manifest: dict[str, Any]) -> bytes:
    return json.dumps(
        manifest,
        ensure_ascii=False,
        sort_keys=True,
        separators=(",", ":"),
    ).encode("utf-8")


def _hook_commands(payload: bytes, source_path: str) -> set[str]:
    try:
        document = json.loads(payload.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise MigrationBlocked(
            f"hook registration is not valid UTF-8 JSON: {source_path}"
        ) from exc
    if not isinstance(document, dict) or not isinstance(document.get("hooks"), dict):
        raise MigrationBlocked(
            f"hook registration has no hooks object: {source_path}"
        )
    commands: set[str] = set()

    def visit(value: Any) -> None:
        if isinstance(value, dict):
            command = value.get("command")
            if isinstance(command, str):
                commands.add(command)
            for child in value.values():
                visit(child)
        elif isinstance(value, list):
            for child in value:
                visit(child)

    visit(document["hooks"])
    return commands


def validate_manifest(
    project_root: Path,
    manifest: Any,
    *,
    reserved_targets: Iterable[str] = (),
) -> ValidatedMigration:
    root = project_root.resolve()
    top = _expect_keys(manifest, {"schema_version", "sources"}, "manifest")
    if top["schema_version"] != SCHEMA_VERSION:
        raise MigrationBlocked(f"manifest schema_version must be {SCHEMA_VERSION}")
    records = top["sources"]
    if not isinstance(records, list):
        raise MigrationBlocked("manifest sources must be an array")
    scanned = scan_sources(root)
    scanned_by_path = {item.source_path: item for item in scanned}
    source_paths_folded = {item.source_path.casefold() for item in scanned}
    reserved_folded = {str(item).casefold() for item in reserved_targets}
    seen_sources: set[str] = set()
    seen_targets: dict[str, int] = {}
    targets: list[TargetWrite] = []
    source_keys = {
        "asset_id",
        "source_path",
        "source_sha256",
        "kind",
        "confirmed",
        "retire_source",
        "summary",
        "retirement_reason",
        "decisions",
        "discarded",
    }
    decision_keys = {
        "target",
        "asset_type",
        "reason",
        "target_before_sha256",
        "content_utf8",
    }
    discarded_keys = {"summary", "reason"}
    for index, raw in enumerate(records):
        record = _expect_keys(raw, source_keys, f"manifest sources[{index}]")
        source_path = _plain_relative(
            record["source_path"],
            f"manifest sources[{index}].source_path",
        )
        if source_path in seen_sources:
            raise MigrationBlocked(f"manifest repeats source: {source_path}")
        seen_sources.add(source_path)
        source = scanned_by_path.get(source_path)
        if source is None:
            raise MigrationBlocked(f"manifest names an unknown source: {source_path}")
        if record["asset_id"] != source.asset_id or record["kind"] != source.kind:
            raise MigrationBlocked(f"manifest source identity drifted: {source_path}")
        if record["source_sha256"] != source.source_sha256:
            raise MigrationBlocked(f"manifest source hash drifted: {source_path}")
        if record["confirmed"] is not True:
            raise MigrationBlocked(f"source migration is not user-confirmed: {source_path}")
        if record["retire_source"] is not True:
            raise MigrationBlocked(
                f"legacy source must be retired in this transaction: {source_path}"
            )
        _nonempty_text(record["summary"], f"summary for {source_path}")
        retirement_reason = _nonempty_text(
            record["retirement_reason"],
            f"retirement_reason for {source_path}",
        )
        decisions = record["decisions"]
        discarded = record["discarded"]
        if not isinstance(decisions, list) or not isinstance(discarded, list):
            raise MigrationBlocked(f"decisions/discarded must be arrays: {source_path}")
        for discard_index, raw_discard in enumerate(discarded):
            discard = _expect_keys(
                raw_discard,
                discarded_keys,
                f"discarded[{discard_index}] for {source_path}",
            )
            _nonempty_text(discard["summary"], f"discarded summary for {source_path}")
            _nonempty_text(discard["reason"], f"discarded reason for {source_path}")
        if source.fixed_retirement:
            if decisions or discarded or retirement_reason != FIXED_DERIVED_RETIREMENT:
                raise MigrationBlocked(
                    "derived memory must use fixed retirement without semantic "
                    f"migration: {source_path}"
                )
        elif not decisions and not discarded:
            raise MigrationBlocked(
                f"source package has neither a target nor an explicit discarded item: {source_path}"
            )
        source_hook_targets: list[str] = []
        source_hook_registration_commands: set[str] = set()
        for decision_index, raw_decision in enumerate(decisions):
            decision = _expect_keys(
                raw_decision,
                decision_keys,
                f"decisions[{decision_index}] for {source_path}",
            )
            target = _plain_relative(
                decision["target"],
                f"target for {source_path}",
            )
            folded_target = target.casefold()
            if folded_target in source_paths_folded:
                raise MigrationBlocked(f"migration target aliases a retired source: {target}")
            if folded_target in reserved_folded:
                raise MigrationBlocked(f"migration target collides with current baseline: {target}")
            if folded_target.startswith(".codex/memory/") or folded_target == ".codex/memory":
                raise MigrationBlocked(f"migration target uses retired project memory: {target}")
            asset_type = decision["asset_type"]
            if asset_type not in TARGET_TYPES or not _valid_target_type(asset_type, target):
                raise MigrationBlocked(
                    f"asset_type {asset_type!r} cannot target {target!r}"
                )
            reason = _nonempty_text(decision["reason"], f"target reason for {target}")
            content = decision["content_utf8"]
            if not isinstance(content, str):
                raise MigrationBlocked(f"content_utf8 must be text: {target}")
            try:
                payload = content.encode("utf-8")
            except UnicodeEncodeError as exc:
                raise MigrationBlocked(f"content_utf8 is not valid UTF-8: {target}") from exc
            target_path = _inside(root, target, f"migration target {target}")
            if target_path.exists() and not target_path.is_file():
                raise MigrationBlocked(f"migration target is not a plain file: {target}")
            actual_before = _sha256_path(target_path) if target_path.is_file() else None
            declared_before = decision["target_before_sha256"]
            if declared_before is not None and (
                not isinstance(declared_before, str)
                or not declared_before.startswith("sha256:")
                or len(declared_before) != 71
            ):
                raise MigrationBlocked(f"target_before_sha256 is invalid: {target}")
            if declared_before != actual_before:
                raise MigrationBlocked(f"migration target hash drifted: {target}")
            target_write = TargetWrite(
                source_asset_ids=(source.asset_id,),
                target=target,
                asset_type=asset_type,
                reason=reason,
                before_sha256=actual_before,
                payload=payload,
            )
            if asset_type == "hook":
                source_hook_targets.append(target)
            elif asset_type == "hook-registration":
                source_hook_registration_commands.update(
                    _hook_commands(payload, source_path)
                )
            if folded_target in seen_targets:
                target_index = seen_targets[folded_target]
                existing = targets[target_index]
                if (
                    existing.target != target_write.target
                    or existing.asset_type != target_write.asset_type
                    or existing.before_sha256 != target_write.before_sha256
                    or existing.payload != target_write.payload
                ):
                    raise MigrationBlocked(
                        "shared migration target must have one identical final payload: "
                        f"{existing.target}, {source_path} -> {target}"
                    )
                targets[target_index] = replace(
                    existing,
                    source_asset_ids=(
                        *existing.source_asset_ids,
                        source.asset_id,
                    ),
                )
                continue
            seen_targets[folded_target] = len(targets)
            targets.append(target_write)
        if source_hook_targets:
            if not source_hook_registration_commands:
                raise MigrationBlocked(
                    f"hook migration has no hooks.json registration: {source_path}"
                )
            for hook_target in source_hook_targets:
                expected = ".venv/Scripts/python.exe " + hook_target
                if expected not in source_hook_registration_commands:
                    raise MigrationBlocked(
                        "hook registration does not reference its project entrypoint: "
                        f"{source_path} -> {hook_target}"
                    )
        elif source_hook_registration_commands:
            raise MigrationBlocked(
                f"hook registration has no project entrypoint in the same package: {source_path}"
            )
    missing = sorted(set(scanned_by_path) - seen_sources)
    if missing:
        raise MigrationBlocked(
            "manifest does not cover every legacy source: " + ", ".join(missing)
        )
    manifest_hash = _sha256_bytes(_canonical_manifest_payload(top))
    return ValidatedMigration(
        sources=scanned,
        targets=tuple(sorted(targets, key=lambda item: item.target.casefold())),
        manifest_sha256=manifest_hash,
    )
