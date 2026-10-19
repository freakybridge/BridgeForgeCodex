---
lifecycle: completed
validation_status: verified
---

# git-sync 三处重复流程优化

## 目标与授权

2026-10-19 用户明确要求按优先级优化：开发检查逐项恢复、发布分析复用、历史 Git 读取复用，并保留防漂移、事务回滚和收据校验、补齐回归。本轮禁止提交、推送或发布，真实下游不写入；保留此前新增 Rust 包修复的未提交改动。本卡承接 TODO-009，旧编排优化收据不冒充本轮验证。

M 级跨模块有界改动，主对话顺序实现，工厂要求一名 review-auditor 独立审计。用户先确认整体 120 分钟 / 约 35k token，真实跨调用预检发现遥测字段导致缓存失效后，又明确选择“追加预算，完成最终完整验证”，确认总计 180 分钟 / 约 45k token / 仍只用原一名 Agent / 最多两轮验证。预算包含所有已执行工作。token 未实测；本机暂定系数 0.152 个周额度百分点/10k token，对应总计约 0.7%，不作为实测扣减。

## 最终规则与传播

1. 检查复用显式 opt-in，逐项保存成功结果；绑定源码、配置、仓库和暂存区、有效 Git 配置、工具和环境，验证日志哈希。失败、超时、输入漂移不认证；无法可靠建模环境时继续正常执行。记录文档仅在两次调用之间允许轻量复核后复用，检查中漂移保护保留。
2. 同一次同步在 fetch 前规划、fetch 后核验输入；输入一致复用，变化重算。预览保持零写入，因此外部预览和实际执行仍分别计算。Rust 规划直接复用自身加载并验证的包集合，诊断入口保持独立校验。
3. 单次分析对不可变提交保存树和文件内容（含不存在），批量读取需要的 blobs，ownership 合同按状态一次解析。HEAD/工作区查询不缓存；异常批次不得降级为不存在或成功。
4. 本轮属于通用产品层，模板与 dogfood 同步；工厂专属检查配置仅修改自身 `.codex/development-checks.json`，不下沉。版本与 CHANGELOG 留给另获授权的受管发布事务，当前 VERSION 保持 2.5.1。

## 验证合同

| 范围 | 必须证明 |
|---|---|
| 检查恢复 | 后续失败后仅重跑失败项、重复准备复用、未 opt-in 仍执行、损坏日志重跑、源码/暂存区/环境/命令变化失效 |
| 计划复用 | 无变化只规划一次、fetch 后变化重算、重复发布不重复升版、普通同步不升版 |
| 历史读取 | 多文件一批、重复读取无额外 Git、二进制内容与不存在语义、可变 HEAD 实时读取、超时/损坏批次阻断 |
| 原有保护 | 预览零写入、锁文件精确更新、执行失败回滚、Hook 改写阻断、收据与终态一致 |
| 工厂收口 | 镜像、manifest、metadata、baseline、结构、完整 workspace/fixture、独立审计及受管准备 |

## 当前记录

源码实现和独立审计已完成。定向验证覆盖检查恢复/未 opt-in/工具变化/损坏日志/暂存区与源码变化、真实 attributes 规则变化后准确失败、计划复用/fetch 内容与 replacement 历史变化、快照无法建立的保守降级、历史批量读取/异常/可变 HEAD、submodule 与无关大二进制，以及既有 Hook 回滚与产物复用。最新源码的完整工程检查与真实重复准备结果见“最终验证”；本次优化已由用户验收，正式发布以受管同步最终收据为准，真实同步端到端提速仍需单独验证。

独立 review-auditor `/root/audit_additions` 已确认缓存身份、历史解释状态、attributes 内容、按需批量读取、setup 结束 guard、原始路径字节语义和可选快照降级，最终无未关闭源码发现。审计不代替测试。

环境记录：sandbox 临时 `.git/config` 写入受限，同一受管测试窄范围提权后通过。开发期间一次链接目标仍在测试运行而被 Windows 占用，待原测试退出后重试通过。一次在途构建受审计补修影响，源码漂移 guard 正确阻断。直接从已安装 CLI 调用覆盖自身的 build-assets 安装阶段发生 os error 5，事务回滚；最终改用已完成受管 workspace 构建的 CLI 启动同一安装事务，不改 ACL、不杀进程、不手工复制二进制。

## 首轮完整验证与跨调用修正

首轮完整 workspace 为 337 通过、8 跳过，fixture 为 90 通过、6 跳过，合计 427 通过、14 跳过、0 失败；七项检查完成并生成隔离目标 2.5.2 的 prepared 收据，根 VERSION 仍 2.5.1。workspace 日志 SHA 为 `db492051f432feeaa8e21a73f68a66302dfdcb7d2597a3612cbb5dc8993fa657`，fixture 日志 SHA 为 `5d8fc731d26e93d4b956f0777c15cbc453e7ec50e90a333f1c27a5564b021e4e`。该轮指纹为 `sha256:0636ec4e8f37091889d6bfda9e70a91e841c3157fa69c11a129c4e774497859f`。

跨调用重复准备前的状态核验却返回 environment changed，因此没有重启长检查。相邻两次工具调用对环境变量值仅保存 SHA-256 摘要并比较，实际变化字段为 `CODEX_TOOL_CALL_ID`。修正只排除此逐调用追踪号，不排除其他 CODEX 字段或业务变量；使用该追踪号作为业务输入的检查不得 opt-in。新增真实 CLI 跨进程回归验证追踪号变化复用、业务环境变化重跑、目标版本与工作区版本正确。独立审计已复核此修正及测试断言。

此次源码变化后，首轮结果仅保留为修正前证据，没有直接沿用旧 prepared 或手工改写缓存指纹；第二轮实际结果见下节。

修正版真实跨进程定向回归已通过：`cargo test --locked --manifest-path scripts/tests/Cargo.toml real_cli_check_reuse_ignores_invocation_trace_but_binds_business_environment -- --test-threads=1`，1 项通过；同一夹具四次真实 CLI 调用分别得到执行、复用、业务环境改变后执行、再次复用，目标始终 1.0.1，工作区 VERSION 始终 1.0.0。修正版源码审计通过，当前指纹为 `sha256:2ff2f3b05c43c8c6b709360c997772a310d1649c2b670851490fe9a920310181`；第二轮完整准备启动前冻结源码与非记录文档。

## 最终验证

| 证据 | 修正版实际结果 |
|---|---|
| 七项受管检查 | baseline、metadata、manifest、structure、mirror-drift、dogfood-tests、factory-fixtures 全部通过 |
| 完整 workspace | 337 通过、8 按既有设置跳过、0 失败；2011884 ms |
| 完整 factory fixture | 91 通过、6 按既有设置跳过、0 失败；1199825 ms；包含新的真实跨进程环境回归 |
| 测试总计 | 428 通过、14 跳过、0 失败；跳过项不计作已执行 |
| 独立源码审计 | 原同一 review-auditor 最终 passed，无未关闭发现；审计与测试证据分别记录 |
| 冷准备 | prepared，目标 2.5.2，checks_executed=7、checks_reused=0 |
| 真实工厂重复准备 | prepared，checks_executed=0、checks_reused=7；入口实测 13665 ms；目标产物哈希与冷准备相同 |
| 开发交付时产品版本 | 根 VERSION 为 2.5.1；VERSION、CHANGELOG 未产生开发阶段预写版本变更 |

workspace 日志 SHA：`b5084339fedf6af0bb2bb58400dd9827958951cd9761696370c27fe59ec49eeb`；fixture 日志 SHA：`b9e184f2ec3cf80bd96ed46ade484f6adbbf29542ce7709eb9e6fb6480849416`。日志位于 `.runtime/bridgeforge-codex/release-preparation/blobs/`。最终指纹仍为 `sha256:2ff2f3b05c43c8c6b709360c997772a310d1649c2b670851490fe9a920310181`。

本机证据：`.runtime/workflow-reuse-cold.json` 保存修正版冷准备，`.runtime/workflow-reuse-warm.json` 保存实际重复准备输出和入口计时，`.runtime/bridgeforge-codex/release-preparation/current.json` 为当前 prepared；`.runtime/workflow-reuse-audit.json` 记录真实源码审计。当前版本 CLI/Hook 已通过受管构建安装，源树 SHA 为 `55f5d495a60d2bfd35f979586a65baeaceb9515652066594700ad6e91586361e`，CLI 自检包含 `git-sync-analysis-reuse-v1`；隔离目标 2.5.2 尚未发布。

复用是同源码、命令配置、Git 状态及可建模环境下的结果复用。13.7 秒为重复准备 CLI 入口时间，不是用户指令到最终回复的总耗时，也不是一次真实 Git 同步的提速对照。普通同步仍不升版，开发准备不是同步/发布的前置；开发阶段没有真实仓库提交、推送、发布或下游写入。真实下游与用户级分发、自然 Hook 运行、真实同步端到端提速尚未验证。

## 用户验收与发布授权2026-10-19

用户明确调用 `$summary accept`，验收本卡三处重复流程优化及上述工程证据；随后明确调用 `$git-sync release`，授权受管入口对当前累计改动执行预览、升版、提交和推送，不授权写入真实下游或安装用户级 Skill。

本卡开发交付完成。关联 TODO-009 的原始范围还要求按同一口径实测用户指令到最终回复的真实同步耗时，本次 13.7 秒只证明重复准备入口的检查复用，因此 TODO-009 更新为待验证而非整体已完成。TODO-010 的真实下游适配与发布边界不由本卡验收结算。正式版本、commit、推送目标和同步终态以本次受管 release 最终收据为准，不预写成功结果。
