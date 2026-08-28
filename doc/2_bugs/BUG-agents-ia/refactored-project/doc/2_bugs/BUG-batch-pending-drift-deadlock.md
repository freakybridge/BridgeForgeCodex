---
status: source-fixed-awaiting-release-and-runtime
severity: high
scope: factory-only bridgeforge-codex-batch pending target drift
reported_at: 2026-08-27
factory_head: 734b9544bd2e27884a655a6808c32bf518ed065a
product_version: 1.5.6
---

# BUG：批次 pending 目标漂移后进入不可恢复死路

## 结论

`bridgeforge-codex-batch` 在首次串行处理中发现 pending 目标现场变化时，既不能把该目标转为
deferred，也不能刷新并重新确认计划。控制器会同时拒绝 `begin`、`finish deferred` 和
`refresh-plan`，导致后续正常项目也无法继续，整个 active batch 只能停在不可达状态。

这是工厂批次控制器的状态机缺陷，不是下游项目失败。所有拒绝都发生在目标写入前，现场已保留。

## 真实复现

2026-08-27 继续一个包含四个真实下游的 active batch：前两个项目已经完成，第三个目标在旧批次
快照之后由用户正常推进，当前分支 HEAD 与 dirty 摘要均已变化。

按 Skill 协议执行第三个目标时得到以下闭环：

1. `begin` 拒绝：`该项目状态已变化，需要重新展示异常计划并确认`。
2. `finish --outcome deferred` 拒绝：`该项目尚未开始处理`。
3. `refresh-plan` 拒绝：`必须先按顺序处理完首次计划，再重新确认异常项目`。
4. 第三个目标仍为 pending，因此严格顺序也禁止开始第四个目标。

没有下游文件写入、stash、reset、rebase、merge 或冲突处理；已完成项目保持干净并与远端 0/0。

## 源码证据

`.codex/skills/bridgeforge-codex-batch/scripts/batch_control.py` 当前约束互相闭锁：

- `begin_target()` 仅在快照完全一致时才把 pending 改为 running；快照变化直接抛错，不改变状态。
- `finish_target()` 只接受 running，因此无法把未开始但已漂移的目标登记为 deferred。
- `refresh_plan()` 要求不存在 pending/running，且目标必须已经是 deferred。
- `_next_target()` 继续把该 pending 目标视为唯一下一项，后续目标不能开始。

因此不存在任何官方命令序列能把该状态推进到 deferred -> refresh-plan -> reconfirm。

## 修复边界

推荐在 `begin_target()` 的快照漂移分支中原子完成安全延期：

1. 目标保持零写入且不创建 running attempt。
2. 将当前 pending 目标转为 deferred，并记录不含路径、commit 或逐文件差异的稳定白话原因。
3. 返回可识别的延期收据，使主对话继续首次串行处理后续正常目标。
4. 全部首次处理结束后，现有 `refresh-plan -> reconfirm -> begin` 路径必须可达。

禁止通过关闭 active batch、手改状态 JSON、放宽顺序、静默吸收新快照或跳过再次确认来绕过。

## 验收标准

- 首个、中间和最后一个 pending 目标发生漂移时，`begin` 都零目标写入并原子转为 deferred。
- 后续未漂移目标仍按确认顺序继续；共享 Git common dir 的 worktree 仍禁止并行。
- 首次处理完成后，deferred 目标可生成新计划、重新确认并完整重试。
- 漂移原因和用户摘要不泄露绝对路径、commit、逐文件差异或 traceback。
- 状态写入失败时不得留下半转换结果；active batch 与 lock 语义不回归。
- `scripts/tests/test_bridgeforge_codex_batch_skill.py` 增加真实状态机回归，并通过完整自动测试。
- 修复发布并 `$git-sync` 后，当前真实批次必须通过 `restart` 从新 factory HEAD 全量重跑。

## 六类证据

| 类别 | 当前状态 |
|---|---|
| 源码 | 已修复；pending 漂移或不可读会原子 deferred，reconfirm 恢复 pending |
| 产品传播 | 不适用；这是工厂专属 Skill，不下沉 Template 或共享 Skills |
| dogfood | 已在工厂 active batch 真实复现 |
| fixture | 15 项 Batch 专项测试通过；覆盖首/中/末漂移、写入失败原子性与修复见证 |
| 真实下游 | 两个项目已成功，第三个在目标写入前触发死路，现场保留 |
| runtime | Codex 主对话真实命令序列已复现；修复发布后的原批次 restart 尚未验证 |

## 传播四问

1. 层级：自身配置层；修改工厂专属 `.codex/skills/bridgeforge-codex-batch/**`，不是下游产品层。
2. 通用性：只影响工厂批量分发，禁止下沉 `templates/**` 或共享 `skills/**`。
3. 版本与 CHANGELOG：修复应在 `CHANGELOG.md` 标记 `[repo]`；不因该工厂专属修复 bump 产品 `VERSION`。
4. dogfood：必须更新批次控制器及其 `scripts/tests/test_bridgeforge_codex_batch_skill.py`，并用当前真实 active batch 完成 restart 复验。
