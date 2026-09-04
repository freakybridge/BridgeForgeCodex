---
title: 受管 Markdown 同节项目表与注释示例表被误判为歧义
lifecycle: active
validation_status: awaiting_validation
date: 2026-09-05
---

# 受管 Markdown 同节项目表与注释示例表被误判为歧义

## 现象

`project-sync` 规划 HoldemTrainer 时，在 `doc/README.md` 的 `## 1_delivery/`
区域返回 `managed Markdown table is missing or ambiguous`。该区域同时包含项目正在使用的
索引表，以及骨架保留在 HTML 注释中的示例表；规划阶段零写入退出。

## 根因

keyed-table 合并器只允许标题区域内出现一个 Markdown 表格分隔行。它没有利用合同已经登记的
`managed_keys` 区分受管示例表与项目自有表，因此将两张合法且用途不同的表误判为无法定位。

## 修复

- 单表路径保持原有严格校验。
- 多表路径只在恰有一张表包含受管 key 时选择该表，其他项目表逐字保留。
- 没有唯一候选，或多张表都包含受管 key 时继续 fail-closed，禁止猜测和写入。
- 工厂源码与 Template 镜像同步修改，并增加真实结构回归：项目实表与注释示例表共存时只更新
  示例表；两张表都出现受管 key 时必须阻断。

## 验证记录

- 针对性 Rust 回归 `markdown_upgrade*`：2/2 通过。
- 工厂 Rust workspace：Core 102/102、Hook 15/15、CLI 9/9 通过；4 个 subprocess
  fixture 按设计忽略。
- 完整 factory fixture：81/81 通过；2 个需显式外部输入的 fixture 按设计忽略。
- 锁定 release build、manifest `--check`、factory-version、完整 baseline、项目结构和 Skill
  metadata 硬闸全部通过；dogfood 生成资产已重建并通过完整 baseline。
- 正式 dogfood CLI 对真实 HoldemTrainer 重新执行只读 `project-sync` 规划，结果为
  `planned`，`gaps=[]`、`blockers=[]`；未执行 `apply`，下游零写入。
- 源码、产品传播、dogfood、fixture、真实下游与 runtime 六类证据已取得；独立发布审计
  未发现 correctness 或发布阻断。版本发布、commit 和 push 尚未执行。
- 本 Bug 不直接修改任何下游项目。
