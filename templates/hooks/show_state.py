#!/usr/bin/env python3
"""SessionStart hook：一次性打印项目状态和接续提示。

输出分支、未提交数、ahead/behind、骨架版本、最新 snapshot 与归档候选。
提交、推送或需要最新状态时，由 Agent 重新调用 Git 工具取得实时证据。

用法：`python show_state.py session-start`
"""
import re
import subprocess
import sys
from pathlib import Path

# Windows 终端默认不是 UTF-8，中文会乱码 → 强制 stdout 用 UTF-8
try:
    sys.stdout.reconfigure(encoding="utf-8")  # type: ignore[attr-defined]
except Exception:
    pass

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
HOST_DIR = Path(__file__).resolve().parent.parent


def _run(cmd: list[str]) -> str:
    try:
        r = subprocess.run(
            cmd, cwd=str(REPO_ROOT), capture_output=True, text=True,
            encoding="utf-8", errors="replace", timeout=5,
        )
        return r.stdout.strip() if r.returncode == 0 else ""
    except Exception:
        return ""


def _version() -> str:
    """只显示当前宿主的 bridgeforge-codex 骨架版本戳。"""
    try:
        version = (HOST_DIR / ".bridgeforge_codex_version").read_text(encoding="utf-8").strip()
    except Exception:
        return "?"
    return version or "?"


def _parse_git_status(raw: str) -> tuple[str, int, str]:
    """Parse one ``git status --porcelain=v2 --branch`` receipt."""
    branch = "?"
    dirty = 0
    ahead_behind = "no-upstream"
    for line in raw.splitlines():
        if line.startswith("# branch.head "):
            value = line.removeprefix("# branch.head ").strip()
            if value and value != "(detached)":
                branch = value
        elif line.startswith("# branch.ab "):
            match = re.fullmatch(r"\+(\d+) -(\d+)", line.removeprefix("# branch.ab ").strip())
            if match:
                ahead_behind = f"{match.group(1)}/{match.group(2)}"
        elif line and not line.startswith("# "):
            dirty += 1
    return branch, dirty, ahead_behind


def _git_state() -> tuple[str, int, str]:
    raw = _run(["git", "status", "--porcelain=v2", "--branch"])
    return _parse_git_status(raw)


def _latest_snapshot() -> str:
    """返回最新 snapshot 的提示行；无则返空串。"""
    snap_dir = REPO_ROOT / ".runtime" / "session_state"
    if not snap_dir.exists():
        return ""
    snaps = sorted(snap_dir.glob("*.md"), key=lambda p: p.stat().st_mtime, reverse=True)
    if not snaps:
        return ""
    latest = snaps[0]
    return f"[snapshot] 最新存档: {latest.name} — 输入 $resume 可接续上下文"


def _archive_hint() -> str:
    """调 archive_scan.py --count 看是否有归档候选；有则提示。"""
    script = REPO_ROOT / ".codex" / "scripts" / "archive_scan.py"
    if not script.exists():
        return ""
    try:
        r = subprocess.run(
            [sys.executable, str(script), "--count"],
            cwd=str(REPO_ROOT),
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=10,
        )
        if r.returncode != 0:
            return ""
        n = int(r.stdout.strip() or "0")
        if n > 0:
            return f"[archive] delivery / bugs 有 {n} 个候选可归档 — 输入 $archive-scan 查看"
    except Exception:
        pass
    return ""


def main() -> int:
    prefix = sys.argv[1] if len(sys.argv) > 1 else "state"
    branch, dirty, ab = _git_state()
    v = _version()
    print(f"[{prefix}] branch={branch} | dirty={dirty} | ahead/behind={ab} | skeleton=v{v}")

    # SessionStart 时额外提示：snapshot 接续 + 归档候选
    if prefix == "session-start":
        snap_hint = _latest_snapshot()
        if snap_hint:
            print(snap_hint)
        arch_hint = _archive_hint()
        if arch_hint:
            print(arch_hint)
    return 0


if __name__ == "__main__":
    sys.exit(main())
