---
lifecycle: superseded
validation_status: verified
superseded_by: ../../2_bugs/BUG-agents-ia/README.md
---

# AGENTS.md 精简与人类友好性评审确认卡

> 状态：已确认  
> 确认日期：2026-08-27  
> 任务规模：L（只读设计评审）

## 目标

评审优化草案中的 `AGENTS.md` 是否已经足够精简，以及文本是否已经足够人类友好，并给出可以直接用于下一轮改稿的明确结论。

## 非目标

- 本轮不替换真实 `AGENTS.md` 或 `templates/AGENTS.md`。
- 本轮不实施根 README 公共区改造。
- 本轮不修改 Hook、Skill、同步器或其他运行时资产。
- 本轮不以“行数下降”直接替代对信息密度、可读性和职责边界的判断。

## 已核验事实

- 现行工厂根 `AGENTS.md` 约 188 行，现行 `templates/AGENTS.md` 约 165 行。
- 方案草案 `proposal/factory/AGENTS.md` 为 91 行，`proposal/template/AGENTS.md` 为 78 行。
- 两份草案的 BridgeForge 公共区逐字一致；工厂草案另含工厂专属区。
- 草案当前只供评审，不参与运行时，不替换真实 Template、Hook、Skill 或文档。
- 已确认 D-01：原“Codex 项目操作指南”未来并入根 README 的 BridgeForge 公共区，不再作为独立文档长期存在。

## 评审规则

- 必须分别回答“是否足够精简”和“是否足够人类友好”，不得合并成笼统评价。
- 必须引用草案中的具体章节或句子，并说明保留、删除、合并或改写的理由。
- 必须区分真正删除复杂度与把复杂度转移到 README、Hook、Skill 或参考文档。
- 必须检查精简后是否损失安全、证据、版本、memory、文档和升级边界等必要红线。
- 人类友好性必须从首次阅读路径、标题可预测性、句子长度、术语密度、动作主体和例外条件等角度判断。
- 两个评审 agent 必须形成真实分歧并相互回应；禁止只写两份互不相干的意见。

## 计划范围

- 主评审对象：
  - `proposal/factory/AGENTS.md`
  - `proposal/template/AGENTS.md`
- 边界参照：
  - `proposal/README.md`
  - 当前根 `AGENTS.md`
  - 当前 `templates/AGENTS.md`
  - 已确认的 README 归属结论 D-01
- 产物：`debates/2026-08-27_agents-md-simplification-and-readability.md`。

## 验收口径

- 对两个议题各给出明确的“是 / 否 / 有条件成立”结论。
- 列出仍然冗余或难懂的具体位置，并给出可执行改法。
- 列出不可继续压缩的必要红线，避免把精简做成约束缺失。
- 说明配套文件是否真正减轻 `AGENTS.md`，还是只把阅读负担搬家。
- 最终建议必须包含优先级、主要风险和下一轮改稿边界。

## 预算

- 时间预算：45 分钟。
- Token 预算：约 20k 新增 token（估算，当前无精确计量收据）。
- Agent 预算：2 个独立 agent。
- 轮次预算：默认 2 轮；仅在核心分歧未收敛时进入第 3 轮。
- 验证预算：主对话完成 1 次静态核验。

## 假设与风险

- 假设草案代表当前准备评审的最新方案。
- 风险一：内容移出 `AGENTS.md` 后，可能只是转移复杂度而非消除复杂度。
- 风险二：过度口语化可能削弱约束的准确性、可执行性或异常边界。
- 风险三：工厂版与 Template 的不同职责可能让单纯行数比较失真。

## 自动化边界

本轮仅允许写入确认卡和辩论记录。任何真实骨架、README、Hook、Skill 或同步器改动，都必须在辩论结论经用户接受后另行实施。
