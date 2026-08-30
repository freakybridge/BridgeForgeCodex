#!/usr/bin/env python3
"""pre-commit hook: validate bridgeforge-codex skill metadata and loading shape.

Scope:
- Only checks the factory source tree `skills/<name>/SKILL.md`.
- Downstream projects normally do not keep common skills in repo root `skills/`,
  so this hook self-gates to no-op there.

Hard gates cover discoverability plus unsafe context growth: required metadata,
single-line descriptions <= 500 chars, SKILL.md <= 500 lines, live one-level
`references/` links with no orphan Markdown files, and bridgeforge-codex invocation metadata. Descriptions
over 300 chars are soft warnings.
"""
from __future__ import annotations

import re
import sys
import tomllib
from collections.abc import Iterable
from pathlib import Path

try:
    sys.stdout.reconfigure(encoding="utf-8")  # type: ignore[attr-defined]
    sys.stderr.reconfigure(encoding="utf-8")  # type: ignore[attr-defined]
except Exception:
    pass

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
SKILLS_DIR = REPO_ROOT / "skills"
BOM = b"\xef\xbb\xbf"
DESCRIPTION_WARN_CHARS = 300
DESCRIPTION_MAX_CHARS = 500
SKILL_MAX_LINES = 500
CATALOG_DESCRIPTION_MAX_CHARS = 4_000
MD_LINK_RE = re.compile(r"\[[^\]]+\]\(([^)]+\.md(?:#[^)]*)?)\)")
AGENT_NAME_RE = re.compile(r"^[a-z][a-z0-9-]*$")
DELEGATION_CUE_RE = re.compile(r"(?:显式)?(?:分派|委派)给", re.IGNORECASE)
EXPLICIT_AGENT_RE = re.compile(
    r"(?:显式)?(?:分派|委派)给\s*`([a-z][a-z0-9-]*)`",
    re.IGNORECASE,
)
UNNAMED_AGENT_RE = re.compile(
    r"(?:必须先由|让|交给)\s*(?:一个|某个)?\s*(?:独立|子)\s*agent"
    r"|使用一次匹配职责的子\s*agent",
    re.IGNORECASE,
)
BUILTIN_AGENT_NAMES = frozenset({"default", "worker", "explorer"})
AGENT_ROLE_MARKER = "agent-role:"


def load_agent_names(agent_dirs: Iterable[Path]) -> tuple[set[str], list[str]]:
    names = set(BUILTIN_AGENT_NAMES)
    issues: list[str] = []
    for agent_dir in agent_dirs:
        if not agent_dir.exists():
            continue
        if not agent_dir.is_dir():
            issues.append(f"{AGENT_ROLE_MARKER} Agent path is not a directory: {agent_dir}")
            continue
        for path in sorted(agent_dir.glob("*.toml")):
            if path.is_symlink() or bool(getattr(path, "is_junction", lambda: False)()):
                issues.append(f"{AGENT_ROLE_MARKER} reparse Agent file is not allowed: {path}")
                continue
            try:
                payload = tomllib.loads(path.read_text(encoding="utf-8"))
            except Exception as exc:
                issues.append(f"{AGENT_ROLE_MARKER} cannot parse {path}: {exc}")
                continue
            name = payload.get("name")
            if not isinstance(name, str) or not AGENT_NAME_RE.fullmatch(name):
                issues.append(f"{AGENT_ROLE_MARKER} invalid Agent name in {path}: {name!r}")
                continue
            names.add(name)
    return names, issues


def _agent_role_findings(text: str, known_agent_names: set[str]) -> list[str]:
    findings: list[str] = []
    for match in EXPLICIT_AGENT_RE.finditer(text):
        role = match.group(1)
        if role not in known_agent_names:
            line = text.count("\n", 0, match.start()) + 1
            findings.append(
                f"{AGENT_ROLE_MARKER} line {line} references unknown Agent role {role!r}"
            )

    for match in DELEGATION_CUE_RE.finditer(text):
        sentence_start = max(
            text.rfind("\n", 0, match.start()),
            text.rfind("。", 0, match.start()),
        )
        if "禁止" in text[sentence_start + 1 : match.start()]:
            continue
        following = text[match.end() : match.end() + 96].lstrip()
        if not re.match(r"`[a-z][a-z0-9-]*`", following):
            line = text.count("\n", 0, match.start()) + 1
            findings.append(
                f"{AGENT_ROLE_MARKER} line {line} delegates without an explicit Agent role"
            )

    for match in UNNAMED_AGENT_RE.finditer(text):
        line = text.count("\n", 0, match.start()) + 1
        findings.append(
            f"{AGENT_ROLE_MARKER} line {line} uses a generic Agent label instead of a role"
        )
    return list(dict.fromkeys(findings))


def _parse_frontmatter(path: Path) -> tuple[dict[str, str], list[str]]:
    issues: list[str] = []
    try:
        data = path.read_bytes()
    except Exception as exc:
        return {}, [f"cannot read file: {exc}"]

    if data.startswith(BOM):
        issues.append("starts with UTF-8 BOM; frontmatter must start at byte 0 with ---")
        data = data[len(BOM) :]

    try:
        text = data.decode("utf-8")
    except UnicodeDecodeError as exc:
        return {}, [f"not valid UTF-8: {exc}"]

    lines = text.splitlines()
    if not lines or lines[0].strip() != "---":
        return {}, issues + ["missing opening frontmatter line ---"]

    close_idx: int | None = None
    for i, line in enumerate(lines[1:], start=1):
        if line.strip() == "---":
            close_idx = i
            break
    if close_idx is None:
        return {}, issues + ["missing closing frontmatter line ---"]

    meta: dict[str, str] = {}
    for raw in lines[1:close_idx]:
        if not raw.strip() or raw.lstrip().startswith("#"):
            continue
        if raw[0].isspace():
            continue
        if ":" not in raw:
            issues.append(f"invalid frontmatter line: {raw}")
            continue
        key, value = raw.split(":", 1)
        meta[key.strip()] = value.strip()
    return meta, issues


def _validate_skill(
    skill_file: Path,
    expected_name: str | None = None,
    *,
    report_root: Path = REPO_ROOT,
    known_agent_names: set[str] | None = None,
    agent_role_warnings: bool = False,
) -> tuple[list[str], list[str]]:
    rel = skill_file.relative_to(report_root).as_posix()
    meta, issues = _parse_frontmatter(skill_file)
    warnings: list[str] = []
    expected_name = expected_name or skill_file.parent.name

    if not meta:
        return [f"{rel}: {issue}" for issue in issues], warnings

    name = meta.get("name", "")
    if name != expected_name:
        issues.append(f"name must be {expected_name!r}, got {name!r}")

    description = meta.get("description", "")
    if not description:
        issues.append("description is required")
    elif description in {"|", ">", "|-", ">-"}:
        issues.append("description must be a compact single line, not a YAML block scalar")
    elif len(description) > DESCRIPTION_MAX_CHARS:
        issues.append(f"description exceeds {DESCRIPTION_MAX_CHARS} chars ({len(description)})")
    elif len(description) > DESCRIPTION_WARN_CHARS:
        warnings.append(f"description exceeds recommended {DESCRIPTION_WARN_CHARS} chars ({len(description)})")

    if "user-invocable" in meta:
        issues.append("use current user_invocable, not user-invocable")

    has_invocation = "user_invocable" in meta or "argument" in meta
    if has_invocation:
        if meta.get("user_invocable", "").lower() != "true":
            issues.append("invocation metadata requires user_invocable: true")
        if not meta.get("argument", ""):
            issues.append(
                "invocation metadata requires argument; "
                "use `argument: 无` for no-argument skills"
            )

    try:
        text = skill_file.read_text(encoding="utf-8")
    except Exception as exc:
        issues.append(f"cannot read body: {exc}")
        text = ""
    line_count = len(text.splitlines())
    if line_count > SKILL_MAX_LINES:
        issues.append(f"SKILL.md exceeds {SKILL_MAX_LINES} lines ({line_count}); split conditional detail into references/")

    if known_agent_names is not None:
        role_text = text
        lines = text.splitlines()
        if lines and lines[0].strip() == "---":
            for index, line in enumerate(lines[1:], start=1):
                if line.strip() == "---":
                    role_text = "\n" * (index + 1) + "\n".join(lines[index + 1 :])
                    break
        role_findings = _agent_role_findings(role_text, known_agent_names)
        if agent_role_warnings:
            warnings.extend(role_findings)
        else:
            issues.extend(role_findings)

    linked_references: dict[Path, str] = {}
    for target in MD_LINK_RE.findall(text):
        clean = target.split("#", 1)[0].strip()
        parts = Path(clean).parts
        # Only a skill's own one-level references/ directory is a packaged
        # resource. Links to a downstream project's docs or <agent-dir>
        # placeholders are usage examples, not files in this factory skill.
        if not parts or parts[0].lower() != "references":
            continue
        if len(parts) > 2:
            issues.append(f"reference nesting must stay one level deep: {clean}")
            continue
        resolved = (skill_file.parent / clean).resolve()
        if not resolved.exists():
            issues.append(f"dead markdown reference: {clean}")
            continue
        linked_references[resolved] = clean

    references_dir = skill_file.parent / "references"
    if references_dir.is_dir():
        packaged_references = {
            path.resolve(): path.relative_to(skill_file.parent).as_posix()
            for path in references_dir.glob("*.md")
            if path.is_file()
        }
        for resolved, clean in sorted(
            packaged_references.items(),
            key=lambda item: item[1],
        ):
            if resolved not in linked_references:
                issues.append(
                    f"orphan markdown reference: {clean}; "
                    "link it from SKILL.md with an explicit read condition"
                )

    if known_agent_names is not None:
        for resolved, clean in sorted(
            linked_references.items(),
            key=lambda item: item[1],
        ):
            try:
                reference_text = resolved.read_text(encoding="utf-8")
            except Exception as exc:
                issues.append(f"cannot read linked reference {clean}: {exc}")
                continue
            role_findings = [
                f"{clean}: {finding}"
                for finding in _agent_role_findings(
                    reference_text,
                    known_agent_names,
                )
            ]
            if agent_role_warnings:
                warnings.extend(role_findings)
            else:
                issues.extend(role_findings)

    prefix = f"{rel}: "
    return [prefix + issue for issue in issues], [prefix + warning for warning in warnings]


def validate_skill_tree(
    skills_dir: Path,
    *,
    known_agent_names: set[str] | None = None,
    agent_role_warnings: bool = False,
) -> tuple[list[str], list[str]]:
    issues: list[str] = []
    warnings: list[str] = []
    if not skills_dir.is_dir():
        return issues, warnings
    report_root = skills_dir.parent
    for entry in sorted(skills_dir.iterdir()):
        relative = entry.relative_to(report_root).as_posix()
        is_reparse = entry.is_symlink() or bool(
            getattr(entry, "is_junction", lambda: False)()
        )
        if is_reparse:
            issues.append(f"{relative}: reparse entries are not allowed in the skill tree")
            continue
        if not entry.is_dir():
            issues.append(f"{relative}: skill root may contain directories only")
            continue
        skill_file = entry / "SKILL.md"
        if not skill_file.is_file():
            issues.append(f"{relative}: missing SKILL.md")
            continue
        for child in entry.rglob("*"):
            if child.is_symlink() or bool(
                getattr(child, "is_junction", lambda: False)()
            ):
                issues.append(
                    f"{child.relative_to(report_root).as_posix()}: "
                    "reparse entries are not allowed in the skill tree"
                )
        skill_issues, skill_warnings = _validate_skill(
            skill_file,
            report_root=report_root,
            known_agent_names=known_agent_names,
            agent_role_warnings=agent_role_warnings,
        )
        issues.extend(skill_issues)
        warnings.extend(skill_warnings)
    return issues, warnings


def main() -> int:
    try:
        if not SKILLS_DIR.is_dir():
            return 0

        issues: list[str] = []
        warnings: list[str] = []
        agent_dir = REPO_ROOT / "templates" / "agents"
        if not agent_dir.is_dir():
            issues.append(f"{AGENT_ROLE_MARKER} factory Agent directory is missing: {agent_dir}")
        known_agent_names, agent_issues = load_agent_names((agent_dir,))
        issues.extend(agent_issues)
        tree_issues, tree_warnings = validate_skill_tree(
            SKILLS_DIR,
            known_agent_names=known_agent_names,
        )
        issues.extend(tree_issues)
        warnings.extend(tree_warnings)

        root_skill = REPO_ROOT / "SKILL.md"
        if root_skill.exists():
            root_issues, root_warnings = _validate_skill(root_skill, "bridgeforge-codex")
            issues.extend(root_issues)
            warnings.extend(root_warnings)

        catalog_files = list(sorted(SKILLS_DIR.glob("*/SKILL.md")))
        if root_skill.exists():
            catalog_files.append(root_skill)
        catalog_chars = sum(len(_parse_frontmatter(path)[0].get("description", "")) for path in catalog_files)
        if catalog_chars > CATALOG_DESCRIPTION_MAX_CHARS:
            issues.append(
                f"skill catalog descriptions exceed {CATALOG_DESCRIPTION_MAX_CHARS} chars "
                f"({catalog_chars}); shorten discovery metadata"
            )

        for warning in warnings:
            print(f"[skill-metadata] warning: {warning}", file=sys.stderr)

        if not issues:
            return 0

        print("[skill-metadata] pre-commit 硬拦: 通用 skill frontmatter 无效, 提交被阻断", file=sys.stderr)
        for issue in issues:
            print(f"[skill-metadata]   {issue}", file=sys.stderr)
        print(
            "[skill-metadata] 修法: 使用标准 name/description；若保留旧 invocation 字段则必须成对完整，"
            "并缩短入口或把低频细节移到一层 references/。",
            file=sys.stderr,
        )
        return 2
    except Exception as exc:
        print(f"[skill-metadata] internal gate failure: {type(exc).__name__}: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
