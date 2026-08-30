#!/usr/bin/env python3
"""Deterministic, read-only baseline probe for managed Hook context."""
from __future__ import annotations

import importlib.util
import io
import json
import math
import sys
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path
from unittest import mock


ROOT = Path(__file__).resolve().parents[2]
CASES = Path(__file__).resolve().parent / "hook_context_cases.json"
TEMPLATE_HOOKS = ROOT / "templates" / "hooks"


def _load_module(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    with redirect_stdout(io.StringIO()), redirect_stderr(io.StringIO()):
        spec.loader.exec_module(module)
    return module


def _call_main(module, payload: dict[str, object], argv: list[str]) -> tuple[int, str, str]:
    stdin_before = sys.stdin
    stdout = io.StringIO()
    stderr = io.StringIO()
    try:
        sys.stdin = io.StringIO(json.dumps(payload, ensure_ascii=False))
        with mock.patch.object(sys, "argv", argv), redirect_stdout(stdout), redirect_stderr(stderr):
            result = module.main()
    finally:
        sys.stdin = stdin_before
    return int(result or 0), stdout.getvalue().strip(), stderr.getvalue().strip()


def _metrics(context: str) -> dict[str, object]:
    return {
        "emitted": bool(context),
        "characters": len(context),
        "utf8_bytes": len(context.encode("utf-8")),
    }


def _percentile(values: list[int], percentile: float) -> int:
    if not values:
        return 0
    ordered = sorted(values)
    index = max(0, math.ceil(percentile * len(ordered)) - 1)
    return ordered[index]


def _state_context(module) -> str:
    with (
        mock.patch.object(module, "_git_state", return_value=("codex/ia08", 3, "1/0")),
        mock.patch.object(module, "_version", return_value="1.4.31"),
    ):
        _code, stdout, _stderr = _call_main(
            module, {}, ["show_state.py", "prompt-state"]
        )
    return stdout


def run_baseline(case_path: Path = CASES) -> dict[str, object]:
    corpus = json.loads(case_path.read_text(encoding="utf-8"))
    show_state = _load_module("ia08_show_state", TEMPLATE_HOOKS / "show_state.py")
    dispatcher = _load_module(
        "ia08_dispatcher",
        TEMPLATE_HOOKS / "hook_dispatcher.py",
    )
    runtime_targets = sorted({
        target
        for targets in dispatcher.RUNTIME_ROUTES.values()
        for target in targets
    })
    show_state_routes = sorted(
        route
        for route, targets in dispatcher.RUNTIME_ROUTES.items()
        if "hooks/show_state.py" in targets
    )
    project_memory_routes = [
        target
        for target in runtime_targets
        if "memory" in Path(target).name.casefold()
    ]

    session_state_context = _state_context(show_state)
    prompt_state_context = (
        session_state_context if "user-prompt" in show_state_routes else ""
    )
    rows = []
    for case in corpus["prompt_cases"]:
        combined = prompt_state_context
        rows.append(
            {
                **case,
                "project_memory": {"context": "", **_metrics("")},
                "state": {
                    "context": prompt_state_context,
                    **_metrics(prompt_state_context),
                },
                "combined": {"context": combined, **_metrics(combined)},
            }
        )

    combined_sizes = [int(row["combined"]["characters"]) for row in rows]

    return {
        "schema_version": 2,
        "corpus": {
            "prompt_cases": len(rows),
            "focus_sequences": len(corpus["focus_sequences"]),
            "focus_turns": sum(
                len(item["prompts"]) for item in corpus["focus_sequences"]
            ),
        },
        "show_state": {
            "runtime_present": bool(show_state_routes),
            "active_routes": show_state_routes,
            "prompt_emitted_cases": sum(
                bool(row["state"]["emitted"]) for row in rows
            ),
            "relevant_cases": sum(bool(row["state_relevant"]) for row in rows),
            "irrelevant_cases": sum(not bool(row["state_relevant"]) for row in rows),
            "prompt_context_characters": len(prompt_state_context),
            "session_start_context_characters": len(session_state_context),
        },
        "project_memory": {
            "runtime_present": bool(project_memory_routes),
            "active_routes": project_memory_routes,
            "prompt_context_characters": 0,
            "session_start_context_characters": 0,
        },
        "focus": {"runtime_present": False, "sequences": []},
        "combined_prompt_context": {
            "minimum_characters": min(combined_sizes),
            "median_characters": _percentile(combined_sizes, 0.5),
            "p95_characters": _percentile(combined_sizes, 0.95),
            "maximum_characters": max(combined_sizes),
            "twenty_turn_p95_characters": 20 * _percentile(combined_sizes, 0.95),
        },
        "runtime_tokens": {
            "measured": False,
            "value": None,
            "reason": "requires isolated Codex A/B runtime receipts",
        },
        "cases": rows,
    }


def main() -> int:
    print(json.dumps(run_baseline(), ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
