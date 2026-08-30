---
status: implemented_with_external_validation_gaps
topic: project-memory-retirement
confirmed_at: 2026-08-30
scope: bridgeforge-codex 工厂、候选工程及所有下游项目
size: L
time_budget: 90 分钟
token_budget: 45k 新增 token（估算，未实测）
agent_budget: 1 个独立审计 agent
verification_budget: 最多两轮
---

# 项目级 Memory 退役与下游受控迁移需求

## 结论

bridgeforge-codex 退役项目 `.codex/memory/`，保留 Codex 原生 `~/.codex/memories/`。任何下游已有项目 Memory 都必须经过“程序扫描、Agent 语义审核、用户逐项确认、受控迁移与独立清理授权”；同步器禁止直接删除或把 legacy 当成已迁移。

## 目标

1. 候选工程先对全部 24 个项目 Memory 资产建立逐文件账本并完成用户逐项审核。
2. 后续下游运行 `$bridgeforge-codex` 时，确定性发现项目 Memory legacy，生成零写入清单和 gap 收据。
3. Agent 根据程序事实判断独有信息、目标 owner、重复风险和迁移方式，但不得代替用户批准。
4. 只有用户批准的内容才可迁入 `AGENTS.md`、`doc/0_architecture`、`doc/1_delivery`、`doc/3_reference` 或 `doc/4_archive`。
5. 内容迁移与 legacy 清理分为两次授权；任一未决资产都必须原样保留。
6. 原生 Memory 同步、用户级 Hook、私有 GitHub 整树快照和单写入设备模型保持不变并单独回归。

## 非目标

- 不让程序自行理解自然语言或自动判定“没有独有信息”。
- 不把 `.codex/memory/` 原树机械复制到 `doc/4_archive/` 制造第二份垃圾索引。
- 不在 `.codex/memory/` 内建立 `_archive/`、墓碑或新索引。
- 不让 `$summary`、同步器或 Hook 创建新的项目 Memory。
- 不读取、编辑、整理或删除 Codex 原生 `~/.codex/memories/` 正文。
- 不因发现 legacy 阻止无关的只读扫描；是否影响目标 readiness 沿用项目同步器统一 gap 语义，不另建状态系统。

## 单一工作流

### 1. 程序扫描

程序只产出可机械复现的事实：

- 枚举 `.codex/memory/` 下全部文件，记录相对路径、大小、SHA-256 和 metadata。
- 区分正文、topic、派生索引和统计，但不推断自然语言是否独有。
- 检查源文件被哪些 Hook、Skill、manifest、managed asset、同步器和测试引用。
- 检查候选目标文件是否存在，并记录精确匹配或明显重复锚点。
- 对完整扫描输入计算 aggregate fingerprint。
- 输出稳定 JSON 收据和人类可读账本；`--check`、扫描与 plan 必须零写入。

程序不得因为名称像 `MEMORY.md`、状态是 `completed` 或目标文件存在，就自动判定可删除。

### 2. Agent 语义审核

Agent 必须基于程序收据逐文件给出：

- 信息类型与当前有效性。
- 目标文件及 owner。
- 合并、迁移、归档、删除或保留建议。
- 独有信息摘要和防重复依据。
- 未验证项、冲突和需要用户裁决的边界。

Agent 可以提出文本修改，但在对应行获批前禁止写目标文件。目标不唯一或信息冲突时，该行保持 `hold`，禁止猜测。

### 3. 用户逐项确认

24 项或任一下游 N 项资产都必须完整过账。默认每次只向用户提交一项；用户明确要求批量审核时，每批最多三项，且必须逐项展示、逐项记录决定，禁止用“整批同意”省略 asset id。每项展示：

1. 源文件原本解决的问题和产生背景。
2. 关键行为与结论；每项必须展开到足以让用户理解机制，不得只复述 description。
3. 哪些结论仍有效、哪些已过时或被替代。
4. 与 doc 的明确关系：现有 doc 是否完整、后续是否写 doc、写入哪个 doc、具体写入什么；不写时必须明确写“只核对现有 doc，不新增内容”。
5. Memory 的最终处理：补写或核对完成后是删除、保留还是暂停；禁止只写“合并 / 迁移 / 归档 / 去重”让用户反推动作。
6. 目标现有内容与重复证据。
7. 独有信息、删除后的真实损失及遗漏风险。
8. 推荐 disposition 和恢复边界。

用户决定只能是 `approve`、`change-target`、`preserve` 或 `hold`。批量确认也必须明确列出每个 asset id；“同意总体方向”不等于批准任何具体迁移或删除。

### 4. 受控迁移与验证

对应行获批后，Agent 才能把独有信息写入已批准目标。每完成一项必须：

- 重验源 SHA-256、目标基线和 aggregate fingerprint；漂移则停止并重新审核。
- 只迁移独有信息；派生索引、统计和重复正文不复制。
- 检查 AGENTS 没有吸收说明性叙事，Skill 没有吸收低频架构，文档没有冒充运行时指令。
- 记录目标路径、目标变更 hash、验证命令与真实收据。
- 保持源 `.codex/memory/` 原样，直到全部资产完成并取得独立清理授权。

### 5. 独立清理授权

内容迁移完成不自动授权删除。只有全部资产进入 `verified` 或经用户决定 `preserve`，且活跃运行时引用清零后，才能展示一次独立清理清单。

用户明确批准清理后，执行器仍必须重新计划并核对 fingerprint。任何新文件、hash 漂移、未决引用、失败验证或 native Memory 路径误入范围都必须零删除停止。

## 逐资产账本合同

每行至少包含：

| 字段 | 含义 |
|---|---|
| `asset_id` | 稳定行号，不因排序变化重用 |
| `source_path` | 下游项目内相对路径 |
| `source_sha256` | 用户审核时的源内容指纹 |
| `information_type` | 正文、topic、索引或统计 |
| `proposed_target` | 明确目标文件或“无” |
| `disposition` | merge、migrate、archive、delete、preserve、hold |
| `unique_information` | 独有事实与遗漏风险 |
| `dedup_evidence` | 已存在事实源及语义等价依据 |
| `user_decision` | 用户对本行的明确决定 |
| `migration_receipt` | 目标 hash、验证和未验证边界 |
| `cleanup_decision` | 独立删除授权；不得从 `user_decision` 推导 |

状态顺序固定为：

```text
discovered -> proposed -> approved|preserve|hold
approved -> migrated|no-migration-required -> verified
verified -> cleanup-pending -> cleanup-approved -> retired
```

`hold`、`preserve`、缺收据或状态跳跃都禁止删除源文件。

## 下游 `$bridgeforge-codex` 行为

- 首次发现 `.codex/memory/` 时，项目同步器必须停止安装和调用旧项目 Memory 运行时，把目录报告为 `legacy_project_memory` gap，并保留全部字节。
- 机器账本标准路径为 `doc/2_bugs/BUG-project-memory-retirement/ledger.json`。首次扫描只报告该路径和 `discovered` 清单，不自动创建；Agent 取得逐项确认后由受控开发流程写入。schema 1 以 `asset_id` 为 records key，保存 `source_path`、`source_sha256`、`migration_status`、目标、disposition、用户决定和独立 cleanup 决定。
- 后续运行必须报告账本进度，不重复创建第二份账本，也不把未决项静默改为已处理。
- 同步器只读对账账本；账本损坏、未知 asset、scan fingerprint 或 source hash 漂移必须继续报告 legacy gap，并把对应项标为 `invalid` / `drifted`，禁止据此写入或删除 Memory。
- 没有账本时只输出扫描候选；有账本时以 asset id、source hash 和用户决定对账。
- 同步器可以继续执行与 legacy 无关且满足现有 ownership 契约的安全动作，但不得把 legacy gap 描述成成功迁移。
- 下游项目没有 `.codex/memory/` 时，流程保持 no-op，不创建空目录或空账本。

## 原生 Memory 排除边界

以下资产不进入项目 Memory 退役账本：

- 用户目录 `~/.codex/memories/` 及其官方生成内容。
- `scripts/codex_memory_sync.py` 与用户级同步 Hook。
- 私有 GitHub 整树快照、parentless commit、`--force-with-lease`、pending 和 reconcile 收据。
- 单写入设备约束及 `scripts/tests/test_memory_native_sync.py`。

扫描器必须以项目根 lexical/resolved 边界限制输入；不得跟随 junction、symlink 或 reparse point 进入用户级 Memory。

## 验收条件

### 候选工程

- 24/24 资产都有 source hash、目标、disposition、独有信息和防重复判断。
- 用户逐项审核 24/24，未审核行保持零写入。
- 迁移后的目标通过文档索引、引用、重复事实源和 IA 风险复核。
- `$summary` 建议零写入；项目 Memory 活跃引用清零。

### 下游产品机制

- 无 legacy、完整 legacy、派生-only、正文漂移、目标漂移、账本损坏和 reparse point fixture 均有覆盖。
- scan/plan/check 零写入且相同输入产生相同清单和 fingerprint。
- 未确认、部分确认、迁移失败和 cleanup 未授权时，原目录逐字节保留。
- 至少一个低定制和一个高定制真实下游完成扫描与人工确认演练；未执行清理也必须能稳定保持 gap。
- native Memory 同步全量回归通过，并验证扫描范围不包含用户级目录。
- runtime smoke 证明旧项目 Memory Hook/Skill 不再安装或调用；native Memory 官方注入仍可用。

## 当前状态

- 已确认：目标架构、角色分工、逐项人工审核、两次授权和下游产品化要求。
- 已完成：候选工程 24 项 P0 账本、内容指纹和 24/24 用户逐项裁决。
- 已授权：P1 的 O-01、T-03、T-13 三项 doc 写入，以及 `$summary`、AGENTS 和相关 Skill/文档的 native-only 零写入重构。
- P1 实现已完成：O-01、T-03、T-13 迁移，21 项 no-migration-required 核对，AGENTS/Template、`$summary`、`$todo`、`$find-doc`、`$resume` 与活跃架构文档已切换到 native-only/legacy-preserve 边界；独立审计指出的三项残留已修正。
- P1 验证已完成：候选工程 `.venv` 下定向回归 150/150，通过 summary/Skill/manifest/AGENTS/结构与 native Memory sync；manifest `--check` 无变化，24/24 源 hash 匹配且 Memory 目录零 diff，`git diff --check` 通过。
- P2 实现已完成：旧项目 Memory 的 Hook、dispatcher route、pre-commit、git-sync rebuild、manifest、managed assets、模板脚本与 `$find-memory` 已退役；下游扫描现在逐文件输出 path、size、SHA-256、机械分类、frontmatter metadata、账本初态和 scan fingerprint。
- P2 legacy 行为已完成：update/rebuild 都将既有 `.codex/memory/` 报告为 `R:legacy-project-memory` / `legacy-gap`，计划与执行收据为 `action_required` / `completed_with_gaps`，整树逐字节快照、漂移检查和回滚均保留；同步器不 lint、不重建索引、不写入或直接删除 legacy。
- P2 审计已完成：独立审计提出“人类收据误报 ready、扫描/持久账本闭环不足、跨会话/跨项目测试自证式”三项 P1 问题，均已修正并通过 146 项定向回归。
- 外部验证仍有缺口：未在用户点名的真实低定制/高定制下游执行扫描演练，也未取得真实 Codex runtime smoke；当前只有隔离 fixture 和真实 dispatcher 子进程测试，禁止宣称真实下游/runtime 已通过。
- 删除仍未授权：候选工程 `.codex/memory/` 必须保持原样，直到 P2 独立清理确认。

## P1 实施计划与预算

- 规模：M；涉及三项 doc 写入、`$summary`、AGENTS 和相关 Skill/文档，不在本阶段删除运行时资产。
- 预算：45 分钟、20k 新增 token（估算，平台无可靠精确计量器）、最多 1 个独立审计 agent、最多两轮验证。
- 实施顺序：先完成 O-01/T-03/T-13 的 doc 写入与索引，再收口 `$summary` 和指令面，最后执行定向测试与独立 IA 审计。
- 超预算停止点：发现需要提前删除 Hook/脚本/manifest 资产、修改原生 Memory 正文或用户级配置、扩大到 P2 产品同步器实现，或两轮验证后仍存在关键失败。
- 2026-08-30：用户完成 24/24 资产审核后明确回复“开始”，P1 开工闸通过。

## P2 实施计划与预算

- 规模：L。涉及同步器、manifest、managed assets、Hooks、dispatcher、Skills、fixture 与跨项目/跨会话验证。
- 预算：90 分钟、45k 新增 token（估算，平台无可靠精确计量器）、主对话实现、1 个独立 `review-auditor`、最多两轮验证。
- 授权范围：退役项目 Memory 运行时并实现下游 legacy preserve/gap；保留并回归 native Memory sync。
- 明确排除：不删除候选工程 24 个源文件，不 commit、不 push；源删除仍需最终独立授权。
- 2026-08-30：用户明确回复“确认”，P2 开工与预算闸通过。

## P2 实施与验证收据

- 产品版本：`1.5.11`；CHANGELOG 标记 `[product][repo][meta]`，manifest 与 Template/dogfood managed skeleton 已重建。
- 完整回归第 1 轮：329 项运行，3 个旧 `render_memory_indexes` mock 报错；删除退役耦合后进入第 2 轮。
- 完整回归第 2 轮：331 项运行，329 项通过、1 项 skipped、1 项失败；唯一失败是测试把已清空但仍存在于文件系统的空目录误判为活跃资产。改为检查目录文件集合后，该失败项定向通过。两轮后不再重复完整回归。
- 审计修正后定向回归：project sync、Hook dispatcher、跨项目/跨会话、native Memory sync 与 `$summary` 共 146/146 通过；其中真实创建两个隔离项目和三个 session，运行实际 dispatcher 子进程，legacy sentinel 均未进入输出；标准持久账本的 absent/current/invalid、progress 与只读保护均有覆盖。
- 下游 fixture：4/4 通过（current init 幂等、旧项目确认式 rebuild、current drift 零写阻断、Skill routing gap 零写）。
- manifest/mirror/doc：manifest `--check` 无变化；shared distribution、project structure 与 dogfood mirror 34/34 通过；三个 Template/dogfood `git diff --no-index` 均为 0；`git diff --check` 为 0。结构检查只有既有 archive advisory，无硬失败。
- 源资产边界：账本 24 项、实际文件 24 项、SHA-256 错误 0；`.codex/memory/` Git diff 为 0。`scripts/codex_memory_sync.py` 与 `scripts/codex_memory_hook.cmd` Git diff 为 0。
- 未验证：真实下游扫描/人工确认演练、真实 Codex 原生 Memory 注入 runtime smoke。两项都需要超出候选工程的额外目标与运行授权。
