# AGENTS.md 第三版持续验证辩论

> 状态：进行中  
> 日期：2026-08-27  
> 确认卡：[`../requirements_2026-08-27_agents-iterative-validation.md`](../requirements_2026-08-27_agents-iterative-validation.md)

## 当前候选

- 工厂根草案：119 行、53 个项目符号。
- Template 草案：101 行、38 个项目符号。
- 新增三个工厂嵌套 `AGENTS.md`、README 公共区、Hook signal、架构说明和语义迁移矩阵。

## 议题

1. 第三版是否已经足够精简？
2. 第三版是否已经足够人类友好？
3. 第三版是否仍有语义损失？

## 参与者

- Agent A：`/root/agents_v3_debate_a`（implementation-worker）。负责证明第三版的结构、载体和语义闭合。
- Agent B：`/root/agents_v3_debate_b`（review-auditor）。负责寻找重复、难读、规则真空和静默弱化。
- 主对话：主持、修订和静态核验。

## 第一轮

### Agent A

- 精简仅通过结构线，未通过“无重复 owner”：工厂根与 `scripts/AGENTS.md`、`skills/AGENTS.md` 重复同步、版本戳、runtime 和模型规则。
- 人类友好未通过：工厂区和嵌套区仍堆叠大量未解释术语；README 还把真实“项目级专区”写成“项目执行上下文”。
- 语义仍有损失：受管资产合同被错误缩到 `scripts/**` 作用域；传播四问、dogfood 唯一证明、目录地图一致性、诚信禁令和操作指南部分细节没有 owner。
- 公共新标题尚未同步 Hook、trusted hash 和 manifest，属于直接实施阻断。

### Agent B

- 精简不通过：根与嵌套同时承载稳定身份、回滚、gap、版本戳和 runtime 声明；减行没有形成唯一 owner。
- 人类友好不通过：导航改善，但同步器术语、模糊入口和复合义务仍然密集。
- 语义不通过：工具规则强度下降，GUI 用户例外、传播四问、项目占位提示、排障输出边界、升级入口和若干操作指南语义缺失。
- `scripts/AGENTS.md` 只在进入目录时加载，不能承载“从根运行同步命令时始终必须看见”的停止边界。
- 根与 README 同时用命令式文字承载 Memory、文档和版本，形成重复 owner。
- 语义矩阵缺少现行行号、作用域、加载时点、硬闸和 draft/installed 状态，不能作为无损证据。

### 第一轮共识

- 第三版三个议题均未通过。
- 第四版不能继续机械删行，必须先解决 owner、作用域和加载时点。
- proposal 若要称为可验证候选，必须给出公共标题、README region、嵌套资产和旧 Hook 指针的完整机器合同草案。

## 第二轮

### Agent A

- 根 `AGENTS.md` 只承担跨目录结果边界：工厂身份、传播四问、上下游 playbook、禁止工厂自用下游同步、版本分类、受管资产身份合同、发布门和真实下游授权。
- 从项目根执行同步时，`scripts/AGENTS.md` 不保证加载；plan/apply、fingerprint、gap、回滚、版本戳和逐文件收据必须由 `$bridgeforge-codex` Skill 承担。
- `scripts/AGENTS.md` 只约束同步器实现和测试；`skills/AGENTS.md` 只约束 Skill 产品分发；`doc/2_bugs/AGENTS.md` 只约束 Bug 关闭证据。
- README 只解释位置和用法，不复制根文件的命令式红线。Memory、文档和版本分别保留一个唯一规则 owner。
- 第四版必须提供可执行的机器合同候选和临时覆盖模拟；传播清单本身不能算验证。
- A 接受 B 的修正：不能新增 `factory/templates/AGENTS.md`，因为真实 `templates/AGENTS.md` 同时是下游根模板，会把工厂规则污染到下游。

### Agent B

- 同意“根=跨目录结果、Skill=从根运行的流程、嵌套=修改该目录时的局部不变量、README=说明”的四分法。
- P0 是修正同步 owner、补齐精确机器合同、把语义矩阵改成逐条 ledger，并消除根/README 与根/scripts 的双 owner。
- P1 是补中文术语入口、恢复明确 Skill 名、恢复最可能根因/证据/验证动作和架构主要风险、补齐 Template 的风控/合规与快速命令范围。
- 静态候选必须在临时 fixture 中验证公共区、项目区、Hook 标题、manifest/hash、README 区域、二次 no-op、旧引用、BOM 和至少两种下游定制程度。
- 真实嵌套加载、生命周期 Hook、真实下游更新/回滚、发布/runtime 和用户试读留给实施阶段，不能伪装成候选验证。

### 收敛结论

第三版不通过。第四版按四分法重构，补齐逐条语义 ledger 与可执行机器合同模拟；完成后换一组独立评审者重新审阅。

进行中。

## 第三轮

仅在核心分歧未收敛时启用。

## 结论与下一版动作

待记录。
