#!/usr/bin/env python3
"""Fail-closed project layout gate for bridgeforge-codex-managed repositories."""
from __future__ import annotations

import argparse
import json
import os
import re
import sys
from pathlib import Path
from typing import Any
from urllib.parse import unquote

REQUIRED_DOC_LAYERS = {
    "0_architecture",
    "1_delivery",
    "2_bugs",
    "3_reference",
    "4_archive",
}
ALLOWED_DOC_ENTRIES = REQUIRED_DOC_LAYERS | {"README.md"}
LIFECYCLES = {"active", "completed", "superseded", "archived"}
VALIDATION_STATUSES = {
    "not_started",
    "in_progress",
    "awaiting_validation",
    "awaiting_user_acceptance",
    "verified",
}
ARCHIVE_READY_LIFECYCLES = {"completed", "superseded"}
MD_LINK_RE = re.compile(r"(?<!!)\[[^\]\n]+\]\(([^)\n]+)\)")
URI_SCHEME_RE = re.compile(r"^[a-z][a-z0-9+.-]*:", re.IGNORECASE)
FROZEN_DOC_SUBTREES = {
    ("2_bugs", "BUG-agents-ia", "refactored-project"),
}


def _repo_root() -> Path:
    return Path(__file__).resolve().parents[2]


def _is_reparse(path: Path) -> bool:
    try:
        attrs = getattr(path.lstat(), "st_file_attributes", 0)
    except OSError:
        return False
    return path.is_symlink() or bool(attrs & 0x400)


def _exists_lexical(path: Path) -> bool:
    return os.path.lexists(path)


def _read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8-sig", errors="replace")


def _delivery_layout(readme: str) -> str | None:
    match = re.search(r"(?m)^delivery_layout:\s*(flat|milestone)\s*(?:#.*)?$", readme)
    return match.group(1) if match else None


def _frontmatter_fields(path: Path) -> dict[str, str]:
    lines = _read_text(path).splitlines()
    if not lines or lines[0].strip() != "---":
        return {}
    try:
        end = next(
            index
            for index, line in enumerate(lines[1:], start=1)
            if line.strip() == "---"
        )
    except StopIteration:
        return {}
    fields: dict[str, str] = {}
    for line in lines[1:end]:
        key, separator, value = line.partition(":")
        if separator:
            fields[key.strip()] = value.strip().strip("\"'")
    return fields


def _lifecycle(
    path: Path,
    root: Path,
    errors: list[dict[str, str]],
) -> str | None:
    fields = _frontmatter_fields(path)
    raw_lifecycle = fields.get("lifecycle")
    if raw_lifecycle is None:
        return None
    lifecycle = raw_lifecycle.casefold()
    relative = path.relative_to(root).as_posix()
    if lifecycle not in LIFECYCLES:
        errors.append({
            "code": "invalid-document-lifecycle",
            "path": relative,
            "message": f"lifecycle 不是允许值：{raw_lifecycle}",
        })
        return None
    validation_status = fields.get("validation_status", "").casefold()
    if validation_status not in VALIDATION_STATUSES:
        errors.append({
            "code": "invalid-document-validation-status",
            "path": relative,
            "message": "声明 lifecycle 的文档必须同时提供合法 validation_status",
        })
    if lifecycle == "completed" and validation_status != "verified":
        errors.append({
            "code": "completed-document-not-verified",
            "path": relative,
            "message": "lifecycle: completed 必须同时为 validation_status: verified",
        })
    if lifecycle == "superseded" and not fields.get("superseded_by"):
        errors.append({
            "code": "superseded-document-missing-target",
            "path": relative,
            "message": "lifecycle: superseded 必须同时提供 superseded_by",
        })
    return lifecycle


def _unsafe_entry(
    errors: list[dict[str, str]],
    path: Path,
    root: Path,
) -> None:
    relative = path.relative_to(root).as_posix()
    if any(item["code"] == "unsafe-doc-entry" and item["path"] == relative for item in errors):
        return
    errors.append({
        "code": "unsafe-doc-entry",
        "path": relative,
        "message": "doc 内禁止 symlink/junction/reparse point，检查器不会跟随该路径",
    })


def _visible_directories(
    path: Path,
    root: Path,
    errors: list[dict[str, str]],
) -> list[Path]:
    if not path.is_dir() or _is_reparse(path):
        return []
    directories: list[Path] = []
    for item in path.iterdir():
        if item.name.startswith("."):
            continue
        if _is_reparse(item):
            _unsafe_entry(errors, item, root)
            continue
        if item.is_dir():
            directories.append(item)
    return sorted(directories, key=lambda item: item.name.casefold())


def _contains_markdown(
    path: Path,
    root: Path,
    errors: list[dict[str, str]],
) -> bool:
    found = False
    pending = [path]
    while pending:
        current = pending.pop()
        for item in current.iterdir():
            if _is_reparse(item):
                _unsafe_entry(errors, item, root)
                continue
            if item.is_dir():
                pending.append(item)
            elif item.is_file() and item.suffix.casefold() == ".md":
                found = True
    return found


def _visible_markdown_links(text: str) -> list[tuple[int, str]]:
    links: list[tuple[int, str]] = []
    fence_char: str | None = None
    fence_length = 0
    for line_number, line in enumerate(text.splitlines(), start=1):
        stripped = line.lstrip()
        fence = re.match(r"(`{3,}|~{3,})", stripped)
        if fence:
            marker = fence.group(1)
            if fence_char is None:
                fence_char = marker[0]
                fence_length = len(marker)
            elif marker[0] == fence_char and len(marker) >= fence_length:
                fence_char = None
                fence_length = 0
            continue
        if fence_char is not None:
            continue
        links.extend((line_number, match) for match in MD_LINK_RE.findall(line))
    return links


def _local_markdown_target(raw: str) -> str | None:
    target = raw.strip()
    if target.startswith("<") and ">" in target:
        target = target[1 : target.index(">")]
    else:
        target = target.split(maxsplit=1)[0]
    target = unquote(target).replace("\\", "/")
    target = target.split("#", 1)[0].split("?", 1)[0]
    if (
        not target
        or target.endswith("/")
        or target.startswith(("/", "#"))
        or URI_SCHEME_RE.match(target)
        or any(marker in target for marker in ("{{", "}}", "<", ">"))
    ):
        return None
    return target


def _is_frozen_doc(path: Path, doc: Path) -> bool:
    parts = path.relative_to(doc).parts
    return parts[:1] == ("4_archive",) or any(
        parts[: len(prefix)] == prefix
        for prefix in FROZEN_DOC_SUBTREES
    )


def _active_markdown_files(
    doc: Path,
    root: Path,
    errors: list[dict[str, str]],
) -> list[Path]:
    files: list[Path] = []
    pending = [doc]
    while pending:
        current = pending.pop()
        for item in current.iterdir():
            if _is_frozen_doc(item, doc):
                continue
            if _is_reparse(item):
                _unsafe_entry(errors, item, root)
                continue
            if item.is_dir():
                pending.append(item)
            elif item.is_file() and item.suffix.casefold() == ".md":
                files.append(item)
    return sorted(files, key=lambda item: item.as_posix().casefold())


def _append_dead_reference_issues(
    doc: Path,
    root: Path,
    errors: list[dict[str, str]],
) -> None:
    for source in _active_markdown_files(doc, root, errors):
        for line_number, raw_target in _visible_markdown_links(_read_text(source)):
            target = _local_markdown_target(raw_target)
            if target is None:
                continue
            resolved = (source.parent / target).resolve()
            try:
                resolved.relative_to(root)
            except ValueError:
                exists = False
            else:
                exists = resolved.exists()
            if not exists:
                errors.append({
                    "code": "dead-doc-reference",
                    "path": f"{source.relative_to(root).as_posix()}:{line_number}",
                    "message": f"活动文档引用不存在：{raw_target}",
                })


def inspect_project(root: Path) -> dict[str, list[dict[str, str]]]:
    root = root.absolute()
    errors: list[dict[str, str]] = []
    advisories: list[dict[str, str]] = []

    if _is_reparse(root) or not root.is_dir():
        return {
            "errors": [{
                "code": "unsafe-project-root",
                "path": str(root),
                "message": "项目根必须是普通目录，禁止 symlink/junction/reparse point",
            }],
            "advisories": advisories,
        }

    for name in ("test", "tests"):
        target = root / name
        if _exists_lexical(target):
            errors.append({
                "code": "legacy-test-root",
                "path": name,
                "message": f"顶层 {name}/ 已禁止；测试代码必须迁入 scripts/tests/**",
            })

    doc = root / "doc"
    if not _exists_lexical(doc):
        return {"errors": errors, "advisories": advisories}
    if _is_reparse(doc) or not doc.is_dir():
        errors.append({
            "code": "unsafe-doc-root",
            "path": "doc",
            "message": "doc 必须是项目内普通目录，禁止 symlink/junction/reparse point",
        })
        return {"errors": errors, "advisories": advisories}

    readme_path = doc / "README.md"
    if not readme_path.is_file() or _is_reparse(readme_path):
        errors.append({
            "code": "missing-doc-index",
            "path": "doc/README.md",
            "message": "doc/README.md 是唯一总索引，必须存在且为普通文件",
        })
        return {"errors": errors, "advisories": advisories}

    readme = _read_text(readme_path)
    layout = _delivery_layout(readme)
    if layout is None:
        errors.append({
            "code": "missing-delivery-layout",
            "path": "doc/README.md",
            "message": "缺少 delivery_layout: flat|milestone 单一事实源",
        })

    for item in sorted(doc.iterdir(), key=lambda value: value.name.casefold()):
        if _is_reparse(item):
            _unsafe_entry(errors, item, root)
            continue
        if item.name not in ALLOWED_DOC_ENTRIES:
            errors.append({
                "code": "unexpected-doc-entry",
                "path": item.relative_to(root).as_posix(),
                "message": "doc/ 顶层只允许五层目录和 README.md",
            })

    delivery = doc / "1_delivery"
    if delivery.is_dir() and not _is_reparse(delivery):
        topic_paths: list[Path] = []
        if layout == "flat":
            topic_paths = _visible_directories(delivery, root, errors)
        elif layout == "milestone":
            for milestone in _visible_directories(delivery, root, errors):
                topic_paths.extend(_visible_directories(milestone, root, errors))
        for topic in topic_paths:
            if not _contains_markdown(topic, root, errors):
                continue
            relative = topic.relative_to(delivery).as_posix()
            requirements = sorted(topic.glob("requirements_*.md"))
            lifecycles = [
                _lifecycle(requirement, root, errors)
                for requirement in requirements
            ]
            if any(lifecycle == "archived" for lifecycle in lifecycles):
                errors.append({
                    "code": "archived-document-in-current-layer",
                    "path": topic.relative_to(root).as_posix(),
                    "message": "lifecycle: archived 的交付文档不得留在 doc/1_delivery",
                })
            if (
                "active" in lifecycles
                and relative not in readme
                and topic.name not in readme
            ):
                errors.append({
                    "code": "unindexed-delivery-topic",
                    "path": topic.relative_to(root).as_posix(),
                    "message": f"活跃 delivery topic 未进入 doc/README.md：{relative}",
                })
            if requirements and all(
                lifecycle in ARCHIVE_READY_LIFECYCLES
                for lifecycle in lifecycles
            ):
                advisories.append({
                    "code": "delivery-archive-candidate",
                    "path": topic.relative_to(root).as_posix(),
                    "message": "全部需求卡 lifecycle 已完成或被替代，可经 $archive-scan 确认归档",
                })

    bugs = doc / "2_bugs"
    if bugs.is_dir() and not _is_reparse(bugs):
        bug_records: list[tuple[Path, Path]] = []
        for source in sorted(bugs.glob("BUG-*"), key=lambda value: value.name.casefold()):
            if _is_reparse(source):
                _unsafe_entry(errors, source, root)
                continue
            if source.is_file() and source.suffix.casefold() == ".md":
                bug_records.append((source, source))
                continue
            if not source.is_dir():
                continue
            evidence = source / "README.md"
            if _is_reparse(evidence):
                _unsafe_entry(errors, evidence, root)
            elif evidence.is_file():
                bug_records.append((source, evidence))
        for source, evidence in sorted(
            bug_records,
            key=lambda item: item[0].name.casefold(),
        ):
            lifecycle = _lifecycle(evidence, root, errors)
            if lifecycle == "active" and source.name not in readme:
                errors.append({
                    "code": "unindexed-bug",
                    "path": source.relative_to(root).as_posix(),
                    "message": f"活跃 Bug 未进入 doc/README.md：{source.name}",
                })
            if lifecycle in ARCHIVE_READY_LIFECYCLES:
                advisories.append({
                    "code": "bug-archive-candidate",
                    "path": source.relative_to(root).as_posix(),
                    "message": "Bug lifecycle 已完成或被替代，可经 $archive-scan 确认归档",
                })
            elif lifecycle == "archived":
                errors.append({
                    "code": "archived-document-in-current-layer",
                    "path": source.relative_to(root).as_posix(),
                    "message": "lifecycle: archived 的 Bug 不得留在 doc/2_bugs",
                })

    archive = doc / "4_archive"
    if archive.is_dir() and not _is_reparse(archive):
        for item in sorted(archive.iterdir(), key=lambda value: value.name.casefold()):
            if item.is_file() and item.name != "README.md":
                advisories.append({
                    "code": "legacy-archive-file",
                    "path": item.relative_to(root).as_posix(),
                    "message": "旧式归档仍散放在 4_archive/ 根；后续确认后迁入 legacy/",
                })

    _append_dead_reference_issues(doc, root, errors)

    return {"errors": errors, "advisories": advisories}


def _print_human(report: dict[str, list[dict[str, str]]]) -> None:
    for item in report["advisories"]:
        print(
            f"[project-structure] ADVISORY 提醒 {item['path']}: {item['message']}",
            file=sys.stderr,
        )
    for item in report["errors"]:
        print(
            f"[project-structure] BLOCKED 未完成 {item['path']}: {item['message']}",
            file=sys.stderr,
        )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=_repo_root())
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--pre-commit", action="store_true")
    args = parser.parse_args()
    try:
        report = inspect_project(args.root)
    except Exception as exc:
        print(
            f"[project-structure] BLOCKED 未完成：结构检查器失败并安全阻断。技术详情：{exc}",
            file=sys.stderr,
        )
        return 2
    if args.json:
        print(json.dumps(report, ensure_ascii=False, indent=2))
    else:
        _print_human(report)
    return 2 if report["errors"] else 0


if __name__ == "__main__":
    raise SystemExit(main())
