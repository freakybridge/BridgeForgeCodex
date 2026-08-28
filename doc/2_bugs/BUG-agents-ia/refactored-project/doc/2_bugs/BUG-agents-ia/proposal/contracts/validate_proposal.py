from __future__ import annotations

import hashlib
import importlib.util
import io
import json
import re
import shutil
import subprocess
import sys
import tempfile
import unittest
import venv
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path


HERE = Path(__file__).resolve().parent
PROPOSAL = HERE.parent
REPO = next(parent for parent in HERE.parents if (parent / ".git").exists())
OVERLAY_PATH = HERE / "instruction-contract.json"
ZERO_HASH = "sha256:" + "0" * 64
HOST_PYTHON = sys.executable


def load_module(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


def read_bytes(path: Path) -> bytes:
    data = path.read_bytes()
    if data.startswith(b"\xef\xbb\xbf"):
        raise AssertionError(f"UTF-8 BOM: {path}")
    return data


def read_text(path: Path) -> str:
    return read_bytes(path).decode("utf-8").replace("\r\n", "\n").replace("\r", "\n")


def sha(payload: bytes) -> str:
    return "sha256:" + hashlib.sha256(payload).hexdigest()


def git_sha(payload: bytes) -> str:
    return sha(payload.replace(b"\r\n", b"\n").replace(b"\r", b"\n"))


def managed_visible_tree_snapshot(root: Path) -> tuple[tuple[str, ...], dict[str, str]]:
    """Snapshot the project-visible transaction target; exclude Git, venv, and caches."""
    directories: list[str] = []
    files: dict[str, str] = {}
    for path in sorted(root.rglob("*")):
        relative = path.relative_to(root)
        if any(part in {".venv", ".git", "__pycache__"} for part in relative.parts):
            continue
        key = relative.as_posix()
        if path.is_dir():
            directories.append(key)
        elif path.is_file():
            files[key] = sha(path.read_bytes())
    return tuple(directories), files


def marker_region(text: str, begin: str, end: str) -> str:
    if text.count(begin) != 1 or text.count(end) != 1:
        raise AssertionError(f"marker count invalid: {begin} / {end}")
    start = text.index(begin)
    finish = text.find("\n", text.index(end, start))
    finish = len(text) if finish < 0 else finish + 1
    return text[start:finish]


def operative_markdown(text: str) -> str:
    """Return visible prose; fail closed on malformed comments or fences."""
    output: list[str] = []
    fence_char: str | None = None
    fence_length = 0
    html_comment = False
    for line in text.splitlines(keepends=True):
        raw = line.rstrip("\r\n")
        if fence_char is not None:
            if re.fullmatch(r" {0,3}" + re.escape(fence_char) + r"{" + str(fence_length) + r",}[ \t]*", raw):
                fence_char = None
                fence_length = 0
            continue
        if not html_comment:
            fence = re.fullmatch(r" {0,3}(`{3,}|~{3,})[^\r\n]*", raw)
            if fence:
                marker = fence.group(1)
                fence_char = marker[0]
                fence_length = len(marker)
                continue
        if not html_comment and (line.startswith("    ") or line.startswith("\t")):
            continue
        visible: list[str] = []
        cursor = 0
        while cursor < len(line):
            if html_comment:
                stop = line.find("-->", cursor)
                if stop < 0:
                    cursor = len(line)
                    break
                html_comment = False
                cursor = stop + 3
                continue
            start = line.find("<!--", cursor)
            if start < 0:
                visible.append(line[cursor:])
                break
            visible.append(line[cursor:start])
            html_comment = True
            cursor = start + 4
        visible_line = "".join(visible)
        visible_raw = visible_line.rstrip("\r\n")
        if not html_comment:
            fence = re.fullmatch(r" {0,3}(`{3,}|~{3,})[^\r\n]*", visible_raw)
            if fence:
                marker = fence.group(1)
                fence_char = marker[0]
                fence_length = len(marker)
                continue
        output.append(visible_line)
    if html_comment:
        raise AssertionError("semantic target contains an unclosed HTML comment")
    if fence_char is not None:
        raise AssertionError("semantic target contains an unclosed fenced code block")
    return "".join(output)


def visible_markdown_headings(text: str) -> list[tuple[int, str]]:
    records: list[tuple[int, str]] = []
    fence_char: str | None = None
    fence_length = 0
    html_comment = False
    for index, line in enumerate(text.splitlines(keepends=True)):
        raw = line.rstrip("\r\n")
        if fence_char is not None:
            if re.fullmatch(r" {0,3}" + re.escape(fence_char) + r"{" + str(fence_length) + r",}[ \t]*", raw):
                fence_char = None
                fence_length = 0
            continue
        if not html_comment:
            fence = re.fullmatch(r" {0,3}(`{3,}|~{3,})[^\r\n]*", raw)
            if fence:
                marker = fence.group(1)
                fence_char = marker[0]
                fence_length = len(marker)
                continue
        if not html_comment and (line.startswith("    ") or line.startswith("\t")):
            continue
        visible: list[str] = []
        cursor = 0
        had_comment = html_comment or "<!--" in line
        while cursor < len(line):
            if html_comment:
                stop = line.find("-->", cursor)
                if stop < 0:
                    cursor = len(line)
                    break
                html_comment = False
                cursor = stop + 3
                continue
            start = line.find("<!--", cursor)
            if start < 0:
                visible.append(line[cursor:])
                break
            visible.append(line[cursor:start])
            html_comment = True
            cursor = start + 4
        candidate = "".join(visible).rstrip("\r\n")
        if not had_comment and re.match(r"^#{1,6} ", candidate):
            records.append((index, candidate))
    if html_comment:
        raise AssertionError("semantic target contains an unclosed HTML comment")
    if fence_char is not None:
        raise AssertionError("semantic target contains an unclosed fenced code block")
    return records


def markdown_section(text: str, heading: str) -> str:
    lines = text.splitlines(keepends=True)
    records = visible_markdown_headings(text)
    matches = [index for index, candidate in records if candidate == heading]
    if len(matches) != 1:
        raise AssertionError(f"semantic target section is not unique: {heading}")
    start = matches[0]
    level = len(heading) - len(heading.lstrip("#"))
    stop = len(lines)
    for index, candidate in records:
        if index <= start:
            continue
        candidate_level = len(candidate) - len(candidate.lstrip("#"))
        if candidate_level <= level:
            stop = index
            break
    return "".join(lines[start:stop])


def require_adjacent_comment(text: str, heading: str, expected: str) -> None:
    section = markdown_section(text, heading)
    lines = section.splitlines()
    following = [line.strip() for line in lines[1:] if line.strip()]
    wanted = f"<!-- {expected} -->"
    if not following or following[0] != wanted or text.count(wanted) != 1:
        raise AssertionError(f"required project placeholder comment is not exact and adjacent: {heading}")


def proposal_payload(asset: dict) -> bytes:
    return read_bytes(PROPOSAL / asset["proposal_source"])


def manifest_asset(asset: dict) -> dict:
    return {key: value for key, value in asset.items() if key != "proposal_source"}


def candidate_instruction_source() -> bytes:
    source = read_text(REPO / "templates/hooks/instruction_source_check.py")
    headings = ["## BridgeForge 公共区", "## 1 先找对位置", "## 2 交付与证据", "## 3 环境与安全", "## 4 任务控制与排障", "## 5 协作与项目资料", "## 6 版本与升级"]
    block = "PUBLIC_REQUIRED_HEADINGS = (\n" + "".join(f"    {heading!r},\n" for heading in headings) + ")"
    patched, count = re.subn(r"PUBLIC_REQUIRED_HEADINGS = \(\n.*?\n\)", block, source, count=1, flags=re.DOTALL)
    if count != 1:
        raise AssertionError("candidate instruction hook patch count is not one")
    source = patched
    visible_headings = '''def _visible_heading_positions(
    text: str,
    headings: tuple[str, ...],
) -> dict[str, list[int]]:
    configured = set(headings)
    matches = {heading: [] for heading in headings}
    fence_char: str | None = None
    fence_length = 0
    html_comment = False
    offset = 0
    for line in text.splitlines(keepends=True):
        raw = line.rstrip("\\r\\n")
        if fence_char is not None:
            if re.fullmatch(r" {0,3}" + re.escape(fence_char) + r"{" + str(fence_length) + r",}[ \\t]*", raw):
                fence_char = None
                fence_length = 0
            offset += len(line)
            continue
        if not html_comment:
            fence = re.fullmatch(r" {0,3}(`{3,}|~{3,})[^\\r\\n]*", raw)
            if fence:
                marker = fence.group(1)
                fence_char = marker[0]
                fence_length = len(marker)
                offset += len(line)
                continue
        if not html_comment and (line.startswith("    ") or line.startswith("\\t")):
            offset += len(line)
            continue
        visible: list[str] = []
        cursor = 0
        had_comment = html_comment or "<!--" in line
        while cursor < len(line):
            if html_comment:
                stop = line.find("-->", cursor)
                if stop < 0:
                    cursor = len(line)
                    break
                html_comment = False
                cursor = stop + 3
                continue
            start = line.find("<!--", cursor)
            if start < 0:
                visible.append(line[cursor:])
                break
            visible.append(line[cursor:start])
            html_comment = True
            cursor = start + 4
        stripped = "".join(visible).rstrip("\\r\\n")
        if not html_comment:
            fence = re.fullmatch(r" {0,3}(`{3,}|~{3,})[^\\r\\n]*", stripped)
            if fence:
                marker = fence.group(1)
                fence_char = marker[0]
                fence_length = len(marker)
                offset += len(line)
                continue
        heading = stripped.lstrip(" ")
        if not had_comment and len(stripped) - len(heading) <= 3 and heading in configured:
            matches[heading].append(offset)
        offset += len(line)
    if html_comment:
        raise ValueError("AGENTS contains an unclosed HTML comment")
    if fence_char is not None:
        raise ValueError("AGENTS contains an unclosed fenced code block")
    return matches
'''
    source, count = re.subn(
        r"def _visible_heading_positions\(\n.*?\n    return matches\n",
        lambda _match: visible_headings,
        source,
        count=1,
        flags=re.DOTALL,
    )
    if count != 1:
        raise AssertionError("candidate instruction hook visible-heading patch count is not one")
    nested_specs = {
        "scripts/AGENTS.md": ("# scripts 目录指令", "scripts/**", sha(read_text(PROPOSAL / "factory/scripts/AGENTS.md").encode("utf-8"))),
        "skills/AGENTS.md": ("# skills 目录指令", "skills/**", sha(read_text(PROPOSAL / "factory/skills/AGENTS.md").encode("utf-8"))),
        "doc/2_bugs/AGENTS.md": ("# 工厂 Bug 文档指令", "doc/2_bugs/**", sha(read_text(PROPOSAL / "factory/doc/2_bugs/AGENTS.md").encode("utf-8"))),
    }
    factory_project = marker_region(read_text(PROPOSAL / "factory/AGENTS.md"), "<!-- BRIDGEFORGE:PROJECT:BEGIN -->", "<!-- BRIDGEFORGE:PROJECT:END -->")
    nested_contract = (
        "FACTORY_REQUIRED_NESTED = " + repr(nested_specs) + "\n"
        + "FACTORY_TEMPLATE_SHA256 = " + repr(sha(read_text(PROPOSAL / "template/AGENTS.md").encode("utf-8"))) + "\n"
        + "FACTORY_PROJECT_SHA256 = " + repr(sha(factory_project.encode("utf-8"))) + "\n"
    )
    anchor = 'PUBLIC_BEGIN = "<!-- BRIDGEFORGE:PUBLIC:BEGIN -->"'
    if source.count(anchor) != 1:
        raise AssertionError("candidate instruction hook nested constant anchor is not unique")
    source = source.replace(anchor, nested_contract + "\n" + anchor)
    factory_check = '''        root_parts = _zone_parts(text)
        if root_parts is None or "sha256:" + hashlib.sha256(root_parts[1].encode("utf-8")).hexdigest() != FACTORY_PROJECT_SHA256:
            issues.append("factory AGENTS project region drifted")
        for relative, contract in FACTORY_REQUIRED_NESTED.items():
            required_tokens = contract[:2]
            expected_sha256 = contract[2]
            nested_path = root / relative
            nested_text, nested_error = _read(nested_path, root)
            if nested_error:
                issues.append(f"factory nested instruction missing or invalid: {relative}: {nested_error}")
                continue
            assert nested_text is not None
            for token in required_tokens:
                if token not in nested_text:
                    issues.append(f"factory nested instruction contract missing: {relative}: {token}")
            if "sha256:" + hashlib.sha256(nested_text.encode("utf-8")).hexdigest() != expected_sha256:
                issues.append(f"factory nested instruction hash drifted: {relative}")
            if f"`{relative}`" not in text:
                issues.append(f"factory AGENTS nested index is missing: {relative}")
'''
    anchor = "        template_text, template_error = _read(template, root)"
    if source.count(anchor) != 1:
        raise AssertionError("candidate instruction hook factory check anchor is not unique")
    source = source.replace(anchor, factory_check + anchor)
    template_hash_anchor = "        elif template_text is not None:\n            try:"
    template_hash_check = "        elif template_text is not None:\n            if \"sha256:\" + hashlib.sha256(template_text.encode(\"utf-8\")).hexdigest() != FACTORY_TEMPLATE_SHA256:\n                issues.append(\"factory templates/AGENTS.md hash drifted\")\n            try:"
    if source.count(template_hash_anchor) != 1:
        raise AssertionError("candidate instruction hook template hash anchor is not unique")
    source = source.replace(template_hash_anchor, template_hash_check)
    staged_check = '''    if (root / "templates" / "AGENTS.md").is_file():
        staged_root = staged
        staged_parts = _zone_parts(staged_root)
        if staged_parts is None or "sha256:" + hashlib.sha256(staged_parts[1].encode("utf-8")).hexdigest() != FACTORY_PROJECT_SHA256:
            issues.append("staged factory AGENTS project region drifted")
        staged_template = _git_text(root, "INDEX", "templates/AGENTS.md")
        if staged_template is None or "sha256:" + hashlib.sha256(staged_template.encode("utf-8")).hexdigest() != FACTORY_TEMPLATE_SHA256:
            issues.append("staged factory templates/AGENTS.md hash drifted")
        for relative, contract in FACTORY_REQUIRED_NESTED.items():
            required_tokens = contract[:2]
            expected_sha256 = contract[2]
            staged_nested = _git_text(root, "INDEX", relative)
            if staged_nested is None:
                issues.append(f"staged factory nested instruction is missing: {relative}")
                continue
            for token in required_tokens:
                if token not in staged_nested:
                    issues.append(f"staged factory nested instruction contract missing: {relative}: {token}")
            if "sha256:" + hashlib.sha256(staged_nested.encode("utf-8")).hexdigest() != expected_sha256:
                issues.append(f"staged factory nested instruction hash drifted: {relative}")
            if f"`{relative}`" not in staged_root:
                issues.append(f"staged factory AGENTS nested index is missing: {relative}")
'''
    source = source.replace(
        "def _git_agents(root: Path, ref: str) -> str | None:\n    object_name = \":AGENTS.md\" if ref == \"INDEX\" else f\"{ref}:AGENTS.md\"",
        "def _git_text(root: Path, ref: str, relative: str) -> str | None:\n    object_name = f\":{relative}\" if ref == \"INDEX\" else f\"{ref}:{relative}\"",
    )
    source = source.replace(
        "    return result.stdout.replace(\"\\r\\n\", \"\\n\").replace(\"\\r\", \"\\n\") if result.returncode == 0 else None\n\n\ndef instruction_source_issues",
        "    return result.stdout.replace(\"\\r\\n\", \"\\n\").replace(\"\\r\", \"\\n\") if result.returncode == 0 else None\n\n\ndef _git_agents(root: Path, ref: str) -> str | None:\n    return _git_text(root, ref, \"AGENTS.md\")\n\n\ndef instruction_source_issues",
    )
    source = source.replace(
        "def _staged_agents_issues(root: Path) -> list[str]:\n    staged = _git_agents(root, \"INDEX\")\n    if staged is None or staged == _git_agents(root, \"HEAD\"):\n        return []\n    return _root_agents_issues(\n        staged,\n        root,\n        label=\"staged AGENTS.md\",\n    )",
        "def _staged_agents_issues(root: Path) -> list[str]:\n    staged = _git_agents(root, \"INDEX\")\n    if staged is None:\n        return []\n    root_changed = staged != _git_agents(root, \"HEAD\")\n    factory = (root / \"templates\" / \"AGENTS.md\").is_file()\n    if not root_changed and not factory:\n        return []\n    issues = _root_agents_issues(staged, root, label=\"staged AGENTS.md\") if root_changed else []\n" + staged_check + "    return issues",
    )
    return source.encode("utf-8")


def candidate_baseline_source() -> bytes:
    source = read_text(REPO / "templates/scripts/current_baseline.py")
    anchor = '                raise BaselineError(f"asset {asset_id} region ownership is invalid")'
    policy_check = '''
            missing_marker = region.get("missing_marker", "fail-closed")
            if missing_marker not in {"fail-closed", "append"}:
                raise BaselineError(f"asset {asset_id} has invalid missing-marker policy")
            if missing_marker == "append" and (
                asset_id != "root.readme.bridgeforge-public" or target != "README.md"
            ):
                raise BaselineError(f"asset {asset_id} is not allowed to append a missing region")'''
    if source.count(anchor) != 1:
        raise AssertionError("candidate baseline region policy anchor is not unique")
    return source.replace(anchor, anchor + policy_check).encode("utf-8")


def candidate_syncer_source() -> bytes:
    source = read_text(REPO / "scripts/bridgeforge_codex_project_sync.py")
    replacement = '''def _replace_region(source: bytes, current: bytes | None, region: dict[str, Any]) -> bytes:
    missing_marker = str(region.get("missing_marker", "fail-closed"))
    if missing_marker not in {"fail-closed", "append"}:
        raise SyncBlocked(f"unsupported missing-marker policy: {missing_marker}")
    if current is None:
        return _git_blob_bytes(source)
    begin = str(region["begin"])
    end = str(region["end"])
    def raw_span(payload: bytes) -> tuple[int, int]:
        lines = payload.splitlines(keepends=True)
        begin_bytes = begin.encode("utf-8")
        end_bytes = end.encode("utf-8")
        starts = [index for index, line in enumerate(lines) if line.rstrip(b"\\r\\n") == begin_bytes]
        stops = [index for index, line in enumerate(lines) if line.rstrip(b"\\r\\n") == end_bytes]
        if len(starts) != 1 or len(stops) != 1 or starts[0] >= stops[0]:
            raise SyncBlocked(f"managed markers are missing, duplicated, or reversed: {begin} / {end}")
        return sum(len(line) for line in lines[:starts[0]]), sum(len(line) for line in lines[:stops[0] + 1])

    source_start, source_stop = raw_span(source)
    source_block = source[source_start:source_stop]
    if missing_marker == "fail-closed":
        current_start, current_stop = raw_span(current)
        return current[:current_start] + source_block + current[current_stop:]

    lines = current.splitlines(keepends=True)
    begin_bytes = begin.encode("utf-8")
    end_bytes = end.encode("utf-8")
    counts = (
        sum(line.rstrip(b"\\r\\n") == begin_bytes for line in lines),
        sum(line.rstrip(b"\\r\\n") == end_bytes for line in lines),
    )
    if counts == (0, 0):
        if not current:
            return source_block
        newline = b"\\r\\n" if b"\\r\\n" in current and b"\\n" not in current.replace(b"\\r\\n", b"") else b"\\n"
        separator = b"" if current.endswith(newline + newline) else newline if current.endswith(newline) else newline + newline
        return current + separator + source_block
    current_start, current_stop = raw_span(current)
    return current[:current_start] + source_block + current[current_stop:]
'''
    patched, count = re.subn(
        r"def _replace_region\(source: bytes, current: bytes \| None, region: dict\[str, Any\]\) -> bytes:\n.*?\n\ndef _preserve_selected_region",
        lambda _match: replacement + "\n\ndef _preserve_selected_region",
        source,
        count=1,
        flags=re.DOTALL,
    )
    if count != 1:
        raise AssertionError("candidate syncer region patch anchor is not unique")
    return patched.encode("utf-8")


def build_candidate_root(temp: Path, overlay: dict) -> tuple[dict, object]:
    shutil.copytree(REPO / "templates", temp / "templates", dirs_exist_ok=True)
    baseline_payload = candidate_baseline_source()
    syncer_payload = candidate_syncer_source()
    expected = overlay["candidate_implementation"]
    if expected["current_baseline_sha256"] != ZERO_HASH and sha(baseline_payload) != expected["current_baseline_sha256"]:
        raise AssertionError("candidate current_baseline hash mismatch")
    if expected["project_sync_sha256"] != ZERO_HASH and sha(syncer_payload) != expected["project_sync_sha256"]:
        raise AssertionError("candidate project synchronizer hash mismatch")
    (temp / "templates/scripts/current_baseline.py").write_bytes(baseline_payload)
    (temp / "scripts").mkdir(parents=True, exist_ok=True)
    (temp / "scripts/bridgeforge_codex_project_sync.py").write_bytes(syncer_payload)
    baseline = load_module("candidate_current_baseline", temp / "templates/scripts/current_baseline.py")
    for asset in overlay["asset_replacements"]:
        target = temp / asset["source"]
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(proposal_payload(asset))
    for entry in overlay["pointer_migrations"]:
        if not entry["target"].startswith("templates/"):
            continue
        target = temp / entry["target"]
        target.write_bytes(apply_pointer_entry(entry, target.read_bytes()))
        if entry.get("mirror"):
            mirror = temp / entry["mirror"]
            mirror.parent.mkdir(parents=True, exist_ok=True)
            mirror.write_bytes(target.read_bytes())
    for generated in overlay["generated_sources"]:
        if generated["generator"] != "replace-public-required-headings":
            raise AssertionError(f"unknown generated source: {generated['generator']}")
        payload = candidate_instruction_source()
        target = temp / generated["source"]
        target.write_bytes(payload)
        mirror = temp / generated["mirror"]
        mirror.parent.mkdir(parents=True, exist_ok=True)
        mirror.write_bytes(payload)
    contract = json.loads(read_text(REPO / overlay["base_contract"]))
    replacements = {asset["id"]: manifest_asset(asset) for asset in overlay["asset_replacements"]}
    kept = [asset for asset in contract["assets"] if asset["id"] not in replacements]
    contract["assets"] = kept + list(replacements.values())
    changed_sources = {
        entry["target"] for entry in overlay["pointer_migrations"] if entry["target"].startswith("templates/")
    } | {entry["source"] for entry in overlay["generated_sources"]} | {"templates/scripts/current_baseline.py"}
    for asset in contract["assets"]:
        if asset["source"] in changed_sources:
            payload = (temp / asset["source"]).read_bytes()
            asset["current_sha256"] = baseline._normalized_render_hash(payload, asset, temp)
    for path in (temp / "templates/managed-skeleton.json", temp / ".codex/managed-skeleton.json"):
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(contract, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n")
    loaded = baseline.load_contract(temp / "templates/managed-skeleton.json")
    return loaded, baseline


def candidate_skill_payload(overlay: dict) -> bytes:
    contract = json.loads(read_text(PROPOSAL / overlay["skill_contract"]))
    payload = read_bytes(REPO / contract["source"])
    text = payload.decode("utf-8")
    for replacement in contract["replacements"]:
        old = replacement["from"]
        if text.count(old) != 1:
            raise AssertionError(f"Skill patch anchor is not unique: {old}")
        text = text.replace(old, replacement["to"])
    candidate = text.encode("utf-8")
    if contract["expected_sha256"] != ZERO_HASH and sha(candidate) != contract["expected_sha256"]:
        raise AssertionError("complete candidate Skill hash mismatch")
    return candidate


def copy_candidate_repo(destination: Path) -> None:
    def ignored(_path: str, names: list[str]) -> set[str]:
        return {name for name in names if name in {".git", ".venv", ".runtime", "__pycache__"}}

    shutil.copytree(REPO, destination, ignore=ignored)


def build_candidate_factory(destination: Path, overlay: dict) -> tuple[dict, object]:
    copy_candidate_repo(destination)
    contract, baseline = build_candidate_root(destination, overlay)
    (destination / "AGENTS.md").write_bytes(read_bytes(PROPOSAL / "factory/AGENTS.md"))
    migration = load_module("candidate_factory_region", HERE / "region_migration.py")
    readme_asset = next(item for item in overlay["asset_replacements"] if item["id"] == "root.readme.bridgeforge-public")
    readme = destination / "README.md"
    readme.write_bytes(
        migration.replace_or_append_region(
            proposal_payload(readme_asset),
            readme.read_bytes() if readme.exists() else None,
            readme_asset["region"],
        )
    )
    hook_doc = destination / "doc/3_reference/codex-hook-signals.md"
    hook_doc.parent.mkdir(parents=True, exist_ok=True)
    hook_doc.write_bytes(read_bytes(PROPOSAL / "shared-docs/codex-hook-signals.md"))
    for relative in ("scripts/AGENTS.md", "skills/AGENTS.md", "doc/2_bugs/AGENTS.md"):
        source = PROPOSAL / "factory" / relative
        target = destination / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(read_bytes(source))
    (destination / "skills/bridgeforge-codex/SKILL.md").write_bytes(candidate_skill_payload(overlay))
    for entry in overlay["pointer_migrations"]:
        if entry["target"].startswith("templates/"):
            continue
        target = destination / entry["target"]
        target.write_bytes(apply_pointer_entry(entry, target.read_bytes()))
    old_guide = destination / "doc/3_reference/codex-project-operating-guide.md"
    if old_guide.exists():
        old_guide.unlink()
    return contract, baseline


def emit_hashes() -> None:
    overlay = json.loads(read_text(OVERLAY_PATH))
    draft = json.loads(json.dumps(overlay))
    for asset in draft["asset_replacements"]:
        asset["current_sha256"] = ZERO_HASH
        if "agents_zones" in asset:
            asset["agents_zones"]["public"]["current_sha256"] = ZERO_HASH
        if "region" in asset:
            asset["region"]["current_sha256"] = ZERO_HASH
    draft["candidate_implementation"]["current_baseline_sha256"] = ZERO_HASH
    draft["candidate_implementation"]["project_sync_sha256"] = ZERO_HASH
    with tempfile.TemporaryDirectory(prefix="agents-v6-hash-") as name:
        temp = Path(name)
        contract, baseline = build_candidate_root(temp, draft)
        by_id = {asset["id"]: asset for asset in contract["assets"]}
        for item in draft["asset_replacements"]:
            asset = by_id[item["id"]]
            payload = (temp / asset["source"]).read_bytes()
            print(f"{item['id']}.current_sha256={baseline._normalized_render_hash(payload, asset, temp)}")
            if "agents_zones" in asset:
                public = asset["agents_zones"]["public"]
                block = baseline._marker_block(payload, public["begin"], public["end"])
                print(f"{item['id']}.public_sha256={baseline._normalized_render_hash(block, asset, temp)}")
            if "region" in asset:
                projection = baseline.ownership_projection(asset, payload, temp)
                print(f"{item['id']}.region_sha256={projection.public_sha256}")
        for item in overlay["generated_sources"]:
            print(f"{item['id']}.current_sha256={by_id[item['id']]['current_sha256']}")
    for entry in overlay["pointer_migrations"]:
        payload = apply_pointer_entry(entry, read_bytes(REPO / entry["target"]))
        print(f"pointer:{entry['target']}={git_sha(payload)}")
    print(f"skill.candidate_sha256={sha(candidate_skill_payload(overlay))}")
    print(f"candidate.current_baseline_sha256={sha(candidate_baseline_source())}")
    print(f"candidate.project_sync_sha256={sha(candidate_syncer_source())}")


def check_manifest(overlay: dict) -> None:
    with tempfile.TemporaryDirectory(prefix="agents-v6-manifest-") as name:
        temp = Path(name)
        contract, baseline = build_candidate_root(temp, overlay)
        if contract["schema_version"] != overlay["candidate_manifest_schema"]:
            raise AssertionError("candidate manifest schema mismatch")
        by_id = {asset["id"]: asset for asset in contract["assets"]}
        for item in overlay["asset_replacements"]:
            asset = by_id[item["id"]]
            baseline.verify_contract_payload(asset, (temp / asset["source"]).read_bytes(), temp)
        for entry in overlay["pointer_migrations"]:
            if entry["target"].startswith("templates/"):
                asset = next(asset for asset in contract["assets"] if asset["source"] == entry["target"])
                if asset["current_sha256"] != entry["current_sha256"]:
                    raise AssertionError(f"candidate manifest pointer hash mismatch: {entry['target']}")
                baseline.verify_contract_payload(asset, (temp / entry["target"]).read_bytes(), temp)
        for generated in overlay["generated_sources"]:
            asset = by_id[generated["id"]]
            if asset["current_sha256"] != generated["current_sha256"]:
                raise AssertionError(f"candidate manifest generated hash mismatch: {generated['id']}")
            baseline.verify_contract_payload(asset, (temp / generated["source"]).read_bytes(), temp)
        root_asset = by_id["root.agents"]
        if root_asset["strategy"] != "whole" or "agents_zones" not in root_asset:
            raise AssertionError("root.agents is not real schema whole + agents_zones")
        readme_asset = by_id["root.readme.bridgeforge-public"]
        precommit_asset = by_id["codex.precommit"]
        if readme_asset["region"].get("missing_marker") != "append":
            raise AssertionError("README does not explicitly own missing-marker append")
        if precommit_asset["region"].get("missing_marker", "fail-closed") != "fail-closed":
            raise AssertionError("pre-commit region no longer defaults to fail-closed")
        invalid = json.loads(json.dumps(contract))
        next(item for item in invalid["assets"] if item["id"] == "root.readme.bridgeforge-public")["region"]["missing_marker"] = "unsafe"
        invalid_path = temp / "invalid-contract.json"
        invalid_path.write_text(json.dumps(invalid, ensure_ascii=False), encoding="utf-8")
        try:
            baseline.load_contract(invalid_path)
        except baseline.BaselineError:
            pass
        else:
            raise AssertionError("real candidate schema parser accepted an invalid missing-marker policy")
        unauthorized = json.loads(json.dumps(contract))
        next(item for item in unauthorized["assets"] if item["id"] == "codex.precommit")["region"]["missing_marker"] = "append"
        unauthorized_path = temp / "unauthorized-append-contract.json"
        unauthorized_path.write_text(json.dumps(unauthorized, ensure_ascii=False), encoding="utf-8")
        try:
            baseline.load_contract(unauthorized_path)
        except baseline.BaselineError:
            pass
        else:
            raise AssertionError("real candidate schema parser allowed a non-README region to append")


def candidate_instruction_hook(overlay: dict, temp: Path) -> object:
    root_asset = next(asset for asset in overlay["asset_replacements"] if asset["id"] == "root.agents")
    project = tuple(root_asset["agents_zones"]["project"]["required_headings"])
    if project != ("## 项目级专区", "### 项目架构红线", "### 项目业务与安全红线", "### 项目目录地图", "### 项目快速命令", "### 目录级 AGENTS 索引"):
        raise AssertionError("project heading schema changed")
    return load_module("candidate_instruction_source_check", temp / ".codex/hooks/instruction_source_check.py")


def check_instruction_entry(overlay: dict) -> None:
    with tempfile.TemporaryDirectory(prefix="agents-v6-hook-") as name:
        workspace = Path(name)
        downstream = workspace / "downstream"
        downstream.mkdir()
        contract, _baseline = build_candidate_root(workspace / "candidate", overlay)
        shutil.copytree(workspace / "candidate/.codex", downstream / ".codex")
        (downstream / "AGENTS.md").write_bytes(read_bytes(PROPOSAL / "template/AGENTS.md"))
        hook = candidate_instruction_hook(overlay, downstream)
        issues = hook.instruction_source_issues(downstream)
        if issues:
            raise AssertionError(f"real candidate instruction entry rejected downstream: {issues}")

        agents_path = downstream / "AGENTS.md"
        good_agents = agents_path.read_text(encoding="utf-8")
        heading = "### 项目架构红线"
        visibility_cases = {
            "HTML comment": good_agents.replace(heading, f"<!--\n{heading}\n-->", 1),
            "tilde fence": good_agents.replace(heading, f"~~~text\n{heading}\n~~~", 1),
            "unclosed HTML comment": good_agents.replace(heading, f"<!--\n{heading}", 1),
            "unclosed tilde fence": good_agents.replace(heading, f"~~~text\n{heading}", 1),
        }
        for label, broken_agents in visibility_cases.items():
            agents_path.write_text(broken_agents, encoding="utf-8", newline="")
            hidden_issues = hook.instruction_source_issues(downstream)
            if not hidden_issues:
                raise AssertionError(f"candidate instruction hook allowed project heading hidden by {label}")
        agents_path.write_text(good_agents, encoding="utf-8", newline="")
        compatible_cases = {
            "backtick fence": f"```text\nliteral <!-- inside fence\n```\n{heading}",
            "tilde fence": f"~~~text\nliteral <!-- inside fence\n~~~\n{heading}",
            "indented code": f"    literal <!-- inside indented code\n{heading}",
        }
        for label, insertion in compatible_cases.items():
            compatible_agents = good_agents.replace(heading, insertion, 1)
            agents_path.write_text(compatible_agents, encoding="utf-8", newline="")
            compatibility_issues = hook.instruction_source_issues(downstream)
            if compatibility_issues:
                raise AssertionError(f"candidate instruction hook rejected legal {label}: {compatibility_issues}")
        agents_path.write_text(good_agents, encoding="utf-8", newline="")
        if hook.instruction_source_issues(downstream):
            raise AssertionError("candidate instruction hook did not recover after visibility probes")

        factory = workspace / "factory"
        _factory_contract, _factory_baseline = build_candidate_factory(factory, overlay)
        hook = candidate_instruction_hook(overlay, factory)
        issues = hook.instruction_source_issues(factory)
        if issues:
            raise AssertionError(f"real candidate instruction entry rejected factory: {issues}")
        subprocess.run(["git", "init", "-q"], cwd=factory, check=True, capture_output=True)
        subprocess.run(["git", "add", "."], cwd=factory, check=True, capture_output=True)
        subprocess.run(
            ["git", "-c", "user.name=proposal", "-c", "user.email=proposal@example.invalid", "commit", "-qm", "candidate"],
            cwd=factory,
            check=True,
            capture_output=True,
        )
        hook_path = factory / ".codex/hooks/instruction_source_check.py"
        clean = subprocess.run([HOST_PYTHON, "-B", str(hook_path)], cwd=factory, capture_output=True, text=True)
        if clean.returncode != 0:
            raise AssertionError(f"candidate instruction CLI rejected clean factory: {clean.stderr}")
        cases = [
            (
                "AGENTS.md",
                lambda payload: payload.replace("bridgeforge-codex 工厂所有".encode("utf-8"), "bridgeforge-codex 工厂所X".encode("utf-8"), 1),
                "factory AGENTS project region drifted",
                "staged factory AGENTS project region drifted",
            ),
            ("templates/AGENTS.md", lambda payload: payload + b"\n# drift\n", "factory templates/AGENTS.md hash drifted", "staged factory templates/AGENTS.md hash drifted"),
            ("scripts/AGENTS.md", lambda payload: payload + b"\n# drift\n", "factory nested instruction hash drifted", "staged factory nested instruction hash drifted"),
            ("skills/AGENTS.md", lambda payload: payload + b"\n# drift\n", "factory nested instruction hash drifted", "staged factory nested instruction hash drifted"),
            ("doc/2_bugs/AGENTS.md", lambda payload: payload + b"\n# drift\n", "factory nested instruction hash drifted", "staged factory nested instruction hash drifted"),
        ]
        for relative, mutate, worktree_error, staged_error in cases:
            path = factory / relative
            good = path.read_bytes()
            broken = mutate(good)
            if broken == good:
                raise AssertionError(f"factory gate mutation did not change payload: {relative}")
            path.write_bytes(broken)
            worktree = subprocess.run([HOST_PYTHON, "-B", str(hook_path)], cwd=factory, capture_output=True, text=True)
            if worktree.returncode != 2 or worktree_error not in worktree.stderr:
                raise AssertionError(f"candidate factory worktree gate allowed drift: {relative}: {worktree.stderr}")
            path.write_bytes(good)
            path.write_bytes(broken)
            subprocess.run(["git", "add", relative], cwd=factory, check=True, capture_output=True)
            path.write_bytes(good)
            staged = subprocess.run([HOST_PYTHON, "-B", str(hook_path)], cwd=factory, capture_output=True, text=True)
            if staged.returncode != 2 or staged_error not in staged.stderr:
                raise AssertionError(f"candidate factory staged gate allowed restored-worktree bypass: {relative}: {staged.stderr}")
            subprocess.run(["git", "add", relative], cwd=factory, check=True, capture_output=True)

        deleted_relative = "doc/2_bugs/AGENTS.md"
        deleted_path = factory / deleted_relative
        deleted_good = deleted_path.read_bytes()
        deleted_path.unlink()
        deleted_worktree = subprocess.run([HOST_PYTHON, "-B", str(hook_path)], cwd=factory, capture_output=True, text=True)
        if deleted_worktree.returncode != 2 or "factory nested instruction missing or invalid" not in deleted_worktree.stderr:
            raise AssertionError("candidate factory worktree gate allowed deleted nested AGENTS")
        deleted_path.write_bytes(deleted_good)
        subprocess.run(["git", "rm", "-q", deleted_relative], cwd=factory, check=True, capture_output=True)
        deleted_path.parent.mkdir(parents=True, exist_ok=True)
        deleted_path.write_bytes(deleted_good)
        deleted_staged = subprocess.run([HOST_PYTHON, "-B", str(hook_path)], cwd=factory, capture_output=True, text=True)
        if deleted_staged.returncode != 2 or "staged factory nested instruction is missing" not in deleted_staged.stderr:
            raise AssertionError("candidate factory staged gate allowed restored-worktree deletion bypass")
        subprocess.run(["git", "add", deleted_relative], cwd=factory, check=True, capture_output=True)


def check_region_migration(overlay: dict) -> None:
    migration = load_module("candidate_region_migration", HERE / "region_migration.py")
    asset = next(asset for asset in overlay["asset_replacements"] if asset["id"] == "root.readme.bridgeforge-public")
    source = proposal_payload(asset)
    begin = asset["region"]["begin"].encode("utf-8")
    end = asset["region"]["end"].encode("utf-8")
    fixtures = [
        b"",
        b"# Demo",
        b"# Demo\n",
        b"# Demo\n\n\n",
        b"# Demo\r\n",
        b"# Demo\r\n\r\n\r\n",
        b"# Demo\n\n" + begin + b"\nold\n" + end + b"\n\nproject-tail\n",
        b"# Demo\r\n\r\n" + begin + b"\r\nold\r\n" + end + b"\r\n\r\nproject-tail\r\n",
    ]
    with tempfile.TemporaryDirectory(prefix="agents-v6-region-") as name:
        project = Path(name)
        candidate = project / "candidate"
        build_candidate_root(candidate, overlay)
        syncer = load_module("candidate_syncer_dispatch", candidate / "scripts/bridgeforge_codex_project_sync.py")
        for index, before in enumerate(fixtures):
            after = syncer._desired_payload(manifest_asset(asset), source, before, project)
            assert after is not None
            if begin not in before:
                if after[: len(before)] != before:
                    raise AssertionError(f"README fixture {index}: original bytes are not an exact prefix")
            else:
                old_start, old_stop = migration._marker_span(before, begin.decode(), end.decode())
                new_start, new_stop = migration._marker_span(after, begin.decode(), end.decode())
                if before[:old_start] != after[:new_start] or before[old_stop:] != after[new_stop:]:
                    raise AssertionError(f"README fixture {index}: marker outside bytes changed")
            second = syncer._desired_payload(manifest_asset(asset), source, after, project)
            if second != after:
                raise AssertionError(f"README fixture {index}: second apply is not byte-identical")
        invalid = [begin + b"\n", end + b"\n", begin + b"\n" + begin + b"\n" + end + b"\n", end + b"\n" + begin + b"\n"]
        for payload in invalid:
            try:
                syncer._desired_payload(manifest_asset(asset), source, payload, project)
            except syncer.SyncBlocked:
                continue
            raise AssertionError("invalid README markers did not fail closed")
        precommit = json.loads(read_text(REPO / "templates/managed-skeleton.json"))
        precommit_asset = next(item for item in precommit["assets"] if item["id"] == "codex.precommit")
        precommit_source = read_bytes(REPO / precommit_asset["source"])
        before = b"#!/bin/sh\necho project\n"
        try:
            syncer._desired_payload(precommit_asset, precommit_source, before, project)
        except syncer.SyncBlocked:
            pass
        else:
            raise AssertionError("pre-commit without markers did not remain fail-closed")
        if before != b"#!/bin/sh\necho project\n":
            raise AssertionError("pre-commit fail-closed fixture changed bytes")
        precommit_begin = precommit_asset["region"]["begin"].encode("utf-8")
        precommit_end = precommit_asset["region"]["end"].encode("utf-8")
        managed_fixtures = [
            b"#!/bin/sh\r\n# project-prefix\r\n" + precommit_begin + b"\r\nold\r\n" + precommit_end + b"\r\n# project-suffix",
            b"#!/bin/sh\n# project-prefix\n" + precommit_begin + b"\nold\n" + precommit_end + b"\n# project-suffix\n",
            b"#!/bin/sh\r\n# mixed-prefix\n" + precommit_begin + b"\r\nold\n" + precommit_end + b"\r\n# mixed-suffix",
        ]
        for index, managed_before in enumerate(managed_fixtures):
            old_start, old_stop = migration._marker_span(managed_before, precommit_begin.decode(), precommit_end.decode())
            managed_after = syncer._desired_payload(precommit_asset, precommit_source, managed_before, project)
            assert managed_after is not None
            new_start, new_stop = migration._marker_span(managed_after, precommit_begin.decode(), precommit_end.decode())
            if managed_before[:old_start] != managed_after[:new_start] or managed_before[old_stop:] != managed_after[new_stop:]:
                raise AssertionError(f"pre-commit fixture {index}: marker outside bytes changed")
            managed_second = syncer._desired_payload(precommit_asset, precommit_source, managed_after, project)
            if managed_second != managed_after:
                raise AssertionError(f"pre-commit fixture {index}: second replacement is not byte-identical")


def check_real_sync_plan_apply(overlay: dict) -> None:
    with tempfile.TemporaryDirectory(prefix="agents-v6-sync-") as name:
        root = Path(name)
        template_root = root / "factory"
        project_root = root / "downstream"
        project_root.mkdir()
        (project_root / "README.md").write_bytes(b"# Demo\n\n\n")
        venv.EnvBuilder(with_pip=False).create(project_root / ".venv")
        expected_python = project_root / ".venv/Scripts/python.exe"
        if not expected_python.exists():
            expected_python = project_root / ".venv/bin/python"
        build_candidate_root(template_root, overlay)
        syncer = load_module("candidate_syncer_transaction", template_root / "scripts/bridgeforge_codex_project_sync.py")
        syncer.sys.executable = str(expected_python)
        shutil.copy2(REPO / "VERSION", template_root / "VERSION")

        blocked_root = root / "blocked-region"
        blocked_root.mkdir()
        blocked_readme = b"# Broken\n\n<!-- BRIDGEFORGE:README:BEGIN -->\n"
        (blocked_root / "README.md").write_bytes(blocked_readme)
        venv.EnvBuilder(with_pip=False).create(blocked_root / ".venv")
        blocked_python = blocked_root / ".venv/Scripts/python.exe"
        if not blocked_python.exists():
            blocked_python = blocked_root / ".venv/bin/python"
        syncer.sys.executable = str(blocked_python)
        blocked_before = managed_visible_tree_snapshot(blocked_root)
        blocked_out = io.StringIO()
        with redirect_stdout(blocked_out), redirect_stderr(io.StringIO()):
            blocked_rc = syncer.main(["--project-root", str(blocked_root), "--template-root", str(template_root), "--mode", "init"])
        blocked = json.loads(blocked_out.getvalue())
        if blocked_rc == 0 or blocked.get("readiness") != "blocked" or "managed markers" not in str(blocked.get("error")):
            raise AssertionError(f"invalid README marker did not return a structured blocker: {blocked}")
        if managed_visible_tree_snapshot(blocked_root) != blocked_before:
            raise AssertionError("invalid README marker plan changed the managed visible tree")
        cli_before = managed_visible_tree_snapshot(blocked_root)
        cli = subprocess.run(
            [
                str(blocked_python),
                "-B",
                str(template_root / "scripts/bridgeforge_codex_project_sync.py"),
                "--project-root",
                str(blocked_root),
                "--template-root",
                str(template_root),
                "--mode",
                "init",
            ],
            capture_output=True,
            text=True,
        )
        try:
            cli_blocked = json.loads(cli.stdout)
        except json.JSONDecodeError as exc:
            raise AssertionError(f"candidate syncer CLI did not emit one JSON receipt: {cli.stdout!r}") from exc
        cli_error = str(cli_blocked.get("error", ""))
        expected_stderr = f"BLOCKED: {cli_error}\n"
        if cli.returncode != 2 or cli.stderr != expected_stderr or cli_blocked.get("readiness") != "blocked" or "managed markers" not in cli_error:
            raise AssertionError(f"candidate syncer CLI blocker contract failed: rc={cli.returncode} stdout={cli.stdout!r} stderr={cli.stderr!r}")
        if managed_visible_tree_snapshot(blocked_root) != cli_before:
            raise AssertionError("candidate syncer CLI blocker changed the managed visible tree")

        precommit_root = root / "blocked-precommit"
        precommit_root.mkdir()
        precommit_bytes = b"#!/bin/sh\necho project\n"
        venv.EnvBuilder(with_pip=False).create(precommit_root / ".venv")
        precommit_python = precommit_root / ".venv/Scripts/python.exe"
        if not precommit_python.exists():
            precommit_python = precommit_root / ".venv/bin/python"
        syncer.sys.executable = str(precommit_python)
        seed_plan_out = io.StringIO()
        with redirect_stdout(seed_plan_out), redirect_stderr(io.StringIO()):
            seed_rc = syncer.main(["--project-root", str(precommit_root), "--template-root", str(template_root), "--mode", "init"])
        seed_plan = json.loads(seed_plan_out.getvalue())
        if seed_rc != 0:
            raise AssertionError(f"pre-commit seed plan failed: {seed_plan}")
        seed_apply_out = io.StringIO()
        with redirect_stdout(seed_apply_out), redirect_stderr(io.StringIO()):
            seed_rc = syncer.main([
                "--project-root", str(precommit_root),
                "--template-root", str(template_root),
                "--mode", "init",
                "--apply",
                "--plan-fingerprint", seed_plan["aggregate_fingerprint"],
                "--confirmed-risk",
            ])
        seed_receipt = json.loads(seed_apply_out.getvalue())
        if seed_rc != 0 or seed_receipt.get("execution_status") != "completed":
            raise AssertionError(f"pre-commit seed apply failed: {seed_receipt}")
        (precommit_root / ".githooks/pre-commit").write_bytes(precommit_bytes)
        precommit_before = managed_visible_tree_snapshot(precommit_root)
        precommit_out = io.StringIO()
        with redirect_stdout(precommit_out), redirect_stderr(io.StringIO()):
            precommit_rc = syncer.main(["--project-root", str(precommit_root), "--template-root", str(template_root), "--mode", "update"])
        precommit_receipt = json.loads(precommit_out.getvalue())
        precommit_reason = str(precommit_receipt.get("error", "")) + str(precommit_receipt.get("blockers", []))
        if precommit_rc == 0 or precommit_receipt.get("readiness") != "blocked" or "managed markers" not in precommit_reason:
            raise AssertionError(f"markerless pre-commit did not return a structured blocker: {precommit_receipt}")
        if managed_visible_tree_snapshot(precommit_root) != precommit_before:
            raise AssertionError("markerless pre-commit plan changed the managed visible tree")

        syncer.sys.executable = str(expected_python)
        plan_before = managed_visible_tree_snapshot(project_root)
        plan_out = io.StringIO()
        with redirect_stdout(plan_out), redirect_stderr(io.StringIO()):
            rc = syncer.main(["--project-root", str(project_root), "--template-root", str(template_root), "--mode", "init"])
        plan = json.loads(plan_out.getvalue())
        if rc != 0 or plan["status"] != "planned" or plan["blockers"]:
            raise AssertionError(f"real sync plan failed: {plan}")
        if managed_visible_tree_snapshot(project_root) != plan_before:
            raise AssertionError("read-only plan changed the managed visible tree")
        apply_out = io.StringIO()
        with redirect_stdout(apply_out), redirect_stderr(io.StringIO()):
            rc = syncer.main([
                "--project-root", str(project_root),
                "--template-root", str(template_root),
                "--mode", "init",
                "--apply",
                "--plan-fingerprint", plan["aggregate_fingerprint"],
                "--confirmed-risk",
            ])
        receipt = json.loads(apply_out.getvalue())
        if rc != 0 or receipt.get("execution_status") != "completed" or not receipt.get("stamp_written_last"):
            raise AssertionError(f"real sync apply failed: {receipt}")
        if not (project_root / "README.md").read_bytes().startswith(b"# Demo\n\n\n"):
            raise AssertionError("real sync apply changed pre-existing README prefix")
        noop_before = managed_visible_tree_snapshot(project_root)
        noop_out = io.StringIO()
        with redirect_stdout(noop_out), redirect_stderr(io.StringIO()):
            rc = syncer.main(["--project-root", str(project_root), "--template-root", str(template_root), "--mode", "update"])
        noop = json.loads(noop_out.getvalue())
        if rc != 0 or noop["safe"] or noop["risk"] or noop["gaps"] or noop["blockers"]:
            raise AssertionError(f"real sync second plan is not no-op: {noop}")
        if managed_visible_tree_snapshot(project_root) != noop_before:
            raise AssertionError("no-op plan changed the managed visible tree")

        failure_root = root / "rollback-downstream"
        failure_root.mkdir()
        failure_readme = b"# Rollback demo\r\n\r\n\r\n"
        (failure_root / "README.md").write_bytes(failure_readme)
        venv.EnvBuilder(with_pip=False).create(failure_root / ".venv")
        failure_python = failure_root / ".venv/Scripts/python.exe"
        if not failure_python.exists():
            failure_python = failure_root / ".venv/bin/python"
        syncer.sys.executable = str(failure_python)
        failure_before_plan = managed_visible_tree_snapshot(failure_root)
        failure_plan_out = io.StringIO()
        with redirect_stdout(failure_plan_out), redirect_stderr(io.StringIO()):
            rc = syncer.main(["--project-root", str(failure_root), "--template-root", str(template_root), "--mode", "init"])
        failure_plan = json.loads(failure_plan_out.getvalue())
        if rc != 0:
            raise AssertionError(f"rollback fixture plan failed: {failure_plan}")
        if managed_visible_tree_snapshot(failure_root) != failure_before_plan:
            raise AssertionError("rollback fixture plan changed the managed visible tree")
        failure_before_apply = managed_visible_tree_snapshot(failure_root)
        original_atomic = syncer._atomic_write
        calls = {"count": 0}

        def fail_once(path, payload, staging_root):
            calls["count"] += 1
            if calls["count"] == 3:
                raise OSError("proposal injected write failure")
            return original_atomic(path, payload, staging_root)

        syncer._atomic_write = fail_once
        failure_out = io.StringIO()
        try:
            with redirect_stdout(failure_out), redirect_stderr(io.StringIO()):
                rc = syncer.main([
                    "--project-root", str(failure_root),
                    "--template-root", str(template_root),
                    "--mode", "init",
                    "--apply",
                    "--plan-fingerprint", failure_plan["aggregate_fingerprint"],
                    "--confirmed-risk",
                ])
        finally:
            syncer._atomic_write = original_atomic
        failure_receipt = json.loads(failure_out.getvalue())
        if rc != 2 or not failure_receipt.get("rollback_performed"):
            raise AssertionError(f"injected failure did not report rollback: {failure_receipt}")
        if managed_visible_tree_snapshot(failure_root) != failure_before_apply:
            raise AssertionError("rollback fixture did not restore the managed visible tree")


def apply_pointer_entry(entry: dict, source: bytes) -> bytes:
    if "replacement_source" in entry:
        return read_bytes(PROPOSAL / entry["replacement_source"])
    text = source.decode("utf-8")
    for replacement in entry["replacements"]:
        old = replacement["from"]
        if old not in text:
            raise AssertionError(f"pointer source missing in {entry['target']}: {old}")
        text = text.replace(old, replacement["to"])
        if old in text:
            raise AssertionError(f"pointer source remained in {entry['target']}: {old}")
        if replacement["to"] and replacement["to"] not in text:
            raise AssertionError(f"pointer destination missing in {entry['target']}: {replacement['to']}")
    return text.encode("utf-8")


def check_pointer_migrations(overlay: dict) -> None:
    patched: dict[str, bytes] = {}
    for entry in overlay["pointer_migrations"]:
        target = entry["target"]
        patched[target] = apply_pointer_entry(entry, read_bytes(REPO / target))
        if git_sha(patched[target]) != entry["current_sha256"]:
            raise AssertionError(f"pointer candidate hash mismatch: {target}")
        mirror = entry.get("mirror")
        if mirror:
            mirror_bytes = apply_pointer_entry(entry, read_bytes(REPO / mirror))
            if mirror_bytes != patched[target]:
                raise AssertionError(f"template/dogfood pointer migration differs: {target} / {mirror}")
    for generated in overlay["generated_sources"]:
        payload = candidate_instruction_source()
        if sha(payload) != generated["current_sha256"]:
            raise AssertionError(f"generated source hash mismatch: {generated['id']}")


def covered_lines(text: str, label: str) -> set[int]:
    covered: set[int] = set()
    for start_text, stop_text in re.findall(rf"`{re.escape(label)}:(\d+)(?:-(\d+))?`", text):
        start = int(start_text)
        stop = int(stop_text or start_text)
        covered.update(range(start, stop + 1))
    return covered


def parse_source_ref(reference: str) -> tuple[str, set[int]]:
    match = re.fullmatch(r"(.+?):(\d+)(?:-(\d+))?", reference)
    if match is None:
        raise AssertionError(f"invalid semantic source ref: {reference}")
    start = int(match.group(2))
    stop = int(match.group(3) or match.group(2))
    return match.group(1), set(range(start, stop + 1))


def check_semantic_contract(overlay: dict) -> None:
    contract = json.loads(read_text(PROPOSAL / overlay["semantic_contract"]))
    if contract.get("schema_version") != 1 or not isinstance(contract.get("items"), list):
        raise AssertionError("semantic contract schema is invalid")
    source_paths = {
        "AGENTS.md": REPO / "AGENTS.md",
        "templates/AGENTS.md": REPO / "templates/AGENTS.md",
        "codex-project-operating-guide.md": REPO / "doc/3_reference/codex-project-operating-guide.md",
        "skills/summary/SKILL.md": REPO / "skills/summary/SKILL.md",
    }
    for label, expected in contract.get("source_files", {}).items():
        if label not in source_paths or sha(read_bytes(source_paths[label])) != expected:
            raise AssertionError(f"semantic source file hash drifted: {label}")
    ids: set[str] = set()
    coverage: dict[str, dict[int, int]] = {}
    visibility_probe = operative_markdown(
        "错误正文\n"
        "<!-- 每个项目必须自建 CPython 3.11+ `.venv` -->\n"
        "```text\n禁止全局安装\n```\n"
        "~~~text\n必须保留完整调用链\n~~~\n"
        "    必须使用可验证入口\n"
    )
    for hidden in ("每个项目必须自建", "禁止全局安装", "必须保留完整调用链", "必须使用可验证入口"):
        if hidden in visibility_probe:
            raise AssertionError(f"semantic visibility filter allows hidden rule text: {hidden}")
    for malformed in ("正文\n<!-- 未闭合", "正文\n~~~text\n未闭合"):
        try:
            operative_markdown(malformed)
        except AssertionError:
            pass
        else:
            raise AssertionError("semantic visibility filter did not fail closed on malformed Markdown")
    compatible_markdown = {
        "backtick fence": "```text\nliteral <!-- inside fence\n```\n可见正文\n",
        "tilde fence": "~~~text\nliteral <!-- inside fence\n~~~\n可见正文\n",
        "indented code": "    literal <!-- inside indented code\n可见正文\n",
    }
    for label, sample in compatible_markdown.items():
        if operative_markdown(sample) != "可见正文\n":
            raise AssertionError(f"semantic visibility filter rejected or leaked legal {label}")
    hidden_sections = (
        "<!--\n## 隐藏章节\n-->\n正文\n",
        "~~~text\n## 隐藏章节\n~~~\n正文\n",
    )
    for sample in hidden_sections:
        try:
            markdown_section(sample, "## 隐藏章节")
        except AssertionError:
            pass
        else:
            raise AssertionError("semantic section lookup accepted a hidden heading")
    for item in contract["items"]:
        semantic_id = item["id"]
        if semantic_id in ids:
            raise AssertionError(f"duplicate semantic id: {semantic_id}")
        ids.add(semantic_id)
        for reference in item["source_refs"]:
            label, lines = parse_source_ref(reference)
            source_path = source_paths.get(label, REPO / label)
            source_lines = read_text(source_path).splitlines()
            if not lines or max(lines) > len(source_lines):
                raise AssertionError(f"semantic source ref is stale: {reference}")
            counts = coverage.setdefault(label, {})
            for line_number in lines:
                counts[line_number] = counts.get(line_number, 0) + 1
        for target in item["targets"]:
            text = (
                candidate_skill_payload(overlay).decode("utf-8")
                if target["path"] == "$candidate_skill"
                else read_text(PROPOSAL / target["path"])
            )
            for comment in target.get("required_adjacent_comments", []):
                require_adjacent_comment(text, comment["heading"], comment["text"])
            if target.get("section"):
                text = markdown_section(text, target["section"])
            for token in target.get("required_raw_all", []):
                if token not in text:
                    raise AssertionError(f"semantic raw target missing: {semantic_id}: {target['path']}: {token}")
            raw_line_positions: list[int] = []
            raw_lines = text.splitlines()
            for expected_line in target.get("required_raw_lines", []):
                matches = [index for index, line in enumerate(raw_lines) if line == expected_line]
                if len(matches) != 1:
                    raise AssertionError(f"semantic exact raw line is not unique: {semantic_id}: {target['path']}: {expected_line}")
                raw_line_positions.append(matches[0])
            if raw_line_positions != sorted(raw_line_positions):
                raise AssertionError(f"semantic exact raw lines are out of order: {semantic_id}: {target['path']}")
            text = operative_markdown(text)
            for token in target.get("required_all", []):
                if token not in text:
                    raise AssertionError(f"semantic target missing: {semantic_id}: {target['path']}: {token}")
            for pattern in target.get("required_regex", []):
                if re.search(pattern, text, flags=re.DOTALL) is None:
                    raise AssertionError(f"semantic target relation missing: {semantic_id}: {target['path']}: {pattern}")
    current_agents = read_text(REPO / "AGENTS.md").splitlines()
    current_template = read_text(REPO / "templates/AGENTS.md").splitlines()
    current_guide = read_text(REPO / "doc/3_reference/codex-project-operating-guide.md").splitlines()
    for number, line in enumerate(current_agents, 1):
        if re.search(r"必须|禁止|只能|不得|不允许", line) and coverage.get("AGENTS.md", {}).get(number) != 1:
            raise AssertionError(f"semantic contract must map current AGENTS redline exactly once: {number}")
    for number, line in enumerate(current_guide, 1):
        if re.search(r"必须|禁止|不得|不允许", line) and coverage.get("codex-project-operating-guide.md", {}).get(number) != 1:
            raise AssertionError(f"semantic contract must map current guide redline exactly once: {number}")
    root_begin = current_agents.index("<!-- BRIDGEFORGE:PUBLIC:BEGIN -->") + 1
    root_end = current_agents.index("<!-- BRIDGEFORGE:PUBLIC:END -->") + 1
    template_begin = current_template.index("<!-- BRIDGEFORGE:PUBLIC:BEGIN -->") + 1
    template_end = current_template.index("<!-- BRIDGEFORGE:PUBLIC:END -->") + 1
    if root_end - root_begin != template_end - template_begin:
        raise AssertionError("current factory/template public regions have different line topology")
    for number, line in enumerate(current_template, 1):
        if re.search(r"必须|禁止|只能|不得|不允许", line) is None:
            continue
        if template_begin <= number <= template_end:
            root_number = root_begin + number - template_begin
            if line != current_agents[root_number - 1]:
                raise AssertionError(f"current factory/template public redline differs: Template line {number}")
            count = coverage.get("AGENTS.md", {}).get(root_number)
        else:
            count = coverage.get("templates/AGENTS.md", {}).get(number)
        if count != 1:
            raise AssertionError(f"semantic contract must map current Template redline exactly once: {number}")
    required_active_blocks = {
        "AGENTS.md": ((80, 86), (177, 183)),
        "templates/AGENTS.md": ((162, 164),),
    }
    for label, ranges in required_active_blocks.items():
        for start, stop in ranges:
            for number in range(start, stop + 1):
                if coverage.get(label, {}).get(number) != 1:
                    raise AssertionError(f"semantic contract must map active source block exactly once: {label}:{number}")
    ledger = read_text(PROPOSAL / "semantic-migration-matrix.md")
    if "# 第十一版语义迁移账本" not in ledger or "| 映射 | 静态验证 | 安装 |" not in ledger:
        raise AssertionError("semantic ledger does not separate mapping/static/install status")
    for line in ledger.splitlines():
        if not line.startswith("|") or "现行来源" in line or line.startswith("|---"):
            continue
        columns = [column.strip() for column in line.strip("|").split("|")]
        if len(columns) >= 3 and re.search(r"(?:template|factory|README|shared-docs|contracts)/[^`|]*:\d+", columns[2]):
            raise AssertionError(f"semantic ledger owner uses a drift-prone proposal line number: {columns[2]}")


def check_active_pointer_surface(factory: Path, overlay: dict) -> None:
    roots = [
        factory / "templates",
        factory / ".codex/hooks",
        factory / ".codex/scripts",
        factory / ".codex/skills",
        factory / ".githooks",
        factory / "skills",
        factory / "scripts",
        factory / "doc/0_architecture",
        factory / "doc/3_reference",
    ]
    forbidden = tuple(sorted({
        replacement["from"]
        for entry in overlay["pointer_migrations"]
        for replacement in entry.get("replacements", [])
        if replacement.get("from")
    }))
    suffixes = {".md", ".py", ".ps1", ".json", ".yaml", ".yml", ".toml"}
    paths = [
        path
        for path in factory.iterdir()
        if path.is_file() and path.suffix.lower() in suffixes
    ]
    for root in roots:
        if root.exists():
            paths.extend(path for path in root.rglob("*") if path.is_file() and path.suffix.lower() in suffixes)
    for path in sorted(set(paths)):
        text = read_text(path)
        for token in forbidden:
            if token in text:
                raise AssertionError(f"old pointer remains in candidate active surface: {path.relative_to(factory)}: {token}")
    changelog_head = "\n".join(read_text(factory / "CHANGELOG.md").splitlines()[:100])
    if "AGENTS.md §3" in changelog_head:
        raise AssertionError("old AGENTS section pointer remains in active CHANGELOG header")


def check_candidate_factory_and_skill(overlay: dict) -> None:
    with tempfile.TemporaryDirectory(prefix="agents-v6-factory-") as name:
        factory = Path(name) / "factory"
        contract, baseline = build_candidate_factory(factory, overlay)
        check_active_pointer_surface(factory, overlay)
        by_id = {item["id"]: item for item in contract["assets"]}
        for asset_id in ("root.agents", "root.readme.bridgeforge-public", "codex.doc.hook-signals"):
            asset = by_id[asset_id]
            if asset_id == "root.agents":
                payload = read_bytes(factory / "AGENTS.md")
            else:
                payload = read_bytes(factory / asset["target"])
            baseline.verify_contract_payload(asset, payload, factory)
        skill = candidate_skill_payload(overlay)
        if skill != read_bytes(factory / "skills/bridgeforge-codex/SKILL.md"):
            raise AssertionError("installed candidate Skill differs from deterministic patch output")
        test_module = load_module(
            "candidate_root_skill_test",
            factory / "scripts/tests/test_bridgeforge_codex_root_skill.py",
        )
        suite = unittest.defaultTestLoader.loadTestsFromModule(test_module)
        stream = io.StringIO()
        result = unittest.TextTestRunner(stream=stream, verbosity=0).run(suite)
        if not result.wasSuccessful():
            raise AssertionError(f"real root Skill tests rejected candidate factory: {stream.getvalue()}")
        rebuild = subprocess.run(
            [HOST_PYTHON, "-B", "scripts/rebuild_shared_skill_manifest.py"],
            cwd=factory,
            capture_output=True,
            text=True,
        )
        if rebuild.returncode != 0:
            raise AssertionError(f"real Skill manifest rebuild rejected candidate Skill: {rebuild.stdout}{rebuild.stderr}")
        skill_contract = json.loads(read_text(PROPOSAL / overlay["skill_contract"]))
        if skill_contract["expected_sha256"] not in read_text(factory / "bridgeforge-codex-manifest.json"):
            raise AssertionError("formal distribution manifest does not contain complete candidate Skill hash")
        check = subprocess.run(
            [HOST_PYTHON, "-B", "scripts/rebuild_shared_skill_manifest.py", "--check"],
            cwd=factory,
            capture_output=True,
            text=True,
        )
        if check.returncode != 0:
            raise AssertionError(f"real Skill manifest checker rejected candidate Skill: {check.stdout}{check.stderr}")


def check_unique_owners() -> None:
    readme = read_text(PROPOSAL / "readme/bridgeforge-public-section.md")
    if re.search(r"必须|禁止|不得|只能|不允许|只允许", operative_markdown(readme)):
        raise AssertionError("README explanation contains command-strength language owned by AGENTS or Skill")
    for token in ("plan/apply", "aggregate fingerprint", "聚合指纹", "保留差异（gap）", "ready", "degraded", "G0–G5", "G* 清单", "所有权分类器"):
        if token in readme:
            raise AssertionError(f"README leaks operator detail: {token}")
    scripts = read_text(PROPOSAL / "factory/scripts/AGENTS.md")
    for token in ("aggregate fingerprint", "聚合指纹", "gap", "degraded", "版本戳最后"):
        if token in scripts:
            raise AssertionError(f"scripts nested duplicates operator algorithm: {token}")
    root = read_text(PROPOSAL / "template/AGENTS.md")
    if "骨架版本戳只能由统一同步器修改，其他操作禁止修改" not in root or "版本戳最后" in root:
        raise AssertionError("root version ownership boundary is wrong")
    skill = candidate_skill_payload(json.loads(read_text(OVERLAY_PATH))).decode("utf-8")
    for token in ("apply 前必须", "核对 fingerprint", "gap", "回滚本事务全部写入", "最后写 `.codex/.bridgeforge_codex_version`", "逐文件 `G*`"):
        if token not in skill:
            raise AssertionError(f"Skill operator contract missing: {token}")


def check_semantic_sentinels() -> None:
    template = read_text(PROPOSAL / "template/AGENTS.md")
    factory = read_text(PROPOSAL / "factory/AGENTS.md")
    readme = read_text(PROPOSAL / "readme/bridgeforge-public-section.md")
    hook_signals = read_text(PROPOSAL / "shared-docs/codex-hook-signals.md")
    required = {
        "template": ("空泛安抚", "未验证 / 不知道", "禁止用 `find`", "归咎于用户、工具或环境", "用户明确要求可见交互窗口", "禁止因焦虑跨层连带修改", "`$escalate` 或 `$debate`", "散落到项目根与源码目录", "`.codex/skills/<name>/SKILL.md`", "禁止先改主项目或 lockfile", "禁止只凭 CHANGELOG"),
        "factory": ("传播四问", "dogfood 一致性硬闸", "项目目录地图", "资产编号（asset id）", "所有权策略（ownership strategy）"),
        "README": ("已暂存版本", "暂存后恢复工作树不能绕过", "`rust-toolchain.toml`", "`$archive-scan`", "常用入口按用途分组", "完整操作合同以该 Skill 为准"),
        "hook-signals": ("`$spinoff`", "`$todo`", "可选路线", "`$develop`", "评估或咨询类任务直接给结论和风险"),
    }
    texts = {"template": template, "factory": factory, "README": readme, "hook-signals": hook_signals}
    for label, tokens in required.items():
        for token in tokens:
            if token not in texts[label]:
                raise AssertionError(f"semantic sentinel missing: {label}: {token}")


def check_links_encoding_and_size() -> None:
    paths = sorted(PROPOSAL.rglob("*.md")) + sorted(PROPOSAL.rglob("*.json")) + sorted(PROPOSAL.rglob("*.py"))
    for path in paths:
        text = read_text(path)
        if path.suffix == ".md":
            for target in re.findall(r"\[[^\]]+\]\(([^)]+)\)", text):
                target = target.split("#", 1)[0]
                if target and "://" not in target and not (path.parent / target).resolve().exists():
                    raise AssertionError(f"broken link: {path}: {target}")
    template_lines = len(read_text(PROPOSAL / "template/AGENTS.md").splitlines())
    factory_lines = len(read_text(PROPOSAL / "factory/AGENTS.md").splitlines())
    if template_lines > 125 or factory_lines > 135:
        raise AssertionError(f"candidate roots exceed size gate: template={template_lines}, factory={factory_lines}")
    for relative in ("template/AGENTS.md", "factory/AGENTS.md"):
        for number, line in enumerate(read_text(PROPOSAL / relative).splitlines(), 1):
            if line.startswith("- ") and (line.count("；") > 2 or len(line) > 180):
                raise AssertionError(f"candidate rule is too dense for cold reading: {relative}:{number}")


def main() -> int:
    if "--emit-hashes" in sys.argv:
        emit_hashes()
        return 0
    overlay = json.loads(read_text(OVERLAY_PATH))
    check_manifest(overlay)
    factory_public = marker_region(read_text(PROPOSAL / "factory/AGENTS.md"), "<!-- BRIDGEFORGE:PUBLIC:BEGIN -->", "<!-- BRIDGEFORGE:PUBLIC:END -->")
    template_public = marker_region(read_text(PROPOSAL / "template/AGENTS.md"), "<!-- BRIDGEFORGE:PUBLIC:BEGIN -->", "<!-- BRIDGEFORGE:PUBLIC:END -->")
    if factory_public != template_public:
        raise AssertionError("factory/template public zones differ")
    check_instruction_entry(overlay)
    check_region_migration(overlay)
    check_real_sync_plan_apply(overlay)
    check_pointer_migrations(overlay)
    check_semantic_contract(overlay)
    check_candidate_factory_and_skill(overlay)
    check_unique_owners()
    check_semantic_sentinels()
    check_links_encoding_and_size()
    print("proposal-contract: PASS")
    print("assertions: hashed candidate baseline/syncer loaded from candidate paths, README-only append policy, raw-byte non-README region boundaries across CRLF/LF/mixed/no-tail fixtures, subprocess CLI structured blocker, managed-visible-tree plan/no-op/rollback proof excluding .git/.venv/__pycache__, deterministic complete Skill plus explicit tests/manifest, five-surface normalized-content worktree/staged gate, source-hashed exact-once root/Template mapping with section-bound visible-text relations and comment/code negative control, enumerated active-surface pointer scan derived from migrations, README non-normative owner gate, links, UTF-8 no BOM, size/density gates")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except AssertionError as exc:
        print(f"proposal-contract: FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1)
