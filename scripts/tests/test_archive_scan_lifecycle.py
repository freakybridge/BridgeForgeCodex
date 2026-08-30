#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
TEMPLATE_SCRIPT = ROOT / "templates" / "scripts" / "archive_scan.py"
DOGFOOD_SCRIPT = ROOT / ".codex" / "scripts" / "archive_scan.py"


def load_archive_scan():
    spec = importlib.util.spec_from_file_location("archive_scan_lifecycle_test", TEMPLATE_SCRIPT)
    if spec is None or spec.loader is None:
        raise RuntimeError("cannot load archive_scan.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class ArchiveScanLifecycleTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.module = load_archive_scan()
        self.module.REPO_ROOT = self.root
        self.module.DELIVERY_DIR = self.root / "doc" / "1_delivery"
        self.module.BUG_DIR = self.root / "doc" / "2_bugs"
        self.module.ARCHIVE_DIR = self.root / "doc" / "4_archive"

    def tearDown(self) -> None:
        self.temp.cleanup()

    def write(self, relative: str, text: str) -> Path:
        path = self.root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")
        return path

    def test_lifecycle_candidates_exclude_active_and_unclassified(self) -> None:
        self.write(
            "doc/1_delivery/done/requirements_done.md",
            "---\nlifecycle: completed\nvalidation_status: verified\n---\n",
        )
        self.write(
            "doc/1_delivery/replaced/requirements_old.md",
            "---\nlifecycle: superseded\nsuperseded_by: ../new\n---\n",
        )
        self.write(
            "doc/1_delivery/active/requirements_active.md",
            "---\nlifecycle: active\nvalidation_status: in_progress\n---\n",
        )
        self.write(
            "doc/1_delivery/mixed/requirements_done.md",
            "---\nlifecycle: completed\n---\n",
        )
        self.write(
            "doc/1_delivery/mixed/requirements_legacy.md",
            "# no lifecycle\n",
        )
        self.write(
            "doc/2_bugs/BUG-done.md",
            "---\nlifecycle: completed\nvalidation_status: verified\n---\n",
        )
        self.write(
            "doc/2_bugs/BUG-active.md",
            "---\nlifecycle: active\nvalidation_status: in_progress\n---\n",
        )

        results = self.module.scan()
        sources = {item["source"].replace("\\", "/") for item in results}

        self.assertEqual(
            sources,
            {
                "doc/1_delivery/done",
                "doc/1_delivery/replaced",
                "doc/2_bugs/BUG-done.md",
            },
        )
        self.assertTrue(
            all("legacy evidence" not in reason for item in results for reason in item["reasons"])
        )

    def test_legacy_markers_remain_explicit_compatibility_evidence(self) -> None:
        self.write(
            "doc/1_delivery/legacy/acceptance.md",
            "状态：已完成\n",
        )
        self.write(
            "doc/2_bugs/BUG-legacy.md",
            "状态: resolved\n",
        )

        results = self.module.scan()

        self.assertEqual(len(results), 2)
        self.assertTrue(
            all("legacy evidence" in item["reasons"][0] for item in results)
        )

    def test_bug_packages_are_scanned_without_descending_into_evidence_trees(self) -> None:
        self.write(
            "doc/2_bugs/BUG-package/README.md",
            "---\nlifecycle: completed\nvalidation_status: verified\n---\n",
        )
        self.write(
            "doc/2_bugs/BUG-package/refactored-project/doc/2_bugs/BUG-shadow.md",
            "---\nlifecycle: completed\nvalidation_status: verified\n---\n",
        )

        results = self.module.scan()
        sources = {item["source"].replace("\\", "/") for item in results}

        self.assertEqual(sources, {"doc/2_bugs/BUG-package"})

    def test_template_and_dogfood_scripts_match(self) -> None:
        self.assertEqual(TEMPLATE_SCRIPT.read_bytes(), DOGFOOD_SCRIPT.read_bytes())


if __name__ == "__main__":
    unittest.main()
