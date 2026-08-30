from __future__ import annotations

import hashlib
import importlib.util
import json
import sys
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
PROBE_PATH = Path(__file__).resolve().parent / "hook_context_probe.py"
CASES = Path(__file__).resolve().parent / "hook_context_cases.json"
PRODUCT_HOOKS = tuple(
    path
    for root in (ROOT / "templates" / "hooks", ROOT / ".codex" / "hooks")
    for path in root.glob("*.py")
)


def load_probe():
    spec = importlib.util.spec_from_file_location(
        "ia08_hook_context_probe", PROBE_PATH
    )
    if spec is None or spec.loader is None:
        raise RuntimeError(PROBE_PATH)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


PROBE = load_probe()


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


class HookContextProbeTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.report = PROBE.run_baseline(CASES)

    def test_corpus_covers_required_prompt_classes(self) -> None:
        corpus = json.loads(CASES.read_text(encoding="utf-8"))
        categories = {item["category"] for item in corpus["prompt_cases"]}
        self.assertEqual(
            categories,
            {
                "continuation",
                "clear-small",
                "clear-large",
                "ambiguous-large",
                "command",
            },
        )
        self.assertGreaterEqual(len(corpus["prompt_cases"]), 30)
        self.assertEqual(len(corpus["focus_sequences"]), 3)

    def test_probe_is_deterministic_and_product_hooks_are_read_only(self) -> None:
        before = {path: sha256(path) for path in PRODUCT_HOOKS}
        repeated = PROBE.run_baseline(CASES)
        after = {path: sha256(path) for path in PRODUCT_HOOKS}
        self.assertEqual(self.report, repeated)
        self.assertEqual(before, after)

    def test_clarify_hook_is_retired_from_all_runtime_owners(self) -> None:
        retired = "clarify_reminder.py"
        self.assertFalse((ROOT / "templates" / "hooks" / retired).exists())
        self.assertFalse((ROOT / ".codex" / "hooks" / retired).exists())
        for path in (
            ROOT / "templates" / "hooks" / "hook_dispatcher.py",
            ROOT / ".codex" / "hooks" / "hook_dispatcher.py",
            ROOT / "templates" / "managed-skeleton.json",
            ROOT / ".codex" / "managed-skeleton.json",
        ):
            self.assertNotIn(retired, path.read_text(encoding="utf-8"))
        self.assertNotIn("clarify", self.report)

    def test_focus_hook_is_retired_from_all_runtime_owners(self) -> None:
        retired = "focus_reminder.py"
        self.assertFalse((ROOT / "templates" / "hooks" / retired).exists())
        self.assertFalse((ROOT / ".codex" / "hooks" / retired).exists())
        for path in (
            ROOT / "templates" / "hooks" / "hook_dispatcher.py",
            ROOT / ".codex" / "hooks" / "hook_dispatcher.py",
            ROOT / "templates" / "managed-skeleton.json",
            ROOT / ".codex" / "managed-skeleton.json",
        ):
            self.assertNotIn(retired, path.read_text(encoding="utf-8"))
        self.assertFalse(self.report["focus"]["runtime_present"])
        self.assertEqual(self.report["focus"]["sequences"], [])

    def test_cost_report_separates_characters_from_runtime_tokens(self) -> None:
        combined = self.report["combined_prompt_context"]
        self.assertEqual(
            combined["p95_characters"], combined["minimum_characters"]
        )
        self.assertEqual(
            combined["twenty_turn_p95_characters"],
            20 * combined["p95_characters"],
        )
        self.assertFalse(self.report["runtime_tokens"]["measured"])
        self.assertIsNone(self.report["runtime_tokens"]["value"])

    def test_project_memory_contributes_zero_context_across_sessions_and_projects(self) -> None:
        project_memory = self.report["project_memory"]
        self.assertFalse(project_memory["runtime_present"])
        self.assertEqual(project_memory["active_routes"], [])
        self.assertEqual(project_memory["prompt_context_characters"], 0)
        self.assertEqual(project_memory["session_start_context_characters"], 0)
        for row in self.report["cases"]:
            self.assertFalse(row["project_memory"]["emitted"])
            self.assertEqual(row["project_memory"]["characters"], 0)
        sentinels = ("SESSION_A_ONLY", "SESSION_B_ONLY", "PROJECT_B_ONLY")
        all_context = "\n".join(
            str(row["combined"]["context"])
            for row in self.report["cases"]
        )
        self.assertTrue(all(item not in all_context for item in sentinels))


if __name__ == "__main__":
    unittest.main()
