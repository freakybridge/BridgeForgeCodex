# AGENTS 第十一版独立复评

> 状态：用户中断，未通过
> 需求卡：`../requirements_2026-08-27_agents-iterative-validation.md`

## 议题

1. 是否足够精简，且每类决定只有一个规范 owner。
2. 是否对人类友好，能从具体事情找到位置与入口。
3. 是否无语义损失，包括主体、强度、触发、顺序、例外和跨平台入口。
4. 自动验证是否同时阻断隐藏规则和避免误伤合法 Markdown。

## 第十版阻断及第十一版修复

| 第十版阻断 | 第十一版处理 |
|---|---|
| 公共区禁止直接修改被弱化 | 恢复禁止主体与强度，并纳入合同 |
| 禁止隐式写用户级配置被缩窄 | 恢复原禁令，并与可重建配置规则并列 |
| 换机触发只剩 README 步骤 | 根文件恢复触发路由，README 继续承载步骤 |
| 工厂快速命令与 Template 目录索引未做来源覆盖 | 补迁移账本、source-target 合同和活动块覆盖门 |
| fence 内字面量 `<!--` 被误伤 | fence / 缩进代码优先于 HTML 注释解析，并加兼容正例 |

## 停止条件

自动合同、结构检查、diff 检查和两名全新独立审阅者全部通过，才把本版标为最终通过。

## 自动收据

- `validate_proposal.py`：PASS；新增公共区、用户配置、换机触发、工厂命令、目录索引和 fence 兼容合同。
- `project_structure_check.py --json`：`errors=[]`；仅有本任务外的既有归档提示。
- `git diff --check`：PASS。

## 独立复评与中断现场

- A 方发现：工厂三条快速命令使用子串包含检查；把 `run_downstream_fixture.py` 改成 `run_downstream_fixture.py.removed` 后，语义合同仍会通过。
- 主线已把该项改成逐行精确、唯一、顺序一致的候选合同。
- 上述最后修补尚未复跑完整 proposal validator；A 方其余检查未收口，B 方未给出最终结论。
- 用户随后明确要求中断讨论，因此本版不得标为最终通过，也不再自动进入第十二版。

## 结论

第十一版未通过。共落盘 11 个 debate 版本；继续以“再也找不到任何新反例”为停止条件没有可证明终点，后续若重启，必须先改成有限、明确、可复核的验收边界。

## 2026-08-28 后续用户决定

- 用户暂定 V11 为后续收口基线。
- 官方加载机制复核确认：从项目根启动的任务不会因后来修改 `scripts/**` 而动态加载 `scripts/AGENTS.md`。
- 后续讨论确认：不能动态加载不等于嵌套指令无价值，`.codex/rules/*.rules` 也只能约束命令权限，不能承载目录语义。
- 最终已知方案改为根级目录读取路由、三个工厂嵌套 `AGENTS.md` 和 Hook/测试硬闸；此前删除 `scripts/AGENTS.md` 的决定撤回。
- 后续核对确认 `$summary` 验收模式仍保留 Claude 式 Markdown `paths:` Rule 生成分支；该分支在 Codex 中无原生加载语义，必须改为根/嵌套 `AGENTS.md`、Hook/测试、`.rules` 命令权限与 Memory/文档五类路由。
- 下游只读抽查确认：StratusAgent 的 21 个和 CausisRiskSuite 的 6 个 `.codex/rules/*.md` 都不是 Codex 原生命令 Rule；两者目前仅靠根 `AGENTS.md` 显式读取索引软路由。BridgePersonalAssist 已无该目录。
- 用户决定把“下游旧 Rule 逐文件迁移”和“Summary Skill 原生路由”纳入 V11 实施范围；本轮只更新 proposal 和交接状态，不写真实下游。

## 2026-08-29 有限边界独立收口

- 用户明确把停止条件收敛为：一名新的独立审计者判断当前 V11、完整候选镜像和卡点 #1/#2 是否存在阻止进入 IA 系列的 P0/P1；不重开无限 debate，也不设计 V12。
- 独立首审发现 1 个 P1：旧 `merge/region/seed` 资产与 incoming `whole` 使用同一 target 时，planner 只记录旧 target 集合，可能误列 safe 并静默删除项目自有内容。
- 候选同步器改为保留 `target -> asset`，并用统一判定区分真正 whole-owned 与部分所有权；新增三种跨策略 risk、未确认零写、confirmed apply、CLI 和二次 no-op 回归。
- 修复后候选同步测试 55/55、下游 fixture 3/3、manifest `--check`、结构门、proposal validator 与 `git diff --check` 均通过。
- 同一独立审计者有限复审结论为 `PASS`：无 P0/P1，允许进入 IA-01～IA-14。此结论不代表真实骨架替换、真实下游、runtime、release 或用户试读已经完成。
