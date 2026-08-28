---
category: topic
topic: agents-md-simplification-review
status: active
description: AGENTS.md 信息架构重构在完整项目镜像中逐项实施并由用户手动审阅，真实骨架暂不替换。
kind: delivery
tags:
  - agents
  - information-architecture
  - comprehensibility
  - refactor
related_paths:
  - doc/1_delivery/agents-md-simplification-review/requirements_2026-08-27_agents-iterative-validation.md
  - doc/2_bugs/BUG-agents-ia/README.md
  - doc/2_bugs/BUG-agents-ia/refactored-project/
---

# AGENTS.md 信息架构重构

## 当前目标

- 在 Bug 目录中的完整项目镜像内逐项解决 AGENTS.md 与公共骨架的信息架构问题。
- 用户手动查看并确认候选项目之前，不替换仓库根目录的真实运行资产。
- 最终验收必须区分候选静态验证、真实下游、Codex runtime smoke 与用户试读。

## 已确认决定

- 候选采用完整项目镜像，不采用只保存改动文件的差异镜像。
- 已退役机制不得留在当前有效骨架中；历史 Delivery、Bug 与 CHANGELOG 保留。
- 不为已退役机制新增“禁止复活”规则、Hook 或负向测试。
- `focus_reminder.py` 的防漂移功能仍然有效；旧 `§9.6` 只是源码说明中的失效章节号，不是运行故障。
- managed contract、VERSION 与 CHANGELOG 在整轮重构收口时统一更新，不为单行整洁修改单独发布。

## 已完成

- 在 `doc/2_bugs/BUG-agents-ia/refactored-project/` 建立基于提交 `53722fc845f6d6934f4c7d88fec36257171392b5` 的完整镜像，共 397 个 Git 跟踪文件；未复制 `.git`、`.venv`、缓存、临时文件或镜像自身。
- 在外层 Bug README 记录完整镜像的基线、边界和使用方式。
- IA-01 的 `section_layout` 当前面已清理：现行架构文档改为真实 marker / ownership review 行为，现有测试删除该退役名称的负向断言；历史记录未修改。
- `templates/hooks/focus_reminder.py` 与 `.codex/hooks/focus_reminder.py` 的开发说明已由旧 `§9.6` 改为稳定标题 `AGENTS.md「任务防漂移」`；Hook 逻辑与实际输出未修改。

## 已有收据

- `section_layout` 在上述两个当前文件中的命中为 0。
- `scripts.tests.test_instruction_source_check` 针对性测试 10/10 通过。
- 镜像项目结构检查返回 `errors=[]`；仅有既存 archive advisory。
- 两份 `focus_reminder.py` 的旧指针命中为 0、新标题命中为 2，Template 与 dogfood SHA-256 一致。

## 当前问题与下一步

- `templates/AGENTS.md` 引用 `doc/3_reference/codex-hook-signals.md`，但 Template 目录只有 `templates/doc/3_reference/.gitkeep`，`templates/managed-skeleton.json` 也没有对应资产；工厂自身有该文档，下游不会收到它。
- 下一步先解释并确认该下游断链的正确处理方式，再只在完整镜像内实施。
- Hook 内容变更后 managed contract hash 尚未重建；这是整轮候选收口项，不是已验证完成项。

## 未验证与边界

- V11 第二名独立评审、真实骨架替换、正式 release suite、真实下游更新、runtime smoke 和用户试读均未完成。
- 当前 runtime trust 未验证。
- 仓库根工作树仍有外层 Bug README 修改和未跟踪完整镜像；未暂存、未 commit、未 push。
