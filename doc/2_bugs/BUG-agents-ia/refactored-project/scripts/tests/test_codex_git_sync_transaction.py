from __future__ import annotations

import argparse
import contextlib
import importlib.util
import io
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock


ROOT = Path(__file__).resolve().parents[2]
SCRIPT_DIR = ROOT / "templates" / "scripts"
sys.path.insert(0, str(SCRIPT_DIR))


def load_module(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


SYNC = load_module("bridgeforge_transaction_git_sync", SCRIPT_DIR / "codex_git_sync.py")


def git(root: Path, *args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["git", *args],
        cwd=root,
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        check=False,
    )


class GitSyncTransactionTests(unittest.TestCase):
    def setUp(self) -> None:
        self.original_root = SYNC.REPO_ROOT
        self.original_receipt = SYNC.ADAPTATION_RECEIPT

    def tearDown(self) -> None:
        SYNC.REPO_ROOT = self.original_root
        SYNC.ADAPTATION_RECEIPT = self.original_receipt

    def _repository(self, root: Path) -> None:
        self.assertEqual(git(root, "init", "-q").returncode, 0)
        self.assertEqual(git(root, "config", "user.name", "BridgeForge Test").returncode, 0)
        self.assertEqual(
            git(root, "config", "user.email", "test@example.invalid").returncode,
            0,
        )
        (root / "tracked.txt").write_text("base\n", encoding="utf-8")
        self.assertEqual(git(root, "add", "tracked.txt").returncode, 0)
        self.assertEqual(git(root, "commit", "-qm", "base").returncode, 0)

    def test_empty_normalized_change_set_produces_no_release_writes(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            project = Path(raw)
            (project / "VERSION").write_text("2.3.4\n", encoding="utf-8")
            (project / "CHANGELOG.md").write_text("# Changelog\n", encoding="utf-8")
            SYNC.REPO_ROOT = project

            with mock.patch.object(
                SYNC,
                "_load_factory_manifest_module",
                return_value=None,
            ):
                plan = SYNC._build_sync_write_plan(
                    "chore: 同步当前骨架",
                    set(),
                )

            self.assertIsNone(plan.release)
            self.assertEqual(plan.writes, {})
            self.assertEqual((project / "VERSION").read_text(encoding="utf-8"), "2.3.4\n")

    def test_false_dirty_with_empty_changed_paths_does_not_require_message(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            project = self._repository_with_remote(Path(raw))
            SYNC.REPO_ROOT = project
            SYNC.ADAPTATION_RECEIPT = (
                project / ".runtime" / "bridgeforge-codex" /
                "explicit-adaptation.json"
            )
            args = argparse.Namespace(
                message=None,
                message_file=None,
                remote="origin",
                skip_fetch=True,
                skip_push=False,
            )

            with (
                mock.patch.object(SYNC, "_status", side_effect=[" M tracked.txt", ""]),
                mock.patch.object(SYNC, "_changed_paths", return_value=set()),
                mock.patch.object(SYNC, "_read_message") as read_message,
                mock.patch.object(SYNC, "verify_current_baseline"),
                mock.patch.object(SYNC, "_load_factory_manifest_module", return_value=None),
                mock.patch.object(SYNC, "_check_factory_version_worktree"),
            ):
                return_code = SYNC.sync(args)

            self.assertEqual(return_code, 0)
            read_message.assert_not_called()

    def _repository_with_remote(self, base: Path) -> Path:
        project = base / "project"
        remote = base / "remote.git"
        project.mkdir()
        self.assertEqual(git(base, "init", "--bare", "-q", str(remote)).returncode, 0)
        self._repository(project)
        self.assertEqual(git(project, "branch", "-M", "main").returncode, 0)
        self.assertEqual(git(project, "remote", "add", "origin", str(remote)).returncode, 0)
        self.assertEqual(git(project, "push", "-qu", "origin", "main").returncode, 0)
        return project

    @staticmethod
    def _args(message: str) -> argparse.Namespace:
        return argparse.Namespace(
            message=message,
            message_file=None,
            remote="origin",
            skip_fetch=True,
            skip_push=False,
        )

    def test_rollback_restores_complete_index_and_worktree_boundary(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            project = Path(raw)
            self._repository(project)
            derived = project / "derived.txt"
            derived.write_text("before\n", encoding="utf-8")
            git(project, "add", "derived.txt")
            git(project, "commit", "-qm", "derived")

            tracked = project / "tracked.txt"
            tracked.write_text("staged\n", encoding="utf-8")
            git(project, "add", "tracked.txt")
            tracked.write_text("worktree\n", encoding="utf-8")
            tracked_before = tracked.read_bytes()
            (project / "user.txt").write_text("unstaged\n", encoding="utf-8")
            before_status = git(project, "status", "--porcelain=v1").stdout

            SYNC.REPO_ROOT = project
            plan = SYNC.SyncWritePlan({derived: b"after\n"}, None)
            snapshot = SYNC._snapshot_sync_plan(plan)
            derived_before = derived.read_bytes()
            SYNC._apply_sync_plan(plan)
            self.assertEqual(git(project, "add", ".").returncode, 0)
            expected_index = snapshot.index_path.read_bytes()

            SYNC._restore_sync_plan(plan, snapshot, expected_index)

            self.assertEqual(derived.read_bytes(), derived_before)
            self.assertEqual(tracked.read_bytes(), tracked_before)
            self.assertEqual(snapshot.index_path.read_bytes(), snapshot.index_bytes)
            self.assertEqual(
                git(project, "status", "--porcelain=v1").stdout,
                before_status,
            )

    def test_split_index_is_blocked_before_any_target_write(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            project = Path(raw)
            self._repository(project)
            result = git(project, "update-index", "--split-index")
            if result.returncode != 0:
                self.skipTest("Git build does not support split index")
            target = project / "derived.txt"
            SYNC.REPO_ROOT = project
            plan = SYNC.SyncWritePlan({target: b"never-written\n"}, None)
            with self.assertRaisesRegex(SYNC.SyncStop, "split index"):
                SYNC._snapshot_sync_plan(plan)
            self.assertFalse(target.exists())

    def test_real_rejected_commit_restores_original_index_boundary(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            project = Path(raw)
            self._repository(project)
            derived = project / "derived.txt"
            derived.write_text("before\n", encoding="utf-8")
            self.assertEqual(git(project, "add", "derived.txt").returncode, 0)
            self.assertEqual(git(project, "commit", "-qm", "derived").returncode, 0)

            tracked = project / "tracked.txt"
            tracked.write_text("staged\n", encoding="utf-8")
            self.assertEqual(git(project, "add", "tracked.txt").returncode, 0)
            tracked.write_text("worktree\n", encoding="utf-8")
            (project / "user.txt").write_text("unstaged\n", encoding="utf-8")
            before_status = git(project, "status", "--porcelain=v1").stdout

            hook = project / ".git" / "hooks" / "pre-commit"
            hook.write_text("#!/bin/sh\nexit 1\n", encoding="utf-8", newline="\n")
            os.chmod(hook, 0o755)

            SYNC.REPO_ROOT = project
            plan = SYNC.SyncWritePlan({derived: b"after\n"}, None)
            snapshot = SYNC._snapshot_sync_plan(plan)
            SYNC._apply_sync_plan(plan)
            self.assertEqual(git(project, "add", ".").returncode, 0)
            expected_index = snapshot.index_path.read_bytes()
            expected_tree = SYNC._index_tree()

            rejected = git(project, "commit", "-m", "rejected")
            self.assertNotEqual(rejected.returncode, 0)
            SYNC._restore_sync_plan(
                plan,
                snapshot,
                expected_index,
                expected_tree,
            )

            self.assertEqual(snapshot.index_path.read_bytes(), snapshot.index_bytes)
            self.assertEqual(
                git(project, "status", "--porcelain=v1").stdout,
                before_status,
            )

    def test_sync_real_rejected_hook_restores_only_transaction_boundary(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            project = self._repository_with_remote(Path(raw))
            tracked = project / "tracked.txt"
            tracked.write_text("user change\n", encoding="utf-8")
            derived = project / "derived.txt"
            derived.write_text("before\n", encoding="utf-8")
            self.assertEqual(git(project, "add", "derived.txt").returncode, 0)
            self.assertEqual(git(project, "commit", "-qm", "derived").returncode, 0)
            derived_before = derived.read_bytes()
            tracked.write_text("user change\n", encoding="utf-8")
            before_status = git(project, "status", "--porcelain=v1").stdout
            hook = project / ".git" / "hooks" / "pre-commit"
            hook.write_text("#!/bin/sh\nexit 1\n", encoding="utf-8", newline="\n")
            os.chmod(hook, 0o755)

            SYNC.REPO_ROOT = project
            before_identity = SYNC._repository_identity()
            output = io.StringIO()
            with mock.patch.object(SYNC, "verify_current_baseline"), \
                    mock.patch.object(
                        SYNC,
                        "_build_sync_write_plan",
                        return_value=SYNC.SyncWritePlan({derived: b"after\n"}, None),
                    ), \
                    mock.patch.object(SYNC, "_check_factory_version_worktree"), \
                    contextlib.redirect_stdout(output):
                with self.assertRaisesRegex(SYNC.SyncStop, "git commit failed"):
                    SYNC.sync(self._args("fix: rejected hook"))

            self.assertEqual(SYNC._repository_identity(), before_identity)
            self.assertEqual(derived.read_bytes(), derived_before)
            self.assertEqual(
                git(project, "status", "--porcelain=v1").stdout,
                before_status,
            )
            receipt = output.getvalue()
            self.assertIn("original Git index were restored", receipt)
            self.assertIn("repository identity remained unchanged", receipt)

    def test_sync_simulated_rejected_hook_restores_when_identity_is_stable(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            project = self._repository_with_remote(Path(raw))
            tracked = project / "tracked.txt"
            tracked.write_text("user change\n", encoding="utf-8")
            derived = project / "derived.txt"
            derived.write_text("before\n", encoding="utf-8")
            self.assertEqual(git(project, "add", "derived.txt").returncode, 0)
            self.assertEqual(git(project, "commit", "-qm", "derived").returncode, 0)
            derived_before = derived.read_bytes()
            tracked.write_text("user change\n", encoding="utf-8")
            before_status = git(project, "status", "--porcelain=v1").stdout
            original_run_git = SYNC._run_git

            def reject_commit(
                args: list[str],
                *,
                timeout: int = 120,
                label: str | None = None,
            ):
                if args[:1] == ["commit"]:
                    raise SYNC.SyncStop("git commit failed: simulated hook rejection", 1)
                return original_run_git(args, timeout=timeout, label=label)

            SYNC.REPO_ROOT = project
            output = io.StringIO()
            with mock.patch.object(SYNC, "verify_current_baseline"), \
                    mock.patch.object(
                        SYNC,
                        "_build_sync_write_plan",
                        return_value=SYNC.SyncWritePlan({derived: b"after\n"}, None),
                    ), \
                    mock.patch.object(SYNC, "_check_factory_version_worktree"), \
                    mock.patch.object(SYNC, "_run_git", side_effect=reject_commit), \
                    contextlib.redirect_stdout(output):
                with self.assertRaisesRegex(SYNC.SyncStop, "simulated hook rejection"):
                    SYNC.sync(self._args("fix: simulated rejected hook"))

            self.assertEqual(derived.read_bytes(), derived_before)
            self.assertEqual(
                git(project, "status", "--porcelain=v1").stdout,
                before_status,
            )
            self.assertIn("repository identity remained unchanged", output.getvalue())

    def test_hook_core_bare_drift_is_high_severity_and_not_auto_restored(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            project = self._repository_with_remote(Path(raw))
            tracked = project / "tracked.txt"
            tracked.write_text("user change\n", encoding="utf-8")
            derived = project / "derived.txt"
            derived.write_text("before\n", encoding="utf-8")
            self.assertEqual(git(project, "add", "derived.txt").returncode, 0)
            self.assertEqual(git(project, "commit", "-qm", "derived").returncode, 0)
            tracked.write_text("user change\n", encoding="utf-8")
            hook = project / ".git" / "hooks" / "pre-commit"
            hook.write_text(
                "#!/bin/sh\ngit config core.bare true\nexit 1\n",
                encoding="utf-8",
                newline="\n",
            )
            os.chmod(hook, 0o755)

            SYNC.REPO_ROOT = project
            output = io.StringIO()
            with mock.patch.object(SYNC, "verify_current_baseline"), \
                    mock.patch.object(
                        SYNC,
                        "_build_sync_write_plan",
                        return_value=SYNC.SyncWritePlan({derived: b"after\n"}, None),
                    ), \
                    mock.patch.object(SYNC, "_check_factory_version_worktree"), \
                    contextlib.redirect_stdout(output):
                with self.assertRaisesRegex(
                    SYNC.SyncStop,
                    "HIGH: repository identity drift detected",
                ) as captured:
                    SYNC.sync(self._args("fix: detect core bare drift"))

            self.assertIn("no automatic repository recovery", str(captured.exception))
            self.assertNotIn("were restored", output.getvalue())
            self.assertEqual(derived.read_bytes(), b"after\n")
            config = (project / ".git" / "config").read_text(encoding="utf-8")
            self.assertIn("bare = true", config)

    def test_hook_common_config_drift_is_detected_without_misleading_receipt(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            project = self._repository_with_remote(Path(raw))
            tracked = project / "tracked.txt"
            tracked.write_text("user change\n", encoding="utf-8")
            derived = project / "derived.txt"
            derived.write_text("before\n", encoding="utf-8")
            self.assertEqual(git(project, "add", "derived.txt").returncode, 0)
            self.assertEqual(git(project, "commit", "-qm", "derived").returncode, 0)
            tracked.write_text("user change\n", encoding="utf-8")
            hook = project / ".git" / "hooks" / "pre-commit"
            hook.write_text(
                "#!/bin/sh\ngit config bridgeforge.identity-drift true\nexit 1\n",
                encoding="utf-8",
                newline="\n",
            )
            os.chmod(hook, 0o755)

            SYNC.REPO_ROOT = project
            output = io.StringIO()
            with mock.patch.object(SYNC, "verify_current_baseline"), \
                    mock.patch.object(
                        SYNC,
                        "_build_sync_write_plan",
                        return_value=SYNC.SyncWritePlan({derived: b"after\n"}, None),
                    ), \
                    mock.patch.object(SYNC, "_check_factory_version_worktree"), \
                    contextlib.redirect_stdout(output):
                with self.assertRaisesRegex(
                    SYNC.SyncStop,
                    "changed=common_config_digest",
                ) as captured:
                    SYNC.sync(self._args("fix: detect config drift"))

            self.assertIn("no automatic repository recovery", str(captured.exception))
            self.assertNotIn("were restored", output.getvalue())
            self.assertEqual(derived.read_bytes(), b"after\n")

    def test_failed_pre_push_hook_identity_drift_is_detected(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            project = self._repository_with_remote(Path(raw))
            tracked = project / "tracked.txt"
            tracked.write_text("push change\n", encoding="utf-8")
            hook = project / ".git" / "hooks" / "pre-push"
            hook.write_text(
                "#!/bin/sh\ngit config bridgeforge.pre-push-drift true\nexit 1\n",
                encoding="utf-8",
                newline="\n",
            )
            os.chmod(hook, 0o755)

            SYNC.REPO_ROOT = project
            output = io.StringIO()
            with mock.patch.object(SYNC, "verify_current_baseline"), \
                    mock.patch.object(
                        SYNC,
                        "_build_sync_write_plan",
                        return_value=SYNC.SyncWritePlan({}, None),
                    ), \
                    mock.patch.object(SYNC, "_check_factory_version_worktree"), \
                    contextlib.redirect_stdout(output):
                with self.assertRaisesRegex(
                    SYNC.SyncStop,
                    "HIGH: repository identity drift detected after "
                    "git push/pre-push hook",
                ) as captured:
                    SYNC.sync(self._args("fix: detect pre-push identity drift"))

            self.assertIn("no automatic repository recovery", str(captured.exception))
            self.assertNotIn("were restored", output.getvalue())
            self.assertIn(
                "pre-push-drift = true",
                (project / ".git" / "config").read_text(encoding="utf-8"),
            )

    def test_initial_bare_repository_is_blocked_before_any_write(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            project = self._repository_with_remote(Path(raw))
            tracked = project / "tracked.txt"
            tracked.write_text("user change\n", encoding="utf-8")
            self.assertEqual(git(project, "config", "core.bare", "true").returncode, 0)
            config_before = (project / ".git" / "config").read_bytes()
            SYNC.REPO_ROOT = project

            with mock.patch.object(SYNC, "_build_sync_write_plan") as build_plan:
                with self.assertRaisesRegex(
                    SYNC.SyncStop,
                    "core.bare=true",
                ):
                    SYNC.sync(self._args("fix: must not start"))

            build_plan.assert_not_called()
            self.assertEqual(tracked.read_text(encoding="utf-8"), "user change\n")
            self.assertEqual((project / ".git" / "config").read_bytes(), config_before)

    def test_sync_restores_plan_and_index_when_git_add_fails(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            project = self._repository_with_remote(Path(raw))
            tracked = project / "tracked.txt"
            tracked.write_text("user change\n", encoding="utf-8")
            derived = project / "derived.txt"
            derived.write_text("before\n", encoding="utf-8")
            self.assertEqual(git(project, "add", "derived.txt").returncode, 0)
            self.assertEqual(git(project, "commit", "-qm", "derived").returncode, 0)
            tracked.write_text("user change\n", encoding="utf-8")
            derived_before = derived.read_bytes()
            before_status = git(project, "status", "--porcelain=v1").stdout
            original_git = SYNC._git

            def fail_add(args: list[str], *, timeout: int = 120):
                if args == ["add", "."]:
                    return subprocess.CompletedProcess(
                        ["git", "add", "."], 1, "", "injected add failure"
                    )
                return original_git(args, timeout=timeout)

            SYNC.REPO_ROOT = project
            plan = SYNC.SyncWritePlan({derived: b"after\n"}, None)
            with mock.patch.object(SYNC, "verify_current_baseline"), \
                    mock.patch.object(SYNC, "_build_sync_write_plan", return_value=plan), \
                    mock.patch.object(SYNC, "_check_factory_version_worktree"), \
                    mock.patch.object(SYNC, "_git", side_effect=fail_add):
                with self.assertRaisesRegex(SYNC.SyncStop, "git add failed"):
                    SYNC.sync(self._args("fix: add failure"))

            self.assertEqual(derived.read_bytes(), derived_before)
            self.assertEqual(git(project, "status", "--porcelain=v1").stdout, before_status)

    def test_manifest_render_failure_is_zero_write(self) -> None:
        renderer = mock.Mock()
        renderer.render_all_outputs.side_effect = ValueError("injected manifest failure")
        with mock.patch.object(SYNC, "build_release_plan", return_value=None), \
                mock.patch.object(SYNC, "_load_factory_manifest_module", return_value=renderer):
            with self.assertRaisesRegex(SYNC.SyncStop, "shared manifest render blocked"):
                SYNC._build_sync_write_plan("fix: render", {"tracked.txt"})

    def test_push_failure_keeps_successful_local_commit(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            project = self._repository_with_remote(Path(raw))
            tracked = project / "tracked.txt"
            tracked.write_text("user change\n", encoding="utf-8")
            before_head = git(project, "rev-parse", "HEAD").stdout.strip()
            original_run_git = SYNC._run_git

            def fail_push(args: list[str], *, timeout: int = 120, label: str | None = None):
                if args == ["push"]:
                    raise SYNC.SyncStop("injected push failure", 1)
                return original_run_git(args, timeout=timeout, label=label)

            SYNC.REPO_ROOT = project
            plan = SYNC.SyncWritePlan({}, None)
            with mock.patch.object(SYNC, "verify_current_baseline"), \
                    mock.patch.object(SYNC, "_build_sync_write_plan", return_value=plan), \
                    mock.patch.object(SYNC, "_check_factory_version_worktree"), \
                    mock.patch.object(SYNC, "_run_git", side_effect=fail_push):
                with self.assertRaisesRegex(SYNC.SyncStop, "injected push failure"):
                    SYNC.sync(self._args("fix: keep commit"))

            after_head = git(project, "rev-parse", "HEAD").stdout.strip()
            self.assertNotEqual(after_head, before_head)
            self.assertEqual(git(project, "status", "--porcelain=v1").stdout, "")

    def test_obsolete_receipt_is_retired_only_after_current_only_commit(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            base = Path(raw)
            project = self._repository_with_remote(base)
            receipt = base / "explicit-adaptation.json"
            receipt.write_text("{}\n", encoding="utf-8")
            (project / "tracked.txt").write_text("user change\n", encoding="utf-8")
            receipt_payload = {"schema_version": 2}
            plan = SYNC.SyncWritePlan({}, None)
            SYNC.REPO_ROOT = project
            SYNC.ADAPTATION_RECEIPT = receipt

            with mock.patch.object(SYNC, "verify_current_baseline"), \
                    mock.patch.object(
                        SYNC,
                        "_read_adaptation_proof",
                        return_value=receipt_payload,
                    ), \
                    mock.patch.object(
                        SYNC,
                        "_build_sync_write_plan",
                        return_value=plan,
                    ) as build_plan, \
                    mock.patch.object(SYNC, "_check_factory_version_worktree"):
                self.assertEqual(SYNC.sync(self._args("fix: consume receipt")), 0)

            self.assertFalse(receipt.exists())
            self.assertEqual(len(build_plan.call_args.args), 2)

        with tempfile.TemporaryDirectory() as raw:
            base = Path(raw)
            project = self._repository_with_remote(base)
            receipt = base / "explicit-adaptation.json"
            receipt.write_text("{}\n", encoding="utf-8")
            (project / "tracked.txt").write_text("user change\n", encoding="utf-8")
            hook = project / ".git" / "hooks" / "pre-commit"
            hook.write_text("#!/bin/sh\nexit 1\n", encoding="utf-8", newline="\n")
            os.chmod(hook, 0o755)
            SYNC.REPO_ROOT = project
            SYNC.ADAPTATION_RECEIPT = receipt

            with mock.patch.object(SYNC, "verify_current_baseline"), \
                    mock.patch.object(
                        SYNC,
                        "_read_adaptation_proof",
                        return_value={"schema_version": 2},
                    ), \
                    mock.patch.object(
                        SYNC,
                        "_build_sync_write_plan",
                        return_value=SYNC.SyncWritePlan({}, None),
                    ), \
                    mock.patch.object(SYNC, "_check_factory_version_worktree"):
                with self.assertRaisesRegex(SYNC.SyncStop, "git commit failed"):
                    SYNC.sync(self._args("fix: reject commit"))

            self.assertTrue(receipt.is_file())

    def test_linked_worktree_uses_its_own_git_reported_index(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            parent = Path(raw)
            project = parent / "main"
            linked = parent / "linked"
            project.mkdir()
            self._repository(project)
            result = git(project, "worktree", "add", "-qb", "linked", str(linked))
            self.assertEqual(result.returncode, 0, result.stderr)

            SYNC.REPO_ROOT = linked
            plan = SYNC.SyncWritePlan({}, None)
            snapshot = SYNC._snapshot_sync_plan(plan)
            identity = SYNC._repository_identity()

            self.assertTrue(snapshot.index_path.is_file())
            self.assertNotEqual(snapshot.index_path, (project / ".git" / "index").resolve())
            self.assertIn("worktrees", snapshot.index_path.as_posix())
            self.assertEqual(identity.index_path, os.path.normcase(str(snapshot.index_path)))
            self.assertEqual(
                identity.common_dir,
                os.path.normcase(str((project / ".git").resolve())),
            )
            self.assertIn("worktrees", identity.git_dir.replace("\\", "/"))
            self.assertEqual(identity.symbolic_head, "refs/heads/linked")
            self.assertFalse(identity.core_bare)
            self.assertTrue(identity.common_config_digest.startswith("sha256:"))


if __name__ == "__main__":
    unittest.main()
