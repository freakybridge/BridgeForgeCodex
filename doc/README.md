---
delivery_layout: flat
---

# bridgeforge-codex Documents

本仓库采用 bridgeforge-codex 五层文档体系。`1_delivery/` 采用扁平布局：每个 topic 直接位于该目录下；若未来交付规模需要里程碑，可改为 `milestone` 并迁入 `M1/<topic>/`。

## 从这里开始

| 你要做什么 | 先读这里 |
|---|---|
| 当前架构 | [`0_architecture/`](0_architecture/)；项目必须在该目录索引当前有效的架构资料 |
| 当前交付 | [`1_delivery/`](1_delivery/)；从 `lifecycle: active` 的交付包继续 |
| 开放 Bug | [`2_bugs/`](2_bugs/)；只把尚未归档的问题作为当前问题 |
| 项目操作 | 根 [`AGENTS.md`](../AGENTS.md) 和 [`3_reference/`](3_reference/) |
| 历史记录 | [`4_archive/`](4_archive/)；历史材料不作为当前运行合同 |
| 本工厂当前架构 | [`design-rationale.md`](0_architecture/design/design-rationale.md) 和 [`codex-native-instruction-architecture.md`](0_architecture/design/codex-native-instruction-architecture.md) |
| 本工厂当前重构 | [`BUG-agents-ia`](2_bugs/BUG-agents-ia/README.md) |
| 本工厂操作手册 | [`INSTALL.md`](../INSTALL.md) 和 [`codex-project-operating-guide.md`](3_reference/codex-project-operating-guide.md) |

本节只维护第一阅读路径；每个事项的详细状态仍由目标文档单独维护。

## 文档生命周期

需求卡和 Bug 使用 `lifecycle` 表示是否仍属当前工作，使用 `validation_status` 表示验证进度：

| 字段 | 允许值 | 含义 |
|---|---|---|
| `lifecycle` | `active` / `completed` / `superseded` / `archived` | 当前推进、已完成、已被替代、已移入 `4_archive/` |
| `validation_status` | `not_started` / `in_progress` / `awaiting_validation` / `awaiting_user_acceptance` / `verified` | 验证阶段；具体缺口写在正文，不扩展状态词 |

- 新需求卡必须以 `lifecycle: active`、`validation_status: not_started` 开始。
- `completed` 必须已经满足验收条件并取得用户验收；`superseded` 必须同时写 `superseded_by`。
- 只有 `$archive-scan` 在用户确认移动后才能写 `archived`；只有 `active` 事项进入当前交付导航。
- 迁移前的 `status` 或正文状态只作为历史证据；缺少 `lifecycle` 的事项视为 `unclassified`，禁止自动当作 active 或 completed。

## 索引

| 目录 | 作用 | 当前内容 |
|---|---|---|
| `0_architecture/` | 架构与设计依据 | `design/` |
| `1_delivery/` | 需求确认、计划、验收、协作与专题讨论 | 见下方 topic 索引 |
| `2_bugs/` | 已知故障及其修复记录 | 27 条故障记录 |
| `3_reference/` | 外部资料与可复用参考实现 | `examples/antifab-deny-hook.py` |
| `4_archive/` | 已完成或已失效的历史材料 | 既有历史档案；后续按 `delivery/`、`bugs/` 分类归档 |

## 架构

- 设计资料：[`0_architecture/design/`](0_architecture/design/)，包括 [`codex-project-sync.md`](0_architecture/design/codex-project-sync.md)、[`codex-native-instruction-architecture.md`](0_architecture/design/codex-native-instruction-architecture.md)、[`codex-native-memory-sync.md`](0_architecture/design/codex-native-memory-sync.md)、[`user-facing-result-contract.md`](0_architecture/design/user-facing-result-contract.md)、`design-rationale.md` 与上游同步 playbook。
- 操作参考：[`codex-project-operating-guide.md`](3_reference/codex-project-operating-guide.md) 与主动澄清参考 [`codex-hook-signals.md`](3_reference/codex-hook-signals.md)；自动 Clarify / Focus Hook 已退役。
- `$bridgeforge-codex` 的运行手册属于产品源码，位于 [`skills/bridgeforge-codex/references/`](../skills/bridgeforge-codex/references/)，不纳入 `doc/`。

## Delivery topic

| Topic | 主要记录 |
|---|---|
| `agents-md-simplification-review` | AGENTS.md 优化草案的确认卡、11 版双 agent 辩论、V11 中断现场、有限验收边界及“根级目录路由 + 工厂嵌套指令 + 硬闸”的后续决定（2026-08-27、2026-08-28） |
| `hook-utf8-memory-loading` | Windows GBK 环境下由公共 Hook dispatcher 强制 UTF-8，恢复项目 memory 的 SessionStart 加载，并以四个真实下游隔离 worktree 验证（2026-08-26） |
| `bridgeforge-codex-batch` | BridgeForgeCodex 仓库专属的下游骨架批量升级、Git 同步、异常隔离与全量重跑；新增 1.5.6 Git 环境污染和 Batch 状态机死锁的修复、真实恢复需求（2026-08-21、2026-08-27） |
| `bridgeforge-home-layout` | 用户级目录演进 |
| `bridgeforge-codex-naming-contract` | 用户命令、菜单、聊天显示、GitHub identity 与内部技术标识的分层命名契约（2026-08-17） |
| `bridgeforge-latency-optimization` | 用户级 sparse canonical fast path、项目 planner 去重、并行终态验证与阶段计时收据（2026-08-15） |
| `bridgeforge-actionable-readiness` | 双状态更新结果、可执行完善清单、程序推荐与用户自定义部分确认，并保持整体 0/1 次确认（2026-08-15） |
| `bridgeforge-upstream-absorption-modes` | A 激进吸收、B 温和自定义、C 保守停止的单卡上游受管区块吸收契约（2026-08-15） |
| `bridgeforge-keyed-index-merge` | 规则与目录索引按稳定键合并，防止 A 激进吸收删除下游独有条目（2026-08-16） |
| `bridgeforge-managed-rule-safety` | 高定制 Rule 的 project-owned/preserve ownership、缺失标题 fail-closed、fenced Markdown 解析与事务回滚（2026-08-16） |
| `bridgeforge-update-strategy` | 旧 AGENTS 分类、清理、精确去重与项目专区合并，以及 `$git-sync` 合法 no-op 分类（2026-08-18） |
| `target-cleanup-retirement` | 核心 Rust/Cargo `target_cleanup.py` 退役、官方旧副本受控删除与修改版项目所有权保留（2026-08-18） |
| `native-memory-durable-consent` | Native Memory 首次长期授权、双向自动同步、本地 hook 自维护与远端 reconcile 职责解耦（2026-08-18） |
| `project-sync-release-preflight` | 项目同步与 `$git-sync` 共用 transition proof，并在写骨架版本戳前完成只读 release preflight（2026-08-19） |
| `project-sync-schema-v1-lineage` | 可信 schema v1 HEAD 先验证历史 hash 与旧戳，再确定性映射到 schema v2 稳定资产（2026-08-19） |
| `project-sync-single-release-standard` | Planner、Apply 与 `$git-sync` 的骨架 transition 直接调用同一验收函数，消除 plan `ready` / apply `blocked` 双标准（2026-08-19） |
| `project-sync-versioned-region-lineage` | pre-commit region 退役历史 marker/hash，只保留当前规则并要求旧项目显式适配（2026-08-19） |
| `project-sync-agents-zones-single-rule` | 根 AGENTS 退役旧标题 projection，只保留 agents_zones 并要求旧项目显式适配（2026-08-19） |
| `project-sync-hooks-zones-single-rule` | 用户级与项目级 hooks 统一为 managed/external 逻辑分区，受管 handler 唯一、canonical、独占 group（2026-08-19） |
| `project-venv-hook-runtime-single-rule` | 所有骨架与用户级 Native Memory Hook 统一使用当前项目 CPython 3.11+ `.venv`，并为用户级 Hooks repair 增加锁与 CAS（2026-08-19） |
| `project-sync-explicit-adaptation-transaction` | 精确选择 G 项并以 fingerprint 和一次性本地收据贯通 project-sync Apply 与后续 `$git-sync`（2026-08-19） |
| `project-sync-four-project-zero-blocker-rollout` | 闭合 #1～#9 后按共享 Git 与 Native Memory 串行约束推进四项目骨架更新，并记录 M2/Causis 现场补出的 1.4.24～1.4.26 三态凭证与全程收据（2026-08-20） |
| `cba-bridgeforge-1-4-26-clean-reinstall` | ClaudeBridgeAssist 以项目保留清单保存 vault hooks、skills、memory、映射和 AGENTS 项目区，放弃旧骨架谱系并一次性干净安装 BridgeForge 1.4.26（2026-08-20） |
| `bridgeforge-1-4-28-clean-baseline` | 全仓删除仅服务 1.4.28 以前的历史兼容层，以 `PreservationManifest` 破坏性重建和 1.4.28+ current-only 本地基线建立长期恒定体积的干净骨架（2026-08-20） |
| `bridgeforge-1-4-29-cleanliness-audit` | 完成 1.4.29 洁净审计与架构 debate，以及 1.4.30 两期核心安全修复和 current-only 整体洁净清理（2026-08-21） |
| `bridgeforge-1-4-31-clean-baseline` | 将最低清洁基线提升到 1.4.31：旧项目通过一次性 `PreservationManifest` 破坏性重装，1.4.31+ 保持 schema 3 current-only 更新（2026-08-21） |
| `user-skill-compatibility-retirement` | 彻底退役旧 `$bridgeforge` 用户级兼容 manifest、过渡入口与旧资产迁移链，只保留正式 active Skill 清单（2026-08-19） |
| `bridgeforge-single-confirmation` | `init`、`adopt`、`update`、`switch` 的零确认安全路径、单次风险确认与 Codex 窄权限规则需求（2026-08-15） |
| `bridgeforge-repository-structure-governance` | 文档五层结构机器检查、BridgeForge 测试迁入 `scripts/tests/**`，以及下游旧测试目录只报告不自动迁移（2026-08-16） |
| `bridgeforgecodex-codex-only-rebrand` | BridgeForgeCodex 1.0.0：彻底退役 Claude、删除 switch/parity、全技术标识改名、`0.86.0+` Codex 直升与最终仓库改名（2026-08-16） |
| `template-root-flattening` | Codex-only 模板从 `templates/codex/**` 提升为 `templates/**`，保留 `0.86.0+` 历史 lineage 并同步同步器、schema、dogfood 与测试（2026-08-16） |
| `factory-template-dogfood-contract` | Template 公共 AGENTS/Rules 单一事实源、bridgeforge-codex 项目定制区与工厂 Overlay，以及编辑/提交/发布防漂移硬闸（2026-08-16） |
| `codex-rule-runtime-simplification` | 退役未实际加载的 Markdown path rule，将有效红线无损迁入原生 AGENTS、hook、skill 与文档，并安全保留下游定制（2026-08-16） |
| `codex-agents-structure-reorganization` | Codex AGENTS 信息架构重组、历史标题安全迁移、项目必填区双状态硬闸与 `ctx-budget` 完整退役（2026-08-16） |
| `shared-skill-model-inheritance` | 删除 shared skill 的 Claude 专用 `model:` 覆盖，双宿主统一继承当前会话模型，并合并精简 Codex 模板的模型选择与执行分工说明（2026-08-16） |
| `confirm-workflow`、`develop-demand-discovery`、`explain-skill` | 需求确认与通用 skill 演进；后两者含 `research/` |
| `cross-project-write-guard`、`non-ascii-shell-guard` | 安全防护；后者新增 [memory writer stdin 编码旁路报告](1_delivery/non-ascii-shell-guard/research/2026-08-04_memory-writer-stdin-encoding-bypass-report.md) |
| `ctx-management` | 上下文治理；Stall Warning 已裁定从双宿主骨架及下游更新中移除（2026-07-30） |
| `doc-unification`、`document-lifecycle` | 文档体系演进 |
| `git-sync-latency-optimization` | `$git-sync` 单脚本直跑、失败前置、重复重建消除与完整同步收据（2026-08-01） |
| `git-sync-version-automation` | BridgeForge 与下游项目双版本域的 `$git-sync` 自动 bump、原生字段同步与统一 CHANGELOG 需求；新增 [下游项目 Rule 被误判为受管骨架的所有权缺口报告](1_delivery/git-sync-version-automation/research/2026-08-12_downstream-rule-managed-skeleton-boundary-gap.md)（2026-08-12） |
| `memory-rule-organization` | 双 Memory 交付历史；项目 `.codex/memory/` 部分已被 `project-memory-retirement` 替代，当前只保留原生 Memory 同步事实并指向 `codex-native-memory-sync.md`（2026-08-30） |
| `project-memory-retirement` | 退役项目 `.codex/memory/`，为候选工程与未来下游建立程序扫描、Agent 语义审核、用户逐项确认、受控迁移及独立清理授权（2026-08-30） |
| `shared-skill-distribution` | 用户级 shared skill 分发 |
| `skill-runtime-efficiency` | 非根 skill 的确定性 fast path、重复 agent/索引消除与高频 Git 单进程优化（2026-08-15） |

每个 topic 内以 `requirements_*.md` 保存确认卡；实现计划、验收方案、协作记录和正式讨论分别与该确认卡同域保存。仅 topic 内路径可作为该事项的工作上下文。

## Bug records

- [`BUG-agents-ia/`](2_bugs/BUG-agents-ia/README.md)（当前状态与验证收据见该总账；IA-14 用户与 runtime 验收见 [`IA-14-user-runtime-acceptance.md`](2_bugs/BUG-agents-ia/IA-14-user-runtime-acceptance.md)）
- [`BUG-codex-backend-unexpected-control-exit.md`](2_bugs/BUG-codex-backend-unexpected-control-exit.md)
- [`BUG-project-memory-sessionstart-gbk-output.md`](2_bugs/BUG-project-memory-sessionstart-gbk-output.md)
- [`BUG-windows-codex-hooks-open-visible-terminal.md`](2_bugs/BUG-windows-codex-hooks-open-visible-terminal.md)
- [`BUG-codex-desktop-native-memory-powershell-hook-not-entering-python.md`](2_bugs/BUG-codex-desktop-native-memory-powershell-hook-not-entering-python.md)
- [`BUG-legacy-head-contract-missing-release-blocks-rebuild.md`](2_bugs/BUG-legacy-head-contract-missing-release-blocks-rebuild.md)
- [`BUG-rebuild-drops-project-doc-index.md`](2_bugs/BUG-rebuild-drops-project-doc-index.md)
- [`BUG-rebuild-blocks-required-project-maps.md`](2_bugs/BUG-rebuild-blocks-required-project-maps.md)
- [`BUG-project-sync-schema-v1-baseline-and-native-memory-hook-race.md`](2_bugs/BUG-project-sync-schema-v1-baseline-and-native-memory-hook-race.md)
- [`BUG-native-memory-maintain-misclassified-safe-data-egress.md`](2_bugs/BUG-native-memory-maintain-misclassified-safe-data-egress.md)
- [`BUG-target-cleanup-core-skeleton-ownership.md`](2_bugs/BUG-target-cleanup-core-skeleton-ownership.md)
- [`BUG-aggressive-absorption-drops-downstream-rule-semantics.md`](2_bugs/BUG-aggressive-absorption-drops-downstream-rule-semantics.md)
- [`BUG-switch-codex-left-claude-live-dir.md`](2_bugs/BUG-switch-codex-left-claude-live-dir.md)
- [`BUG-git-sync-sandbox-permission.md`](2_bugs/BUG-git-sync-sandbox-permission.md)
- [`BUG-git-sync-contract-transition-classification.md`](2_bugs/BUG-git-sync-contract-transition-classification.md)（1.5.5 修复已通过工厂验证与独立审计，等待 Batch restart 的真实下游复验）
- [`BUG-current-baseline-gitattributes-hook-reinitializes-real-repository.md`](2_bugs/BUG-current-baseline-gitattributes-hook-reinitializes-real-repository.md)（源码与回归已修复，等待发布和 StratusAgent 真实恢复）
- [`BUG-batch-pending-drift-deadlock.md`](2_bugs/BUG-batch-pending-drift-deadlock.md)（状态机与修复见证已修复，等待发布后重启原批次）
- [`BUG-bridgeforge-codex-145-end-to-end-acceptance-gaps.md`](2_bugs/BUG-bridgeforge-codex-145-end-to-end-acceptance-gaps.md)
- [`BUG-hooks-template-stale-context-budget-comment-blocks-downstream-upgrade.md`](2_bugs/BUG-hooks-template-stale-context-budget-comment-blocks-downstream-upgrade.md)
- [`BUG-shared-skill-manifest-line-endings.md`](2_bugs/BUG-shared-skill-manifest-line-endings.md)
- [`BUG-summary-writes-global-memory-instead-of-project-memory.md`](2_bugs/BUG-summary-writes-global-memory-instead-of-project-memory.md)
- [`BUG-migration-drops-project-pre-commit-extension.md`](2_bugs/BUG-migration-drops-project-pre-commit-extension.md)
- [`BUG-bridgeforge-references-omitted-from-user-skill.md`](2_bugs/BUG-bridgeforge-references-omitted-from-user-skill.md)
- [`BUG-update-stamped-before-memory-migration.md`](2_bugs/BUG-update-stamped-before-memory-migration.md)
- [`BUG-codex-native-memory-empty-snapshot-reconcile.md`](2_bugs/BUG-codex-native-memory-empty-snapshot-reconcile.md)
- [`BUG-finalizer-timeout-protected-host-tempfile.md`](2_bugs/BUG-finalizer-timeout-protected-host-tempfile.md)
- [`BUG-create-worktree-sandbox-half-created.md`](2_bugs/BUG-create-worktree-sandbox-half-created.md)

## 归档与参考

`4_archive/` 内现有文件为迁移前历史档案，继续保持可追溯性；Codex harness parity 设计与交付材料已归档到 [`codex-harness-parity-design.md`](4_archive/codex-harness-parity-design.md) 和 [`codex-harness-parity-delivery/`](4_archive/codex-harness-parity-delivery/)，旧运行手册打包结构见 [`bridgeforge-doc-runtime-packaging`](4_archive/delivery/bridgeforge-doc-runtime-packaging/summary.md)。新归档按 `4_archive/delivery/<topic>/` 或 `4_archive/bugs/` 落位。外部资料与仅供参考的实现放入 `3_reference/`，不作为运行时资产。

| 归档日期 | 类型 | 记录 |
|---|---|---|
| 2026-08-30 | Delivery | [`bridgeforge-command-clarity`](4_archive/delivery/bridgeforge-command-clarity/) |
| 2026-08-30 | Delivery | [`bridgeforge-switch-direct-sync`](4_archive/delivery/bridgeforge-switch-direct-sync/) |
| 2026-08-30 | Delivery | [`bridgeforge-switch-semantic-migration`](4_archive/delivery/bridgeforge-switch-semantic-migration/) |
| 2026-08-30 | Delivery | [`codex-cost-routing`](4_archive/delivery/codex-cost-routing/) |
| 2026-08-30 | Delivery | [`codex-model-routing`](4_archive/delivery/codex-model-routing/) |
| 2026-08-30 | Delivery | [`codex-model-routing-56`](4_archive/delivery/codex-model-routing-56/) |
| 2026-08-30 | Delivery | [`codex-project-zone-ownership`](4_archive/delivery/codex-project-zone-ownership/) |
| 2026-08-30 | Delivery | [`codex-skeleton-refactor`](4_archive/delivery/codex-skeleton-refactor/) |
| 2026-08-30 | Delivery | [`codex-skill-routing-dispatch`](4_archive/delivery/codex-skill-routing-dispatch/) |
| 2026-08-30 | Delivery | [`codex-subscription-routing`](4_archive/delivery/codex-subscription-routing/) |
| 2026-08-30 | Delivery | [`create-worktree-skill`](4_archive/delivery/create-worktree-skill/) |
| 2026-08-30 | Delivery | [`memory-lifecycle-governance`](4_archive/delivery/memory-lifecycle-governance/) |
| 2026-08-30 | Delivery | [`token-context-optimization`](4_archive/delivery/token-context-optimization/) |
| 2026-08-30 | Bug | [`BUG-downstream-business-version-rule-without-enforcement.md`](4_archive/bugs/BUG-downstream-business-version-rule-without-enforcement.md) |
