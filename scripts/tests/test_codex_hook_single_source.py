from __future__ import annotations

import json
import importlib.util
import io
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path
from unittest import mock


ROOT = Path(__file__).resolve().parents[2]
TEMPLATE = ROOT / "templates"


def run(command: list[str], cwd: Path, payload: dict | None = None) -> subprocess.CompletedProcess[str]:
    env = dict(os.environ)
    env["PYTHONUTF8"] = "1"
    return subprocess.run(
        command,
        cwd=cwd,
        input=json.dumps(payload) if payload is not None else None,
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        env=env,
        check=False,
    )


def load_module(path: Path, name: str):
    spec = importlib.util.spec_from_file_location(name, path)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def prepare_project_runtime(project_root: Path) -> Path:
    created = run(
        [
            sys.executable,
            "-m",
            "venv",
            "--without-pip",
            str(project_root / ".venv"),
        ],
        project_root,
    )
    if created.returncode != 0:
        raise AssertionError(created.stdout + created.stderr)
    return (
        project_root / ".venv" / "Scripts" / "python.exe"
        if os.name == "nt"
        else project_root / ".venv" / "bin" / "python"
    )


def prepare_dispatcher_runtime(project_root: Path) -> Path:
    project_python = prepare_project_runtime(project_root)
    host = project_root / ".codex"
    scripts = host / "scripts"
    scripts.mkdir(parents=True, exist_ok=True)
    shutil.copy2(
        TEMPLATE / "scripts" / "project_runtime.py",
        scripts / "project_runtime.py",
    )
    dispatcher = load_module(
        TEMPLATE / "hooks" / "hook_dispatcher.py",
        f"dispatcher_fixture_routes_{id(project_root)}",
    )
    for relative in {
        item
        for targets in dispatcher.RUNTIME_ROUTES.values()
        for item in targets
    }:
        target = host / Path(relative)
        target.parent.mkdir(parents=True, exist_ok=True)
        if not target.exists():
            target.write_text("raise SystemExit(0)\n", encoding="utf-8")
    return project_python


class HookSingleSourceTest(unittest.TestCase):
    def test_positive_suite_uses_project_python_311_or_newer(self) -> None:
        self.assertGreaterEqual(sys.version_info, (3, 11))
        self.assertEqual(
            Path(sys.executable).resolve(),
            (ROOT / ".venv" / "Scripts" / "python.exe").resolve(),
        )

    def test_precommit_rejects_low_project_venv_without_path_fallback(self) -> None:
        shell = shutil.which("sh")
        if shell is None:
            git = shutil.which("git")
            if git is not None:
                git_root = Path(git).resolve().parent.parent
                for candidate in (git_root / "bin" / "sh.exe", git_root / "usr" / "bin" / "sh.exe"):
                    if candidate.is_file():
                        shell = str(candidate)
                        break
        if shell is None:
            self.skipTest("POSIX shell is required to exercise pre-commit hooks")
        precommits = (
            ROOT / ".githooks" / "pre-commit",
            ROOT / "templates" / ".githooks" / "pre-commit",
        )
        for precommit in precommits:
            with self.subTest(precommit=precommit), tempfile.TemporaryDirectory() as raw:
                project = Path(raw)
                low_python = project / ".venv" / "bin" / "python"
                low_python.parent.mkdir(parents=True)
                low_python.write_text("#!/bin/sh\nexit 2\n", encoding="utf-8")
                low_python.chmod(0o755)
                stamp = project / ".bridgeforge_codex_version"
                stamp.write_text("old\n", encoding="utf-8")
                sentinel = project / "sentinel.txt"
                sentinel.write_text("unchanged\n", encoding="utf-8")
                before = (stamp.read_bytes(), sentinel.read_bytes())
                result = subprocess.run(
                    [shell, str(precommit)],
                    cwd=project,
                    capture_output=True,
                    text=True,
                    encoding="utf-8",
                    errors="replace",
                    check=False,
                )
                self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
                self.assertIn("PATH fallback is forbidden", result.stderr)
                self.assertEqual(before, (stamp.read_bytes(), sentinel.read_bytes()))

            with self.subTest(precommit=precommit, runtime="missing"), \
                    tempfile.TemporaryDirectory() as raw:
                project = Path(raw)
                sentinel = project / "sentinel.txt"
                sentinel.write_text("unchanged\n", encoding="utf-8")
                result = subprocess.run(
                    [shell, str(precommit)],
                    cwd=project,
                    capture_output=True,
                    text=True,
                    encoding="utf-8",
                    errors="replace",
                    check=False,
                )
                self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
                self.assertIn("project .venv is missing", result.stderr)
                self.assertEqual(sentinel.read_text(encoding="utf-8"), "unchanged\n")

    def test_dispatcher_routes_are_current_safe_runtime_files(self) -> None:
        dispatcher_path = TEMPLATE / "hooks" / "hook_dispatcher.py"
        dispatcher = load_module(dispatcher_path, "hook_dispatcher_audit")
        self.assertEqual(dispatcher.runtime_route_errors(), [])
        self.assertNotIn(
            "hooks/target_cleanup.py",
            {
                target
                for targets in dispatcher.RUNTIME_ROUTES.values()
                for target in targets
            },
        )
        broken_routes = {
            route: tuple(targets)
            for route, targets in dispatcher.RUNTIME_ROUTES.items()
        }
        broken_routes["pre-shell"] += ("hooks/not-present.py",)
        errors = dispatcher.runtime_route_errors(broken_routes)
        self.assertIn("pre-shell runtime target is missing: hooks/not-present.py", errors)

    def test_dispatcher_forces_utf8_for_child_hook_when_parent_utf8_is_disabled(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            hooks = root / ".codex" / "hooks"
            hooks.mkdir(parents=True)
            shutil.copy2(
                TEMPLATE / "hooks" / "hook_dispatcher.py",
                hooks / "hook_dispatcher.py",
            )
            project_python = prepare_dispatcher_runtime(root)
            (hooks / "test_receipt.py").write_text(
                "import json,os\n"
                "context = 'utf8=' + os.environ.get('PYTHONUTF8', '') + "
                "';io=' + os.environ.get('PYTHONIOENCODING', '') + ';chars=🔍²'\n"
                "print(json.dumps({'hookSpecificOutput': {"
                "'hookEventName': 'PostToolUse', 'additionalContext': context}}, "
                "ensure_ascii=False))\n",
                encoding="utf-8",
            )
            env = dict(os.environ)
            env["PYTHONUTF8"] = "0"
            env.pop("PYTHONIOENCODING", None)
            result = subprocess.run(
                [str(project_python), str(hooks / "hook_dispatcher.py"), "post-shell"],
                cwd=root,
                input=json.dumps({"tool_name": "Bash", "tool_input": {"command": "test"}}),
                capture_output=True,
                text=True,
                encoding="utf-8",
                errors="replace",
                env=env,
                check=False,
            )
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            output = json.loads(result.stdout)
            context = output["hookSpecificOutput"]["additionalContext"]
            self.assertIn("utf8=1", context)
            self.assertIn("io=utf-8", context)
            self.assertIn("chars=🔍²", context)
            self.assertNotIn("UnicodeEncodeError", result.stderr)

    def test_dispatcher_blocks_invalid_project_runtime_before_any_route(self) -> None:
        dispatcher = load_module(
            TEMPLATE / "hooks" / "hook_dispatcher.py",
            "hook_dispatcher_runtime_contract",
        )
        for reason in (
            "project runtime must be CPython",
            "project runtime prefix is not the target project .venv",
            "current Python is not the target project .venv interpreter",
        ):
            with self.subTest(reason=reason), mock.patch.object(
                dispatcher,
                "_project_runtime_error",
                return_value=reason,
            ), mock.patch.object(dispatcher, "_read_payload") as read_payload, \
                    mock.patch.object(sys, "argv", ["hook_dispatcher.py", "pre-tool"]):
                stderr = io.StringIO()
                with redirect_stderr(stderr):
                    result = dispatcher.main()
                self.assertEqual(result, 2)
                self.assertIn(reason, stderr.getvalue())
                read_payload.assert_not_called()

    def test_context_budget_is_absent_from_active_contract(self) -> None:
        template_hooks = json.loads(
            (TEMPLATE / "hooks.json").read_text(encoding="utf-8")
        )
        dogfood_hooks = json.loads(
            (ROOT / ".codex/hooks.json").read_text(encoding="utf-8")
        )
        self.assertEqual(template_hooks, dogfood_hooks)
        self.assertNotIn("UserPromptSubmit", template_hooks["hooks"])

        dispatcher = load_module(
            TEMPLATE / "hooks" / "hook_dispatcher.py",
            "hook_dispatcher_without_context_budget",
        )
        self.assertNotIn("user-prompt", dispatcher.RUNTIME_ROUTES)
        self.assertIn(
            "hooks/show_state.py",
            dispatcher.RUNTIME_ROUTES["session-after"],
        )
        contract = json.loads(
            (TEMPLATE / "managed-skeleton.json").read_text(encoding="utf-8")
        )
        self.assertFalse(any(
            asset["id"] == "codex.hook.context-warning"
            for asset in contract["assets"]
        ))
        self.assertFalse(any(
            asset["id"] == "codex.hook.focus-reminder"
            for asset in contract["assets"]
        ))
        self.assertFalse((TEMPLATE / "hooks" / "focus_reminder.py").exists())
        self.assertFalse((ROOT / ".codex" / "hooks" / "focus_reminder.py").exists())
        focus_skill = (ROOT / "skills" / "focus" / "SKILL.md").read_text(
            encoding="utf-8"
        )
        self.assertIn("没有自动 Hook 捕获、更新或提醒", focus_skill)
        self.assertIn("只在用户显式调用时", focus_skill)

    def test_project_memory_runtime_is_absent_from_all_active_owners(self) -> None:
        retired_hooks = (
            "allow_memory_write.py", "memory_dup_check.py", "memory_lint.py",
        )
        retired_scripts = (
            "memory_context.py", "memory_rebuild_index.py", "memory_router.py",
            "memory_search.py", "memory_usage.py",
            "project_memory_recovery.py", "project_memory_writer.py",
        )
        for base in (TEMPLATE, ROOT / ".codex"):
            self.assertTrue(all(
                not (base / "hooks" / name).exists()
                for name in retired_hooks
            ))
            self.assertTrue(all(
                not (base / "scripts" / name).exists()
                for name in retired_scripts
            ))
        self.assertEqual(
            list((ROOT / "templates" / "memory").glob("*")),
            [],
        )
        self.assertEqual(list((ROOT / "skills" / "find-memory").glob("*")), [])
        contract_text = (
            ROOT / "templates" / "managed-skeleton.json"
        ).read_text(encoding="utf-8")
        manifest_text = (
            ROOT / "bridgeforge-codex-manifest.json"
        ).read_text(encoding="utf-8")
        for name in (*retired_hooks, *retired_scripts):
            self.assertNotIn(name, contract_text)
        self.assertNotIn('"name": "find-memory"', manifest_text)

    def test_template_registration_is_single_source_and_git_rooted(self) -> None:
        settings = json.loads((TEMPLATE / "settings.json").read_text(encoding="utf-8"))
        dogfood_settings = json.loads((ROOT / ".codex" / "settings.json").read_text(encoding="utf-8"))
        hooks = json.loads((TEMPLATE / "hooks.json").read_text(encoding="utf-8"))
        self.assertNotIn("hooks", settings)
        self.assertNotIn("hooks", dogfood_settings)
        self.assertIsInstance(hooks.get("hooks"), dict)
        commands = []
        for blocks in hooks["hooks"].values():
            for block in blocks:
                for hook in block.get("hooks", []):
                    commands.append(hook)
        self.assertEqual(len(commands), 6)
        for hook in commands:
            self.assertIn("git rev-parse --show-toplevel", hook["command"])
            self.assertIn("git rev-parse --show-toplevel", hook["commandWindows"])
            self.assertIn("hook_dispatcher.py", hook["command"])
            self.assertTrue(hook["commandWindows"].startswith("powershell.exe "))
            self.assertIn("-NonInteractive", hook["commandWindows"])
            self.assertIn("-WindowStyle Hidden", hook["commandWindows"])
            self.assertTrue(
                hook["commandWindows"].endswith('; exit $LASTEXITCODE"')
            )
        session_start = hooks["hooks"]["SessionStart"][0]["hooks"][0]
        self.assertEqual(session_start["additionalContextLimit"], 0)

    def test_dogfood_registration_matches_template_and_uses_project_venv(self) -> None:
        template = json.loads((TEMPLATE / "hooks.json").read_text(encoding="utf-8"))
        dogfood = json.loads((ROOT / ".codex" / "hooks.json").read_text(encoding="utf-8"))
        self.assertEqual(template, dogfood)
        for blocks in template["hooks"].values():
            for block in blocks:
                for hook in block.get("hooks", []):
                    self.assertIn(
                        '$(git rev-parse --show-toplevel)/.venv/Scripts/python.exe',
                        hook["command"],
                    )
                    self.assertIn(
                        "(Join-Path (git rev-parse --show-toplevel) '.venv/Scripts/python.exe')",
                        hook["commandWindows"],
                    )
                    self.assertIn("-WindowStyle Hidden", hook["commandWindows"])

    def test_public_agents_requires_hidden_windows_hook_launches(self) -> None:
        template = (TEMPLATE / "AGENTS.md").read_text(encoding="utf-8")
        dogfood = (ROOT / "AGENTS.md").read_text(encoding="utf-8")
        requirement = "启动非交互、无人值守命令时，必须使用可验证的无可见控制台窗口入口"
        self.assertIn(requirement, template)
        self.assertIn(requirement, dogfood)
        self.assertIn("除非用户明确要求可见交互窗口", template)

    @unittest.skipUnless(os.name == "nt", "Windows hidden process semantics")
    def test_hidden_windows_launcher_preserves_native_exit_code(self) -> None:
        command = (
            f"& '{sys.executable}' -c 'import sys; sys.exit(2)'; "
            "exit $LASTEXITCODE"
        )
        completed = subprocess.run(
            [
                "powershell.exe",
                "-NoProfile",
                "-NonInteractive",
                "-WindowStyle",
                "Hidden",
                "-Command",
                command,
            ],
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            check=False,
        )
        self.assertEqual(completed.returncode, 2, completed.stderr)

    def test_dispatcher_never_runs_project_memory_runtime(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            host = root / ".codex"
            (host / "hooks").mkdir(parents=True)
            (host / "scripts").mkdir()
            shutil.copy2(TEMPLATE / "hooks" / "hook_dispatcher.py", host / "hooks" / "hook_dispatcher.py")
            project_python = prepare_dispatcher_runtime(root)
            log = root / "order.log"
            stub = (
                "import os\n"
                f"open({str(log)!r}, 'a', encoding='utf-8').write(os.path.basename(__file__)+'\\n')\n"
            )
            hook_names = (
                "encoding_check.py", "instruction_source_check.py",
                "requirements_check.py", "cargo_default_run_check.py",
                "fallback_smell_check.py", "memory_lint.py",
            )
            for name in hook_names:
                (host / "hooks" / name).write_text(stub, encoding="utf-8")
            (host / "scripts" / "memory_rebuild_index.py").write_text(stub, encoding="utf-8")
            payload = {
                "tool_name": "apply_patch",
                "tool_input": {"command": "*** Begin Patch\n*** Update File: .codex/memory/topic.md\n@@\n-old\n+new\n*** End Patch"},
            }
            result = run([str(project_python), str(host / "hooks" / "hook_dispatcher.py"), "post-edit"], root, payload)
            self.assertEqual(result.returncode, 0, result.stderr)
            order = log.read_text(encoding="utf-8").splitlines()
            self.assertEqual(order[0], "encoding_check.py")
            self.assertNotIn("memory_rebuild_index.py", order)
            self.assertNotIn("memory_lint.py", order)

    def test_pre_edit_decisions_are_serial_and_mixed_patch_is_not_auto_allowed(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            hooks = root / ".codex" / "hooks"
            hooks.mkdir(parents=True)
            shutil.copy2(TEMPLATE / "hooks" / "hook_dispatcher.py", hooks / "hook_dispatcher.py")
            project_python = prepare_dispatcher_runtime(root)
            log = root / "pre.log"
            ordinary = (
                "import os\n"
                f"open({str(log)!r}, 'a', encoding='utf-8').write(os.path.basename(__file__)+'\\n')\n"
            )
            for name in ("cross_project_write_guard.py", "user_config_write_guard.py"):
                (hooks / name).write_text(ordinary, encoding="utf-8")
            single = {"tool_name": "apply_patch", "tool_input": {"command": "*** Add File: .codex/memory/topic.md"}}
            allowed = run([str(project_python), str(hooks / "hook_dispatcher.py"), "pre-tool"], root, single)
            self.assertEqual(allowed.returncode, 0, allowed.stderr)
            self.assertEqual(
                log.read_text(encoding="utf-8").splitlines(),
                ["cross_project_write_guard.py", "user_config_write_guard.py"],
            )
            self.assertEqual(allowed.stdout, "")

            log.write_text("", encoding="utf-8")
            mixed = {"tool_name": "apply_patch", "tool_input": {"command": "*** Add File: .codex/memory/topic.md\n*** Update File: .codex/hooks.json"}}
            default_boundary = run([str(project_python), str(hooks / "hook_dispatcher.py"), "pre-tool"], root, mixed)
            self.assertEqual(default_boundary.returncode, 0, default_boundary.stderr)
            self.assertEqual(
                log.read_text(encoding="utf-8").splitlines(),
                [
                    "cross_project_write_guard.py", "user_config_write_guard.py",
                    "cross_project_write_guard.py", "user_config_write_guard.py",
                ],
            )
            self.assertEqual(default_boundary.stdout, "")

    def test_move_to_is_checked_as_a_write_target(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            hooks = root / ".codex" / "hooks"
            scripts = root / ".codex" / "scripts"
            hooks.mkdir(parents=True)
            scripts.mkdir()
            for name in ("hook_dispatcher.py", "cross_project_write_guard.py", "user_config_write_guard.py"):
                shutil.copy2(TEMPLATE / "hooks" / name, hooks / name)
            project_python = prepare_dispatcher_runtime(root)
            outside = root.parent / "outside-move.md"
            payload = {
                "tool_name": "apply_patch",
                "tool_input": {"command": f"*** Update File: inside.md\n*** Move to: {outside}"},
            }
            blocked = run([str(project_python), str(hooks / "hook_dispatcher.py"), "pre-tool"], root, payload)
            self.assertEqual(blocked.returncode, 2)
            self.assertIn("cross-project-write-guard", blocked.stderr)

            # Let the dedicated user-config guard observe the same normalized
            # Move target instead of being short-circuited by the broader guard.
            (hooks / "cross_project_write_guard.py").write_text("raise SystemExit(0)\n", encoding="utf-8")
            user_config = Path.home() / ".codex" / "config.toml"
            user_payload = {
                "tool_name": "apply_patch",
                "tool_input": {"command": f"*** Update File: inside.md\n*** Move to: {user_config}"},
            }
            protected = run([str(project_python), str(hooks / "hook_dispatcher.py"), "pre-tool"], root, user_payload)
            self.assertEqual(protected.returncode, 2)
            self.assertIn("user-config-write-guard", protected.stderr)

            observed = root / "observed.txt"
            probe = (
                "import json,sys\n"
                f"open({str(observed)!r}, 'a', encoding='utf-8').write(json.load(sys.stdin)['tool_input']['file_path']+'\\n')\n"
            )
            for name in (
                "encoding_check.py", "instruction_source_check.py",
                "rule_index_check.py", "rule_size_check.py",
                "requirements_check.py", "cargo_default_run_check.py", "fallback_smell_check.py",
            ):
                (hooks / name).write_text(probe if name == "encoding_check.py" else "raise SystemExit(0)\n", encoding="utf-8")
            post = run([str(project_python), str(hooks / "hook_dispatcher.py"), "post-edit"], root, payload)
            self.assertEqual(post.returncode, 0, post.stderr)
            self.assertIn(str(outside), observed.read_text(encoding="utf-8").splitlines())

    def test_post_tool_stdout_is_one_additional_context_json(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            hooks = root / ".codex" / "hooks"
            scripts = root / ".codex" / "scripts"
            hooks.mkdir(parents=True)
            scripts.mkdir()
            shutil.copy2(TEMPLATE / "hooks" / "hook_dispatcher.py", hooks / "hook_dispatcher.py")
            project_python = prepare_dispatcher_runtime(root)
            for name in (
                "encoding_check.py", "instruction_source_check.py",
                "rule_index_check.py", "rule_size_check.py",
                "requirements_check.py", "cargo_default_run_check.py",
            ):
                (hooks / name).write_text("raise SystemExit(0)\n", encoding="utf-8")
            (hooks / "fallback_smell_check.py").write_text("print('[fallback-smell] soft warning')\n", encoding="utf-8")
            payload = {"tool_name": "apply_patch", "tool_input": {"command": "*** Update File: app.py"}}
            edited = run([str(project_python), str(hooks / "hook_dispatcher.py"), "post-edit"], root, payload)
            edit_output = json.loads(edited.stdout)
            self.assertEqual(edit_output["hookSpecificOutput"]["hookEventName"], "PostToolUse")
            self.assertIn("[fallback-smell]", edit_output["hookSpecificOutput"]["additionalContext"])
            self.assertEqual(edited.stdout.count("hookSpecificOutput"), 1)

            (hooks / "test_receipt.py").write_text("print('[test-receipt] recorded')\n", encoding="utf-8")
            shell = run(
                [str(project_python), str(hooks / "hook_dispatcher.py"), "post-shell"],
                root,
                {"tool_name": "Bash", "tool_input": {"command": "pytest"}},
            )
            shell_output = json.loads(shell.stdout)
            self.assertEqual(shell_output["hookSpecificOutput"]["hookEventName"], "PostToolUse")
            self.assertIn("[test-receipt]", shell_output["hookSpecificOutput"]["additionalContext"])
            self.assertEqual(shell.stdout.count("hookSpecificOutput"), 1)

    def test_session_start_is_best_effort_but_returns_first_failure(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            hooks = root / ".codex" / "hooks"
            scripts = root / ".codex" / "scripts"
            hooks.mkdir(parents=True)
            scripts.mkdir()
            shutil.copy2(TEMPLATE / "hooks" / "hook_dispatcher.py", hooks / "hook_dispatcher.py")
            project_python = prepare_dispatcher_runtime(root)
            log = root / "session.log"
            stub = (
                "import os,sys\n"
                f"open({str(log)!r}, 'a', encoding='utf-8').write(os.path.basename(__file__)+'\\n')\n"
                "print('[session-step] '+os.path.basename(__file__))\n"
                "sys.exit(7 if os.path.basename(__file__) == 'config_health_check.py' else 0)\n"
            )
            for name in (
                "config_health_check.py", "enforce_no_effortlevel.py",
                "githooks_path_check.py", "show_state.py", "skill_sync_check.py",
            ):
                (hooks / name).write_text(stub, encoding="utf-8")
            result = run([str(project_python), str(hooks / "hook_dispatcher.py"), "session-start"], root, {})
            self.assertEqual(result.returncode, 7)
            order = log.read_text(encoding="utf-8").splitlines()
            self.assertEqual(len(order), 5)
            self.assertLess(order.index("config_health_check.py"), order.index("show_state.py"))
            output = json.loads(result.stdout)
            self.assertEqual(output["hookSpecificOutput"]["hookEventName"], "SessionStart")
            self.assertIn("show_state.py", output["hookSpecificOutput"]["additionalContext"])

    def test_session_start_has_no_project_memory_steps(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            hooks = root / ".codex" / "hooks"
            scripts = root / ".codex" / "scripts"
            hooks.mkdir(parents=True)
            scripts.mkdir()
            shutil.copy2(TEMPLATE / "hooks" / "hook_dispatcher.py", hooks / "hook_dispatcher.py")
            project_python = prepare_dispatcher_runtime(root)
            log = root / "session.log"
            stub = (
                "import os,sys\n"
                f"open({str(log)!r}, 'a', encoding='utf-8').write(os.path.basename(__file__)+'\\n')\n"
            )
            for name in (
                "config_health_check.py", "enforce_no_effortlevel.py",
                "githooks_path_check.py", "show_state.py", "skill_sync_check.py",
            ):
                (hooks / name).write_text(stub, encoding="utf-8")
            result = run([str(project_python), str(hooks / "hook_dispatcher.py"), "session-start"], root, {})
            self.assertEqual(result.returncode, 0)
            order = log.read_text(encoding="utf-8").splitlines()
            self.assertEqual(len(order), 5)
            self.assertNotIn("memory_rebuild_index.py", order)
            self.assertNotIn("memory_context.py", order)
            self.assertIn("show_state.py", order)

    def test_two_projects_and_sessions_do_not_recall_legacy_memory(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            base = Path(raw)
            sentinels = {
                "project-a": "SESSION_A_PROJECT_A_ONLY",
                "project-b": "SESSION_B_PROJECT_B_ONLY",
            }
            outputs: list[str] = []
            for project_name, sentinel in sentinels.items():
                project = base / project_name
                project.mkdir()
                legacy = project / ".codex" / "memory" / "legacy.md"
                legacy.parent.mkdir(parents=True)
                legacy.write_text(sentinel + "\n", encoding="utf-8")
                project_python = prepare_dispatcher_runtime(project)
                dispatcher = project / ".codex" / "hooks" / "hook_dispatcher.py"
                shutil.copy2(
                    TEMPLATE / "hooks" / "hook_dispatcher.py",
                    dispatcher,
                )
                log = project / "route.log"
                route_stub = (
                    "import os\n"
                    f"open({str(log)!r}, 'a', encoding='utf-8').write("
                    "os.path.basename(__file__)+'\\n')\n"
                )
                module = load_module(
                    TEMPLATE / "hooks" / "hook_dispatcher.py",
                    f"dispatcher_cross_project_{project_name}",
                )
                for relative in {
                    item
                    for targets in module.RUNTIME_ROUTES.values()
                    for item in targets
                }:
                    (project / ".codex" / Path(relative)).write_text(
                        route_stub,
                        encoding="utf-8",
                    )
                prompt = run(
                    [str(project_python), str(dispatcher), "user-prompt"],
                    project,
                    {"prompt": "回顾上一轮", "session_id": "first"},
                )
                self.assertEqual(prompt.returncode, 2, prompt.stderr)
                self.assertIn("unknown hook event route", prompt.stderr)
                started = run(
                    [str(project_python), str(dispatcher), "session-start"],
                    project,
                    {"session_id": "third"},
                )
                self.assertEqual(started.returncode, 0, started.stderr)
                outputs.append(started.stdout)
                routes = log.read_text(encoding="utf-8").splitlines()
                self.assertNotIn("focus_reminder.py", routes)
                self.assertNotIn("memory_router.py", routes)
                self.assertNotIn("memory_context.py", routes)
            combined = "\n".join(outputs)
            for sentinel in sentinels.values():
                self.assertNotIn(sentinel, combined)

    def test_strict_health_gate_rejects_both_illegal_sources(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            codex = root / ".codex"
            (codex / "hooks").mkdir(parents=True)
            (codex / "scripts").mkdir()
            shutil.copy2(TEMPLATE / "hooks" / "config_health_check.py", codex / "hooks" / "config_health_check.py")
            shutil.copy2(TEMPLATE / "scripts" / "hook_config_policy.py", codex / "scripts" / "hook_config_policy.py")
            shutil.copy2(TEMPLATE / "scripts" / "project_runtime.py", codex / "scripts" / "project_runtime.py")
            (codex / "hooks.json").write_text('{"hooks": {}}\n', encoding="utf-8")
            (codex / "settings.json").write_text('{"hooks": {}}\n', encoding="utf-8")
            (codex / "config.toml").write_text("[hooks]\n", encoding="utf-8")
            project_python = prepare_project_runtime(root)
            result = run([str(project_python), str(codex / "hooks" / "config_health_check.py"), "--strict"], root)
            self.assertEqual(result.returncode, 2)
            self.assertIn("settings.json contains hooks", result.stdout)
            self.assertIn("config.toml contains a hooks table", result.stdout)

    def test_strict_health_gate_rejects_non_project_python(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            codex = root / ".codex"
            (codex / "hooks").mkdir(parents=True)
            (codex / "scripts").mkdir()
            shutil.copy2(
                TEMPLATE / "hooks" / "config_health_check.py",
                codex / "hooks" / "config_health_check.py",
            )
            shutil.copy2(
                TEMPLATE / "scripts" / "hook_config_policy.py",
                codex / "scripts" / "hook_config_policy.py",
            )
            shutil.copy2(
                TEMPLATE / "scripts" / "project_runtime.py",
                codex / "scripts" / "project_runtime.py",
            )
            (codex / "hooks.json").write_text('{"hooks": {}}\n', encoding="utf-8")
            (codex / "settings.json").write_text(
                '{"permissions": {}}\n',
                encoding="utf-8",
            )

            result = run(
                [
                    sys.executable,
                    str(codex / "hooks" / "config_health_check.py"),
                    "--strict",
                ],
                root,
            )

            self.assertEqual(result.returncode, 2)
            self.assertIn("PROJECT_RUNTIME: project .venv is missing", result.stdout)

    def test_strict_health_gate_matches_merge_for_inline_toml_forms(self) -> None:
        forms = (
            "[hooks]\n",
            '["hooks"]\n',
            "['hooks']\n",
            '["ho\\u006fks"]\n',
            "[[hooks.PreToolUse]]\n",
            '[["hooks".PreToolUse]]\n',
            '[["hooks".PreToolUse.hooks]]\n',
            " [[ hooks . PreToolUse . hooks ]] # inline\n",
            "hooks.PreToolUse = []\n",
            "hooks = { PreToolUse = [] }\n",
        )
        for form in forms:
            with self.subTest(form=form), tempfile.TemporaryDirectory() as raw:
                root = Path(raw)
                codex = root / ".codex"
                (codex / "hooks").mkdir(parents=True)
                (codex / "scripts").mkdir()
                shutil.copy2(TEMPLATE / "hooks" / "config_health_check.py", codex / "hooks" / "config_health_check.py")
                shutil.copy2(TEMPLATE / "scripts" / "hook_config_policy.py", codex / "scripts" / "hook_config_policy.py")
                shutil.copy2(TEMPLATE / "scripts" / "project_runtime.py", codex / "scripts" / "project_runtime.py")
                (codex / "hooks.json").write_text('{"hooks": {}}\n', encoding="utf-8")
                (codex / "settings.json").write_text('{"permissions": {}}\n', encoding="utf-8")
                (codex / "config.toml").write_text(form, encoding="utf-8")
                project_python = prepare_project_runtime(root)
                result = run([str(project_python), str(codex / "hooks" / "config_health_check.py"), "--strict"], root)
                self.assertEqual(result.returncode, 2)
                self.assertIn("config.toml contains a hooks table", result.stdout)

    def test_quoted_non_hooks_tables_are_allowed_by_health(self) -> None:
        forms = (
            '["not-hooks"]\n',
            "[['not-hooks'.PreToolUse]]\n",
            '[["event".PreToolUse.hooks]]\n',
            "matrix = [\n  [1, 2],\n  [3, 4],\n]\n[database]\nports = [8000, 8001]\n",
        )
        for form in forms:
            with self.subTest(form=form), tempfile.TemporaryDirectory() as raw:
                root = Path(raw)
                codex = root / ".codex"
                (codex / "hooks").mkdir(parents=True)
                (codex / "scripts").mkdir()
                shutil.copy2(
                    TEMPLATE / "hooks" / "config_health_check.py",
                    codex / "hooks" / "config_health_check.py",
                )
                shutil.copy2(
                    TEMPLATE / "scripts" / "hook_config_policy.py",
                    codex / "scripts" / "hook_config_policy.py",
                )
                shutil.copy2(
                    TEMPLATE / "scripts" / "project_runtime.py",
                    codex / "scripts" / "project_runtime.py",
                )
                (codex / "settings.json").write_text('{"permissions": {}}\n', encoding="utf-8")
                (codex / "hooks.json").write_bytes((TEMPLATE / "hooks.json").read_bytes())
                (codex / "config.toml").write_text(form, encoding="utf-8")
                project_python = prepare_project_runtime(root)
                health = run([
                    str(project_python),
                    str(codex / "hooks" / "config_health_check.py"),
                    "--strict",
                ], root)
                self.assertEqual(health.returncode, 0, health.stdout + health.stderr)

    def test_malformed_toml_header_fails_closed_in_health(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            codex = root / ".codex"
            (codex / "hooks").mkdir(parents=True)
            (codex / "scripts").mkdir()
            shutil.copy2(
                TEMPLATE / "hooks" / "config_health_check.py",
                codex / "hooks" / "config_health_check.py",
            )
            shutil.copy2(
                TEMPLATE / "scripts" / "hook_config_policy.py",
                codex / "scripts" / "hook_config_policy.py",
            )
            shutil.copy2(
                TEMPLATE / "scripts" / "project_runtime.py",
                codex / "scripts" / "project_runtime.py",
            )
            (codex / "hooks.json").write_text('{"hooks": {}}\n', encoding="utf-8")
            (codex / "settings.json").write_text('{"permissions": {}}\n', encoding="utf-8")
            (codex / "config.toml").write_text('["hooks\\q"]\n', encoding="utf-8")
            project_python = prepare_project_runtime(root)

            health = run([
                str(project_python),
                str(codex / "hooks" / "config_health_check.py"),
                "--strict",
            ], root)
            self.assertEqual(health.returncode, 2)
            self.assertIn("table header invalid", health.stdout)

    def test_python_310_dispatcher_and_health_fail_closed(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            codex = root / ".codex"
            (codex / "hooks").mkdir(parents=True)
            (codex / "hooks.json").write_text('{"hooks": {}}\n', encoding="utf-8")
            (codex / "settings.json").write_text('{"permissions": {}}\n', encoding="utf-8")
            dispatcher = load_module(
                TEMPLATE / "hooks" / "hook_dispatcher.py",
                "hook_dispatcher_python_310",
            )
            dispatcher_stderr = io.StringIO()
            with redirect_stderr(dispatcher_stderr):
                dispatcher_rc = dispatcher.main((3, 10))
            self.assertEqual(dispatcher_rc, 2)
            self.assertIn("Python 3.11", dispatcher_stderr.getvalue())

            health = load_module(
                TEMPLATE / "hooks" / "config_health_check.py",
                "config_health_python_310",
            )
            health_stdout = io.StringIO()
            with redirect_stdout(health_stdout):
                health_rc = health.main((3, 10), strict=True)
            self.assertEqual(health_rc, 2)
            self.assertIn("PYTHON_VERSION: 3.10", health_stdout.getvalue())

if __name__ == "__main__":
    unittest.main()
