---
lifecycle: active
validation_status: awaiting_validation
record_type: requirements
confirmed_at: 2026-08-31
source: $confirm
---

# Current-only 项目资产迁移与 Native Memory 同步可靠性需求

## 原始需求摘要

用户要求确认并交付四项能力：

1. 下游运行 `$bridgeforge-codex` 时，逐文件整理无效的 Claude 风格 `.codex/rules/*.md`，经用户确认后迁往 `.rules`、嵌套 `AGENTS.md` 或其他正确载体。
2. 下游运行 `$bridgeforge-codex` 时，逐文件整理已退役的项目 `.codex/memory/`，经用户确认后迁往其他项目资产。
3. 修复 `~/.codex/memories/` 到指定私有 GitHub 仓库的无感双向同步，并用真实双电脑验证。
4. 下游接入时只以运行当时的最新正式版本为完整基线，不再维护旧版本升级兼容链。

调用来源：用户直接调用 `$confirm`。

后续交接目标：用户在本卡落盘后选择 `$develop`、`$debate` 或 `$collab`。

## 目标

- 让 `$bridgeforge-codex` 在写盘前完成旧 Rule 与项目 Memory 的逐文件语义迁移确认。
- 将已确认的新资产写入、旧资产删除和版本基线安装收敛为一个可回滚事务。
- 将项目接入模型改为动态 latest current-only：只识别并迁移项目资产，不执行或解释旧骨架合同。
- 让 Native Memory 在一次授权后无感、可观察、可自愈地跨电脑同步，并保留普通 Git 父子提交历史。

## 不做

- 本轮不实际迁移 StratusAgent、BridgePersonalAssist 或其他真实下游的旧 Rule/Memory。
- 不把 `.codex/rules/*.md` 改扩展名后冒充 Codex 原生规则。
- 不把业务语义写入 `.rules`；`.rules` 只承载命令执行策略。
- 不把全部项目 Memory 搬入 `AGENTS.md` 或 `doc/` 形成新的混合事实源。
- 不语义读取、创建、修改或删除 Codex 原生 `~/.codex/memories/`；同步器只把它作为不透明目录处理。
- 不保留任何旧版本逐级升级、旧 schema 解释或旧骨架运行兼容分支。
- 不在 BridgeForge 仓库中自动执行 `git add`、commit 或 push。

## 规模与预算

| 项目 | 预算 |
|---|---|
| 规模 | L |
| 判定依据 | Rule/Memory 语义迁移、事务删除、基线架构切换、用户级双机同步与真实外部验收 |
| 时间 | 120 分钟 |
| Token | 约 50k，平台无可靠计量器，未实测 |
| Agent | 最多 4 个；用户于 2026-08-31 批准从 2 个扩展为 4 个，以满足只读研读、双线并行实现和独立审计 |
| 验证轮次 | 最多 3 轮 |
| 超预算停止点 | 时间、估算 token、agent、验证轮次或范围任一预计越界时，停止并让用户选择扩大预算或缩小范围 |

等待第二台真实电脑不计入执行时间，但第二台电脑未完成 smoke 前，本事项不得标记为已验收关闭。

## 已核实事实

| 事实 | 收据 |
|---|---|
| StratusAgent 仍有 21 个 Claude 风格 Markdown Rule | 只读文件清单 |
| StratusAgent 仍有 429 个项目 Memory 文件 | 只读文件清单 |
| BridgePersonalAssist 仍有 33 个项目 Memory 文件 | 只读文件清单 |
| 当前 Rule 只在旧骨架重建时进入 `user-decision`，没有语义转换器 | `bridgeforge_codex_project_sync.py` 静态核对 |
| 当前项目 Memory 能逐文件生成 hash、类型和台账，但 `proposed_target` 为空，最终只原样保留为 legacy gap | `bridgeforge_codex_project_sync.py` 静态核对 |
| 当前最低 current baseline 为 `1.4.31`，仍保留低版本 adopt/rebuild 路径 | `current_baseline.py` 与项目同步器静态核对 |
| 用户安装产品 home 为 `1.5.26`，工厂为 `1.5.34` | 两份 `VERSION` |
| Native Memory 已授权、启用且用户 Hook 已安装 | 受管产品 home 的 `codex_memory_sync.py status` |
| 远端仓库采用每次无父提交并 force-with-lease 覆盖 main，提交数不能表示同步次数 | `_push_snapshot` 与 GitHub 提交父级核对 |
| 远端当前为 revision 15，最后快照时间 2026-08-26 | 私有仓库 `snapshot-manifest.json` |
| 当前电脑存在 2026-08-22 死亡锁、残留临时目录和 pending；后续 Hook 均返回 `busy` | 本机受管状态目录与 runtime 收据 |
| `busy` 被记为 `status=succeeded`，导致 `hookRuntimeVerified=true` 假健康 | 状态输出与实现静态核对 |
| 用户安装版与工厂 `codex_memory_sync.py` hash 相同，1.5.34 尚未修复该缺陷 | SHA-256 核对 |

## Rule 迁移合同

1. 每个旧 Rule 源文件为一次确认单位。
2. 单文件允许拆分为一个完整迁移包：
   - 业务或代码红线进入最近适用目录的嵌套 `AGENTS.md`；
   - 命令执行权限进入 `.codex/rules/*.rules`；
   - 设计原因、历史、案例和验证说明进入 `doc/`；
   - 重复或失效内容提出删除。
3. 每份迁移包必须展示原文件摘要、拆分内容、每部分目标、删除内容及理由。
4. 无法可靠判断的内容必须阻断并由用户指定，禁止默认迁入、归档、保留或删除。
5. 全部 Rule 与 Memory 完成确认前，目标资产和旧目录必须零写入。

## 项目 Memory 迁移合同

| 信息职责 | 目标资产 |
|---|---|
| 项目红线 | 根或最近适用目录的嵌套 `AGENTS.md` |
| 用户调用流程 | Skill |
| 可自动判定约束 | Hook 或测试 |
| 当前工作 | Delivery、Bug 或 TODO |
| 设计、历史和案例 | `doc/` |
| 重复或失效内容 | 提出删除 |

- 每个正文 Memory 源文件为一次确认单位，单文件允许拆分为完整迁移包。
- `MEMORY.md`、`MEMORY_COLD.md` 和 `_stats.json` 固定视为已退役派生资产，不做语义迁移。
- 无法可靠判断的内容必须阻断并由用户指定，禁止默认处理。
- 确认必须在一次连续流程中完成；中断时不保存任何决定，下次从第一个文件重新确认。
- 确认期间零写入；全部确认后重新校验所有源 hash，再执行单一事务。

## 事务与清理合同

- 同一事务必须包含：新资产写入、最新完整基线安装、已确认旧 Rule/Memory 删除和全部验证。
- 迁移确认同时构成对已列明旧资产删除的授权，不再拆分第二次清理确认。
- 任一转换、写入或验证失败时，必须撤销全部新资产与删除动作，逐字恢复旧 Rule/Memory 和迁移前项目状态。
- 禁止保留部分成功、半迁移目录或提前写版本戳。

## Dynamic latest current-only 基线

- 每次 `$bridgeforge-codex` 先刷新官方产品 home，并以当时最新正式版为唯一完整基线。
- 旧项目版本只用于识别既有骨架身份，不选择历史升级代码路径。
- 旧项目接入时只盘点项目资产、取得逐项决定并直接安装最新完整基线。
- 不再运行、解释或维护旧合同、旧 schema、旧 hash lineage 或逐版本升级分支。
- 没有旧 Rule、项目 Memory 或其他需要决策资产的项目应零确认直接安装最新基线。

## Native Memory 同步合同

### 授权与无感边界

- 首次启用、指定私有仓库或授权范围变化必须取得明确授权。
- 授权有效后，同步、自愈、重试、no-op 和正常升级必须无感：不询问、不弹窗、不注入对话。
- 同文件双机冲突或持续超过 5 分钟的失败允许一次性明确告警；状态未变化时禁止重复刷屏。

### 调度与健康

- SessionStart、Stop、SessionEnd 只登记同步需求；单一后台 worker 合并并发触发。
- 实际内容变化后最多 5 分钟完成同步；无变化必须 no-op 且不产生空提交。
- 死亡锁仅在 PID 已死亡且锁、临时目录均位于受管路径时自动清理，并立即补跑同步。
- `busy` 只能表示等待，禁止计为成功或 `hookRuntimeVerified`。
- 超过 5 分钟仍未同步必须进入 `degraded/failed`，并给出失败原因和唯一修复动作。

### Git 与双机冲突

- 每次实际内容变化创建具有正确远端父提交的普通 commit；禁止无父提交覆盖历史。
- 两台电脑修改不同文件时执行确定性按文件合并。
- 同一文件仅一边修改时采用修改版本。
- 同一文件两边产生不同修改时停止，完整保留两份并由用户决定；禁止按时间戳、本机优先或远端优先静默覆盖。

## 拟修改组件

| 组件 | 修改方向 |
|---|---|
| `skills/bridgeforge-codex/` | 增加 Rule/Memory 逐文件迁移确认和 current-only 路由 |
| `scripts/bridgeforge_codex_project_sync.py` | 增加迁移计划、源 hash 防漂移、单事务写入/删除和整体回滚 |
| `templates/scripts/current_baseline.py` | 删除固定旧基线兼容路由，改为动态最新正式基线合同 |
| `scripts/codex_memory_sync.py` | 修复 Windows 死亡锁、单 worker、5 分钟最终同步、普通 Git 历史、双机合并和健康状态 |
| Native Memory Hook wrapper | 保持隐藏运行，只登记需求并启动/复用单一 worker |
| 同步器用户结果 | 增加逐文件迁移包、冲突、失败和唯一下一步 |
| Template、dogfood、manifest | 同步产品资产与受管 hash |
| `VERSION`、`CHANGELOG.md`、架构文档 | 记录产品传播、迁移边界和发布事实 |

最终精确文件清单由后续只读计划基于当前代码确定；禁止凭确认卡臆测新增模块。

## 验收

| 验收面 | 通过标准 |
|---|---|
| Rule fixture | 混合 Markdown Rule 正确拆分；每个源文件均需确认 |
| Memory fixture | 每个正文文件生成完整迁移包；三个固定派生文件退役 |
| 未确认状态 | 项目逐字节零变化 |
| 完整确认 | 单次事务完成新资产、最新基线和旧目录删除 |
| 漂移 | 任一源 hash 变化即停止 |
| 回滚 | 注入任意失败后完整恢复迁移前状态 |
| Current-only | 任意旧版本直接进入最新完整基线，不执行旧兼容代码 |
| 同步并发 | 多次 Hook 只产生一个 worker |
| 死亡锁 | 自动恢复并在 5 分钟内完成补偿同步 |
| GitHub 历史 | 内容变化产生具有正确父提交的新 commit |
| 双机模拟 | 不同文件自动合并，同文件冲突零覆盖 |
| 当前电脑 | 对真实私有仓库完成一次父子提交和一次 no-op |
| 第二台电脑 | 在另一台真实电脑完成上传、恢复和冲突 smoke |
| 最终关闭 | 自动测试、当前电脑和第二台电脑全部通过，并取得用户验收 |

## 合理假设与风险

- 一次连续确认可能包含数百个文件，存在会话长度和用户操作成本风险；这是用户已确认的取舍，不得改为持久化决定或批量默认批准。
- Agent 负责语义建议，机器负责 source hash、目标边界、事务、回滚和确定性验证；Agent 判断不清时必须 fail-closed。
- 真实 GitHub 验证会写入已授权私有仓库；执行前仍需展示具体影响并遵守外部写入安全门。
- 第二台电脑当前不可访问；在真实 smoke 完成前必须保持 `awaiting_validation`，禁止用模拟结果冒充双机闭环。

## 自动化边界

- 可自动：只读盘点、确定性分类候选生成、hash、防漂移、事务执行、回滚、fixture、同步调度、死亡锁自愈、不同文件机械合并和健康计算。
- 必须用户决定：每个 Rule/Memory 迁移包、无法分类内容、同文件双边冲突、首次 Native Memory 授权、仓库或授权范围变化。
- 禁止自动：语义不明时猜目标、删除未确认内容、静默覆盖冲突、写 Codex 原生 Memory 语义、把旧兼容代码重新带回运行面。

## 实施记录

- 2026-08-31：用户授权按本确认卡进入 `$develop`。
- 2026-08-31：开工预算复核发现 L 级并行交付需要 1 个 `light-explorer`、2 个 `implementation-worker` 和 1 个 `review-auditor`；用户批准 Agent 上限由 2 个扩大为 4 个，其他预算不变。
- 2026-09-01：`$collab` 完成只读研读、两条隔离实现线与主 Agent 串联。项目 Rule/Memory 迁移、current-only 更新、Native Memory 单 worker/普通 Git 历史/冲突保全、Skill 与文档传播均已落地。
- 2026-09-01：版本提升至 `1.6.0`；尚未提交、发布或安装到用户级运行面。
- 2026-09-01：真实单机 GitHub smoke 发现 `gh` 登录失效但系统 Git 凭证有效，补充 Git Credential Manager + GitHub API 无感 private 校验；随后处理旧 parentless 历史 bootstrap conflict，并修复旧 Hook 竞态下的安全决议重放。

## 验证记录

- 项目同步专项：64/64 通过。
- 发布专项：27/27 通过。
- `$bridgeforge-codex` 根 Skill：15/15 通过；项目资产迁移专项：18/18 通过。
- Native Memory 专项：65/65 通过。
- 下游 fixture：5/5 通过，包含迁移事务场景。
- 完整自动测试：共 397 项，其中 396 项通过、1 项跳过、0 失败。
- manifest `--check`：unchanged；`git diff --check`：通过。
- `review-auditor` 先发现 6 项合同缺口；逐项修复并补回归后最终复核为 `no findings`。
- 真实私有 GitHub 单机写入已通过：冲突决议提交 `d73b5be00dfaeb30ec8604e3fbe5000f227097aa`，父提交为 `185fe82b7c557cdde44e0da13580b5808ecb7474`；第二次同步返回 `noop` 且 HEAD 未变化。全程复用系统 Git 凭证，未要求重复登录。
- 尚缺第二台电脑双向 smoke、真实业务下游迁移验收，以及 1.6.0 安装后的 candidate Hook runtime smoke；因此保持 `awaiting_validation`，不得宣称交付已验收。
