---
lifecycle: active
validation_status: in_progress
---

# 干净工厂 git-sync 跳过生成资产修复

## 现象

BridgeForge 工作树与远端完全同步时，`bridgeforge git-sync` 直接返回 `synced`，不会校验或重建 `.codex/bin/`。因此工厂源码和受管合同已经升级到 1.14.1，实际 CLI 与 Hook 仍可能停留在 1.12.5，并让项目级 Hook 审批被 `generated asset receipt drifted` 阻断。

## 根因与修复

`git_sync::sync()` 只在 Git 工作树 dirty 时进入事务式自动写入、生成资产构建和 current baseline。修复后，干净工厂先做不含生成产物的 baseline：该层失败保留原错并直接阻断；该层健康但完整 baseline 失败时，才进入同一事务重建生成资产。健康工厂走零 runtime 写入 fast path。修复过程不生成 release、不要求 commit message、不修改版本或 HEAD；生成资产或最终 baseline 失败仍按原事务回滚并阻断。

## 六类关闭证据

| 类别 | 当前证据 |
|---|---|
| 源码 | `.codex/hooks/crates/bridgeforge-core/src/git_sync.rs` 已覆盖分层 baseline、clean runtime repair 与最终复核；factory workspace 101 core + 15 Hook 测试通过 |
| 产品传播 | `templates/hooks/crates/bridgeforge-core/src/git_sync.rs` 与 dogfood 逐字一致；manifest `--check`、skill metadata 与项目结构硬闸通过，等待发布收据 |
| dogfood | 已复现 CLI/Hook 1.12.5 对 1.14.1 合同的收据漂移；受管 `build-assets` 自举后 full baseline clean，等待本次 `$git-sync` 发布终态 |
| fixture | 完整 factory fixture 81 passed、2 ignored；真实 CLI 用例证明 dirty 发布、损坏收据后的无消息自愈、健康 fast path 零 runtime 写入、Git clean 与 0/0 |
| 真实下游 | 未验证；计划在本次四项目 batch 中验证 |
| runtime | 未验证；等待 Codex 项目级 Hook 审批界面刷新确认 |

在真实下游、当前工厂 runtime 和用户验收完成前，本 Bug 保持开放。
