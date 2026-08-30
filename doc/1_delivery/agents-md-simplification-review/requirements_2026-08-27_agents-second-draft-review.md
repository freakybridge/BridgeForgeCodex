---
lifecycle: superseded
validation_status: verified
superseded_by: ../../2_bugs/BUG-agents-ia/README.md
---

# AGENTS.md 第二版草案复评确认卡

> 状态：已确认  
> 确认日期：2026-08-27  
> 任务规模：L（只读设计评审）

## 原始需求摘要

用户要求对刚落盘的第二版 `AGENTS.md` 草案再次进行 debate，复核是否足够精简、是否足够人类友好，并新增检查是否存在语义损失。

## 目标

分别回答以下三个议题：

1. 第二版是否已经足够精简。
2. 第二版是否已经足够人类友好。
3. 第二版相对现行工厂与 Template 是否存在语义损失。

## 非目标

- 本轮不修改第二版草案。
- 本轮不替换真实 `AGENTS.md`、`templates/AGENTS.md` 或 README。
- 本轮不实现嵌套 `AGENTS.md`、Hook、Skill、manifest 或同步器改造。
- 本轮不执行发布、真实下游写入或 Git 提交同步。

## 已核实事实

- 第二版工厂草案为 98 行、32 个项目符号；现行工厂为 188 行、72 个项目符号。
- 第二版 Template 草案为 79 行、23 个项目符号；现行 Template 为 165 行、50 个项目符号。
- 工厂与 Template 第二版的 BridgeForge 公共区逐字一致。
- 独立 `codex-project-operating-guide.md` 已从 proposal 删除，内容改为 `readme/bridgeforge-public-section.md`。
- README 公共区草案使用 `BRIDGEFORGE:README:BEGIN/END` 标记。
- proposal 内部链接完整，相关草案无 UTF-8 BOM，`git diff --check` 通过。
- 项目结构检查返回 `errors: []`；现有 advisories 与本次草案无关。
- 尚未执行第二版独立评审、runtime smoke 或用户试读。

## 评审规则

- 三个议题必须分别给出“是 / 否 / 有条件成立”的明确结论。
- 精简不能只看总行数；还要检查独立义务数、常驻概念数、适用频率、风险例外和下沉载体可达性。
- 人类友好性必须检查首次阅读路径、标题、句长、术语密度、动作主体、例外和跨文件跳转成本。
- 语义损失必须逐类比较现行与第二版中的安全、证据、版本、Memory、事务、授权、文档、测试和任务控制红线。
- 一条规则即使被改写或下沉，只要适用范围、触发条件、主体、强度或例外发生变化，就必须列为潜在语义损失。
- proposal 中只写“实施时迁移”但尚无真实目标载体的内容，必须区分“方案已映射”和“当前仍存在规则真空风险”。
- 两个 agent 必须引用具体文件和行号，并形成真实交锋。

## 评审对象

- 现行：
  - 根 `AGENTS.md`
  - `templates/AGENTS.md`
- 第二版：
  - `doc/2_bugs/BUG-agents-ia/proposal/factory/AGENTS.md`
  - `doc/2_bugs/BUG-agents-ia/proposal/template/AGENTS.md`
  - `doc/2_bugs/BUG-agents-ia/proposal/readme/bridgeforge-public-section.md`
  - `doc/2_bugs/BUG-agents-ia/proposal/README.md`
  - 相关 Hook signal 和架构草案

## 验收口径

- 对三个议题分别给出结论和证据。
- 列出所有已确认或潜在语义损失，并区分 P0/P1/P2。
- 对每项下沉规则说明目标载体是否已经存在、是否会确定加载、是否有硬闸兜底。
- 给出第二版能否进入真实实施的推荐结论；若不能，列出最小修订边界。

## 预算

- 时间预算：45 分钟。
- Token 预算：约 20k 新增 token（未实测）。
- Agent 预算：2 个独立 agent。
- 轮次预算：默认 2 轮；仅在核心分歧未收敛时进入第 3 轮。
- 验证预算：主对话完成 1 次静态核验。

## 超预算停止点

如果需要增加 agent、超过 3 轮，或扩张到实际实现与 runtime 修复，必须停止并重新确认范围与预算。

## 合理假设与风险

- 假设当前 proposal 第二版就是本次唯一评审基线。
- 风险一：行数下降但独立义务并未真正减少。
- 风险二：白话化可能改变原规则强度或异常边界。
- 风险三：规则虽有迁移计划，但目标载体尚未创建，导致实施时出现规则真空。
- 风险四：README、Skill、嵌套指令与硬闸之间可能形成新的重复事实源。

## 自动化边界

本轮只允许写入确认卡和 debate 记录。任何草案或真实骨架修改都必须在辩论结论经用户接受后另行实施。

## 调用与交接

- 调用来源：用户再次要求 `$debate`。
- 后续交接目标：`$debate`。
- 实施记录：待后续决定。
- 验证记录：待 debate 完成后写入讨论记录。
