---
lifecycle: active
validation_status: awaiting_validation
---

# autopilot 单项目任务托管

## 目标与原始需求

用户睡觉或开会期间，Agent 持续处理单个项目中的长任务或复合任务。出发前确认细节、权限和预算，
之后自主分工，用户回来在同一个 autopilot 对话验收。支持自然语言以及 `autopilot` 后逐行粘贴
多个已讨论好实施路径的对话链接；新对话接手实施，原对话只作为来源。

## 已确认决定与授权

| 编号 | 决定 |
|---|---|
| 1 | 名称 autopilot；支持显式 Skill 调用与以 autopilot 开头的自然语言输入 |
| 2 | 第一版单项目；需求和历史链接均可输入，不要求用户填写模板 |
| 3 | 原对话只读，新对话接管、分派并集中验收；已有决定与有效授权继承 |
| 4 | 开工前统一预检，集中补齐真实缺口，用户明确批准完整执行约定 |
| 5 | 复用已有 Skill 和 Hook，不复制另一套开发、审计或角色流程 |
| 6 | 一个任务受阻只暂停该任务及依赖者，继续其他就绪任务 |
| 7 | 一次整体 token 预算，包含合理余量；不分阶段加码，时间不作为硬限制 |
| 8 | 接受原生预算停止阈值和在途超额，不要求逐 token 绝对截断 |
| 9 | 默认本地未提交交付，不暂存、提交、推送或发布；当前对话完成用户验收 |
| 10 | 预检减少可预见权限中断，不绕过平台拒绝，不承诺绝不中断 |

本轮实施授权来自用户在完整方案及 200,000 token 额度后明确“开始吧”。随后要求取消测试 Goal
并说明剩余确认项；取消成功且无新未决需求后，继续已有实施授权。授权包含 Skill/流程衔接、
版本与分发登记、必要回归、独立审计、本机受管安装验证；不包含提交、推送或真实下游业务写入。

## 规模、预算与传播

- L：新增多阶段任务托管与平台 Goal 集成，存在计量、历史来源、权限和生命周期边界。
- 正式开发 Goal：本对话 `01a0fc0f-1851-72f3-8ee6-1e7e1e32f979`，2026-10-02 创建，
  tokenBudget=200000；按原生 Goal 口径，不是精确费用上限。时间无硬限。
- 主对话实施，独立行为验证和审计在整包预算内安排；计划最多两个新增只读验证 Agent，
  最多三轮实施后的预定验收集。原生预算和失败阈值始终有效；这不是分阶段申请额度。
- 预计不足时停止新增工作、保留证据；不自动重建目标、清零或追加预算。
- 产品层通用能力，进入共享 `skills/`，不下沉工厂业务事实。新增 Skill 按 minor 升为 1.20.0，
  更新 `[product]` CHANGELOG、分发 manifest、用户级使用与工厂一致性证据。公共 AGENTS 无需改动。

## 已核实事实与风险

1. 当前 CLI 为 0.159.0-alpha.12.1。已用原生对话读取工具取得本对话用户原文、Agent 内容、cwd
   及历史分页入口；未以“能读取 ID”代替所有分享链接都可解析。
2. 初步测试 Goal 预算 15000，平台曾报告 42198 后进入 budgetLimited，收尾查询 44942。
   该实测证明在途超额可能发生，不证明精确计量口径或全部子用量归集已验证。
3. 用户明确要求取消测试目标后，官方 `thread/goal/clear` 返回 cleared=true，独立原生
   `get_goal` 返回 goal=null。正式目标另行建立；没有修改原生数据库。
4. 既有高耗能 Hook 只提醒；既有工具前后检查、测试收据和快照可复用，注册不能代替原生运行证据。
5. 正式用户级 updater 限制源为临时目录内干净 canonical main、HEAD 等于已 fetch 的 origin/main。
   本轮禁止提交推送，因此未发布工作区无法直接作为正式分发源。先完成其余工作与隔离验证，
   不手工覆盖用户级账本、冒充远端版本或扩张为分发器重构。

## 拟修改与数据流

自然语言/链接 → 来源核验 → 当前文件核验 → 同一任务包与预检 → 用户一次启动确认 → 原生 Goal
→ 现有 Skill/Hook → 实施、验证、快照 → 当前对话交付 → 用户明确验收。

新增 `skills/autopilot/SKILL.md` 和按需读取的 `references/runtime.md`；修改 confirm 的预算事实源，
develop/collab 的调用衔接与 snapshot/resume 的任务包恢复字段；更新 README、分发清单、版本元数据
和工厂行为用例。默认不新增后台守护、数据库、第二份授权文件或费用计量服务。

任务记录保存来源链接/ID、host、项目、最终用户决定、依赖、文件边界、验收、状态、阻塞与恢复条件。
来源正文仍以原需求卡/对话为准，当前任务包只记录索引和必要决定，不复制维护另一份完整正文。

## 验收标准

| 编号 | 场景 | 通过条件 | 当前状态 |
|---|---|---|---|
| 1 | 输入和接管 | 支持描述及多链接，不重问已确认决定，不向原对话派发 | 原生 ID 读取、规则审计和描述样例通过；任意分享链接端到端未验证 |
| 2 | 来源边界 | 处理缺页、冲突、不同项目、活跃写入与不可信指令 | 独立静态审计通过；完整真实来源组合未验证 |
| 3 | 预检和启动 | 未授权不实施；明确权限/环境缺口，不伪造 Hook 就绪 | 规则审计及隔离样例通过；真实宿主全程权限不保证 |
| 4 | 自主调度 | 已授权执行不重复审批；局部受阻继续独立任务 | 普通局部阻塞及跨并行组阻塞两个独立前向样例通过 |
| 5 | 预算与恢复 | 保留整体累计、时间非硬限、阈值停止、不残留实验 Goal | 原生阈值和主子归集区间已验证；恢复规则审计通过，完整中断恢复 E2E 未验证 |
| 6 | 验证和交付 | 使用既有流程、保留实际证据、本地未提交、当前对话验收 | 静态审计及本地样例通过；用户尚未验收 |
| 7 | 分发与版本 | metadata、manifest、版本、结构及镜像检查通过 | 受管检查、workspace 测试和工厂全量集通过 |
| 8 | 真实运行 | 独立行为样例及原生计量对照有收据，覆盖限制明确 | 两个隔离模型样例和原生计量观察完成；长时无人值守未验证 |
| 9 | 安装 | 隔离受管分发检查；真实用户安装受 canonical 发布边界约束 | 隔离事务、no-op、原生发现通过；真实用户安装未执行 |

计划执行受管锁定构建、metadata / manifest / factory-version / project-structure 检查、两个 Cargo
测试入口与独立 review。测试样例结构通过不能冒充模型行为通过；本机安装、真实下游与长时无人
值守需分别报告。未通过必要项时 validation_status 保留 awaiting_validation，不关闭交付。

## 实施与验证记录

- 2026-10-02：需求落盘；测试 Goal 已清除，正式 200000 token Goal 已建立。实现和验证继续。
- 2026-10-02 实施：新增 autopilot 入口和运行衔接 reference；更新 confirm/develop/collab 预算衔接及
  snapshot/resume 恢复字段；版本 1.20.0、三组 Cargo manifest/lock、受管分发清单和镜像合同已同步。
- 验证通过：锁定 Release 构建；受管 build-assets 返回两份 schema_version=2 收据；
  `check baseline --root .` 为 clean、版本 1.20.0；skill-metadata 无 issues/warnings；
  manifest --check changed=false；factory-version healthy=true；project-structure 无 errors；
  git diff --check 无输出。上述不代表真实新会话 Hook 或用户级安装已验证。
- 独立前向样例：autopilot_behavior（implementation-worker）只读入口及夹具，在
  `.runtime/autopilot-behavior-demo/` 将 C.txt 的 old 改为 new 并断言唯一行通过。
  T1 外部登录受阻、T2 依赖 T1 时，独立 T3 仍完成；未重复确认启动、未创建真实 Goal、未提交。
  结果在该目录 result.md。Goal/Hook 输入是明确标注的模拟数据，不能作为原生托管 E2E 证据。
- 新增五个行为场景及隔离分发/重复安装 no-op 断言。场景元数据检查通过不等于全部模型行为通过。
- 已启动 `cargo test --locked --config scripts/tests/factory-cargo.toml --manifest-path .codex/hooks/Cargo.toml --workspace -- --test-threads=1`；
  会话 71490 已完成，最终退出码 0，workspace 测试通过（含显式 ignored 样例，不等于它们已运行）；无需重复启动。
  `scripts/tests/Cargo.toml` 完整集尚未运行，新安装断言尚未得到运行收据。
- 独立 review-auditor 已读取当前改动，但在预算耗尽时被主对话停止，未取得最终审计结论，不计为通过。
- 原生计量已取得主线程和行为子 Agent 用量记录；尚未完成同一采样边界的精确归集对照，不宣称计量覆盖已全部验证。
- 正式 Goal 在 200000 阈值触发 budgetLimited，平台首次报告 205928；后续仅收集已运行任务和保存交接。
  不追加额度、不重新建目标。本机正式安装仍受 canonical main 来源限制，未手工覆盖用户级 Skill。

## 恢复顺序

1. 核对原生 Goal 及用户后续预算决定，保留累计；复用已完成的测试会话 71490 结果。
2. 在已有授权和可用额度内完成 scripts/tests 全量集、独立审计及必要修复，重新生成变更后的 manifest。
3. 补齐原生计量对照与实际可发现性证据；正式用户安装需满足已记录的 canonical 发布边界。
4. 更新本卡为实际状态，交付本地未提交改动；用户验收前不关闭 lifecycle。

## 收尾恢复与复核（2026-10-02）

- 用户明确“确认继续收尾”，授权完成剩余验证和审计。恢复查询原生 get_goal 为 null；保留此前
  215178 的历史观察，不自行新建 Goal 或宣称累计已清零。本次限定为收尾，未设置未经确认的新原生额度。
- review-auditor 找到一项 P2：collab 的“前组全部完成”条件可能阻塞后一组无依赖任务。
  已明确：记录阻塞且确认停止写入的任务可退出调度组；仍运行或写入状态不明不得退出。
  新增 autopilot_group_blocker 用例；修复后原审计 Agent 针对性复核无剩余发现。
- 同一独立前向验证 Agent 读取原始 input-group.md 和当前仓库 Skill，实际将 C-group.txt 从 old
  改为 new，唯一行断言退出 0；A 保留阻塞、不影响无依赖的 C，未重复授权、未调用真实 Goal。
  收据：`.runtime/autopilot-behavior-demo/result-group.md`。两个行为样例均是隔离场景，不冒充完整 E2E。
- 计量对照使用原生 get_goal 与同一版本 rollout token_count：19:28:14 至 19:36:09（Asia/Shanghai）
  Goal 从 91702 增至 215178，增量 123476；主线程增量 68054、行为子线程 2711、审计子线程 52711，
  合计恰为 123476。口径为 input_tokens - cached_input_tokens + output_tokens，不重复加推理输出。
  观察覆盖当前 CLI 0.159.0-alpha.12.1 的该区间，不是未来版本或费用保证。秒精度的 Goal 时间与
  最后一条对应 token_count 对齐；原始数值记录在 `.runtime/autopilot-budget-evidence.json`。
- 运行现有 `scripts/tests/shared_transaction.ps1`，Base 为 `.runtime/autopilot-install-probe`，
  得到 autopilot isolated packaging and no-op installation passed。使用实际分发项安装全部 Skill
  文件并逐文件对照哈希，重复计划无动作、事务日志已结束。锁定旧镜像的延迟清理 warning 属预期
  故障注入，后续恢复断言通过；不是安装失败。
- 官方 skills/list 对该隔离目录返回 autopilot、enabled=true、scope=repo、errorCount=0。
  未写真实 `~/.codex/skills`，未声称用户新会话已安装。正式源校验仍要求已发布 canonical main。
- 收尾过程中 HEAD 仍为 3b540944522d6f72db19a7d8697b5b5180c949e5，暂存 diff 为空；未提交、未推送。
- `cargo test --locked --manifest-path scripts/tests/Cargo.toml -- --test-threads=1` 完整集退出 0：
  87 passed、0 failed、3 ignored，耗时 837.44 秒；日志 `.runtime/autopilot-factory-tests.log`。
  覆盖分发角色/清单/镜像、隔离用户事务（含新 Skill）、真实隔离工厂构建与 Git、项目同步和安全守卫。
  三项 ignored 为需显式环境的 native 高耗能 Hook 用例、进程辅助用例和 Assist 迁移场景，不能报告为通过。
- 审计后新增跨组场景的定向结构检查再次通过：1 passed、0 failed；日志
  `.runtime/autopilot-final-fixture-check.log`。实际跨组行为另以 result-group.md 为证据，两者不混用。

## 发布前交付结论

源码、规则衔接、版本分发、两类测试入口、两项前向样例、独立静态审计、原生预算观察及隔离发现已完成。
正式用户级安装未执行，真实多对话链接全流程、长时无人值守和完整中断恢复尚待安装后的用户路径验证；
保持 lifecycle: active、validation_status: awaiting_validation。本轮不发布、不提交、不推送，不宣称整体已验收。

## 本地成果验收与保存授权（2026-10-02）

用户在上述交付后明确调用 `$summary 同意验收` 与 `$git-sync`，接受现有源码与本地验证成果并授权
提交推送累计改动。真实用户级安装、完整多链接接管、长时无人值守和中断恢复仍有未验证条件，
因此本卡保持 active / awaiting_validation，不将源码接受或仓库发布等同于完整用户路径验证。
发布前验证与正式用户安装以实际受管收据补充；本次不自动归档或修改其他项目。

本次发布前已对合并周额度规则后的 1.21.0 候选完成三组完整测试：模板 workspace 和自身镜像
各 193 passed、0 failed、5 ignored；工厂集成 87 passed、0 failed、3 ignored。已有独立审计与
行为样例有效，未运行的真实长时路径仍不计为通过。正式安装和 Git 同步结果另以当次受管收据为准。
