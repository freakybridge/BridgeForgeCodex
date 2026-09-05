# develop M/L 交付流程

> 仅在 M/L 需求已记录且实施已授权后读取。Agent 选择和独立 review 规则见
> `agent-execution.md`；S 级禁止读取本文件。

## 1. 完善唯一需求包

1. 在同一需求卡记录实施、验证及授权依据，禁止重建需求或重复确认开工。
2. 项目级长期约束更新 `doc/0_architecture/`；单 feature 保持在确认卡所属的
   `doc/1_delivery/` topic；新 Bug 写入 `doc/2_bugs/`。禁止创建全局 plan 或 pending 文档。
3. 需求包保留背景与目标、非目标、用户可见行为、约束与风险、验收、暂缓项和实施假设。
4. 新增、删除或重命名 `doc/**.md` 时同步 `doc/README.md`。

## 2. 同步事实

- 已确认范围内的实现细节、事实补全和验证状态直接更新需求包或设计文档；开始实施时把 `validation_status` 从 `not_started` 改为 `in_progress`。
- 完成实现后按证据更新 `validation_status` 为 `awaiting_validation` 或 `awaiting_user_acceptance`，并更新变更记录、每项验收状态及相关设计或 rules；`lifecycle` 保持 `active`，只有 `$summary 同意验收` 可以结算为 `completed`。
- 实质变化按主入口重核受影响授权；用户新指令已明确决定则更新记录，否则暂停相关动作。

## 3. 验证与试用

1. 运行与规模和风险匹配的 lint、类型检查、单测、集成测试或手工验证脚本；M 级禁止无
   依据升级为全量测试。
2. 规则和可执行测试优先于 LLM review。需要 Agent review 时再读取 `agent-execution.md`。
3. 交付改动、需求卡、验证收据、用户试用主路径和剩余风险。
4. 当前需求内的小 Bug 直接修复并更新同一需求包；独立 Bug 建立
   `doc/2_bugs/BUG-<id>-<topic>.md`；新范围进入新的 `confirm` / `develop`。
5. 修复失败与恢复按公共 AGENTS 执行，禁止切换阶段清零计数。
