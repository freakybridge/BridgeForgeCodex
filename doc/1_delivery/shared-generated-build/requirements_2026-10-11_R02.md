---
title: R02 共享构建能力独立
lifecycle: completed
validation_status: verified
date: 2026-10-11
---

# R02 共享构建能力独立

## 授权与当前基准

用户在本轮 R02 解释及计划后回复“开始吧”，授权本地实施、必要传播、验证与文档；不提交、推送、发布或修改真实下游。本轮 R02 与历史 refactor-scan 中的 R02 公共持久化不同。

HEAD 为 `1978b367de7376b34e644ea84d6985dea5d12f58`，工作区已有本会话 R01 改动，必须保留。R02 修改前的相关源码及 R01 准备记录保存在忽略目录 `.runtime/R02-before/`，仅供本机差异审计，不是第二份事实源。

## 目标与保留合同

- 将 BuildInputs 从 project_sync 子模块提升为 core 内部直属模块，保持原文件和实现。
- generated_assets 承接隔离构建、自测、产物字节与收据生成、临时构建目录清理。
- managed_paths 承接原 safe_join，保留语义，不合并其他不同策略的路径校验。
- project_sync 继续负责项目锁、计划、安装、版本戳、回滚；Git 同步保留缓存选择和提交事务；project_hooks 保留配置及兼容构建。
- 保持 CLI、公共 API、source_root/target_source_root、Cargo 参数、超时、验证顺序、错误文本、收据 schema 和清理语义。
- 构建模块允许写隔离目录，不安装到项目目标，不改变 Git、版本或项目锁策略。
- 不包含 R03/R04、缓存扩展、性能优化、安装故障修复或新的安全规则。

## 规模、预算与停止点

M；主对话实现，1 名 review-auditor；45 分钟 / 约 20k 新增 token（估算，未实测）/ 最多 2 轮验证。包含全量工程检查、受管构建、审计和合理重试。沿用本会话预算口径；没有适用周额度系数，不编造百分比。预计超额或需要改变保留合同时，暂停受影响动作并报告。

## 传播四问

属于产品层通用构建能力，事实源为 templates/hooks。同步自身 dogfood、受管 manifest 与相关设计说明。将来显式 release 时按 [product] 处理版本和 CHANGELOG；本次不预写 VERSION，不对工厂执行下游 project-sync apply。

## 分步计划与验收

1. P0：复用现有 build_provenance 测试取得旧实现基线，核实三个直接调用路径。
2. P1：机械抽取 generated_writes、构建专用 JSON 比较及原 safe_join；原算法不变。
3. P1：切换 project_sync、git_sync_plan、project_hooks，移除旧归属与反向依赖。
4. P1：同步 dogfood、manifest 和当前版本产物；补设计说明。
5. P2：定向回归、独立审计、全树镜像检查、完整 workspace / factory fixture、受管准备及最终收据核验。

验收覆盖：双产物共享依赖目录但分别验证；错误配方/锁文件/源码漂移阻断；成功 Cargo 无新产物仍失败；自测期间漂移阻断；缓存命中/未命中及提交拒绝保持原恢复行为；项目 Hook 旧格式兼容路径继续工作。既有 build_provenance、core_git_sync、core_project_hooks 是主要验证入口。

## 验证与经验

- 旧实现：`cargo test --locked --manifest-path scripts/tests/Cargo.toml build_provenance -- --test-threads=1`。普通沙箱 7 通过 / 5 失败，错误涉及文件替换 os error 5；同一命令窄范围提权后 12/12 通过，未修改源码。
- 新实现：同一 build_provenance 命令 12/12 通过；`cargo test --locked --config scripts/tests/factory-cargo.toml --manifest-path .codex/hooks/Cargo.toml -p bridgeforge-core project_hooks -- --test-threads=1`，15/15 通过。
- generated_writes 在统一源码换行后与 R02 前函数文本完全一致；BuildInputs 文件内容未改变，safe_join 与构建专用 JSON 比较仅迁移归属。共享模块已无 project_sync 反向引用。
- core_project_sync 测试显式导入 ProcessRequest，避免继续依赖已移出的生产模块导入。现有跨入口回归覆盖此次纯移动风险，未新增重复测试。
- 独立审计 `review-auditor:r01_audit` 对本轮 R02 返回 passed：独立比较 R02 前快照，确认函数等价、无反向依赖、调用方事务职责保留、7 个相关源码的模板/dogfood SHA256 一致，未发现问题。未复用 R01 审计结论代替本轮审计。
- 当前版本 `cargo build --locked --release --manifest-path .codex/hooks/Cargo.toml` 与 `.codex/hooks/target/release/bridgeforge.exe build-assets --project-root .` 均已通过；Hook/CLI 已受管安装并完成自测，二者的 source_tree_sha256 均为 `sha256:78f9d66a7ac4abe8c3a4157f1e5dbabcddfd975b23780a93fe2a633505aa80d9`。工作区版本仍为 2.2.0。
- 开发指纹为 `sha256:85aed057ed3a318cb04c2f61236cd3660988c841e01fa238223e33b268d029fb`，真实审计结果已绑定该指纹。受管准备返回 prepared、checks_passed=7，隔离目标版本为 2.2.1；工作区仍为 2.2.0。真实下游/runtime 用户验收未执行。
- 不将 R01 成果归为 R02；新开发输入使旧 prepared 失效后，必须生成覆盖当前工作区的新准备记录。

职责拆分应以真实消费者为边界：输入快照和构建属于共享能力，缓存选择、安装事务与 Hook 配置继续由调用方拥有。文件长度缩短并非验收标准；返回结果、错误和副作用顺序一致，且依赖方向不回流，才证明本次职责调整成立。

## 最终工程证据

执行 `.codex/bin/bridgeforge.exe git-sync --prepare-release --message-file .runtime/R02-message.txt --audit-file .runtime/R02-audit.json`，覆盖当前 R01＋R02 工作区；没有提交、推送、发布或改写工作区 VERSION。

| 检查 | 本轮结果 |
|---|---|
| baseline、skill metadata、manifest --check、project structure | 全部通过 |
| 源码镜像全树一致 | 1 项通过 |
| dogfood workspace | CLI 14、Core 214、Hook 22，共 250 项通过，5 项条件忽略 |
| 完整工厂 fixture | 89 项通过，5 项条件忽略 |
| R02 独立审计 | passed，无待修问题 |
| 隔离构建与准备 | prepared，7 项检查通过，目标 2.2.1，仅保存准备产物 |

收据为本机 `.runtime/bridgeforge-codex/release-preparation/current.json`。workspace 日志为 `sha256:e988d2b4a58162bb9bc9dfb9486fbff028102936396928c0d43a25ac7c05eceb`，factory 日志为 `sha256:aba527b50be904b665393ef540ae1f43d01ce230150db2c01aeef06436ca7a4b`，均保存在同目录 blobs；这些本机记录不等于真实下游/runtime 验收。

workspace 条件忽略包括 4 个子进程 helper/fixture 和显式 live log 检查；factory 条件忽略包括两项性能实验、原生 Codex 高成本提醒 smoke、进程 helper 和需单独授权来源的 Assist 迁移 fixture，不把条件忽略计为通过。

本轮在 45 分钟实施上限内完成，使用 1 名独立审计 Agent，无源码实质修复失败；token 未实测。已完成工程验证，用户于 2026-10-11 通过 `$summary accept` 验收。

## 用户验收

2026-10-11，用户明确验收本会话 R01–R04。R02 的构建来源、项目 Hook 定向回归、独立审计和累计工程检查满足约定，状态更新为 completed / verified；真实下游及 runtime 用户路径未验证的边界保留。累计验收与发布授权见 [R04 收口记录](../project-map-refresh/requirements_2026-10-11_R04.md#累计验收与发布授权)。
