---
status: verified_pending_cleanup_authorization
record_type: migration_ledger
scope: refactored-project/.codex/memory
inventoried_at: 2026-08-30
approved_for_migration: true
approved_for_deletion: false
reviewed_assets: 24
migrated_assets: 3
no_migration_required_assets: 21
p1_validation: passed
p2_runtime_retirement: implemented
p2_candidate_validation: passed_with_external_gaps
p2_factory_propagation: passed_1_5_13
default_user_decision: pending
product_requirement: doc/1_delivery/project-memory-retirement/requirements_2026-08-30_project-memory-retirement.md
---

# 项目级 Memory 退役迁移账本

## 结论

候选工程 `.codex/memory/` 共 24 个资产：3 个派生索引/统计，7 个通用 Memory，14 个 delivery topic。逐文件审核后：

- 3 个派生资产没有正文价值，只列入 P2 清理候选，不迁移。
- 18 个正文资产的有效信息已由现有 `AGENTS.md`、Hook/测试、交付需求、Bug 或归档文档覆盖；P1 已完成逐项去重核对，不再复制摘要。
- 3 个资产的独有信息已按用户裁决迁入明确 owner 的文档，收据见下文。
- P1/P2 没有修改或删除任何源资产；运行时已退役，但源资产清理仍须独立授权。
- 退役实现已传播到真实工厂 `1.5.13`；根 `.codex/memory/` 25 个 legacy 文件保持原样，正式 Template、dogfood、manifest 与 dispatcher 不再运行或分发项目 Memory 链。

本账本只处理项目 `.codex/memory/`。Codex 原生 `~/.codex/memories/`、`scripts/codex_memory_sync.py`、用户级同步 Hook、私有 GitHub 整树快照、单写入设备模型及其测试不在退役范围。

用户已于 2026-08-30 确认下游必须采用“程序扫描、Agent 语义审核、用户逐项确认、受控迁移、独立清理授权”机制。通用流程、账本字段和验收条件以 [`requirements_2026-08-30_project-memory-retirement.md`](../../1_delivery/project-memory-retirement/requirements_2026-08-30_project-memory-retirement.md) 为唯一事实源；本文件只记录候选工程 24 项资产的具体裁决。

## 判定规则

- **合并**：源文件有少量独有事实，写入既有权威文档；禁止整篇复制。
- **迁移**：源文件存在仍有效的稳定架构事实，但当前没有合适事实源；新建或补齐明确 owner 的文档。
- **归档**：内容只解释已经完成或已被替代的历史机制；不得重新变成运行时指令。
- **删除（去重后）**：源内容是派生产物，或其有效正文已被更权威载体覆盖。删除前仍须复核目标存在、语义等价且本账本 hash 未漂移。
- **防重复**：优先保留代码/Hook/测试的机器合同、AGENTS 的最小红线和 delivery/Bug 的事实记录；禁止把同一稳定规则再手写进多个载体。

## 逐文件迁移账本

| ID | 原路径（相对 `.codex/memory/`） | 信息类型 / 状态 | 目标文件 | 方式 | 独有信息、防重复判断与用户裁决 |
|---|---|---|---|---|---|
| I-01 | `MEMORY.md` | 自动生成热索引 | 无 | 删除 | 仅含 Active/Cold 计数、链接和 `$find-memory` 指针；正文均来自下列 Memory，零独有信息。禁止迁移派生索引。**用户裁决：`approve`（2026-08-30），仅列入最终清理候选，非删除授权。** |
| I-02 | `MEMORY_COLD.md` | 自动生成冷索引 | 无 | 删除 | 仅含 14 个 topic 的 description 与链接；与 topic frontmatter 重复，零独有信息。**用户裁决：`approve`（2026-08-30），仅列入最终清理候选，非删除授权。** |
| I-03 | `_stats.json` | 自动生成创建日期统计 | 无 | 删除 | 只记录 21 个正文文件的 `created_at`；不属于业务或架构事实，禁止迁移统计。**用户裁决：`approve`（2026-08-30），仅列入最终清理候选，非删除授权。** |
| A-01 | `architecture/codex-model-routing-policy.md` | 通用架构 / active | `AGENTS.md` §4.3；`doc/1_delivery/shared-skill-model-inheritance/requirements_2026-08-16_shared-skill-model-inheritance.md`；`doc/4_archive/delivery/codex-cost-routing/requirements_2026-07-10_codex-cost-routing.md` | 删除（去重后） | “继承当前会话 model/effort”已在 AGENTS 与 shared-skill 需求卡；“不写用户级 config”已在 cost-routing 需求卡。日期叙事和无法量化 token 节省不构成新规则，不再复制。**用户裁决：`approve`（2026-08-30），仅列入最终清理候选，非删除授权。** |
| A-02 | `architecture/bridgeforge-switch-direct-sync.md` | 通用架构 / active，但双宿主机制已属历史 | `doc/4_archive/delivery/bridgeforge-switch-direct-sync/requirements_2026-07-25_bridgeforge-switch-direct-sync.md`；同 topic 的 projection 需求与 debates | 删除（去重后） | `created_unowned`、`forked_projection`、`last_generated_sha256`、`interrupted-or-modified`、回滚和 fixture 收据均已存在；不把退役 switch 机制重新写入当前架构。**用户裁决：`approve`（2026-08-30），仅列入最终清理候选，非删除授权。** |
| E-01 | `engineering/bom-free-encoding-gate.md` | 工程规范 / active | `AGENTS.md` 工具与证据红线；`doc/1_delivery/non-ascii-shell-guard/requirements_2026-07-10_non-ascii-shell-guard.md`；`doc/4_archive/调查报告_BOM-no-BOM统一策略与模板污染修复_2026-07-08.md` | 删除（去重后） | UTF-8 no BOM、非 ASCII shell 中转、三连问号/`U+FFFD`、编辑后与 pre-commit 检查均已覆盖；旧 Claude 路径和版本数字只属历史，不进入当前指令。**用户裁决：`approve`（2026-08-30），仅列入最终清理候选，非删除授权。** |
| E-02 | `engineering/codex-ctx-budget-window.md` | 工程机制 / active 标记已失真 | `doc/1_delivery/ctx-management/requirements_2026-07-09_codex-ctx-budget.md`；`requirements_2026-07-30_stall-warning-removal.md` | 删除（去重后） | 258K→353K 校准、usage 算法与验证收据已在需求卡；候选工程已无 `context_warning.py`，因此禁止迁回 AGENTS/Hook。保留 delivery 历史即可。**用户裁决：`approve`（2026-08-30），仅列入最终清理候选，非删除授权。** |
| E-03 | `engineering/confirm-workflow.md` | 工程流程 / active | `AGENTS.md` §4.4；`skills/confirm/SKILL.md`；`doc/1_delivery/confirm-workflow/requirements_2026-07-14_confirm-workflow.md` | 删除（去重后） | 单题访谈、确认卡、develop/debate/collab 复用边界已有运行时 owner 与交付记录；Memory 是重复摘要。**用户裁决：`approve`（2026-08-30），仅列入最终清理候选，非删除授权。** |
| E-04 | `engineering/skill-metadata-precommit-gate.md` | 工程硬闸 / active | `.codex/hooks/skill_metadata_check.py`；`scripts/tests/test_skill_metadata_budget.py`；`doc/4_archive/delivery/create-worktree-skill/requirements_2026-08-15_create-worktree-skill.md`；`CHANGELOG.md` | 删除（去重后） | metadata 合法性应由 Hook/测试唯一判定；`user_invocable` 兼容差异已有 delivery 与 CHANGELOG。禁止把示例 frontmatter 再放进 AGENTS 或参考文档形成第二事实源。**用户裁决：`approve`（2026-08-30），仅列入最终清理候选，非删除授权。** |
| O-01 | `operations/codex-bridgeforge-slash-entry-debug.md` | 运维事故 / active | `doc/4_archive/调查报告_codex-bridgeforge-slash入口不可见_2026-07-07.md` | 合并 | **与 doc 的关系：现有归档报告不完整，P1 要把独有的最终验真写入该既有 doc。**内容包括旧 junction 污染、BOM 首字节、`~/.agents/skills/bridgeforge` 叶子 wrapper、版本联动和 `Force Reload Skills` 顺序；同时把旧 `bridgeforge` skill/plugin 建议标为历史，不改写成当前操作指令。**用户裁决：`approve`（2026-08-30），补写并验真后仅列入最终清理候选，非删除授权。** |
| T-01 | `topics/bridgeforge-actionable-readiness/summary.md` | delivery / completed | `doc/1_delivery/bridgeforge-actionable-readiness/requirements_2026-08-15_bridgeforge-actionable-readiness.md` | 删除（去重后） | **与 doc 的关系：现有需求 doc 已完整，P1 不写 doc，只核对覆盖关系。**双状态、R/C/M/B、A/B/C、fingerprint、兼容字段、测试与 runtime 未验证边界均已在需求卡。**用户裁决：`approve`（2026-08-30），核对通过后仅列入最终清理候选，非删除授权。** |
| T-02 | `topics/bridgeforge-command-model/summary.md` | delivery / superseded | `doc/4_archive/delivery/bridgeforge-command-clarity/requirements_2026-07-08_bridgeforge-command-clarity.md`；`doc/2_bugs/BUG-switch-codex-left-claude-live-dir.md` | 删除（去重后） | **与 doc 的关系：现有需求卡和 Bug 已保存历史，P1 不写 doc。**两入口、预刷新、判场与 cleanup-only 已被覆盖；当前产品已 Codex-only，禁止把 Claude/switch 历史迁入当前 README 或 Skill。**用户裁决：`approve`（2026-08-30），仅列入最终清理候选，非删除授权。** |
| T-03 | `topics/bridgeforge-doc-runtime-packaging/summary.md` | delivery / superseded | `doc/4_archive/delivery/bridgeforge-doc-runtime-packaging/summary.md`（新） | 归档 | **与 doc 的关系：当前运行资料已迁至 `skills/bridgeforge-codex/references/`，但缺少旧打包结构的历史归档；P1 要新建该 archive doc。**只写 `doc/0_playbook` 单子树分发、旧五份手册、0.65.1 收据及替代指针，不恢复旧目录或复制手册全文。**用户裁决：`approve`（2026-08-30），归档并验真后仅列入最终清理候选，非删除授权。** |
| T-04 | `topics/bridgeforge-latency-optimization/summary.md` | delivery / completed | `doc/1_delivery/bridgeforge-latency-optimization/requirements_2026-08-15_bridgeforge-latency-optimization.md`；`doc/0_architecture/design/codex-project-sync.md` | 删除（去重后） | **与 doc 的关系：现有需求 doc 已完整，P1 不写 doc，只核对覆盖关系。**sparse fast path、planner 去重、并行验证、`timings_ms`、本地时延与未验证网络边界均有记录；旧 “canonical memory validator” 不迁移并等待 P2 清除活跃引用。**用户裁决：`approve`（2026-08-30），核对通过后仅列入最终清理候选，非删除授权。** |
| T-05 | `topics/bridgeforge-single-confirmation/summary.md` | delivery / completed | `doc/1_delivery/bridgeforge-single-confirmation/requirements_2026-08-15_bridgeforge-single-confirmation.md`；原生 Memory 相关稳定事实另由 T-13 统一承接 | 删除（去重后） | **与 doc 的关系：现有需求 doc 已完整，P1 不写 doc；native sync 当前架构只由 T-13 的新 doc 承接。**safe/risk/gap、一次确认、legacy consent、窄用户级权限、测试与 runtime 边界已有历史记录；删除前必须完成 T-13 迁移并对账，避免原生 Memory 同步事实空档。**用户裁决：`approve`（2026-08-30），满足 T-13 前置依赖后仅列入最终清理候选，非删除授权。** |
| T-06 | `topics/bridgeforge-switch-semantic-migration/summary.md` | delivery / superseded | `doc/4_archive/delivery/bridgeforge-switch-semantic-migration/requirements_2026-07-25_bridgeforge-switch-semantic-migration.md`；同 topic debates | 删除（去重后） | **与 doc 的关系：现有需求 doc 与 debates 已完整，P1 不写 doc，只核对覆盖关系。**schema v2 receipt、可信沙箱 fail-closed、NFC/casefold/reparse、回滚与 43/43 收据均已覆盖；双宿主 switch 已退役，不创建新的当前架构文档。**用户裁决：`approve`（2026-08-30），核对通过后仅列入最终清理候选，非删除授权。** |
| T-07 | `topics/bridgeforge-upstream-absorption-modes/summary.md` | delivery / completed | `doc/1_delivery/bridgeforge-upstream-absorption-modes/requirements_2026-08-15_bridgeforge-upstream-absorption-modes.md`；`doc/1_delivery/bridgeforge-keyed-index-merge/requirements_2026-08-16_bridgeforge-keyed-index-merge.md`；相关 Bug | 删除（去重后） | **与 doc 的关系：现有两份需求 doc 与相关 Bug 已完整，P1 不写 doc，只核对覆盖关系。**A/B/C、逐 U、keyed table、尾换行回归、事务与审计收据均已存在；`.codex/memory/MEMORY.md seed` 不迁移并等待 P2 清除活跃引用。**用户裁决：`approve`（2026-08-30），核对并清零 seed 引用后仅列入最终清理候选，非删除授权。** |
| T-08 | `topics/claude-template-safety-hooks-review/summary.md` | delivery / completed，但产品已 Codex-only | 现有 Hook/测试、`doc/1_delivery/cross-project-write-guard/requirements_2026-07-10_cross-project-write-guard.md`、`CHANGELOG.md` | 删除（去重后） | **与 doc 的关系：不新建 doc；当前有效的 Codex Hook 由代码、测试、现有 delivery 与 CHANGELOG 承载，Claude-only 历史不迁移。**`memory_dup_check.py` 随项目 Memory 退役；其余 Hook 的当前行为只按现有 owner 核对，禁止用历史移植笔记重新定义。**用户裁决：`approve`（2026-08-30），核对当前 owner 后仅列入最终清理候选，非删除授权。** |
| T-09 | `topics/codex-harness-parity-closure/summary.md` | delivery / completed，双宿主 parity 已退役 | `doc/4_archive/codex-harness-parity-delivery/compatibility_closure_2026-07-08.md`；`doc/4_archive/codex-harness-parity-design.md` | 删除（去重后） | **与 doc 的关系：现有两份 archive doc 已完整，P1 不写 doc，只核对覆盖关系。**20 项分类、`needs-review=0`、memory lint 连字符修复、slash 归一化与命名残留均已逐项归档；Claude/Codex parity 与项目 Memory 检查不迁回当前产品。**用户裁决：`approve`（2026-08-30），核对通过后仅列入最终清理候选，非删除授权。** |
| T-10 | `topics/codex-project-zone-ownership/summary.md` | delivery / completed | `doc/4_archive/delivery/codex-project-zone-ownership/requirements_2026-08-17_codex-project-zone-ownership.md`；`AGENTS.md` 项目架构红线；`doc/0_architecture/design/codex-project-sync.md` | 删除（去重后） | **与 doc 的关系：现有需求 doc、AGENTS 与架构设施已完整，P1 不写 doc，只核对 owner。**marker、公区/项目区 ownership、旧项目 fail-closed、fence 解析、真实样本与 runtime 未验证边界均已有事实源。**用户裁决：`approve`（2026-08-30），核对通过后仅列入最终清理候选，非删除授权。** |
| T-11 | `topics/codex-skeleton-refactor/summary.md` | delivery / completed | `doc/4_archive/delivery/codex-skeleton-refactor/requirements_2026-08-15_codex-skeleton-refactor.md`；`doc/0_architecture/design/codex-project-sync.md` | 删除（去重后） | **与 doc 的关系：核心架构已有完整需求 doc 与设施，P1 不写 doc；P2 另行把当前架构中的旧项目 Memory 连接替换为 legacy preserve/scan。**schema v2、稳定 asset id、safe/risk/gap、事务回滚、stamp-last、真实样本与审计问题均已覆盖；旧 “memory move” 语义不迁移。**用户裁决：`approve`（2026-08-30），运行时退役并回归后仅列入最终清理候选，非删除授权。** |
| T-12 | `topics/create-worktree-skill/summary.md` | delivery / completed | `doc/4_archive/delivery/create-worktree-skill/requirements_2026-08-15_create-worktree-skill.md`；`doc/2_bugs/BUG-create-worktree-sandbox-half-created.md` | 删除（去重后） | **与 doc 的关系：Skill、脚本、测试、现有需求 doc 与 Bug 已完整，P1 不写 doc，只核对 owner。**位置参数、`codex/` 分支、沙箱外首跑、Windows 协议、部分成功码、13/13 与 aaa/bbb 清理均已有逐项记录。**用户裁决：`approve`（2026-08-30），核对通过后仅列入最终清理候选，非删除授权。** |
| T-13 | `topics/memory-rule-organization/summary.md` | delivery / completed；其中项目 Memory 架构已被本任务替代 | `doc/0_architecture/design/codex-native-memory-sync.md`（新）；`doc/1_delivery/memory-rule-organization/requirements_2026-08-14_memory-governance-native-sync.md`（补充替代状态）；既有 native-memory Bug | 拆分迁移 | **与 doc 的关系：P1 要新建 native sync 当前架构 doc，并在既有混合需求卡标明项目 Memory 部分已退役；不再新建重复 archive doc。**迁移 opaque bytes、单写入设备、parentless 整树快照、`--force-with-lease`、空快照、wrapper 与 `hookRuntimeVerified`；项目索引、router、usage、writer、lint、duplicate、`$find-memory` 不迁移。**用户裁决：`approve`（2026-08-30），完成两处 doc 写入、逐项对账及 native sync 回归后仅列入最终清理候选，非删除授权。** |
| T-14 | `topics/skill-runtime-efficiency/summary.md` | delivery / completed | `doc/1_delivery/skill-runtime-efficiency/requirements_2026-08-15_skill-runtime-efficiency.md`；相关 Skill/测试 | 删除（去重后） | **与 doc 的关系：非 Memory 性能机制已有需求 doc、代码与测试，P1 不写 doc；P2 只拆除 `$find-memory`、summary/todo writer/rebuild 等项目 Memory 分支。**fast path、Git 子进程合并、`show_state`/`archive_scan` 优化必须保留。**用户裁决：`approve`（2026-08-30），运行时退役并回归后仅列入最终清理候选，非删除授权。** |

## P1 迁移候选（须先完成 24 项逐项确认）

24 项 disposition 已全部获得用户逐项确认。P1 可处理获批的独有信息；按当前裁决涉及以下三项：

1. O-01：把 slash 入口最终验真追加到既有归档报告，并明确旧建议已失效。
2. T-03：为已退役的 `doc/0_playbook` 分发结构建立最小历史归档。
3. T-13：建立 native Memory 同步当前架构事实源，并在既有混合需求卡标明项目 Memory 部分已被替代；不新建重复 archive doc。

完成三项后，必须复查 `AGENTS.md` 是否新增了说明性叙事、同一规则是否出现在多个手写载体、Skill 是否被塞回低频架构说明。发现任一复发即停止，不进入 P2。

## P1 迁移收据

| 资产 | 结果 | 目标 SHA-256 | 验证边界 |
|---|---|---|---|
| O-01 | `verified`：最终验真合入既有历史调查报告，旧建议明确失效 | `4f4ba7a84cda3f26be97ad5dac5279a6fbd3f4a5fda47b763a89c3fe040375fc` | 只补历史事实，不创建当前 Rule/Hook |
| T-03 | `verified`：建立最小历史归档并指向现行 Skill references | `a6dba48f0427f31b7d96200aa93d0a795efcfeeacc1716a9f1cf29ccd7c0ec14` | 未恢复 `doc/0_playbook`，未复制旧手册全文 |
| T-13a | `verified`：建立 native-only 同步架构事实源 | `dd0702cd9b62f88d55a3b4488a3da5485b8574655a380ba67970e40423a3e2e8` | 不透明整树、单写入设备、原生生成/注入边界已覆盖 |
| T-13b | `verified`：混合需求卡增加 superseded notice | `e0eb769fc06ad01a595cf567492e0a45ed14bf0ca328f86fd848ebe51d12955f` | 旧项目 Memory 正文保留为历史，不再可作为当前架构依据 |
| 其余 21 项 | `no-migration-required -> verified` | 目标文件逐项见主表 | 派生资产不迁移；其余已由既有 owner 覆盖，未新增重复事实源 |

收据：24/24 源文件仍存在且 SHA-256 与 P0 指纹一致；`.codex/memory/` 的 Git diff 为空；
shared Skill manifest 的文件集合与 SHA-256 检查为 0 失败；`git diff --check` 为 0；独立
`review-auditor` 复核确认无源删除、三项迁移没有复制派生索引，并指出的 `find-doc` 与退役
文档残留已在本轮修正。使用候选工程 `.venv` 的 P1 定向回归共 150/150 通过，其中
summary/Skill/manifest/AGENTS/结构 99 项，native Memory sync 51 项。

## P2 删除硬闸

即使用户逐项确认 disposition，P2 删除仍须单独确认。通用清理流程以产品需求为准；本候选工程还必须同时满足：

1. 24 个源文件 SHA-256 与本账本基线一致；任一漂移则退回 P0 重新审核该文件。
2. 3 个独有资产已完成 doc 写入，逐句对账无遗漏。
3. 17 个去重资产的目标文件仍存在，且关键语义检索命中；不得用 CHANGELOG 单独替代完整事实源。
4. 同步器停止安装/调用项目 Memory 运行时，但下游既有 `.codex/memory/` 只标为 legacy/gap 并原样保留。
5. native Memory 同步链、单写入设备与整树私有快照测试通过。
6. 用户再次明确批准删除后，才允许删除候选工程这 24 个资产；本账本本身不构成删除授权。

## P0 内容指纹

以下 SHA-256 锁定本次审核输入；P1/P2 不得在源漂移后沿用旧结论。

| 原路径 | SHA-256 |
|---|---|
| `_stats.json` | `86d31b9cec6c67095b013a37cb55aa9865f16ac92d9b3390699ccd62693b7460` |
| `architecture/bridgeforge-switch-direct-sync.md` | `7a658cee1257a1a7077ca65bc0685cdd1d9505f77addbf7c31978fa4d15b6d3b` |
| `architecture/codex-model-routing-policy.md` | `2e2710ea13bdd6f180a2d50fa23c6cd9528cda58ca360e5f19dfd0aaa633d45d` |
| `engineering/bom-free-encoding-gate.md` | `f12af962c66909d82e45b0651395ec128b51895fdbdd01a0219b7ef179c2fb2a` |
| `engineering/codex-ctx-budget-window.md` | `b97aab0cccd7ef7aff482b4ff4edd651aca6e9ff7e79a4ecfc1d0e03c8d65c9e` |
| `engineering/confirm-workflow.md` | `e3d5ba6f7c75bddd2b4d81d223c6e32dd7543d0b480b56eeb58bea76bade2278` |
| `engineering/skill-metadata-precommit-gate.md` | `c3487fcef713b46e420c808332f5d3976bca2e7749efeff6927c0008afcfe9c9` |
| `MEMORY_COLD.md` | `208f87136b07c68784e75eb003b4733659d1d1b0a5eddf4e16a9e0c0983a5901` |
| `MEMORY.md` | `947806f81f536648935d4d2d4805c965341f9ff867a930dd64f5291649bd0603` |
| `operations/codex-bridgeforge-slash-entry-debug.md` | `cad3945490e6acc3c97ceea5973d3fad2b8e2fc3c363edfd700121ed98fc3110` |
| `topics/bridgeforge-actionable-readiness/summary.md` | `f74a5d0fcb58054c5dd1699e348ceb62dae50e9ef070eb56afbfbdc4b67148c6` |
| `topics/bridgeforge-command-model/summary.md` | `04f17a4a735b82cc7641919a0120bcf13ec0c58d9f7aede0f0fbc1f7bb48dd36` |
| `topics/bridgeforge-doc-runtime-packaging/summary.md` | `7a17681179f205d97ca48d4e7f5e205b00785c93561f6ab4fc778a8159d9b192` |
| `topics/bridgeforge-latency-optimization/summary.md` | `6aafcead7e8bfab126ca48b72477bec0416a1643c4a17960f530627d82b3e696` |
| `topics/bridgeforge-single-confirmation/summary.md` | `4782967f6c20d805f08c957f4cdfbf100a31f8ce8251be0d8b01787b4079849e` |
| `topics/bridgeforge-switch-semantic-migration/summary.md` | `aeaab18d793c3893f6ad802e4c7d4caad25c0a0f5ee4cd0a2ab061bf04fdba0e` |
| `topics/bridgeforge-upstream-absorption-modes/summary.md` | `c3c528b75c1379187a68eb34899542577c57a95cdf22b30f6d3c46d941975332` |
| `topics/claude-template-safety-hooks-review/summary.md` | `7c016e033d3b8336d430e879ac2e697bfbe1b82a84221839e59d67c654a09150` |
| `topics/codex-harness-parity-closure/summary.md` | `fc5c9431d201d4ec734a55fad49f684c3f4a52fac11b658476891c7a7c91d975` |
| `topics/codex-project-zone-ownership/summary.md` | `e0bc6bf5a652eeb97ffff33cc076f62962769dd0f7a68999cb93578d0b958f11` |
| `topics/codex-skeleton-refactor/summary.md` | `8c2e13ab70686ea049d92f5510ba9c1165725649657972b3ed95fbb0c4c407d3` |
| `topics/create-worktree-skill/summary.md` | `71156fa39720f08d9f3747b9b8b0cf84d4f871c70890b182e342dd8661e082bf` |
| `topics/memory-rule-organization/summary.md` | `d656e442d15d6b93d15725f52e5a7036d841a4a030c2b583b4de1903505a1d13` |
| `topics/skill-runtime-efficiency/summary.md` | `837bd3aed3d8494978852d94ba799f1ddec201a176e9917e955a2687694fc469` |

## 当前阶段收口

- 已完成：24/24 资产盘点与用户裁决；O-01、T-03、T-13 迁移；21 项 no-migration-required 覆盖核对；项目 Memory 运行时、`$find-memory`、manifest/managed assets/Hook/git-sync 写入链退役。
- 已完成：下游逐文件扫描收据、`legacy-gap` / `action_required` 人类与机器结果、整树逐字节保护、旧 whole-owned runtime 安全删除计划及跨项目/跨会话零误召回测试。
- 已验证：24/24 源 hash 匹配、Memory Git diff 为 0；审计修正后 146/146 定向回归、4/4 fixture、34/34 manifest/mirror/结构回归通过。
- 未验证：真实低/高定制下游扫描演练与真实 Codex runtime smoke；未取得目标路径和外部运行授权。
- 未授权：源文件删除；`approved_for_deletion` 继续为 `false`。在外部验证缺口关闭且用户再次明确批准前，`.codex/memory/` 必须保持原样。
