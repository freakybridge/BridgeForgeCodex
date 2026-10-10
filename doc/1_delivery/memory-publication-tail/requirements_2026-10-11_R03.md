---
title: R03 Memory 上传收尾归一
lifecycle: completed
validation_status: verified
date: 2026-10-11
---

# R03 Memory 上传收尾归一

## 授权与边界

用户在本轮 R03 解释后回复“开始吧”，授权本地实现、必要验证、传播与文档，不提交、推送、发布或运行真实 Memory 同步。本轮 R03 区别于历史 R03 Memory CLI。

基准 HEAD 为 `1978b367de7376b34e644ea84d6985dea5d12f58`，保留已完成但未提交的 R01/R02。修改前 remote.rs、测试及 R02 prepared 存于忽略目录 `.runtime/R03-before/`，仅供本机审计。

仅抽取三条本地上传分支共同的：重新捕获发布快照 → push_snapshot → record_synced → 条件清 pending → 返回 push。无有效远端快照、首次对接空远端、仅本地变化的分支条件保持不变；恢复、合并、冲突、外层清理与授权检查不改变。错误、返回值、revision、远端 HEAD 约束、本地并发保护与副作用顺序不变；不删除上传前重新捕获快照的操作。

## 规模与预算

实现是局部抽取；因工厂强制完整工程检查与独立审计，按 M 核算：45 分钟 / 约 20k 新增 token（估算，未实测）/ 1 名 review-auditor / 最多 2 轮验证。沿用本会话预算口径，无适用周额度系数，不编造百分比。预计超额或必须改变既有同步语义时暂停受影响动作。

## 传播四问与执行

产品层通用能力，进入 templates/hooks，同步自身 dogfood、manifest、当前版本产物及架构说明；不对工厂执行下游 project-sync apply。将来显式 release 按 [product] 处理版本/CHANGELOG，本次不预写 VERSION。

1. 先补三个入口的成功/失败/新 pending 行为样本，在旧实现上运行。
2. 在 memory/remote.rs 增加一个私有上传收尾函数，三个分支调用；不引入通用状态机。
3. 复跑定向样本及现有 Memory remote 回归，同步受管资产。
4. 独立审计、完整工程检查与隔离准备，最终保存结果。

## 验收

- 三个入口成功后，远端快照、同步收据与 revision 一致，未变化的 pending 清除。
- push 失败时远端 HEAD、既有同步收据和 pending 保留。
- 上传成功但记录状态失败时，不清 pending，不返回成功。
- 上传期间新建 pending 时，旧请求成功不能清除新请求。
- 既有并发本地写入、合并和冲突回归继续通过。
- 测试只用本地隔离裸库和 fixture，不访问真实 GitHub、不读写用户原生 Memory 正文。

## 验证与经验

- 旧实现基线：`cargo test --locked --config scripts/tests/factory-cargo.toml --manifest-path .codex/hooks/Cargo.toml -p bridgeforge-core local_publication -- --test-threads=1`，1 项矩阵测试通过（3 个入口 × 4 模式 = 12 场景），32.39 秒。
- 新实现定向回归：同一入口过滤 `memory::remote::tests`，15/15 通过，88.42 秒；含行为矩阵与并发写入、冲突、恢复、字节保留等原有用例。
- 独立审计 `review-auditor:r01_audit` 对本轮 R03 返回 passed，无待修问题；确认故障注入有效、测试仅使用隔离本地 Git/Memory、源码镜像与传播完整，不复用 R01/R02 审计作为本轮结论。
- 当前版本受管 Cargo release build 与 `.codex/hooks/target/release/bridgeforge.exe build-assets --project-root .` 均通过，Hook/CLI 安装与自测完成，来源树哈希为 `sha256:e9642588c524e85296a9bdd77c48124d0997f86edce12fedc966c8beb0f4b699`。工作区版本仍为 2.2.0。
- 真实独立审计已绑定开发指纹 `sha256:73f3c24d69e05fb43214dad3e9e16e5dd7ab6b164eee29e9e1850e4497c2527e`。受管工程准备已完成，返回 prepared、checks_passed=7、隔离目标版本 2.2.1。真实 Memory/runtime 用户验收未执行。

本次经验：相同业务尾部可以集中维护，但分支条件、最初用于判断的快照和发布前重新捕获的快照应继续区分。失败发生在 push 前还是 push 后，决定可断言哪些远端/本地状态；测试必须同时检查原 pending 与期间到达的新 pending，不能只检查返回字符串。

## 最终工程证据

执行 `.codex/bin/bridgeforge.exe git-sync --prepare-release --message-file .runtime/R03-message.txt --audit-file .runtime/R03-audit.json`，覆盖当前 R01＋R02＋R03 工作区；未提交、推送、发布或运行真实 Memory 同步，未改写工作区 VERSION。

| 检查 | 本轮结果 |
|---|---|
| baseline、skill metadata、manifest --check、project structure | 全部通过 |
| 模板/dogfood 源码全树一致 | 1 项通过 |
| dogfood workspace | CLI 14、Core 215、Hook 22，共 251 项通过，5 项条件忽略；约 633 秒 |
| 完整工厂 fixture | 89 项通过，5 项条件忽略；约 372 秒 |
| R03 独立审计 | passed，无待修问题 |
| 隔离构建及准备 | prepared，7 项检查通过，目标 2.2.1；工作区仍为 2.2.0 |

本机收据为 `.runtime/bridgeforge-codex/release-preparation/current.json`。workspace 日志为 `sha256:5586a0dc599e6f65f1460b170409e73bd761f895002c4825c81304686ee9d16c`；factory 日志为 `sha256:a1fb1e6eb7709cfb6b4539fe3d50705625054356cec76a2d8ef5e7eab2e45aa7`，位于同目录 blobs。

workspace 条件忽略包含 4 个由父测试调用的子进程 helper/fixture 和显式 live log 检查；factory 条件忽略包含两项性能实验、原生 Codex 高成本提醒 smoke、进程 helper 和需单独授权来源的 Assist 迁移 fixture。不将条件忽略、隔离测试或本机准备收据当作真实 Memory/runtime 验收。

本轮在 45 分钟实施上限内完成，使用 1 名独立审计 Agent，无源码实质修复失败；token 未实测。工程验证完成，用户于 2026-10-11 通过 `$summary accept` 验收。

## 用户验收

2026-10-11，用户明确验收本会话 R01–R04。R03 的旧/新实现行为矩阵、Memory remote 回归、独立审计及累计工程检查满足约定，状态更新为 completed / verified；本次没有执行真实 Memory 同步，真实 runtime 用户验收不由隔离用例替代。累计验收与发布授权见 [R04 收口记录](../project-map-refresh/requirements_2026-10-11_R04.md#累计验收与发布授权)。
