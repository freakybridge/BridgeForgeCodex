#!/usr/bin/env python3
"""Run the mechanical git-sync flow as one Codex-approved command.

The model still owns the review and commit-message decision. This runner keeps
the actual git plumbing in one narrow, repo-local command so Codex can request a
single persistent approval for:

    .venv/Scripts/python.exe .codex/scripts/codex_git_sync.py

It deliberately refuses risky history repair. Diverged branches, missing
upstream, stash-pop conflicts, and push races stop for user handling.
"""
from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import os
import subprocess
import sys
import tempfile
from contextlib import contextmanager
from dataclasses import dataclass
from pathlib import Path
from types import ModuleType
from typing import Iterator

from project_runtime import ProjectRuntimeError, validate_project_runtime
from current_baseline import (
    BaselineError,
    detect_repository_role,
    verify_current_baseline,
)
from version_release import ReleaseError, ReleasePlan, build_release_plan

try:
    sys.stdout.reconfigure(encoding="utf-8")  # type: ignore[attr-defined]
    sys.stderr.reconfigure(encoding="utf-8")  # type: ignore[attr-defined]
except Exception:
    pass

REPO_ROOT = Path(__file__).resolve().parents[2]
ADAPTATION_RECEIPT = (
    REPO_ROOT / ".runtime" / "bridgeforge-codex" / "explicit-adaptation.json"
)

class SyncStop(Exception):
    """Expected stop with a user-facing message and exit code."""

    def __init__(self, message: str, code: int = 1) -> None:
        super().__init__(message)
        self.code = code


USER_CONCLUSION_COMPLETED = "已完成。"
USER_CONCLUSION_COMPLETED_WITH_ACTIONS = "已完成，但仍有待处理项。"
USER_CONCLUSION_NOT_COMPLETED = "未完成。"


def _humanize_sync_stop(message: str) -> tuple[str, str]:
    normalized = message.casefold()
    mappings = (
        (
            "no upstream branch",
            "当前分支没有配置上游分支",
            "先配置上游分支，再重新运行 $git-sync。",
        ),
        (
            "remote advanced",
            "远端在同步期间出现了新提交",
            "检查远端变化后，重新运行 $git-sync。",
        ),
        (
            "branch diverged",
            "本地与远端分别存在独有提交",
            "查看下方提交清单并决定合并方式。",
        ),
        (
            "project runtime",
            "项目 .venv 不符合骨架运行要求",
            "修复项目 .venv 后，重新运行 $git-sync。",
        ),
        (
            "commit message is required",
            "当前改动缺少提交说明",
            "提供简体中文提交说明后，重新运行 $git-sync。",
        ),
        (
            "stash pop failed",
            "自动恢复本地改动时发生冲突，stash 已保留",
            "人工处理 stash 冲突后，再继续同步。",
        ),
        (
            "timed out",
            "Git 同步命令执行超时",
            "检查网络或 Hook 状态后，重新运行 $git-sync。",
        ),
    )
    for marker, reason, next_step in mappings:
        if marker in normalized:
            return reason, next_step
    return (
        "Git 同步被安全闸停止，技术原因见下方收据",
        "按技术收据处理原因后，重新运行 $git-sync。",
    )


def _print_user_result(
    conclusion: str,
    pending_items: list[str],
    next_step: str,
    *,
    file: object | None = None,
) -> None:
    stream = file if file is not None else sys.stdout
    print(f"结论：{conclusion}", file=stream)
    print("待处理事项：", file=stream)
    if pending_items:
        for item in pending_items:
            print(f"- {item}", file=stream)
    else:
        print("- 无", file=stream)
    print(f"下一步：{next_step}", file=stream)


@dataclass(frozen=True)
class RepositoryIdentity:
    inside_work_tree: bool
    top_level: str
    git_dir: str
    common_dir: str
    index_path: str
    symbolic_head: str
    core_bare: bool
    common_config_digest: str

def _env() -> dict[str, str]:
    env = os.environ.copy()
    env.setdefault("PYTHONUTF8", "1")
    return env

def _git(args: list[str], *, timeout: int = 120) -> subprocess.CompletedProcess[str]:
    cmd = ["git", "-c", f"safe.directory={REPO_ROOT.as_posix()}", *args]
    return subprocess.run(
        cmd,
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        timeout=timeout,
        env=_env(),
    )

def _run_git(args: list[str], *, timeout: int = 120, label: str | None = None) -> subprocess.CompletedProcess[str]:
    result = _git(args, timeout=timeout)
    if result.returncode != 0:
        name = label or "git " + " ".join(args)
        detail = (result.stderr or result.stdout).strip()
        raise SyncStop(f"{name} failed: {detail}", result.returncode or 1)
    return result


def _canonical_path(value: str) -> str:
    return os.path.normcase(str(Path(value).resolve()))


def _common_config_digest(common_dir: str) -> str:
    config = Path(common_dir) / "config"
    if not config.is_file():
        return "missing"
    return "sha256:" + hashlib.sha256(config.read_bytes()).hexdigest()


def _repository_identity() -> RepositoryIdentity:
    inside_result = _git(["rev-parse", "--is-inside-work-tree"])
    inside = (
        inside_result.returncode == 0
        and inside_result.stdout.strip().casefold() == "true"
    )
    bare_result = _git(["config", "--bool", "--get", "core.bare"])
    if bare_result.returncode == 1:
        core_bare = False
    elif bare_result.returncode == 0 and bare_result.stdout.strip().casefold() in {
        "true",
        "false",
    }:
        core_bare = bare_result.stdout.strip().casefold() == "true"
    else:
        detail = (bare_result.stderr or bare_result.stdout).strip()
        raise SyncStop(
            "HIGH: repository identity is invalid before writes: "
            f"cannot read core.bare ({detail})",
            2,
        )
    if not inside or core_bare:
        detail = (inside_result.stderr or inside_result.stdout).strip()
        raise SyncStop(
            "HIGH: repository identity is invalid before writes: "
            f"inside-work-tree={str(inside).lower()} "
            f"core.bare={str(core_bare).lower()} detail={detail!r}",
            2,
        )
    top_level = _canonical_path(
        _run_git(
            ["rev-parse", "--path-format=absolute", "--show-toplevel"],
            label="git repository top-level",
        ).stdout.strip()
    )
    expected_top_level = _canonical_path(str(REPO_ROOT))
    if top_level != expected_top_level:
        raise SyncStop(
            "HIGH: repository identity is invalid before writes: "
            f"top-level={top_level!r}, expected={expected_top_level!r}",
            2,
        )
    git_dir = _canonical_path(
        _run_git(
            ["rev-parse", "--absolute-git-dir"],
            label="git repository directory",
        ).stdout.strip()
    )
    common_dir = _canonical_path(
        _run_git(
            ["rev-parse", "--path-format=absolute", "--git-common-dir"],
            label="git common directory",
        ).stdout.strip()
    )
    index_path = _canonical_path(
        _run_git(
            ["rev-parse", "--path-format=absolute", "--git-path", "index"],
            label="git index identity",
        ).stdout.strip()
    )
    symbolic_head = _run_git(
        ["symbolic-ref", "-q", "HEAD"],
        label="git symbolic HEAD",
    ).stdout.strip()
    if not symbolic_head.startswith("refs/heads/"):
        raise SyncStop(
            "HIGH: repository identity is invalid before writes: "
            f"symbolic HEAD={symbolic_head!r}",
            2,
        )
    return RepositoryIdentity(
        inside_work_tree=inside,
        top_level=top_level,
        git_dir=git_dir,
        common_dir=common_dir,
        index_path=index_path,
        symbolic_head=symbolic_head,
        core_bare=core_bare,
        common_config_digest=_common_config_digest(common_dir),
    )


def _identity_drift_error(
    expected: RepositoryIdentity,
    phase: str,
) -> SyncStop | None:
    try:
        current = _repository_identity()
    except (OSError, SyncStop) as exc:
        return SyncStop(
            "HIGH: repository identity drift detected after "
            f"{phase}; identity could not be revalidated ({exc}); "
            "no automatic repository recovery was attempted",
            2,
        )
    changed = [
        name
        for name in RepositoryIdentity.__dataclass_fields__
        if getattr(expected, name) != getattr(current, name)
    ]
    if not changed:
        return None
    return SyncStop(
        "HIGH: repository identity drift detected after "
        f"{phase}; changed={','.join(changed)}; "
        "no automatic repository recovery was attempted",
        2,
    )


def _assert_repository_identity(
    expected: RepositoryIdentity,
    phase: str,
) -> None:
    drift = _identity_drift_error(expected, phase)
    if drift is not None:
        raise drift

def _status() -> str:
    return _run_git(["status", "--porcelain=v1"], label="git status").stdout.strip()

def _changed_paths() -> set[str]:
    paths: set[str] = set()
    commands = (
        ["diff", "--name-only"],
        ["diff", "--cached", "--name-only"],
        ["ls-files", "--others", "--exclude-standard"],
    )
    for command in commands:
        output = _run_git(command, label="git changed-path scan").stdout
        paths.update(line.strip().replace("\\", "/") for line in output.splitlines() if line.strip())
    return paths


def _reject_duplicate_keys(pairs: list[tuple[str, object]]) -> dict[str, object]:
    result: dict[str, object] = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def _read_adaptation_proof() -> dict[str, object] | None:
    if not ADAPTATION_RECEIPT.exists():
        return None
    if not ADAPTATION_RECEIPT.is_file():
        raise SyncStop("explicit adaptation receipt is not a plain file", 2)
    relative = ADAPTATION_RECEIPT.relative_to(REPO_ROOT).as_posix()
    ignored = _git(["check-ignore", "--quiet", "--", relative])
    if ignored.returncode != 0:
        raise SyncStop("explicit adaptation receipt is not ignored by Git", 2)
    try:
        payload = json.loads(
            ADAPTATION_RECEIPT.read_text(encoding="utf-8-sig"),
            object_pairs_hook=_reject_duplicate_keys,
        )
    except (OSError, UnicodeDecodeError, json.JSONDecodeError, ValueError) as exc:
        raise SyncStop(f"explicit adaptation receipt is invalid: {exc}", 2) from exc
    if not isinstance(payload, dict):
        raise SyncStop("explicit adaptation receipt root must be an object", 2)
    return payload

@dataclass(frozen=True)
class SyncWritePlan:
    writes: dict[Path, bytes]
    release: ReleasePlan | None


@dataclass(frozen=True)
class _SyncSnapshot:
    head: str
    index_path: Path
    index_bytes: bytes | None
    targets: dict[Path, bytes | None]


def _read_optional(path: Path) -> bytes | None:
    return path.read_bytes() if path.is_file() else None


def _atomic_write(path: Path, payload: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, raw = tempfile.mkstemp(prefix=path.name + ".", dir=path.parent)
    temporary = Path(raw)
    try:
        with os.fdopen(descriptor, "wb") as stream:
            stream.write(payload)
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary, path)
    finally:
        temporary.unlink(missing_ok=True)


def _git_path(name: str) -> Path:
    value = _run_git(
        ["rev-parse", "--path-format=absolute", "--git-path", name],
        label=f"git path {name}",
    ).stdout.strip()
    if not value:
        raise SyncStop(f"git returned no path for {name}", 1)
    return Path(value).resolve()


def _index_tree() -> str:
    return _run_git(["write-tree"], label="git index tree").stdout.strip()


def _reject_split_index() -> None:
    shared = _git(["rev-parse", "--shared-index-path"])
    if shared.returncode == 0 and shared.stdout.strip():
        raise SyncStop(
            "split index is not supported by the transactional git-sync; "
            "convert it outside this tool and retry",
            2,
        )
    configured = _git(["config", "--bool", "core.splitIndex"])
    if configured.returncode == 0 and configured.stdout.strip().casefold() == "true":
        raise SyncStop(
            "split index is enabled; zero writes performed",
            2,
        )


def _load_factory_manifest_module() -> ModuleType | None:
    script = REPO_ROOT / "scripts" / "rebuild_shared_skill_manifest.py"
    if not script.is_file():
        return None
    spec = importlib.util.spec_from_file_location("bridgeforge_manifest_renderer", script)
    if spec is None or spec.loader is None:
        raise SyncStop("cannot load shared manifest renderer", 1)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def _merge_planned_write(
    writes: dict[Path, bytes],
    path: Path,
    payload: bytes,
) -> None:
    target = path.resolve()
    previous = writes.get(target)
    if previous is not None and previous != payload:
        raise SyncStop(f"automatic producers disagree for {target}", 2)
    writes[target] = payload


def _build_sync_write_plan(
    message: str,
    changed_paths: set[str],
) -> SyncWritePlan:
    try:
        release = build_release_plan(
            REPO_ROOT,
            message,
            changed_paths,
        )
    except ReleaseError as exc:
        raise SyncStop(f"automatic version release blocked: {exc}", 2) from exc
    writes: dict[Path, bytes] = {}
    if release is not None:
        for path, payload in release.writes.items():
            _merge_planned_write(writes, path, payload)
    renderer = _load_factory_manifest_module()
    if renderer is not None:
        try:
            outputs = renderer.render_all_outputs(
                release_version=release.new_version if release is not None else None,
            )
        except (OSError, UnicodeDecodeError, ValueError, KeyError, TypeError) as exc:
            raise SyncStop(f"shared manifest render blocked: {exc}", 2) from exc
        for path, payload in outputs.items():
            _merge_planned_write(writes, path, payload)
    return SyncWritePlan(writes, release)


def _snapshot_sync_plan(plan: SyncWritePlan) -> _SyncSnapshot:
    _reject_split_index()
    index_path = _git_path("index")
    head = _run_git(["rev-parse", "HEAD"], label="git HEAD snapshot").stdout.strip()
    return _SyncSnapshot(
        head=head,
        index_path=index_path,
        index_bytes=_read_optional(index_path),
        targets={path: _read_optional(path) for path in plan.writes},
    )


def _apply_sync_plan(plan: SyncWritePlan) -> None:
    for path, payload in plan.writes.items():
        _atomic_write(path, payload)


def _restore_sync_plan(
    plan: SyncWritePlan,
    snapshot: _SyncSnapshot,
    expected_index: bytes | None,
    expected_index_tree: str | None = None,
) -> None:
    current_head = _run_git(["rev-parse", "HEAD"], label="git rollback HEAD").stdout.strip()
    if current_head != snapshot.head:
        raise SyncStop("rollback refused because HEAD changed", 2)
    current_index = _read_optional(snapshot.index_path)
    if current_index != expected_index:
        current_tree = _index_tree() if expected_index_tree is not None else None
        if current_tree != expected_index_tree:
            raise SyncStop("rollback refused because the Git index changed concurrently", 2)
    conflicts = [
        path.relative_to(REPO_ROOT).as_posix()
        for path, before in snapshot.targets.items()
        if _read_optional(path) not in {before, plan.writes[path]}
    ]
    if conflicts:
        raise SyncStop(
            "rollback refused because automatic targets changed concurrently: "
            + ", ".join(conflicts),
            2,
        )
    for path, before in snapshot.targets.items():
        if _read_optional(path) == before:
            continue
        if before is None:
            path.unlink(missing_ok=True)
        else:
            _atomic_write(path, before)
    if snapshot.index_bytes is None:
        snapshot.index_path.unlink(missing_ok=True)
    else:
        _atomic_write(snapshot.index_path, snapshot.index_bytes)


@contextmanager
def _sync_lock() -> Iterator[None]:
    lock = _git_path("bridgeforge-git-sync.lock")
    try:
        descriptor = os.open(lock, os.O_CREAT | os.O_EXCL | os.O_WRONLY)
    except FileExistsError as exc:
        raise SyncStop("another bridgeforge git-sync is already running", 2) from exc
    try:
        os.close(descriptor)
        yield
    finally:
        lock.unlink(missing_ok=True)

def _has_staged_changes() -> bool:
    result = _git(["diff", "--cached", "--quiet"])
    return result.returncode == 1

def _upstream() -> str:
    result = _git(["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"])
    if result.returncode != 0:
        raise SyncStop("no upstream branch; set upstream before running git-sync", 2)
    return result.stdout.strip()

def _push_target() -> str:
    result = _git(["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{push}"])
    if result.returncode != 0:
        raise SyncStop("no push target; configure the current branch before git-sync", 2)
    return result.stdout.strip()

def _ahead_behind() -> tuple[int, int]:
    result = _git(["rev-list", "--left-right", "--count", "HEAD...@{u}"])
    if result.returncode != 0:
        detail = (result.stderr or result.stdout).strip()
        raise SyncStop(f"cannot read ahead/behind state: {detail}", 1)
    parts = result.stdout.strip().split()
    if len(parts) != 2:
        raise SyncStop(f"unexpected ahead/behind output: {result.stdout!r}", 1)
    return int(parts[0]), int(parts[1])

def _print_diverged() -> None:
    _print_user_result(
        USER_CONCLUSION_NOT_COMPLETED,
        ["本地与远端分别存在独有提交"],
        "查看下方提交清单并决定合并方式。",
    )
    print("[git-sync] branch diverged; manual decision required")
    local = _git(["log", "--oneline", "--decorate", "--max-count=5", "@{u}..HEAD"])
    remote = _git(["log", "--oneline", "--decorate", "--max-count=5", "HEAD..@{u}"])
    if local.stdout.strip():
        print("\n[git-sync] local-only commits:")
        print(local.stdout.strip())
    if remote.stdout.strip():
        print("\n[git-sync] remote-only commits:")
        print(remote.stdout.strip())

def _read_message(args: argparse.Namespace) -> str | None:
    if args.message_file:
        return Path(args.message_file).read_text(encoding="utf-8").strip()
    if args.message:
        return args.message.strip()
    return None

def _check_factory_version_worktree() -> None:
    script = REPO_ROOT / ".codex" / "scripts" / "factory_version_check.py"
    if not script.exists():
        return
    result = subprocess.run(
        [sys.executable, str(script), "--worktree"],
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        timeout=120,
        env=_env(),
    )
    if result.returncode != 0:
        detail = (result.stderr or result.stdout).strip()
        raise SyncStop(
            f"factory_version_check.py --worktree failed: {detail}",
            result.returncode or 1,
        )

def _pull_ff_with_optional_stash(dirty: bool) -> None:
    stashed = False
    if dirty:
        result = _run_git(["stash", "push", "-u", "-m", "codex_git_sync_autostash"], label="git stash")
        stashed = "No local changes to save" not in (result.stdout + result.stderr)
    try:
        _run_git(["pull", "--ff-only"], timeout=180, label="git pull --ff-only")
    except SyncStop:
        if stashed:
            print("[git-sync] local changes are still in stash; resolve pull failure before continuing")
        raise
    if stashed:
        result = _git(["stash", "pop"], timeout=180)
        if result.returncode != 0:
            detail = (result.stderr or result.stdout).strip()
            raise SyncStop(f"git stash pop failed; stash is kept for manual recovery: {detail}", 2)

def sync(args: argparse.Namespace) -> int:
    if not (REPO_ROOT / ".git").exists():
        raise SyncStop(f"not a git repository: {REPO_ROOT}", 1)

    initial_identity = _repository_identity()
    _upstream()
    push_target = _push_target()
    dirty = bool(_status())

    if not args.skip_fetch:
        _run_git(["fetch", args.remote], timeout=180, label=f"git fetch {args.remote}")

    ahead, behind = _ahead_behind()

    if ahead and behind:
        _print_diverged()
        return 2

    if behind and not ahead:
        _pull_ff_with_optional_stash(dirty)
        ahead, behind = _ahead_behind()
        dirty = bool(_status())
        if ahead and behind:
            _print_diverged()
            return 2

    _assert_repository_identity(initial_identity, "fetch/pull")

    if dirty:
        with _sync_lock():
            changed_paths = _changed_paths()
            message = _read_message(args) if changed_paths else ""
            if changed_paths and not message:
                raise SyncStop("commit message is required when real changes exist", 2)
            if detect_repository_role(REPO_ROOT).kind != "factory":
                try:
                    verify_current_baseline(REPO_ROOT)
                except (BaselineError, OSError, UnicodeDecodeError, ValueError) as exc:
                    raise SyncStop(f"current baseline blocked git-sync: {exc}", 2) from exc
            obsolete_adaptation_receipt = _read_adaptation_proof()
            plan = _build_sync_write_plan(
                message,
                changed_paths,
            )
            snapshot = _snapshot_sync_plan(plan)
            expected_index = snapshot.index_bytes
            expected_index_tree: str | None = None
            committed = False
            try:
                _assert_repository_identity(initial_identity, "automatic write preparation")
                _apply_sync_plan(plan)
                if plan.release is not None:
                    print(
                        f"[git-sync] version {plan.release.old_version} -> "
                        f"{plan.release.new_version} ({plan.release.classification})"
                    )
                _check_factory_version_worktree()
                add_result = _git(["add", "."])
                expected_index = _read_optional(snapshot.index_path)
                if add_result.returncode != 0:
                    detail = (add_result.stderr or add_result.stdout).strip()
                    raise SyncStop(f"git add failed: {detail}", add_result.returncode or 1)
                expected_index_tree = _index_tree()
                if _has_staged_changes():
                    _run_git(["commit", "-m", message], timeout=180, label="git commit")
                    committed = True
                    _assert_repository_identity(initial_identity, "git commit/hook")
                    if obsolete_adaptation_receipt is not None:
                        ADAPTATION_RECEIPT.unlink()
                        print(
                            "[git-sync] obsolete adaptation receipt retired after "
                            "current-only evaluation"
                        )
                    ahead, behind = _ahead_behind()
                    if ahead and behind:
                        _print_diverged()
                        return 2
            except Exception as exc:
                drift = _identity_drift_error(initial_identity, "automatic write/commit phase")
                if drift is not None:
                    raise drift from exc
                if not committed:
                    _restore_sync_plan(
                        plan,
                        snapshot,
                        expected_index,
                        expected_index_tree,
                    )
                    print(
                        "[git-sync] automatic planned file writes and the original "
                        "Git index were restored; repository identity remained unchanged"
                    )
                if isinstance(exc, SyncStop):
                    raise
                raise SyncStop(f"automatic version release failed: {exc}", 1) from exc

    ahead, behind = _ahead_behind()
    if ahead and behind:
        _print_diverged()
        return 2
    if behind:
        raise SyncStop("remote advanced during git-sync; rerun after reviewing state", 2)
    pushed = False
    if ahead:
        if args.skip_push:
            print(f"[git-sync] {ahead} local commit(s) ready; push skipped by --skip-push")
        else:
            try:
                _run_git(["push"], timeout=240, label="git push")
            except Exception as exc:
                drift = _identity_drift_error(initial_identity, "git push/pre-push hook")
                if drift is not None:
                    raise drift from exc
                raise
            pushed = True

    _assert_repository_identity(initial_identity, "push/completion")
    final_dirty = _status()
    final_ahead, final_behind = _ahead_behind()
    if final_dirty or final_ahead or final_behind:
        if args.skip_push and not final_dirty and final_ahead and not final_behind:
            _print_user_result(
                USER_CONCLUSION_COMPLETED_WITH_ACTIONS,
                [f"仍有 {final_ahead} 个本地提交尚未推送"],
                "需要保存到远端时，再次运行不带 --skip-push 的 $git-sync。",
            )
        else:
            _print_user_result(
                USER_CONCLUSION_NOT_COMPLETED,
                ["同步结束时仍检测到未收口的 Git 状态"],
                "查看下方技术收据，处理剩余状态后重新运行 $git-sync。",
            )
        print("[git-sync] finished with remaining state:")
        if final_dirty:
            print(final_dirty)
        if final_ahead or final_behind:
            print(f"ahead={final_ahead} behind={final_behind}")
        return 3

    commit = _run_git(["rev-parse", "HEAD"], label="git rev-parse HEAD").stdout.strip()
    _print_user_result(
        USER_CONCLUSION_COMPLETED,
        [],
        "本次同步已结束，无需继续处理。",
    )
    print("[git-sync] synced")
    print(f"commit={commit}")
    print(f"push_target={push_target}")
    print(f"push_performed={'true' if pushed else 'false'}")
    print("working_tree=clean")
    print("ahead=0 behind=0")
    return 0

def main() -> int:
    try:
        validate_project_runtime(REPO_ROOT, executable=sys.executable)
    except ProjectRuntimeError as exc:
        _print_user_result(
            USER_CONCLUSION_NOT_COMPLETED,
            ["项目 .venv 不符合骨架运行要求"],
            "修复项目 .venv 后，重新运行 $git-sync。",
            file=sys.stderr,
        )
        print(f"[git-sync] project runtime contract rejected: {exc}", file=sys.stderr)
        return 2
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("-m", "--message", help="Commit message to use when local changes are staged.")
    parser.add_argument("--message-file", help="Read the commit message from a UTF-8 file.")
    parser.add_argument("--remote", default="origin", help="Remote to fetch before syncing. Default: origin.")
    parser.add_argument("--skip-fetch", action="store_true", help="Diagnostic/test mode: do not fetch first.")
    parser.add_argument("--skip-push", action="store_true", help="Diagnostic/test mode: commit but do not push.")
    args = parser.parse_args()

    try:
        return sync(args)
    except subprocess.TimeoutExpired as exc:
        _print_user_result(
            USER_CONCLUSION_NOT_COMPLETED,
            ["Git 同步命令执行超时"],
            "检查网络或 Hook 状态后，重新运行 $git-sync。",
            file=sys.stderr,
        )
        print(f"[git-sync] command timed out: {exc}", file=sys.stderr)
        return 1
    except SyncStop as exc:
        reason, next_step = _humanize_sync_stop(str(exc))
        _print_user_result(
            USER_CONCLUSION_NOT_COMPLETED,
            [reason],
            next_step,
            file=sys.stderr,
        )
        print(f"[git-sync] {exc}", file=sys.stderr)
        return exc.code

if __name__ == "__main__":
    raise SystemExit(main())
