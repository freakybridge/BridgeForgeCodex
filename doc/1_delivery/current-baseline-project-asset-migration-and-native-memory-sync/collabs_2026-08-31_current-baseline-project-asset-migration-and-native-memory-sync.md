---
lifecycle: active
validation_status: awaiting_validation
record_type: collaboration
requirements: requirements_2026-08-31_current-baseline-project-asset-migration-and-native-memory-sync.md
created_at: 2026-08-31
---

# Current-only 项目资产迁移与 Native Memory 同步协作记录

## 交付基线

- 需求卡：[`requirements_2026-08-31_current-baseline-project-asset-migration-and-native-memory-sync.md`](requirements_2026-08-31_current-baseline-project-asset-migration-and-native-memory-sync.md)
- 规模：L。
- 预算：120 分钟、约 50k token（未实测）、最多 4 个 Agent、最多 3 轮验证。
- 当前阶段：只读研读已完成，等待用户确认并行拆分；产品代码尚未修改。

## 只读研读结论

- 项目资产迁移/current-only 与 Native Memory 同步可以按实现文件完全隔离。
- Rule/Memory 的语义归属必须由 Skill/主对话逐文件提出，确定性脚本只验证用户决定、hash、路径、事务和回滚。
- 旧远端历史使用 parentless force-push；首次新版同步没有可靠三方基线。本地和远端均变化时必须 fail-closed 形成 bootstrap 冲突，由用户选定基线。
- 五分钟合同只能保证“五分钟内成功，或明确进入 degraded/failed”；外部 GitHub/网络故障时禁止宣称必然推送成功。
- 根与 Template `AGENTS.md` 的独立清理授权规则与本需求已确认的同事务删除冲突，主 agent 串联时必须同步收口。

## 并行拆分

| 并行组 | 负责人 | 独占文件 | 目标 |
|---|---|---|---|
| G1-A | `implementation-worker` 项目迁移线 | `scripts/project_asset_migration.py`（新增）、`scripts/bridgeforge_codex_project_sync.py`、`scripts/tests/test_project_asset_migration.py`（新增）、`scripts/tests/test_current_baseline_project_sync.py`、`scripts/tests/run_downstream_fixture.py` | 迁移 manifest、逐源决定、hash/路径硬闸、latest current-only、单事务写入/删除、回滚与 fixture |
| G1-B | `implementation-worker` Native Memory 线 | `scripts/codex_memory_sync.py`、`scripts/codex_memory_sync_hook.ps1`、`scripts/tests/test_memory_native_sync.py` | 单 worker、Windows 死亡锁自愈、普通父子提交、三方按文件合并、冲突保全、五分钟健康与无感 Hook |

两名实现 Agent 禁止修改 Skill、架构文档、根/Template `AGENTS.md`、VERSION、CHANGELOG、manifest 或 dogfood 镜像。

## 接口约定

### 项目资产迁移线

- 语义层生成每个源文件的完整迁移包；脚本接收并验证确定性 manifest，不自行判断语义。
- manifest 必须覆盖每个源文件并携带源 hash、目标、目标正文、退役/删除决定和用户确认状态。
- 三个派生文件固定退役；其他未知/遗漏项 fail-closed。
- planner 零写；Apply 即时 replan；新资产、旧 Rule/Memory 删除、验证和版本戳在一个 `_Transaction` 中。
- 任一失败完整回滚。

### Native Memory 线

- 生命周期 Hook 只登记 pending 并启动/复用单 worker。
- worker 消费期间新增事件；真实 live lock 返回等待，死亡锁经受管路径/PID 证明后自愈。
- 普通 commit 必须以远端 HEAD 为父；禁止继续 parentless force-push。
- `last-synced.commit` 是三方基线；不同路径机械合并，同路径双改生成 conflict ID 并保存两份，禁止覆盖。
- 旧历史无基线的首次双边变化按 bootstrap conflict 处理。
- `busy` 不得算健康；五分钟未完成进入 degraded/failed，并保持一次性告警状态。

## 主 Agent 串联边界

- Skill 与 references：`skills/bridgeforge-codex/**`。
- 公共指令与用户文档：根/Template `AGENTS.md`、`INSTALL.md`。
- 架构：`codex-project-sync.md`、`codex-native-memory-sync.md`、`design-rationale.md`、`sync-from-upstream-playbook.md`。
- current-only 检查器及 dogfood 镜像。
- managed skeleton、共享 manifest、VERSION、CHANGELOG、需求卡和本文。
- 定向测试、完整测试、fixture、真实单机 GitHub 验证和后续第二台电脑 smoke 说明。

## 独立审计

两条实现和主 Agent 串联完成后，使用 `review-auditor` 独立读取需求卡、真实 `git diff` 和测试收据，检查遗漏、回归、接口衔接和未验证边界。

## 状态记录

| 阶段 | 状态 | 收据 |
|---|---|---|
| 需求确认 | 完成 | 用户确认需求卡并授权 `$develop` |
| 只读研读 | 完成 | `light-explorer` 给出文件边界、接口和六项现实边界 |
| 拆分确认 | 完成 | 用户确认按 G1-A / G1-B 文件边界并行实施 |
| 并行实现 | 完成 | G1-A 与 G1-B 按独占文件边界完成，未发生交叉覆盖 |
| 主 Agent 串联 | 完成 | Skill、公共 AGENTS、安装/架构文档、manifest、版本与 CHANGELOG 已同步 |
| 独立审计 | 完成 | 首轮发现 6 项，修复后两轮复核收口；最终 `no findings` |
| 验证 | 自动验证与真实单机 GitHub 完成 | 完整自动测试共 397 项，其中 396 项通过、1 项跳过、0 失败；Native Memory 65/65；fixture 5/5、manifest 与 diff check 通过。真实私有仓库冲突合并提交 `d73b5be` 的父提交为 `185fe82b`，二次同步 no-op 且 HEAD 不变；双机、真实下游与 1.6.0 安装后 Hook runtime 待验收 |
