---
lifecycle: completed
validation_status: verified
---

# 骨架路径迁移后的发布分类修复

关联 TODO-034；这是已验收重构的增量缺陷修复，不改写原验收记录。

## 授权、范围与预算

2026-10-25 用户明确授权：修复分类缺陷，补齐下游骨架迁移后 release 的回归测试，完成必要验证和子包版本收口；不提交、不推送、不发布，不修改真实下游或用户级安装。新增隔离 Git fixture 仅测试历史和发布预览，不设置远端、不实际 release；既有必需回归可在临时本地仓库模拟提交、发布及本地 bare remote 交互，不连接真实远端、不修改本仓库 Git 状态。

规模 M：单一已定位的分类问题，涉及分类器、schema 4 调用链及 CLI 集成验证。沿用原重构授权的累计预算上限（8–12 小时、100,000–160,000 token），原记录已用约 5 小时 20 分；本增量预留约 2 小时，包含既有 lifecycle 必需检查、构建与修复重测，不重置原预算。token 未实测，沿用原记录的周额度系数未知口径，不编造百分比。主对话实现；如进行独立审计，按根 AGENTS 使用一个只读 Agent。最多两轮实质验证，预计超出剩余预算则保存证据并停止超额工作。

## 已核实根因与传播四问

- 同一受管资产从旧 `hooks/crates/` 移到 `hooks/source/`，资产 ID 和内容未变。分类器只比较内容投影，`public_changed=false` 导致回落为 Business；schema 4 按路径分类，随后触发 unattributed release input。
- 属于通用产品层修复：实现维护在 `source/lifecycle/src/release.rs`，测试维护在 `scripts/tests/`。
- 应完成 lifecycle 子包的兼容 patch 收口；主程序 VERSION 和正式 CHANGELOG 留待获准 release。
- 需要经工厂 manifest 生成公共投影、清单和 dogfood；不手改镜像，不修改用户级安装或真实下游。

## 实现与验收

1. 依据两端合同身份、路径及校验通过的内容识别路径迁移；不扩大目录忽略、不改业务归属或正式发布基线。
2. 保留项目内容差异；旧路径残留、目标占用、哈希不符、未知文件及身份冲突不得被误排除。
3. 覆盖同内容迁移、迁移同时更新内容、未提交迁移和已提交但未发布的累计迁移；混合业务改动仍须收口。
4. 先运行针对性回归，再复用受管 develop 的 lifecycle 必需测试集；通过实际 CLI 验证隔离下游发布预览及零写入。
5. 完成受管版本事务和最终产物准备，核验清单、投影、版本及收据一致；重复预览无待收口变化。

## 实施与验证记录

工程开发、必要验证与子包收口已完成，用户于 2026-10-25 通过 `$summary accept` 验收。源码、产品传播、dogfood、fixture、真实下游、runtime 分别记录证据；真实下游和用户级安装明确排除，不将本地回归当作真实下游发布成功。

- 修复前：隔离 schema 4 fixture 复现了与 Stratus 一致的旧路径 unattributed release input；两个分类单元用例同样失败。初次沙箱执行的临时 Git 配置拒绝写入已通过同命令获准环境重试区分。
- 修复：同 ID 的两端合同、内容验证成功后，将目标路径变化计入公共变化；保留旧路径残留、目标原有文件及项目内容差异为业务变化；目标缺失阻断。seed 文件始终保留项目路径归属。
- 独立审计：`migration_classification_review` 发现 seed 所有权边界遗漏，已修正并复核，无剩余阻断项。审计为静态证据，不代替测试、版本事务或最终产物。
- 定向验证：`cargo test --locked --offline --config scripts/tests/factory-cargo.toml --manifest-path Cargo.toml -p bridgeforge-lifecycle --lib --target-dir .runtime/source-workspace-refactor/cargo relocated -- --test-threads=1`，4 passed，0 failed，107.09 秒。
- 集成验证：`cargo test --locked --offline --manifest-path scripts/tests/Cargo.toml real_cli_release_preview_after_project_sync_source_relocation -- --test-threads=1`，1 passed，0 failed，37.80 秒。实际 project-sync apply 迁移隔离项目；未提交/已提交骨架均 nothing-to-release，业务改动完成 develop 后 release-planned，预览零写入，无远端或实际 release。
- 公共传播及 dogfood：受管 manifest 已生成；首次准备得到 `prepared=3`、`self_test_executed=true`。版本收口后仍须核验最终产物。
- 首次版本事务：五包计划仅 lifecycle 0.1.0→0.1.1，其他四包证据重验；factory-unit 失败后原生版本写入自动回滚，主程序 VERSION、Git 未改变。
- 阻塞定位：既有 `mixed_product_release_materializer_updates_consumer_lock_without_bumping_harness` 写死目标 7.0.0，产品已发布 7.0.0 后不产生消费者锁写入，141 行取值 panic。已单独重现；仅改测试为从 fixture 当前产品版本计算下一 patch，全部原断言保留，独立复核通过。重编遇到旧测试进程占用 Windows 可执行文件后停止重试，等待诊断结束后顺序验证，不修改权限或缓存规则。
- factory 诊断完整结束：32 passed、1 failed、1 ignored，566.30 秒；唯一失败为上述固定版本用例，ignored 为辅助子进程入口。旧进程退出后修正用例实际重测 1 passed，6.37 秒，随后重新执行完整受管收口。
- 最终受管版本事务实际返回 `development-closed`、`git_written=false`。五组配置检查 `core-unit`、`factory-unit`、`lifecycle-unit`、`skeleton-unit`、`workflow-unit` 全部 passed；不是用定向回归替代。事务实测 7949.03 秒（约 132 分钟），耗时主要为原配置串行检查；超出本增量约 2 小时初估，未增加测试范围或重置原任务预算。整轮 token 未实测。
- 版本结果：lifecycle 0.1.0→0.1.1（兼容 patch）；core 7.0.1、factory 0.1.1、skeleton/workflow 0.1.0 均为 level 0 证据重验，不重复升版。checks、memory 无待收口变化，均保持 0.1.0。lifecycle 未配置稳定验收合同，故仍为 0.x，不转 1.0.0。
- 收口账本由 14 条追加至 19 条，每个新增条目均含必需检查的实际命令和输出摘要。主程序 VERSION=7.0.0、CHANGELOG、版本配置、Git 索引和 HEAD 在事务中保持不变。实际收据：本机忽略目录 `.runtime/migration-release-classification/development-result.json`，持久认证记录 `.codex/package-closures.json`。

## 工程交付与验证边界（2026-10-25）

| 证据类别 | 实际结果 |
|---|---|
| 源码 | `source/lifecycle/src/release.rs` 修复已实现；4 项定向回归及 lifecycle 全部配置检查通过。seed 边界已独立审计并修正；未放宽业务归属或未知文件检查。 |
| 产品传播 | 工厂生成器已更新公共清单、投影及锁；`manifest --check` 返回 `changed=false`。仅本地传播，未正式发布。 |
| dogfood | `.codex/hooks/source/lifecycle/src/release.rs` 与维护源哈希相同；baseline 为 clean、factory-version 为 healthy。 |
| fixture | 最终产物上再次运行真实 project-sync→CLI release-preview 用例，1 passed、0 failed、47.42 秒；覆盖未提交和累计已提交骨架迁移、混合业务包开发收口及预览零写入。新增用例无远端、无实际 release。 |
| 真实下游 | 未修改、未执行；未复验 StratusAgent 的实际 release，用户级安装也未变更。 |
| runtime | 最终版本准备返回 `prepared=3`、`self_test_executed=true`；只读构建收据检查为 current、reused=3、built=0。证明本地 CLI/Hook/工厂产物有效，不代表自然 Codex Hook 生命周期或真实下游已验证。 |

最终只读核验：TODO 检查和 `git diff --check` 通过；重复 `develop --preview` 返回 `nothing-to-close`；不存在 develop 事务日志；旧 14 条账本记录逐项与 HEAD 一致；仅追加上述 5 条。主程序仍为 7.0.0，HEAD 仍为 `ed6b20f1c3ed952699896576f04eeea26c5dad2e`，没有暂存、提交、推送或发布。

本机详细核验收据在 `.runtime/migration-release-classification/final-verification.json`。必需检查的实际命令、输入与输出摘要在 `.codex/package-closures.json`；未跑无关的完整 workspace/全 fixture 集合。整轮 token 未实测，不以 132 分钟的收口事务耗时冒充整轮总耗时。TODO-034 的用户验收与状态结算见下节。

## 经验与适用边界

- 资产内容不变不代表没有公共变化。迁移分类必须联合稳定 ID、两端目标路径和校验通过的内容判断，并覆盖调用方按单路径分类的方式；仅验证整批差异会遗漏删除路径的回落行为。
- 工厂自身发布成功不能证明下游迁移发布可用。本次采用实际 project-sync→CLI release-preview 隔离链路，分别覆盖未提交迁移、正式发布后累计已提交迁移及混合业务变化；正式发布基线和业务收口门槛保持原合同。
- 公共迁移不能吞掉项目所有权。独立审计补出了 seed 边界；项目区域、旧路径残留、目标占用及未知文件均须继续进入业务判断或阻断。这些约束已落实为测试，无须新增常驻 AGENTS、Rule 或 Hook 规则。
- 版本相关 fixture 应从当前版本推导目标，避免固定目标与后续正式版本碰撞。Windows 测试可执行文件仍被进程占用时，先等待旧进程退出再编译，不通过修改 ACL 或缓存策略处理文件锁。

## 用户验收（2026-10-25）

用户以 `$summary accept` 明确验收 TODO-034：骨架路径迁移分类修复、隔离下游累计迁移与 release 预览回归、必要验证及受管子包版本收口。TODO-034 更新为“已完成”。

验收复用上述既有证据：4 项定向回归、最终实际 CLI 集成用例、五组必需检查、独立静态复核，以及清单、dogfood、产物和版本收据核验。lifecycle 已从 0.1.0 升至 0.1.1；主程序仍为 7.0.0，其他包没有因本次修复重复升版，重复开发预览为 `nothing-to-close`。

本次 accept 仅更新本交付文档、TODO 总表与文档索引，并执行轻量文档检查，不重新构建或跑回归。验收不授权提交、推送或发布，也不授权修改真实下游或用户级安装；StratusAgent 的实际 release 和自然 Codex Hook 生命周期仍未验证。下述 TODO-035 为独立待确认事项，不否定已通过的分类修复验收。

## 验收核对新增事项 TODO-035

首次开发收口失败时，受管错误仅返回 `factory-unit: development check failed (exit 101, timeout false): error: test failed, to rerun pass -p bridgeforge-factory --lib` 及版本回滚信息，未给出失败用例或 panic。当前 `source/lifecycle/src/release/schema4/development.rs` 的 `run_check` 失败分支只拼接 stderr，Rust 测试的详细失败信息在 stdout 中；本次因此额外重跑整组 factory 测试取证，耗时 566.30 秒。

登记 TODO-035，状态“待确认”、归属“后续事项”：评估开发收口失败时保留并展示或定位 stdout 诊断，使失败用例可直接追查，避免为取证重跑整组测试。本轮未修改该实现。造成首次失败的固定版本测试已修正，完整受管收口随后通过，因此该诊断缺口不阻断 TODO-034 验收。
