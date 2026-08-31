#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
import contextlib
import io
import json
import os
import shutil
import subprocess
import sys
import tempfile
import time
import unittest
from datetime import datetime, timedelta, timezone
from pathlib import Path
from unittest import mock

ROOT = Path(__file__).resolve().parents[2]


def load(path: Path, name: str):
    spec = importlib.util.spec_from_file_location(name, path)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


sync_mod = load(ROOT / "scripts/codex_memory_sync.py", "bf_codex_memory_sync")


class NativeMemorySyncTests(unittest.TestCase):
    def _args(
        self,
        command: str,
        *arguments: str,
        project_root: Path = ROOT,
    ) -> list[str]:
        return [command, *arguments, "--project-root", str(project_root)]

    def _write_ledger(self, codex: Path, consent: object | None = None) -> Path:
        codex.mkdir(parents=True, exist_ok=True)
        ledger: dict[str, object] = {
            "schema_version": 1,
            "platform": "codex",
            "records": {},
        }
        if consent is not None:
            ledger["consents"] = {"native_memories": consent}
        path = codex / "bridgeforge-codex-managed.json"
        path.write_text(json.dumps(ledger, indent=2) + "\n", encoding="utf-8")
        return path

    def _write_enabled_authorized_home(self, codex: Path) -> None:
        remote = "https://github.com/example/bridgeforge-codex-memories.git"
        self._write_ledger(codex, sync_mod._authorization_payload("approved", remote))
        (codex / "config.toml").write_text(
            "[features]\nmemories = true\n[memories]\n"
            "generate_memories = true\nuse_memories = true\n",
            encoding="utf-8",
        )
        state = codex / ".bridgeforge-codex" / "memory-sync"
        state.mkdir(parents=True)
        (state / "remote.txt").write_text(remote + "\n", encoding="utf-8")

    def test_lifecycle_runtime_rejection_is_a_hard_failure_with_zero_writes(self) -> None:
        for command, arguments in (
            ("reconcile", ("--trigger", "stop")),
            ("mark", ("--trigger", "session-end")),
            ("kick", ("--trigger", "session-end")),
        ):
            with self.subTest(command=command), tempfile.TemporaryDirectory() as raw:
                base = Path(raw)
                codex = base / ".codex"
                codex.mkdir()
                project = base / "project-without-venv"
                project.mkdir()
                before = {
                    path.relative_to(base).as_posix(): path.read_bytes()
                    for path in base.rglob("*")
                    if path.is_file()
                }
                errors = io.StringIO()
                with mock.patch.dict(
                    sync_mod.os.environ,
                    {"CODEX_HOME": str(codex)},
                ), contextlib.redirect_stderr(errors):
                    result = sync_mod.main(
                        self._args(command, *arguments, project_root=project)
                    )
                after = {
                    path.relative_to(base).as_posix(): path.read_bytes()
                    for path in base.rglob("*")
                    if path.is_file()
                }
                self.assertEqual(result, 2)
                self.assertIn("project .venv is missing", errors.getvalue())
                self.assertEqual(after, before)

    @unittest.skipUnless(os.name == "nt", "Windows named mutex semantics")
    def test_abandoned_user_hooks_mutex_is_fail_closed(self) -> None:
        class Function:
            def __init__(self, result: int) -> None:
                self.result = result
                self.calls: list[tuple[object, ...]] = []

            def __call__(self, *args: object) -> int:
                self.calls.append(args)
                return self.result

        kernel32 = mock.Mock()
        kernel32.CreateMutexW = Function(123)
        kernel32.WaitForSingleObject = Function(0x00000080)
        kernel32.ReleaseMutex = Function(1)
        kernel32.CloseHandle = Function(1)
        with mock.patch.object(sync_mod.ctypes, "WinDLL", return_value=kernel32):
            with self.assertRaisesRegex(
                sync_mod.HookLockConflict,
                "mutex was abandoned",
            ):
                with sync_mod.user_hooks_lock(Path("C:/fixture/.codex")):
                    self.fail("abandoned mutex must not enter the writer section")
        self.assertEqual(kernel32.ReleaseMutex.calls, [(123,)])
        self.assertEqual(kernel32.CloseHandle.calls, [(123,)])

    def test_repair_rechecks_disabled_config_inside_user_lock(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            base = Path(raw)
            codex = base / ".codex"
            self._write_enabled_authorized_home(codex)
            project = base / "project"
            project.mkdir()
            config = codex / "config.toml"

            @contextlib.contextmanager
            def disable_before_locked_read(*_args: object, **_kwargs: object):
                config.write_text("[features]\nmemories = false\n", encoding="utf-8")
                yield

            with mock.patch.dict(
                sync_mod.os.environ,
                {"CODEX_HOME": str(codex)},
            ), mock.patch.object(
                sync_mod,
                "_validated_project_runtime",
                return_value=(project.resolve(), project / ".venv/Scripts/python.exe"),
            ), mock.patch.object(
                sync_mod,
                "user_hooks_lock",
                side_effect=disable_before_locked_read,
            ):
                result = sync_mod.main(
                    self._args("repair-hook", project_root=project)
                )
            self.assertEqual(result, 2)
            self.assertFalse((codex / "hooks.json").exists())

    def test_setup_rechecks_enable_confirmation_inside_user_lock(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            base = Path(raw)
            codex = base / ".codex"
            self._write_enabled_authorized_home(codex)
            project = base / "project"
            project.mkdir()
            config = codex / "config.toml"

            @contextlib.contextmanager
            def disable_before_locked_read(*_args: object, **_kwargs: object):
                config.write_text("[features]\nmemories = false\n", encoding="utf-8")
                yield

            with mock.patch.dict(
                sync_mod.os.environ,
                {"CODEX_HOME": str(codex)},
            ), mock.patch.object(
                sync_mod,
                "_validated_project_runtime",
                return_value=(project.resolve(), project / ".venv/Scripts/python.exe"),
            ), mock.patch.object(
                sync_mod,
                "user_hooks_lock",
                side_effect=disable_before_locked_read,
            ), mock.patch.object(
                sync_mod,
                "ensure_github_repository",
            ) as github:
                result = sync_mod.main(self._args("setup", project_root=project))
            self.assertEqual(result, 2)
            github.assert_not_called()
            self.assertFalse((codex / "hooks.json").exists())

    def _create_empty_remote(
        self,
        base: Path,
        manifest_changes: dict[str, object] | None = None,
    ) -> tuple[Path, dict[str, object], str]:
        remote = base / "remote.git"
        subprocess.run(["git", "init", "--bare", str(remote)], check=True, capture_output=True)
        source = base / "empty-source"
        source.mkdir()
        snapshot = base / "empty-snapshot"
        manifest = sync_mod.build_snapshot(source, snapshot, 2)
        if manifest_changes:
            manifest.update(manifest_changes)
            (snapshot / "snapshot-manifest.json").write_text(
                json.dumps(manifest) + "\n",
                encoding="utf-8",
            )
        commit = sync_mod._push_snapshot(snapshot, base / "publish-state", str(remote), None)
        return remote, manifest, commit

    def test_user_hook_command_uses_dynamic_git_root_runtime(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            script = Path(raw) / "product/scripts/codex_memory_sync.py"
            handler = sync_mod._hook_handler("SessionStart", script)
        self.assertIn('git rev-parse --show-toplevel', handler["command"])
        self.assertIn('$root/.venv/Scripts/python.exe', handler["command"])
        self.assertIn("--project-root", handler["command"])
        self.assertTrue(handler["commandWindows"].startswith("powershell.exe "))
        self.assertIn("-NonInteractive", handler["commandWindows"])
        self.assertIn("-WindowStyle Hidden", handler["commandWindows"])
        self.assertIn("-ExecutionPolicy Bypass", handler["commandWindows"])
        self.assertIn(sync_mod.WINDOWS_HOOK_WRAPPER_NAME, handler["commandWindows"])
        self.assertIn("SessionStart", handler["commandWindows"])
        self.assertNotIn("cmd.exe /d /c", handler["commandWindows"].lower())
        self.assertNotIn(str(ROOT), handler["command"])
        self.assertNotIn(str(Path(sys.executable).resolve()), handler["command"])

        spaced = sync_mod._windows_hook_command(
            Path("C:/Users/Example User/product/scripts/codex_memory_sync.py"),
            "Stop",
        )
        self.assertIn('-File "C:\\Users\\Example User', spaced)
        self.assertTrue(spaced.endswith('codex_memory_sync_hook.ps1" Stop'))

        metachar = sync_mod._windows_hook_command(
            Path("C:/Users/A&B/product/scripts/codex_memory_sync.py"),
            "Stop",
        )
        self.assertIn('-File "C:\\Users\\A&B', metachar)
        self.assertTrue(metachar.endswith('codex_memory_sync_hook.ps1" Stop'))

    def test_legacy_cmd_handler_is_independent_of_current_builder(self) -> None:
        script = Path("C:/Users/Example/product/scripts/codex_memory_sync.py")
        expected = sync_mod._legacy_cmd_hook_handler("Stop", script)
        with mock.patch.object(
            sync_mod,
            "_hook_handler",
            return_value={"future": "current-builder-changed"},
        ):
            actual = sync_mod._legacy_cmd_hook_handler("Stop", script)
        self.assertEqual(actual, expected)
        self.assertEqual(actual["timeout"], 120)
        self.assertTrue(actual["async"])
        self.assertIn("hook-run --event Stop", actual["command"])
        self.assertIn("cmd.exe /d /c", actual["commandWindows"])

    def test_status_reports_setup_hook_and_remote_runtime_state(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            codex = Path(raw) / ".codex"
            remote = "https://github.com/example/bridgeforge-codex-memories.git"
            self._write_ledger(
                codex,
                sync_mod._authorization_payload("approved", remote),
            )
            (codex / "config.toml").write_text(
                "[features]\nmemories = true\n[memories]\ngenerate_memories = true\nuse_memories = true\n",
                encoding="utf-8",
            )
            output = io.StringIO()
            with mock.patch.dict(sync_mod.os.environ, {"CODEX_HOME": str(codex)}), contextlib.redirect_stdout(output):
                self.assertEqual(sync_mod.main(self._args("status")), 0)
            receipt = json.loads(output.getvalue())
            self.assertTrue(receipt["enabled"])
            self.assertFalse(receipt["hookInstalled"])
            self.assertFalse(receipt["hookRuntimeVerified"])
            self.assertIsNone(receipt["hookRuntimeReceipt"])
            self.assertFalse(receipt["remoteConfigured"])
            self.assertEqual(receipt["consent"], "approved")
            self.assertEqual(receipt["consentPolicyVersion"], 1)
            self.assertEqual(receipt["syncMode"], "bidirectional")
            self.assertEqual(receipt["configuredRuntime"], sync_mod.DYNAMIC_HOOK_RUNTIME)
            self.assertEqual(receipt["actualRuntime"], str(Path(sync_mod.sys.executable).resolve()))
            self.assertIsNone(receipt["runtimeDriftReason"])

    def test_status_is_strictly_read_only(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            codex = Path(raw) / ".codex"
            self._write_ledger(
                codex,
                sync_mod._authorization_payload("declined", None),
            )
            before = {
                path.relative_to(codex).as_posix(): path.read_bytes()
                for path in codex.rglob("*")
                if path.is_file()
            }
            with mock.patch.dict(sync_mod.os.environ, {"CODEX_HOME": str(codex)}):
                self.assertEqual(sync_mod.main(self._args("status")), 0)
            after = {
                path.relative_to(codex).as_posix(): path.read_bytes()
                for path in codex.rglob("*")
                if path.is_file()
            }
            self.assertEqual(after, before)

    def test_status_emit_alert_acknowledges_the_same_failure_only_once(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            codex = Path(raw) / ".codex"
            self._write_ledger(
                codex,
                sync_mod._authorization_payload("declined", None),
            )
            state = codex / ".bridgeforge-codex/memory-sync"
            state.mkdir(parents=True)
            sync_mod._record_health(state, "failed", error="fixture network failure")
            output = io.StringIO()
            errors = io.StringIO()
            with mock.patch.dict(
                sync_mod.os.environ,
                {"CODEX_HOME": str(codex)},
            ), contextlib.redirect_stdout(output), contextlib.redirect_stderr(errors):
                self.assertEqual(sync_mod.main(self._args("status", "--emit-alert")), 0)
                self.assertEqual(sync_mod.main(self._args("status", "--emit-alert")), 0)
            self.assertEqual(errors.getvalue().count("fixture network failure"), 1)
            health = json.loads((state / "health.json").read_text(encoding="utf-8"))
            self.assertEqual(health["alertedId"], health["alertId"])

    def test_status_emit_alert_is_quiet_and_zero_write_when_healthy(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            codex = Path(raw) / ".codex"
            self._write_ledger(
                codex,
                sync_mod._authorization_payload("declined", None),
            )
            state = codex / ".bridgeforge-codex/memory-sync"
            state.mkdir(parents=True)
            sync_mod._record_health(state, "healthy", action="noop")
            before = {
                path.relative_to(codex).as_posix(): path.read_bytes()
                for path in codex.rglob("*")
                if path.is_file()
            }
            output = io.StringIO()
            errors = io.StringIO()
            with mock.patch.dict(
                sync_mod.os.environ,
                {"CODEX_HOME": str(codex)},
            ), contextlib.redirect_stdout(output), contextlib.redirect_stderr(errors):
                self.assertEqual(sync_mod.main(self._args("status", "--emit-alert")), 0)
            after = {
                path.relative_to(codex).as_posix(): path.read_bytes()
                for path in codex.rglob("*")
                if path.is_file()
            }
            self.assertEqual(errors.getvalue(), "")
            self.assertEqual(after, before)

    def test_hook_run_records_success_and_failure_without_invalid_stdout(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            base = Path(raw)
            codex = base / ".codex"
            project = base / "project"
            project.mkdir()
            self._write_enabled_authorized_home(codex)
            remote = "https://github.com/example/bridgeforge-codex-memories.git"
            output = io.StringIO()
            errors = io.StringIO()
            common = (
                mock.patch.dict(sync_mod.os.environ, {"CODEX_HOME": str(codex)}),
                mock.patch.object(
                    sync_mod,
                    "_validated_project_runtime",
                    return_value=(project.resolve(), project / ".venv/Scripts/python.exe"),
                ),
                mock.patch.object(
                    sync_mod,
                    "require_runtime_authorization",
                    return_value={"remote": remote},
                ),
                mock.patch.object(sync_mod, "verify_private_github_repository"),
            )
            with common[0], common[1], common[2], common[3], mock.patch.object(
                sync_mod,
                "launch_background_reconcile",
                return_value="worker-started",
            ), contextlib.redirect_stdout(output), contextlib.redirect_stderr(errors):
                self.assertEqual(
                    sync_mod.main(
                        self._args(
                            "hook-run",
                            "--event",
                            "SessionStart",
                            project_root=project,
                        )
                    ),
                    0,
                )
            self.assertEqual(output.getvalue(), "")
            self.assertEqual(errors.getvalue(), "")
            state = codex / ".bridgeforge-codex/memory-sync"
            receipt = json.loads(
                (state / "hook-runtime-sessionstart.json").read_text(encoding="utf-8")
            )
            self.assertEqual(receipt["status"], "queued")
            self.assertEqual(receipt["action"], "worker-started")
            self.assertTrue(sync_mod.hook_runtime_verified(receipt))

            with common[0], common[1], common[2], common[3], mock.patch.object(
                sync_mod,
                "launch_background_reconcile",
                side_effect=sync_mod.SyncError("fixture failure"),
            ), contextlib.redirect_stderr(errors):
                self.assertEqual(
                    sync_mod.main(
                        self._args(
                            "hook-run",
                            "--event",
                            "Stop",
                            project_root=project,
                        )
                    ),
                    0,
                )
            failed = json.loads(
                (state / "hook-runtime-stop.json").read_text(encoding="utf-8")
            )
            self.assertEqual(failed["status"], "failed")
            self.assertIn("fixture failure", failed["error"])
            self.assertFalse(sync_mod.hook_runtime_verified(failed))
            self.assertTrue((state / "pending.json").is_file())

    @unittest.skipUnless(os.name == "nt", "Windows hidden wrapper integration")
    def test_windows_hidden_hook_command_handles_metachar_path_and_io(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            scripts = Path(raw) / "A&B Product" / "scripts"
            scripts.mkdir(parents=True)
            script = scripts / "codex_memory_sync.py"
            wrapper = scripts / sync_mod.WINDOWS_HOOK_WRAPPER_NAME
            wrapper.write_text(
                "param([Parameter(Mandatory=$true)][string]$Event)\n"
                "$payload = [Console]::In.ReadToEnd()\n"
                "[Console]::Out.Write(\"event=$Event;input=$payload\")\n"
                "[Console]::Error.Write(\"probe-error\")\n"
                "exit 7\n",
                encoding="utf-8-sig",
            )
            completed = subprocess.run(
                sync_mod._windows_hook_command(script, "Stop"),
                input="probe-input",
                capture_output=True,
                text=True,
                timeout=30,
                shell=True,
            )
        self.assertEqual(completed.returncode, 7)
        self.assertEqual(completed.stdout, "event=Stop;input=probe-input")
        self.assertEqual(completed.stderr, "probe-error")

    @unittest.skipUnless(os.name == "nt", "Windows hidden wrapper integration")
    def test_windows_hidden_hook_wrapper_reaches_project_python(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            codex = Path(raw) / ".codex"
            state = codex / ".bridgeforge-codex/memory-sync"
            state.mkdir(parents=True)
            (codex / "config.toml").write_text(
                "[features]\nmemories = false\n",
                encoding="utf-8",
            )
            environment = os.environ.copy()
            environment["CODEX_HOME"] = str(codex)
            handler = sync_mod._hook_handler(
                "SessionStart",
                ROOT / "scripts/codex_memory_sync.py",
            )
            completed = subprocess.run(
                str(handler["commandWindows"]),
                cwd=ROOT,
                env=environment,
                capture_output=True,
                text=True,
                timeout=30,
                shell=True,
            )
            self.assertEqual(completed.returncode, 0, completed.stderr)
            dispatch = (state / "hook-dispatch.log").read_text(encoding="utf-8")
            self.assertIn("stage=wrapper-start", dispatch)
            self.assertIn("stage=python-exit-0", dispatch)
            receipt = json.loads(
                (state / "hook-runtime-sessionstart.json").read_text(encoding="utf-8")
            )
            self.assertEqual(receipt["status"], "disabled")
            self.assertEqual(receipt["action"], "disabled")

    def test_approved_enabled_repair_is_local_only(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            codex = Path(raw) / ".codex"
            remote = "https://github.com/example/bridgeforge-codex-memories.git"
            self._write_ledger(
                codex,
                sync_mod._authorization_payload("approved", remote),
            )
            (codex / "config.toml").write_text(
                "[features]\nmemories = true\n[memories]\ngenerate_memories = true\nuse_memories = true\n",
                encoding="utf-8",
            )
            state = codex / ".bridgeforge-codex" / "memory-sync"
            state.mkdir(parents=True)
            (state / "remote.txt").write_text(remote + "\n", encoding="utf-8")
            output = io.StringIO()
            with mock.patch.dict(sync_mod.os.environ, {"CODEX_HOME": str(codex)}), mock.patch.object(
                sync_mod,
                "ensure_github_repository",
                side_effect=AssertionError("repair must not access GitHub"),
            ) as github, mock.patch.object(
                sync_mod,
                "reconcile",
                side_effect=AssertionError("repair must not reconcile memories"),
            ) as reconcile, contextlib.redirect_stdout(output):
                self.assertEqual(sync_mod.main(self._args("repair-hook")), 0)
            github.assert_not_called()
            reconcile.assert_not_called()
            self.assertTrue(
                sync_mod.user_hooks_healthy(
                    codex / "hooks.json",
                    Path(sync_mod.__file__).resolve(),
                )
            )
            authorization = sync_mod.native_memories_authorization(
                codex / "bridgeforge-codex-managed.json"
            )
            self.assertIsInstance(authorization, dict)
            self.assertEqual(authorization["remote"], remote.removesuffix(".git"))
            self.assertIn("remote_reconcile=not_requested", output.getvalue())

    def test_two_projects_repair_sequentially_without_runtime_churn(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            base = Path(raw)
            codex = base / ".codex"
            self._write_enabled_authorized_home(codex)
            project_a = base / "project-a"
            project_b = base / "project-b"
            project_a.mkdir()
            project_b.mkdir()
            outputs: list[str] = []

            def validated(root: Path) -> tuple[Path, Path]:
                resolved = root.resolve()
                return resolved, resolved / ".venv/Scripts/python.exe"

            with mock.patch.dict(sync_mod.os.environ, {"CODEX_HOME": str(codex)}), mock.patch.object(
                sync_mod,
                "_validated_project_runtime",
                side_effect=validated,
            ), mock.patch.object(
                sync_mod,
                "ensure_github_repository",
                side_effect=AssertionError("repair must not access GitHub"),
            ), mock.patch.object(
                sync_mod,
                "_git",
                side_effect=AssertionError("repair must not access Git"),
            ), mock.patch.object(
                sync_mod,
                "_memory_files",
                side_effect=AssertionError("repair must not read memories"),
            ), mock.patch.object(
                sync_mod,
                "reconcile",
                side_effect=AssertionError("repair must not reconcile memories"),
            ):
                for project in (project_a, project_b):
                    output = io.StringIO()
                    with contextlib.redirect_stdout(output):
                        self.assertEqual(
                            sync_mod.main(self._args("repair-hook", project_root=project)),
                            0,
                        )
                    outputs.append(output.getvalue())

            self.assertIn("hook_repair=applied", outputs[0])
            self.assertIn("hook_repair=unchanged", outputs[1])
            hooks = (codex / "hooks.json").read_text(encoding="utf-8")
            self.assertNotIn(str(project_a), hooks)
            self.assertNotIn(str(project_b), hooks)
            self.assertIn("git rev-parse --show-toplevel", hooks)

    @unittest.skipUnless(sys.platform == "win32", "Windows project .venv contract required")
    def test_two_project_processes_serialize_hook_repair(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            base = Path(raw)
            codex = base / ".codex"
            self._write_enabled_authorized_home(codex)
            projects = [base / "project-a", base / "project-b"]
            for project in projects:
                project.mkdir()
                created = subprocess.run(
                    [sys.executable, "-m", "venv", str(project / ".venv")],
                    capture_output=True,
                    text=True,
                    encoding="utf-8",
                    errors="replace",
                    check=False,
                )
                self.assertEqual(created.returncode, 0, created.stderr)

            worker = ROOT / "scripts/tests/native_memory_repair_worker.py"
            source = ROOT / "scripts/codex_memory_sync.py"
            processes = [
                subprocess.Popen(
                    [
                        str(project / ".venv/Scripts/python.exe"),
                        str(worker),
                        str(source),
                        str(codex),
                        str(project),
                    ],
                    stdout=subprocess.PIPE,
                    stderr=subprocess.PIPE,
                    text=True,
                    encoding="utf-8",
                    errors="replace",
                )
                for project in projects
            ]
            results = [process.communicate(timeout=60) for process in processes]
            self.assertTrue(all(process.returncode == 0 for process in processes), results)
            receipts = [stdout for stdout, _stderr in results]
            self.assertEqual(sum("hook_repair=applied" in item for item in receipts), 1)
            self.assertEqual(sum("hook_repair=unchanged" in item for item in receipts), 1)
            hooks = (codex / "hooks.json").read_text(encoding="utf-8")
            self.assertTrue(all(str(project) not in hooks for project in projects))
            self.assertTrue(sync_mod.user_hooks_healthy(codex / "hooks.json", source))

    def test_status_is_read_only_even_when_codex_home_is_missing(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            codex = Path(raw) / "missing-codex"
            with mock.patch.dict(sync_mod.os.environ, {"CODEX_HOME": str(codex)}):
                self.assertEqual(sync_mod.main(self._args("status")), 2)
            self.assertFalse(codex.exists())

    def test_setup_leaves_config_untouched_when_github_preflight_fails(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            codex = Path(raw) / ".codex"
            self._write_ledger(codex)
            with mock.patch.dict(sync_mod.os.environ, {"CODEX_HOME": str(codex)}), mock.patch.object(
                sync_mod, "ensure_github_repository", side_effect=sync_mod.SyncError("gh unavailable")
            ):
                self.assertEqual(sync_mod.main(self._args("setup", "--confirmed-enable")), 2)
            self.assertFalse((codex / "config.toml").exists())
            self.assertFalse((codex / "hooks.json").exists())

    def test_setup_persists_only_the_dynamic_project_runtime_contract(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            codex = root / ".codex"
            self._write_ledger(codex)
            venv_python = root / "project/.venv/Scripts/python.exe"
            venv_python.parent.mkdir(parents=True)
            venv_python.write_bytes(b"venv")
            output = io.StringIO()
            with mock.patch.dict(sync_mod.os.environ, {"CODEX_HOME": str(codex)}), mock.patch.object(
                sync_mod.sys,
                "executable",
                str(venv_python),
            ), mock.patch.object(
                sync_mod,
                "_validated_project_runtime",
                return_value=((root / "project").resolve(), venv_python.resolve()),
            ), mock.patch.object(
                sync_mod,
                "ensure_github_repository",
                return_value=("https://github.com/example/bridgeforge-codex-memories.git", "created"),
            ), contextlib.redirect_stdout(output):
                self.assertEqual(
                    sync_mod.main(self._args(
                        "setup",
                        "--confirmed-enable",
                        project_root=root / "project",
                    )),
                    0,
                )
            hooks = json.loads((codex / "hooks.json").read_text(encoding="utf-8"))
            managed = [
                handler
                for entries in hooks["hooks"].values()
                for entry in entries
                for handler in entry["hooks"]
                if handler.get(sync_mod.HOOK_MARKER_KEY, "").startswith(sync_mod.HOOK_ID)
            ]
            self.assertEqual(len(managed), 3)
            self.assertTrue(all(sync_mod.DYNAMIC_HOOK_RUNTIME not in handler["command"] for handler in managed))
            self.assertTrue(all(str(venv_python.resolve()) not in handler["command"] for handler in managed))
            self.assertTrue(all("git rev-parse --show-toplevel" in handler["command"] for handler in managed))
            self.assertIn(f"actual_runtime={venv_python.resolve()}", output.getvalue())
            self.assertIn(f"configured_runtime={sync_mod.DYNAMIC_HOOK_RUNTIME}", output.getvalue())
            self.assertIn("remote_action=created", output.getvalue())
            self.assertEqual(
                (codex / ".bridgeforge-codex/memory-sync/remote.txt").read_text(encoding="utf-8"),
                "https://github.com/example/bridgeforge-codex-memories.git\n",
            )
            self.assertEqual(
                sync_mod.native_memories_consent(codex / "bridgeforge-codex-managed.json"),
                "approved",
            )
            authorization = sync_mod.native_memories_authorization(
                codex / "bridgeforge-codex-managed.json"
            )
            self.assertIsInstance(authorization, dict)
            self.assertEqual(authorization["policy_version"], 1)
            self.assertEqual(authorization["scope"], "~/.codex/memories/**")
            self.assertEqual(authorization["sync_mode"], "bidirectional")
            self.assertTrue(authorization["auto_hook_maintenance"])
            self.assertTrue(authorization["require_private"])

    def test_declined_consent_is_persisted_without_external_or_config_writes(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            codex = Path(raw) / ".codex"
            ledger = self._write_ledger(codex)
            with mock.patch.dict(sync_mod.os.environ, {"CODEX_HOME": str(codex)}), mock.patch.object(
                sync_mod,
                "ensure_github_repository",
            ) as github, mock.patch.object(sync_mod, "merge_user_hooks") as hooks:
                self.assertEqual(sync_mod.main(self._args("decline", "--confirmed")), 0)
            github.assert_not_called()
            hooks.assert_not_called()
            self.assertEqual(sync_mod.native_memories_consent(ledger), "declined")
            self.assertFalse((codex / "config.toml").exists())
            self.assertFalse((codex / "hooks.json").exists())
            self.assertFalse((codex / ".bridgeforge-codex").exists())

    def test_decline_updates_ledger_only_inside_the_shared_user_lock(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            codex = Path(raw) / ".codex"
            self._write_ledger(codex)
            events: list[str] = []

            @contextlib.contextmanager
            def tracked_lock(*_args: object, **_kwargs: object):
                events.append("entered")
                try:
                    yield
                finally:
                    events.append("exited")

            original_record = sync_mod.record_native_memories_consent

            def tracked_record(*args: object, **kwargs: object) -> bool:
                self.assertEqual(events, ["entered"])
                return original_record(*args, **kwargs)

            with mock.patch.dict(
                sync_mod.os.environ,
                {"CODEX_HOME": str(codex)},
            ), mock.patch.object(
                sync_mod,
                "user_hooks_lock",
                side_effect=tracked_lock,
            ), mock.patch.object(
                sync_mod,
                "record_native_memories_consent",
                side_effect=tracked_record,
            ):
                self.assertEqual(
                    sync_mod.main(self._args("decline", "--confirmed")),
                    0,
                )
            self.assertEqual(events, ["entered", "exited"])

    def test_consent_ledger_validation_is_fail_closed(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            codex = Path(raw) / ".codex"
            ledger = self._write_ledger(codex)
            data = json.loads(ledger.read_text(encoding="utf-8"))
            data["consents"] = {"native_memories": "maybe"}
            ledger.write_text(json.dumps(data), encoding="utf-8")
            with self.assertRaisesRegex(sync_mod.SyncError, "invalid native memories consent"):
                sync_mod.native_memories_consent(ledger)

    def test_runtime_authorization_rejects_remote_scope_drift(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            codex = Path(raw) / ".codex"
            first = "https://github.com/example/bridgeforge-codex-memories.git"
            second = "https://github.com/other/bridgeforge-codex-memories.git"
            self._write_ledger(
                codex,
                sync_mod._authorization_payload("approved", first),
            )
            state = codex / ".bridgeforge-codex/memory-sync"
            state.mkdir(parents=True)
            (state / "remote.txt").write_text(second + "\n", encoding="utf-8")
            with self.assertRaisesRegex(sync_mod.SyncError, "remote changed"):
                sync_mod.require_runtime_authorization(
                    codex / "bridgeforge-codex-managed.json",
                    state,
                )

    def test_automatic_reconcile_checks_private_authorization_before_sync(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            codex = Path(raw) / ".codex"
            remote = "https://github.com/example/bridgeforge-codex-memories.git"
            self._write_ledger(
                codex,
                sync_mod._authorization_payload("approved", remote),
            )
            (codex / "config.toml").write_text(
                "[features]\nmemories = true\n[memories]\ngenerate_memories = true\nuse_memories = true\n",
                encoding="utf-8",
            )
            state = codex / ".bridgeforge-codex/memory-sync"
            state.mkdir(parents=True)
            (state / "remote.txt").write_text(remote + "\n", encoding="utf-8")
            with mock.patch.dict(
                sync_mod.os.environ,
                {"CODEX_HOME": str(codex)},
            ), mock.patch.object(
                sync_mod,
                "verify_private_github_repository",
            ) as verify, mock.patch.object(
                sync_mod,
                "reconcile",
                return_value="noop",
            ) as reconcile:
                self.assertEqual(
                    sync_mod.main(self._args("reconcile", "--trigger", "sessionstart")),
                    0,
                )
            verify.assert_called_once_with(remote.removesuffix(".git"))
            reconcile.assert_called_once_with(
                codex / "memories",
                state,
                remote.removesuffix(".git"),
            )

    def test_private_repository_verification_rejects_public_visibility(self) -> None:
        def run(command: list[str], **_kwargs: object) -> subprocess.CompletedProcess[str]:
            if command[:3] == ["gh", "auth", "status"]:
                return subprocess.CompletedProcess(command, 0, "", "")
            return subprocess.CompletedProcess(
                command,
                0,
                json.dumps({
                    "visibility": "PUBLIC",
                    "url": "https://github.com/example/bridgeforge-codex-memories",
                    "nameWithOwner": "example/bridgeforge-codex-memories",
                }),
                "",
            )

        with mock.patch.object(sync_mod.shutil, "which", return_value="gh"):
            with self.assertRaisesRegex(sync_mod.SyncError, "no longer private"):
                sync_mod.verify_private_github_repository(
                    "https://github.com/example/bridgeforge-codex-memories",
                    run=run,
                )

    def test_private_repository_verification_falls_back_to_git_credentials(self) -> None:
        secret = "never-print-this-token"
        calls: list[list[str]] = []

        def run(command: list[str], **_kwargs: object) -> subprocess.CompletedProcess[str]:
            calls.append(command)
            if command[0] == "gh":
                return subprocess.CompletedProcess(command, 1, "", "expired login")
            return subprocess.CompletedProcess(
                command,
                0,
                f"protocol=https\nhost=github.com\nusername=example\npassword={secret}\n",
                "",
            )

        response = mock.MagicMock()
        response.__enter__.return_value.read.return_value = json.dumps({
            "private": True,
            "full_name": "example/bridgeforge-codex-memories",
        }).encode("utf-8")
        with mock.patch.object(sync_mod.shutil, "which", return_value="gh"), mock.patch.object(
            sync_mod.urllib.request,
            "urlopen",
            return_value=response,
        ) as urlopen:
            sync_mod.verify_private_github_repository(
                "https://github.com/example/bridgeforge-codex-memories",
                run=run,
            )
        credential_call = next(command for command in calls if command[0] == "git")
        self.assertEqual(
            credential_call,
            ["git", "-c", "credential.interactive=never", "credential", "fill"],
        )
        request = urlopen.call_args.args[0]
        self.assertEqual(request.get_header("Authorization"), f"Bearer {secret}")

    def test_git_credential_fallback_rejects_public_repository(self) -> None:
        def run(command: list[str], **_kwargs: object) -> subprocess.CompletedProcess[str]:
            return subprocess.CompletedProcess(
                command,
                0,
                "protocol=https\nhost=github.com\npassword=secret\n",
                "",
            )

        response = mock.MagicMock()
        response.__enter__.return_value.read.return_value = json.dumps({
            "private": False,
            "full_name": "example/bridgeforge-codex-memories",
        }).encode("utf-8")
        with mock.patch.object(sync_mod.shutil, "which", return_value=None), mock.patch.object(
            sync_mod.urllib.request,
            "urlopen",
            return_value=response,
        ):
            with self.assertRaisesRegex(sync_mod.SyncError, "no longer private"):
                sync_mod.verify_private_github_repository(
                    "git@github.com:example/bridgeforge-codex-memories.git",
                    run=run,
                )

    def test_git_credential_fallback_does_not_expose_secret_on_api_failure(self) -> None:
        secret = "never-print-this-token"

        def run(command: list[str], **_kwargs: object) -> subprocess.CompletedProcess[str]:
            return subprocess.CompletedProcess(
                command,
                0,
                f"protocol=https\nhost=github.com\npassword={secret}\n",
                "",
            )

        with mock.patch.object(sync_mod.shutil, "which", return_value=None), mock.patch.object(
            sync_mod.urllib.request,
            "urlopen",
            side_effect=sync_mod.urllib.error.URLError("offline"),
        ):
            with self.assertRaises(sync_mod.SyncError) as caught:
                sync_mod.verify_private_github_repository(
                    "https://github.com/example/bridgeforge-codex-memories",
                    run=run,
                )
        self.assertNotIn(secret, str(caught.exception))

    def test_config_merge_requires_confirmation_and_preserves_other_toml(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            config = Path(raw) / "config.toml"
            original = "model = 'custom'\n[features]\nfoo = true\nmemories = false # keep comment\n[memories]\ncustom = 7\n"
            config.write_text(original, encoding="utf-8")
            with self.assertRaises(sync_mod.SyncError):
                sync_mod.enable_memories(config, confirmed=False)
            self.assertEqual(config.read_text(encoding="utf-8"), original)
            self.assertTrue(sync_mod.enable_memories(config, confirmed=True))
            enabled, data = sync_mod.memory_switches(config)
            self.assertTrue(enabled)
            self.assertEqual(data["model"], "custom")
            self.assertEqual(data["memories"]["custom"], 7)
            self.assertIn("memories = true # keep comment", config.read_text(encoding="utf-8"))

    def test_user_hook_merge_is_idempotent_and_preserves_third_party(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            path = Path(raw) / "hooks.json"
            third_party = {"matcher": "*", "vendor": {"opaque": [1, 2]}, "hooks": [{"type": "command", "command": "display", "async": False}]}
            path.write_text(json.dumps({"custom": "keep", "hooks": {"SessionStart": [third_party]}}), encoding="utf-8")
            script = Path(raw) / "runtime.py"
            self.assertTrue(sync_mod.merge_user_hooks(path, script))
            self.assertTrue(sync_mod.user_hooks_healthy(path, script))
            first = path.read_bytes()
            self.assertFalse(sync_mod.merge_user_hooks(path, script))
            self.assertEqual(first, path.read_bytes())
            data = json.loads(first)
            self.assertEqual(data["custom"], "keep")
            self.assertEqual(data["hooks"]["SessionStart"][0], third_party)
            handlers = [
                handler
                for entries in data["hooks"].values()
                for entry in entries
                for handler in entry.get("hooks", [])
                if sync_mod.HOOK_MARKER_KEY in handler
            ]
            self.assertEqual(len(handlers), 3)
            self.assertTrue(all("git rev-parse --show-toplevel" in handler["command"] for handler in handlers))
            self.assertTrue(all("-WindowStyle Hidden" in handler["commandWindows"] for handler in handlers))
            self.assertTrue(all("cmd.exe /d /c" not in handler["commandWindows"] for handler in handlers))
            self.assertTrue(all(sync_mod.WINDOWS_HOOK_WRAPPER_NAME in handler["commandWindows"] for handler in handlers))
            session_end = next(h for h in handlers if h[sync_mod.HOOK_MARKER_KEY].endswith(":SessionEnd"))
            stop = next(h for h in handlers if h[sync_mod.HOOK_MARKER_KEY].endswith(":Stop"))
            session_start = next(h for h in handlers if h[sync_mod.HOOK_MARKER_KEY].endswith(":SessionStart"))
            self.assertEqual(session_end["timeout"], 3)
            self.assertNotIn("async", session_end)
            self.assertIn("hook-run --event SessionEnd", session_end["command"])
            self.assertTrue(stop["async"])
            self.assertEqual(stop["timeout"], 120)
            self.assertEqual(session_start["timeout"], 120)

    def test_exact_legacy_hooks_upgrade_but_edited_legacy_drift_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            base = Path(raw)
            path = base / "hooks.json"
            script = base / "runtime.py"
            third_party = {
                "matcher": "*",
                "hooks": [{"type": "command", "command": "third-party"}],
            }
            legacy_document = {
                "custom": "keep",
                "hooks": {
                    "SessionStart": [
                        third_party,
                        {
                            "hooks": [
                                sync_mod._legacy_inline_powershell_hook_handler(
                                    "SessionStart", script
                                )
                            ]
                        },
                    ],
                    "Stop": [
                        {
                            "hooks": [
                                sync_mod._legacy_inline_powershell_hook_handler(
                                    "Stop", script
                                )
                            ]
                        }
                    ],
                    "SessionEnd": [
                        {
                            "hooks": [
                                sync_mod._legacy_inline_powershell_hook_handler(
                                    "SessionEnd", script
                                )
                            ]
                        }
                    ],
                },
            }
            path.write_text(json.dumps(legacy_document), encoding="utf-8")
            self.assertFalse(sync_mod.user_hooks_healthy(path, script))
            self.assertTrue(sync_mod.merge_user_hooks(path, script))
            self.assertTrue(sync_mod.user_hooks_healthy(path, script))
            upgraded = json.loads(path.read_text(encoding="utf-8"))
            self.assertEqual(upgraded["custom"], "keep")
            self.assertEqual(upgraded["hooks"]["SessionStart"][0], third_party)
            self.assertFalse(sync_mod.merge_user_hooks(path, script))

            legacy_cmd = base / "legacy-cmd.json"
            legacy_cmd.write_text(
                json.dumps(
                    {
                        "hooks": {
                            event: [
                                {
                                    "hooks": [
                                        sync_mod._legacy_cmd_hook_handler(
                                            event,
                                            script,
                                        )
                                    ]
                                }
                            ]
                            for event in sync_mod.HOOK_EVENTS
                        }
                    }
                ),
                encoding="utf-8",
            )
            self.assertFalse(sync_mod.user_hooks_healthy(legacy_cmd, script))
            self.assertTrue(sync_mod.merge_user_hooks(legacy_cmd, script))
            self.assertTrue(sync_mod.user_hooks_healthy(legacy_cmd, script))
            migrated_cmd = json.loads(legacy_cmd.read_text(encoding="utf-8"))
            for event in sync_mod.HOOK_EVENTS:
                command = migrated_cmd["hooks"][event][0]["hooks"][0][
                    "commandWindows"
                ]
                self.assertIn("-WindowStyle Hidden", command)
                self.assertNotIn("cmd.exe /d /c", command)

            edited = base / "edited-legacy.json"
            edited_handler = sync_mod._legacy_inline_powershell_hook_handler(
                "SessionStart", script
            )
            edited_handler["commandWindows"] += " user-edit"
            edited.write_text(
                json.dumps(
                    {
                        "hooks": {
                            "SessionStart": [{"hooks": [edited_handler]}]
                        },
                    }
                ),
                encoding="utf-8",
            )
            before = edited.read_bytes()
            with self.assertRaisesRegex(sync_mod.SyncError, "content drifted"):
                sync_mod.merge_user_hooks(edited, script)
            self.assertEqual(edited.read_bytes(), before)

    def test_user_hook_duplicate_key_and_managed_drift_are_zero_write(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            base = Path(raw)
            script = base / "runtime.py"
            duplicate = base / "duplicate.json"
            duplicate.write_text('{"hooks":{},"hooks":{}}\n', encoding="utf-8")
            duplicate_before = duplicate.read_bytes()
            with self.assertRaisesRegex(sync_mod.SyncError, "duplicate JSON key"):
                sync_mod.merge_user_hooks(duplicate, script)
            self.assertEqual(duplicate.read_bytes(), duplicate_before)

            drift = base / "drift.json"
            handler = sync_mod._hook_handler("SessionStart", script)
            handler["command"] = "user-edited-command"
            drift.write_text(
                json.dumps({"hooks": {"SessionStart": [{"hooks": [handler]}]}}),
                encoding="utf-8",
            )
            drift_before = drift.read_bytes()
            with self.assertRaisesRegex(sync_mod.SyncError, "content drifted"):
                sync_mod.merge_user_hooks(drift, script)
            self.assertEqual(drift.read_bytes(), drift_before)

    def test_user_hook_cas_preserves_an_external_concurrent_write(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            path = Path(raw) / "hooks.json"
            script = Path(raw) / "runtime.py"
            path.write_text('{"hooks":{}}\n', encoding="utf-8")
            external = b'{"external":"wins","hooks":{}}\n'
            original_renderer = sync_mod._render_user_hooks

            def concurrent_write(payload: bytes | None, target: Path, managed_script: Path) -> bytes:
                desired = original_renderer(payload, target, managed_script)
                target.write_bytes(external)
                return desired

            with mock.patch.object(sync_mod, "_render_user_hooks", side_effect=concurrent_write):
                with self.assertRaisesRegex(sync_mod.HookLockConflict, "changed during"):
                    sync_mod.merge_user_hooks(path, script)
            self.assertEqual(path.read_bytes(), external)

    def test_session_end_kick_detaches_reconciliation(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            codex = Path(raw) / ".codex"
            state = codex / ".bridgeforge-codex/memory-sync"
            state.mkdir(parents=True)
            process = mock.Mock(pid=12345)
            with mock.patch.dict(sync_mod.os.environ, {"CODEX_HOME": str(codex)}), mock.patch.object(
                sync_mod.subprocess, "Popen", return_value=process
            ) as popen:
                self.assertEqual(
                    sync_mod.launch_background_reconcile("session-end", ROOT),
                    "worker-started",
                )
        command = popen.call_args.args[0]
        kwargs = popen.call_args.kwargs
        self.assertEqual(command[0], str(Path(sync_mod.sys.executable).resolve()))
        self.assertEqual(command[2], "worker")
        self.assertEqual(command[-2:], ["--project-root", str(ROOT.resolve())])
        self.assertEqual(kwargs["cwd"], ROOT.resolve())
        self.assertIs(kwargs["stdout"], subprocess.DEVNULL)
        if sync_mod.os.name == "nt":
            self.assertTrue(kwargs["creationflags"] & 0x00000008)
        else:
            self.assertTrue(kwargs["start_new_session"])

    def test_external_command_timeout_becomes_a_normal_failure_receipt(self) -> None:
        with mock.patch.object(sync_mod.subprocess, "run", side_effect=subprocess.TimeoutExpired(["git"], 45)):
            result = sync_mod._default_run(["git", "fetch"])
        self.assertEqual(result.returncode, 124)
        self.assertIn("timed out", result.stderr)

    def test_snapshot_excludes_temp_lock_metadata_and_detects_conflict(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            base = Path(raw)
            source = base / "memories"
            source.mkdir()
            (source / "kept.md").write_text("keep", encoding="utf-8")
            (source / "skip.tmp").write_text("temp", encoding="utf-8")
            (source / "writer.lock").write_text("lock", encoding="utf-8")
            manifest = sync_mod.build_snapshot(source, base / "snapshot", 4)
            self.assertEqual(manifest["revision"], 4)
            self.assertEqual([item["path"] for item in manifest["files"]], ["kept.md"])
            self.assertFalse((base / "snapshot/memories/skip.tmp").exists())
            sync_mod.verify_snapshot(base / "snapshot", manifest)
            (base / "snapshot/memories/kept.md").write_text("tampered", encoding="utf-8")
            with self.assertRaises(sync_mod.SyncError):
                sync_mod.verify_snapshot(base / "snapshot", manifest)

    @unittest.skipUnless(sys.platform == "win32", "Windows junction semantics required")
    def test_snapshot_rejects_root_and_nested_directory_junctions(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            base = Path(raw)
            outside = base / "outside"
            outside.mkdir()
            (outside / "secret.md").write_text("outside", encoding="utf-8")

            root_link = base / "root-link"
            nested_root = base / "memories"
            nested_root.mkdir()
            nested_link = nested_root / "linked"
            for link in (root_link, nested_link):
                subprocess.run(
                    ["cmd.exe", "/d", "/c", "mklink", "/J", str(link), str(outside)],
                    check=True,
                    capture_output=True,
                    text=True,
                )
            try:
                with self.assertRaises(sync_mod.SyncError):
                    sync_mod.capture_manifest(root_link, 1)
                with self.assertRaises(sync_mod.SyncError):
                    sync_mod.capture_manifest(nested_root, 1)
            finally:
                os.rmdir(nested_link)
                os.rmdir(root_link)
            self.assertEqual(
                sync_mod.choose_action(
                    "local-new", "remote-new", "old",
                    local_updated_at="2026-08-14T12:00:00+00:00",
                    remote_updated_at="2026-08-14T11:00:00+00:00",
                ),
                "merge",
            )
            self.assertEqual(
                sync_mod.choose_action(
                    "local-new", "remote-new", None,
                    local_updated_at="1970-01-01T00:00:00+00:00",
                    remote_updated_at="2026-08-14T11:00:00+00:00",
                ),
                "merge",
            )

    def test_reconcile_does_not_create_native_memories_without_local_or_remote_content(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            base = Path(raw)
            memories = base / "memories"
            with mock.patch.object(sync_mod, "_read_remote_snapshot", return_value=(None, None, None)):
                action = sync_mod.reconcile(memories, base / "state", "unused")
            self.assertEqual(action, "noop")
            self.assertFalse(memories.exists())

    @unittest.skipUnless(shutil.which("git"), "git required")
    def test_real_empty_remote_is_a_quiet_noop_without_creating_native_memories(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            base = Path(raw)
            remote, manifest, commit = self._create_empty_remote(base)
            codex = base / ".codex"
            codex.mkdir()
            (codex / "config.toml").write_text(
                "[features]\nmemories = true\n[memories]\ngenerate_memories = true\nuse_memories = true\n",
                encoding="utf-8",
            )
            state = codex / ".bridgeforge-codex/memory-sync"
            state.mkdir(parents=True)
            (state / "remote.txt").write_text(str(remote), encoding="utf-8")
            sync_mod.mark_pending(state, "bridgeforge")
            output = io.StringIO()
            errors = io.StringIO()
            with mock.patch.dict(sync_mod.os.environ, {"CODEX_HOME": str(codex)}), contextlib.redirect_stdout(
                output
            ), contextlib.redirect_stderr(errors), mock.patch.object(
                sync_mod,
                "require_runtime_authorization",
                return_value={"remote": str(remote)},
            ), mock.patch.object(sync_mod, "verify_private_github_repository"):
                self.assertEqual(sync_mod.main(self._args("reconcile", "--trigger", "bridgeforge")), 0)
            receipt = json.loads((state / "last-synced.json").read_text(encoding="utf-8"))
            self.assertEqual(output.getvalue(), "[memory-sync] noop\n")
            self.assertEqual(errors.getvalue(), "")
            self.assertFalse((codex / "memories").exists())
            self.assertFalse((state / "pending.json").exists())
            self.assertEqual(receipt["content_sha256"], manifest["content_sha256"])
            self.assertEqual(receipt["commit"], commit)

    @unittest.skipUnless(shutil.which("git"), "git required")
    def test_empty_local_directory_and_empty_remote_are_a_noop(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            base = Path(raw)
            remote, _manifest, _commit = self._create_empty_remote(base)
            memories = base / "memories"
            memories.mkdir()
            state = base / "state"
            sync_mod.mark_pending(state, "stop")
            self.assertEqual(sync_mod.reconcile(memories, state, str(remote)), "noop")
            self.assertTrue(memories.is_dir())
            self.assertEqual(list(memories.iterdir()), [])
            self.assertFalse((state / "pending.json").exists())

    @unittest.skipUnless(shutil.which("git"), "git required")
    def test_local_content_pushes_over_an_empty_remote(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            base = Path(raw)
            remote, _manifest, _commit = self._create_empty_remote(base)
            memories = base / "memories"
            memories.mkdir()
            payloads = {
                "crlf.md": b"local\r\nopaque\r\n",
                "lf.md": b"local\nopaque\n",
            }
            for name, payload in payloads.items():
                (memories / name).write_bytes(payload)
            self.assertEqual(sync_mod.reconcile(memories, base / "state", str(remote)), "push")
            verify_state = base / "verify-state"
            verify_state.mkdir()
            remote_manifest, extracted, _commit = sync_mod._read_remote_snapshot(verify_state, str(remote))
            self.assertIsNotNone(remote_manifest)
            self.assertEqual([item["path"] for item in remote_manifest["files"]], sorted(payloads))
            for name, payload in payloads.items():
                self.assertEqual((extracted / "memories" / name).read_bytes(), payload)

    @unittest.skipUnless(shutil.which("git"), "git required")
    def test_invalid_empty_remote_remains_corrupt(self) -> None:
        for changes in ({"content_sha256": "0" * 64}, {"schema_version": 99}):
            with self.subTest(changes=changes), tempfile.TemporaryDirectory() as raw:
                base = Path(raw)
                remote, _manifest, _commit = self._create_empty_remote(base, changes)
                state = base / "state"
                sync_mod.mark_pending(state, "bridgeforge")
                memories = base / "memories"
                with self.assertRaisesRegex(sync_mod.SyncError, "remote snapshot is corrupt"):
                    sync_mod.reconcile(memories, state, str(remote))
                self.assertFalse(memories.exists())
                self.assertTrue((state / "pending.json").exists())

    def test_snapshot_retries_and_rejects_a_tree_that_changes_during_copy(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            base = Path(raw)
            source = base / "memories"
            source.mkdir()
            (source / "note.md").write_text("stable source", encoding="utf-8")
            real_copy = sync_mod.shutil.copy2

            def tampering_copy(source_path: Path, target_path: Path) -> None:
                real_copy(source_path, target_path)
                Path(target_path).write_text("changed after copy", encoding="utf-8")

            with mock.patch.object(sync_mod.shutil, "copy2", side_effect=tampering_copy):
                with self.assertRaises(sync_mod.SyncError):
                    sync_mod.build_snapshot(source, base / "snapshot", 1)

    def test_concurrent_reconcile_is_deduplicated_and_keeps_pending(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            base = Path(raw)
            state = base / "state"
            state.mkdir()
            descriptor = sync_mod._acquire_reconcile_lock(state)
            self.assertIsNotNone(descriptor)
            try:
                with mock.patch.object(sync_mod, "_read_remote_snapshot") as remote_read:
                    self.assertEqual(sync_mod.reconcile(base / "memories", state, "unused"), "busy")
                remote_read.assert_not_called()
                self.assertTrue((state / "pending.json").is_file())
            finally:
                sync_mod._release_reconcile_lock(state, descriptor)

    def test_stale_incomplete_reconcile_lock_is_recoverable(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            state = Path(raw)
            lock = state / "reconcile.lock"
            lock.write_text("incomplete", encoding="utf-8")
            old = time.time() - 120
            sync_mod.os.utime(lock, (old, old))
            descriptor = sync_mod._acquire_reconcile_lock(state)
            self.assertIsNotNone(descriptor)
            sync_mod._release_reconcile_lock(state, descriptor)
            self.assertFalse(lock.exists())

    def test_dead_reconcile_owner_cleans_only_recorded_managed_workdir(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            state = Path(raw)
            lock = state / "reconcile.lock"
            lock.write_text(json.dumps({"pid": 424242, "utc": sync_mod.utc_now()}), encoding="utf-8")
            stranded = Path(tempfile.mkdtemp(prefix=sync_mod.WORKDIR_PREFIX))
            (stranded / "opaque.bin").write_bytes(b"native-memory")
            sync_mod._record_workdir(state, stranded)
            with mock.patch.object(sync_mod, "_process_alive", return_value=False):
                descriptor = sync_mod._acquire_reconcile_lock(state)
            self.assertIsNotNone(descriptor)
            self.assertFalse(stranded.exists())
            self.assertFalse((state / "transient-workdir.json").exists())
            sync_mod._release_reconcile_lock(state, descriptor)

    def test_background_launch_reuses_the_single_live_worker(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            codex = Path(raw) / ".codex"
            state = codex / ".bridgeforge-codex/memory-sync"
            state.mkdir(parents=True)
            process = mock.Mock(pid=24680)
            with mock.patch.dict(sync_mod.os.environ, {"CODEX_HOME": str(codex)}), mock.patch.object(
                sync_mod.subprocess,
                "Popen",
                return_value=process,
            ) as popen, mock.patch.object(
                sync_mod,
                "_process_alive",
                side_effect=lambda pid: pid in {os.getpid(), 24680},
            ):
                first = sync_mod.launch_background_reconcile("stop", ROOT)
                second = sync_mod.launch_background_reconcile("session-end", ROOT)
            self.assertEqual(first, "worker-started")
            self.assertEqual(second, "worker-reused")
            self.assertEqual(popen.call_count, 1)

    def test_worker_marks_five_minute_busy_as_degraded_not_succeeded(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            base = Path(raw)
            codex = base / ".codex"
            state = codex / ".bridgeforge-codex/memory-sync"
            state.mkdir(parents=True)
            token = "worker-token"
            sync_mod._atomic_json(
                state / "worker.json",
                {"token": token, "pid": 0, "launcherPid": os.getpid(), "startedUtc": sync_mod.utc_now()},
            )
            sync_mod.mark_pending(state, "stop")
            with mock.patch.object(
                sync_mod,
                "validated_runtime_state",
                return_value=(state, {"remote": "unused"}),
            ), mock.patch.object(
                sync_mod,
                "verify_private_github_repository",
            ), mock.patch.object(
                sync_mod,
                "reconcile",
                return_value="busy",
            ), mock.patch.object(
                sync_mod,
                "_pending_age_seconds",
                return_value=301.0,
            ):
                self.assertEqual(
                    sync_mod.run_sync_worker(codex, codex / "memories", state, codex / "ledger.json", token),
                    0,
                )
            receipt = json.loads((state / "last-worker.json").read_text(encoding="utf-8"))
            health = json.loads((state / "health.json").read_text(encoding="utf-8"))
            self.assertEqual(receipt["status"], "degraded")
            self.assertEqual(health["status"], "degraded")
            self.assertTrue((state / "pending.json").is_file())

    def test_failure_alert_is_emitted_once_per_unchanged_state(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            state = Path(raw)
            sync_mod._record_health(state, "failed", error="network unavailable")
            errors = io.StringIO()
            with contextlib.redirect_stderr(errors):
                sync_mod._emit_alert_once(state)
                sync_mod._emit_alert_once(state)
            self.assertEqual(errors.getvalue().count("network unavailable"), 1)

    def test_overdue_pending_is_persisted_and_alerted_exactly_once(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            state = Path(raw)
            old = (
                datetime.now(timezone.utc)
                - timedelta(seconds=sync_mod.SYNC_DEADLINE_SECONDS + 1)
            ).isoformat().replace("+00:00", "Z")
            (state / "pending.json").write_text(
                json.dumps({
                    "schemaVersion": 2,
                    "firstPendingUtc": old,
                    "updatedUtc": old,
                    "trigger": "sessionstart",
                    "triggers": ["sessionstart"],
                }),
                encoding="utf-8",
            )
            errors = io.StringIO()
            with contextlib.redirect_stderr(errors):
                sync_mod._persist_overdue_pending_health(state)
                sync_mod._emit_alert_once(state)
                sync_mod._persist_overdue_pending_health(state)
                sync_mod._emit_alert_once(state)
            health = json.loads((state / "health.json").read_text(encoding="utf-8"))
            self.assertEqual(health["status"], "degraded")
            self.assertEqual(health["alertedId"], health["alertId"])
            self.assertEqual(errors.getvalue().count("more than five minutes"), 1)

    def test_conflict_resolution_fails_closed_while_worker_holds_lock(self) -> None:
        with tempfile.TemporaryDirectory() as raw, mock.patch.object(
            sync_mod,
            "_acquire_reconcile_lock",
            return_value=None,
        ):
            with self.assertRaisesRegex(sync_mod.SyncError, "busy"):
                sync_mod.resolve_conflict(
                    Path(raw) / "memories",
                    Path(raw) / "state",
                    "https://github.com/example/bridgeforge-codex-memories.git",
                    "a" * 64,
                    [],
                )

    def test_overdue_health_does_not_race_a_live_worker_lock(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            state = Path(raw)
            old = (
                datetime.now(timezone.utc)
                - timedelta(seconds=sync_mod.SYNC_DEADLINE_SECONDS + 1)
            ).isoformat().replace("+00:00", "Z")
            (state / "pending.json").write_text(
                json.dumps({"schemaVersion": 2, "firstPendingUtc": old}),
                encoding="utf-8",
            )
            with mock.patch.object(
                sync_mod,
                "_acquire_reconcile_lock",
                return_value=None,
            ):
                self.assertIsNone(sync_mod._persist_overdue_pending_health(state))
            self.assertFalse((state / "health.json").exists())

    def test_recorded_transient_snapshot_is_cleaned_on_the_next_run(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            state = Path(raw)
            stranded = Path(tempfile.mkdtemp(prefix=sync_mod.WORKDIR_PREFIX))
            (stranded / "memory.md").write_text("plaintext", encoding="utf-8")
            try:
                sync_mod._record_workdir(state, stranded)
                sync_mod._cleanup_recorded_workdir(state)
                self.assertFalse(stranded.exists())
                self.assertFalse((state / "transient-workdir.json").exists())
            finally:
                if stranded.exists():
                    shutil.rmtree(stranded)

    def test_hook_reconcile_trigger_has_no_invalid_plaintext_stdout(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            codex = Path(raw) / ".codex"
            codex.mkdir()
            (codex / "config.toml").write_text(
                "[features]\nmemories = true\n[memories]\ngenerate_memories = true\nuse_memories = true\n",
                encoding="utf-8",
            )
            state = codex / ".bridgeforge-codex/memory-sync"
            state.mkdir(parents=True)
            remote = "https://github.com/example/bridgeforge-codex-memories"
            (state / "remote.txt").write_text(remote + "\n", encoding="utf-8")
            output = io.StringIO()
            with mock.patch.dict(sync_mod.os.environ, {"CODEX_HOME": str(codex)}), mock.patch.object(
                sync_mod, "reconcile", return_value="noop"
            ), mock.patch.object(
                sync_mod,
                "require_runtime_authorization",
                return_value={"remote": remote},
            ), mock.patch.object(
                sync_mod,
                "verify_private_github_repository",
            ), contextlib.redirect_stdout(output):
                self.assertEqual(sync_mod.main(self._args("reconcile", "--trigger", "stop")), 0)
            self.assertEqual(output.getvalue(), "{}\n")

    @unittest.skipUnless(shutil.which("git"), "git required")
    def test_existing_ordinary_repository_is_reused_and_initialized(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            base = Path(raw)
            remote = base / "remote.git"
            subprocess.run(["git", "init", "--bare", str(remote)], check=True, capture_output=True)
            ordinary = base / "ordinary"
            ordinary.mkdir()
            for command in (
                ["git", "init", "-b", "main"],
                ["git", "config", "user.name", "Test"],
                ["git", "config", "user.email", "test@example.invalid"],
            ):
                subprocess.run(command, cwd=ordinary, check=True, capture_output=True)
            (ordinary / "README.md").write_text("ordinary repo\n", encoding="utf-8")
            for command in (["git", "add", "README.md"], ["git", "commit", "-m", "readme"], ["git", "remote", "add", "origin", str(remote)], ["git", "push", "origin", "main"]):
                subprocess.run(command, cwd=ordinary, check=True, capture_output=True)

            local = base / "memories"
            local.mkdir()
            (local / "note.md").write_text("native memory", encoding="utf-8")
            self.assertEqual(sync_mod.reconcile(local, base / "state", str(remote)), "push")
            count = subprocess.run(["git", f"--git-dir={remote}", "rev-list", "--count", "main"], check=True, text=True, capture_output=True).stdout.strip()
            manifest = subprocess.run(["git", f"--git-dir={remote}", "show", "main:snapshot-manifest.json"], check=True, text=True, capture_output=True).stdout
            self.assertEqual(count, "2")
            self.assertEqual(json.loads(manifest)["schema_version"], 1)

    @unittest.skipUnless(shutil.which("git"), "git required")
    def test_valid_local_snapshot_repairs_a_corrupt_remote_commit(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            base = Path(raw)
            remote = base / "remote.git"
            subprocess.run(["git", "init", "--bare", str(remote)], check=True, capture_output=True)
            broken = base / "broken"
            (broken / "memories").mkdir(parents=True)
            (broken / "memories/bad.md").write_text("bad", encoding="utf-8")
            (broken / "snapshot-manifest.json").write_text('{}\n', encoding="utf-8")
            bad_commit = sync_mod._push_snapshot(broken, base / "bad-state", str(remote), None)

            local = base / "local"
            local.mkdir()
            (local / "good.md").write_text("good", encoding="utf-8")
            action = sync_mod.reconcile(local, base / "repair-state", str(remote))
            verify_state = base / "verify-state"
            verify_state.mkdir()
            manifest, extracted, repaired_commit = sync_mod._read_remote_snapshot(verify_state, str(remote))

            self.assertEqual(action, "push")
            self.assertNotEqual(repaired_commit, bad_commit)
            self.assertIsNotNone(manifest)
            self.assertEqual((extracted / "memories/good.md").read_text(encoding="utf-8"), "good")

    @unittest.skipUnless(shutil.which("git"), "git required")
    def test_local_state_io_failure_never_overwrites_a_valid_remote(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            base = Path(raw)
            remote = base / "remote.git"
            subprocess.run(["git", "init", "--bare", str(remote)], check=True, capture_output=True)
            cloud = base / "cloud"
            cloud.mkdir()
            (cloud / "cloud.md").write_text("cloud", encoding="utf-8")
            sync_mod.reconcile(cloud, base / "cloud-state", str(remote))

            local = base / "local"
            local.mkdir()
            (local / "local.md").write_text("local", encoding="utf-8")
            state = base / "local-state"
            with mock.patch.object(sync_mod, "_read_remote_snapshot", side_effect=OSError("disk failure")), mock.patch.object(
                sync_mod, "_push_snapshot"
            ) as push:
                with self.assertRaises(OSError):
                    sync_mod.reconcile(local, state, str(remote))
            push.assert_not_called()

    @unittest.skipUnless(shutil.which("git"), "git required")
    def test_two_computers_merge_different_opaque_files_with_shared_baseline(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            base = Path(raw)
            remote = base / "remote.git"
            subprocess.run(["git", "init", "--bare", str(remote)], check=True, capture_output=True)
            machine_a = base / "machine-a"
            machine_a.mkdir()
            (machine_a / "common.bin").write_bytes(b"base\x00")
            state_a = base / "state-a"
            self.assertEqual(sync_mod.reconcile(machine_a, state_a, str(remote)), "push")

            machine_b = base / "machine-b"
            machine_b.mkdir()
            state_b = base / "state-b"
            self.assertEqual(sync_mod.reconcile(machine_b, state_b, str(remote)), "restore")

            (machine_a / "from-a.bin").write_bytes(b"A\r\n")
            self.assertEqual(sync_mod.reconcile(machine_a, state_a, str(remote)), "push")
            (machine_b / "from-b.bin").write_bytes(b"B\n")
            self.assertEqual(sync_mod.reconcile(machine_b, state_b, str(remote)), "merge")

            self.assertEqual((machine_b / "from-a.bin").read_bytes(), b"A\r\n")
            self.assertEqual((machine_b / "from-b.bin").read_bytes(), b"B\n")
            verify = base / "verify"
            verify.mkdir()
            manifest, extracted, _commit = sync_mod._read_remote_snapshot(verify, str(remote))
            self.assertIsNotNone(manifest)
            self.assertEqual((extracted / "memories/from-a.bin").read_bytes(), b"A\r\n")
            self.assertEqual((extracted / "memories/from-b.bin").read_bytes(), b"B\n")
            count = subprocess.run(
                ["git", f"--git-dir={remote}", "rev-list", "--count", "main"],
                check=True,
                text=True,
                capture_output=True,
            ).stdout.strip()
            self.assertEqual(count, "3")

    @unittest.skipUnless(shutil.which("git"), "git required")
    def test_same_path_dual_change_preserves_both_and_controlled_resolve(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            base = Path(raw)
            remote = base / "remote.git"
            subprocess.run(["git", "init", "--bare", str(remote)], check=True, capture_output=True)
            machine_a = base / "machine-a"
            machine_a.mkdir()
            (machine_a / "same.bin").write_bytes(b"base")
            state_a = base / "state-a"
            sync_mod.reconcile(machine_a, state_a, str(remote))

            machine_b = base / "machine-b"
            machine_b.mkdir()
            state_b = base / "state-b"
            sync_mod.reconcile(machine_b, state_b, str(remote))

            (machine_a / "same.bin").write_bytes(b"from-a")
            sync_mod.reconcile(machine_a, state_a, str(remote))
            (machine_b / "same.bin").write_bytes(b"from-b")
            self.assertEqual(sync_mod.reconcile(machine_b, state_b, str(remote)), "conflicted")
            active = json.loads((state_b / "active-conflict.json").read_text(encoding="utf-8"))
            evidence = Path(active["path"])
            self.assertEqual((evidence / "local/memories/same.bin").read_bytes(), b"from-b")
            self.assertEqual((evidence / "remote/memories/same.bin").read_bytes(), b"from-a")
            self.assertEqual((machine_b / "same.bin").read_bytes(), b"from-b")

            commit = sync_mod.resolve_conflict(
                machine_b,
                state_b,
                str(remote),
                active["conflictId"],
                ["same.bin=local"],
            )
            self.assertEqual((machine_b / "same.bin").read_bytes(), b"from-b")
            self.assertFalse((state_b / "active-conflict.json").exists())
            metadata = json.loads((evidence / "conflict.json").read_text(encoding="utf-8"))
            self.assertEqual(metadata["resolvedCommit"], commit)
            verify = base / "verify"
            verify.mkdir()
            _manifest, extracted, remote_commit = sync_mod._read_remote_snapshot(verify, str(remote))
            self.assertEqual(remote_commit, commit)
            self.assertEqual((extracted / "memories/same.bin").read_bytes(), b"from-b")

    @unittest.skipUnless(shutil.which("git"), "git required")
    def test_conflict_resolution_rebases_when_remote_became_captured_local(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            base = Path(raw)
            remote = base / "remote.git"
            subprocess.run(["git", "init", "--bare", str(remote)], check=True, capture_output=True)
            machine_a = base / "machine-a"
            machine_a.mkdir()
            (machine_a / "same.bin").write_bytes(b"base")
            state_a = base / "state-a"
            sync_mod.reconcile(machine_a, state_a, str(remote))

            machine_b = base / "machine-b"
            machine_b.mkdir()
            state_b = base / "state-b"
            sync_mod.reconcile(machine_b, state_b, str(remote))
            (machine_a / "same.bin").write_bytes(b"remote-version")
            sync_mod.reconcile(machine_a, state_a, str(remote))
            (machine_b / "same.bin").write_bytes(b"local-version")
            self.assertEqual(sync_mod.reconcile(machine_b, state_b, str(remote)), "conflicted")
            active = json.loads((state_b / "active-conflict.json").read_text(encoding="utf-8"))

            captured_local = Path(active["path"]) / "local"
            raced_commit = sync_mod._push_snapshot(
                captured_local,
                base / "race-state",
                str(remote),
                active["remoteCommit"],
            )
            resolved_commit = sync_mod.resolve_conflict(
                machine_b,
                state_b,
                str(remote),
                active["conflictId"],
                ["same.bin=remote"],
            )
            self.assertNotEqual(resolved_commit, raced_commit)
            self.assertEqual((machine_b / "same.bin").read_bytes(), b"remote-version")
            parents = subprocess.run(
                ["git", f"--git-dir={remote}", "rev-list", "--parents", "-n", "1", resolved_commit],
                check=True,
                text=True,
                capture_output=True,
            ).stdout.split()
            self.assertEqual(parents, [resolved_commit, raced_commit])

    @unittest.skipUnless(shutil.which("git"), "git required")
    def test_successful_noop_clears_stale_active_conflict(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            base = Path(raw)
            remote = base / "remote.git"
            subprocess.run(["git", "init", "--bare", str(remote)], check=True, capture_output=True)
            memories = base / "memories"
            memories.mkdir()
            (memories / "stable.bin").write_bytes(b"stable")
            state = base / "state"
            self.assertEqual(sync_mod.reconcile(memories, state, str(remote)), "push")
            (state / "active-conflict.json").write_text("{}\n", encoding="utf-8")
            self.assertEqual(sync_mod.reconcile(memories, state, str(remote)), "noop")
            self.assertFalse((state / "active-conflict.json").exists())

    @unittest.skipUnless(shutil.which("git"), "git required")
    def test_no_content_change_does_not_create_an_empty_commit(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            base = Path(raw)
            remote = base / "remote.git"
            subprocess.run(["git", "init", "--bare", str(remote)], check=True, capture_output=True)
            memories = base / "memories"
            memories.mkdir()
            (memories / "stable.bin").write_bytes(b"stable")
            state = base / "state"
            self.assertEqual(sync_mod.reconcile(memories, state, str(remote)), "push")
            first = subprocess.run(
                ["git", f"--git-dir={remote}", "rev-parse", "main"],
                check=True,
                text=True,
                capture_output=True,
            ).stdout.strip()
            self.assertEqual(sync_mod.reconcile(memories, state, str(remote)), "noop")
            second = subprocess.run(
                ["git", f"--git-dir={remote}", "rev-parse", "main"],
                check=True,
                text=True,
                capture_output=True,
            ).stdout.strip()
            self.assertEqual(second, first)

    @unittest.skipUnless(shutil.which("git"), "git required")
    def test_new_machine_with_nonempty_local_and_remote_requires_bootstrap_resolution(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            base = Path(raw)
            remote = base / "remote.git"
            subprocess.run(["git", "init", "--bare", str(remote)], check=True, capture_output=True)
            cloud = base / "cloud-memories"
            cloud.mkdir()
            cloud_file = cloud / "cloud.md"
            cloud_file.write_text("cloud", encoding="utf-8")
            sync_mod.reconcile(cloud, base / "cloud-state", str(remote))

            local = base / "local-memories"
            local.mkdir()
            stale = local / "stale.md"
            stale.write_text("stale", encoding="utf-8")
            action = sync_mod.reconcile(local, base / "local-state", str(remote))

            self.assertEqual(action, "conflicted")
            self.assertEqual(stale.read_text(encoding="utf-8"), "stale")
            self.assertFalse((local / "cloud.md").exists())
            active = json.loads(
                (base / "local-state/active-conflict.json").read_text(encoding="utf-8")
            )
            self.assertIn("bootstrap conflict", active["reason"])
            self.assertFalse((base / ".local-memories.bridgeforge-codex-replaced").exists())
            self.assertTrue((Path(active["path"]) / "local/memories/stale.md").is_file())
            self.assertTrue((Path(active["path"]) / "remote/memories/cloud.md").is_file())

    @unittest.skipUnless(shutil.which("git"), "git required")
    def test_push_preserves_parent_history_and_never_uses_force(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            base = Path(raw)
            remote = base / "remote.git"
            subprocess.run(["git", "init", "--bare", str(remote)], check=True, capture_output=True)
            state = base / "state"
            state.mkdir()
            snapshot = base / "snapshot"
            (snapshot / "memories").mkdir(parents=True)
            (snapshot / "memories/a.md").write_text("one", encoding="utf-8")
            (snapshot / "snapshot-manifest.json").write_text('{}\n', encoding="utf-8")
            calls: list[list[str]] = []
            actual_git = sync_mod._git

            def recording_git(
                command: list[str],
                cwd: Path,
                *,
                env: dict[str, str] | None = None,
            ) -> str:
                calls.append(command)
                return actual_git(command, cwd, env=env)

            with mock.patch.object(sync_mod, "_git", side_effect=recording_git):
                first = sync_mod._push_snapshot(snapshot, state, str(remote), None)
                (snapshot / "memories/a.md").write_text("two", encoding="utf-8")
                second = sync_mod._push_snapshot(snapshot, state, str(remote), first)
            self.assertNotEqual(first, second)
            self.assertFalse(
                any("--force" in argument for command in calls for argument in command)
            )
            pushes = [command for command in calls if command[:1] == ["push"]]
            self.assertEqual(
                pushes,
                [
                    ["push", "origin", "refs/heads/main:refs/heads/main"],
                    ["push", "origin", "refs/heads/main:refs/heads/main"],
                ],
            )
            count = subprocess.run(["git", f"--git-dir={remote}", "rev-list", "--count", "main"], check=True, text=True, capture_output=True).stdout.strip()
            parents = subprocess.run(["git", f"--git-dir={remote}", "rev-list", "--parents", "-1", "main"], check=True, text=True, capture_output=True).stdout.split()
            self.assertEqual(count, "2")
            self.assertEqual(len(parents), 2)
            self.assertEqual(parents[1], first)

    def test_public_repository_needs_explicit_confirmation(self) -> None:
        calls: list[list[str]] = []
        def runner(command: list[str], **_kwargs: object) -> subprocess.CompletedProcess[str]:
            calls.append(command)
            if command[:3] == ["gh", "auth", "status"]:
                return subprocess.CompletedProcess(command, 0, "", "")
            return subprocess.CompletedProcess(command, 0, json.dumps({"visibility": "PUBLIC", "url": "https://example/repo.git", "nameWithOwner": "me/bridgeforge-codex-memories"}), "")
        with mock.patch.object(sync_mod.shutil, "which", return_value="gh"):
            with self.assertRaises(sync_mod.SyncError):
                sync_mod.ensure_github_repository(confirmed_public_to_private=False, run=runner)
            remote, action = sync_mod.ensure_github_repository(confirmed_public_to_private=True, run=runner)
        self.assertEqual(remote, "https://example/repo.git")
        self.assertEqual(action, "made-private")
        edit = next(command for command in calls if command[:3] == ["gh", "repo", "edit"])
        self.assertIn("--accept-visibility-change-consequences", edit)
        auth = next(command for command in calls if command[:3] == ["gh", "auth", "status"])
        self.assertEqual(auth, ["gh", "auth", "status", "--active", "--hostname", "github.com"])


if __name__ == "__main__":
    unittest.main()
