---
status: archived
topic: bridgeforge-doc-runtime-packaging
archived_at: 2026-08-30
superseded_by: skills/bridgeforge-codex/references
---

# BridgeForge 旧运行手册打包结构

## 历史结论

2026-07-25，BridgeForge 曾把 `init.md`、`adopt.md`、`update.md`、`switch.md` 和
`user-skill-maintenance.md` 五份运行手册集中到 `doc/0_playbook/`。分发清单只携带这一棵
`doc/` 子树，避免把需求卡、设计记录、Bug 和 archive 一并安装到用户目录；各 Skill 自己的
`references/` 仍归对应 Skill 所有。

当时的目录移动同时更新了分发 inventory/hash 与下游 fixture，版本从 `0.65.0` 升到
`0.65.1`，共享分发测试 13/13 和完整下游 harness 通过。这些数字只证明当时交付，不是
当前版本或运行时收据。

## 当前替代关系

`doc/0_playbook/` 和双宿主 `switch` 手册已经退役。当前 BridgeForgeCodex 运行资料由
`skills/bridgeforge-codex/references/` 按 Skill 所有权携带；manifest 只分发该 Skill
明确登记的文件。

本档案不恢复旧目录、不复制五份旧手册，也不构成当前安装、更新或 switch 指令。
