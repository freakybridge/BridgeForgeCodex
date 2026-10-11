---
title: R05 项目同步内容合并职责拆分
lifecycle: completed
validation_status: verified
date: 2026-10-11
---

# R05 项目同步内容合并职责拆分

## 需求与授权

用户在 refactor-scan 中讨论 R05，要求实施计划后明确回复“开始吧”。授权范围为 R05 本地实现、受管传播、必要测试、独立审计和工程收口；不提交、推送、发布或写入真实下游，不包含 R06。

开工基准 `97a7624dd8410e137e7f6671ee49a28a0adc2ead`，工作区干净，VERSION 为 2.2.1。

## 目标与保留合同

- 在 project_sync 内部新增 content 模块，集中内容合并与策略分发；继续复用 managed_markdown 基础解析。
- 原模块保留路径检查、文件读取、迁移输入选择、计划、指纹、确认、锁、写入、回滚和版本戳。
- 内容模块只接收已有字节与规则及项目路径信息，不读写文件，不反向调用事务。
- 保持 CLI、公开 API、schema、策略优先级、输出字节、错误文本和出现顺序。
- 迁移输入优先于磁盘内容，但 render_asset 返回的 before 仍为磁盘原始字节。
- 同一输入的内容结果保持一致；新源码引起的受管构建来源哈希变化属于正常传播。

## 规模与预算

M：45 分钟 / 约 20k 新增 token（估算，未实测）/ 最多 1 名 review-auditor / 最多 2 轮验证，包含基线、传播、完整工厂检查和受管准备。沿用 confirm 默认口径，预计超额时暂停超额动作。

本机未发现适用的周额度校准文件；不编造周额度百分比，不以缺少换算系数重复索取已明确的实施授权。

## 传播四问

属于产品层通用能力，事实源为 templates/hooks；需同步自身 dogfood 与受管 manifest。将来显式 release 按 [product] 更新版本及 CHANGELOG；本轮不预写 VERSION，不对工厂运行下游 project-sync apply，不写骨架版本戳。

## 实施与验收计划

1. P0：在旧实现固定策略、错误、迁移覆盖与 before 快照样本。
2. P1：将内容函数和策略分发迁入 project_sync/content.rs，迁移相应单元测试，公开接口不变。
3. P1：核验普通资产规划及迁移两条调用路径，移除旧实现。
4. P1：同步 dogfood、manifest、当前版本产物，运行相同定向样本。
5. P2：按 development-checks.json 完成镜像、workspace、完整 fixture 及其他必需检查，独立审计后取得准备收据。

必须覆盖 AGENTS 项目区保留、Markdown 合并及换行、Hook 注册、JSON 合并、seed/whole/region/merge、迁移输入、幂等性、失败回滚、漂移阻断及版本戳最后写入。

真实下游与 runtime 用户路径未验证；工程验证不替代用户验收。若必须改变内容规则、错误语义或事务行为，则暂停该部分单独说明。

## 实施与验证记录

1. 旧实现行为基线：受管 Cargo 入口过滤 `r05_`，3/3 通过，覆盖策略字节、迁移优先级与磁盘 before、特殊策略优先级及失败顺序。默认沙箱拒绝 Cargo 锁写入，原命令窄范围提权后通过。
2. 已抽出 content 内部模块；只有 render 对父模块可见，内容辅助函数保持私有。迁移 8 个既有内容单测，保留集成与事务测试，并新增上述 3 个边界样本。
3. 首次编译发现遗漏迁移的 Markdown 测试仍引用原模块私有函数；将其迁入内容测试模块后，同一受管入口过滤 `project_sync::`，44/44 通过（测试执行 2.10 秒）。未修改业务规则或扩大辅助函数可见性。
4. 源码已同步 dogfood，受管 `manifest --root .` 已登记新模块，`manifest --check` 返回 changed=false；`check factory-version` 健康，VERSION 仍为 2.2.1。
5. `cargo build --locked --release --manifest-path .codex/hooks/Cargo.toml` 通过（49.41 秒）；从构建目录运行 `bridgeforge.exe build-assets --project-root .` 返回 built，Hook 与 CLI 自测和来源收据齐全，源码树哈希为 `sha256:2871294fe8f405f143f439bc03f913e2ad42405ed3316aa8d5c31464a48b76ea`。
6. 独立 `review-auditor:/root/r05_audit` 无发现：辅助函数与原实现逐字一致、事务未变、旧测试完整保留、新增样本有效、模板与 dogfood 和 manifest 一致。绑定开发指纹 `sha256:2e316e66038b5dc5ff053f2a835d02c13167e1d0d7b02217d91469d8d6743016`，审计记录 `.runtime/R05-audit.json`。
7. 受管 `git-sync --prepare-release --message-file .runtime/R05-message.txt --audit-file .runtime/R05-audit.json` 返回 `prepared`，`checks_passed=7`，指纹与审计一致；隔离目标版本 2.2.2，工作区 VERSION 保持 2.2.1。没有暂存、提交、推送或发布。

定向命令：`cargo test --locked --config scripts/tests/factory-cargo.toml --manifest-path .codex/hooks/Cargo.toml -p bridgeforge-core project_sync:: -- --test-threads=1`。

## 最终工程证据

配置唯一入口为 `.codex/development-checks.json`，实际命令由受管准备执行。收据位于本机 `.runtime/bridgeforge-codex/release-preparation/current.json`，日志位于其同级 blobs 目录，不能作为另一台机器的 runtime 验收证据。

| 检查 | 实际结果 |
|---|---|
| baseline、Skill 元数据、manifest、项目结构 | 全部通过；结构仅含已有归档候选提示 |
| 模板与 dogfood 源码全树一致 | 1 项通过 |
| dogfood 完整 workspace | CLI 14、Core 218、Hook 23，共 255 项通过，0 失败；5 项条件忽略；检查总耗时 648.099 秒 |
| 完整工厂 fixture | 89 项通过，0 失败，6 项条件忽略；检查总耗时 402.937 秒 |
| 独立审计 | passed，无待修复发现 |
| 受管准备与隔离目标构建 | prepared，目标 2.2.2；7 项检查均 exit_code=0 |

workspace 日志：`sha256:a2aaf15c03d244d626507f1e4a351362ae056406e9ec5abbcc56b9536e1cf97f`。5 项忽略包含 4 个由父测试调用的子进程辅助入口，以及需显式提供现场日志的检查，不将忽略项计作通过。

fixture 日志：`sha256:d3012cc4ddf0d93674ffbd321fe6512092eb7b980fb7bee19de3bfadb295c3f4`。6 项忽略包含 3 个显式性能实验、原生 Stop smoke、子进程辅助入口和需授权来源的 Assist 迁移 fixture。R05 不承诺性能收益，也未执行这些条件项。

本轮在 45 分钟实施上限内完成，使用 1 名独立审计 Agent；修正一次测试归属后的检查均通过，token 未实测。实现、产品源码传播、dogfood、自动 fixture 和构建已完成；真实下游与 runtime 用户路径未验证。用户验收见下节。

## 经验记录

本轮按 summary 流程将经验保留在同一交付卡，不新增平行事实源。搬移私有职责时应同时检查直接调用辅助函数的测试；纯内容样本归入子模块测试，集成测试通过新的内容入口验证，避免仅为测试扩大生产函数可见性。原文件快照和参与内容计算的迁移输入不是同一对象，必须分开保留。

该经验由现有修改前核对调用方与等价验证约束覆盖，不新增 AGENTS、Rule、Hook 或 Memory 规则。

## 用户验收与发布授权

2026-10-11，用户明确表示“本轮优化都可以收口了”，调用 `$summary accept` 并同时调用 `$git-sync release`。R05 按内容合并职责拆分、行为保持、工程验证和独立审计的约定范围验收，状态更新为 completed / verified。

R06 后续只增加基准测试与记录，未修改 R05 实现；累计工作区再次通过 7 项工程检查，开发指纹为 `sha256:2469b6dc1cf9f8c82358db1c5bc01c6ada869335eb127d3b26d71639f8775025`，证据见 [R06 工程收口](../memory-snapshot-baseline/requirements_2026-10-11_R06.md#工程收口)。真实下游与 runtime 未验证的边界保留，不因本次验收扩大。

本次 release 授权覆盖 R05、R06 基准和本次验收记录；提交、版本与推送结果以受管 release 的实际收据为准，不将此前准备目标当作已经发布。
