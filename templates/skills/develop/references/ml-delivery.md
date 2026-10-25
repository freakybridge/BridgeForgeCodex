# develop M/L 交付

> 仅在 M/L 路径读取。验证规则以 SKILL.md 为准；
> 本文件只维护需求记录、实施状态和试用闭环。

## 1. 维护唯一需求包

1. 复用同一需求卡，记录已有授权、预算、目标、非目标、
   用户可见行为、约束、风险、验收、暂缓项和实施假设。
   禁止重建第二份需求或因缺记录重复确认开工。
2. 长期项目约束按项目规范进入 doc/0_architecture/；
   feature 记录留在需求卡所属的 doc/1_delivery/ topic；
   独立 Bug 进入 doc/2_bugs/。
3. 新增、删除、移动或重命名文档时同步 doc/README.md。
   不创建全局 plan、pending 或独立授权文档。

## 2. 实施与状态

1. 在已授权范围内实施，普通实现细节和事实补全直接更新原记录。
   开始实施时将 validation_status 改为 in_progress。
2. 按主入口选择并执行验证；需要 Agent 时读取 agent-execution.md，
   项目声明特殊收口要求时读取 completion-release.md。
3. 根据实际证据区分实现完成、验证待完成和等待用户验收，
   更新验收项、变更记录及相关既有设计文档。
   validation_status 使用 awaiting_validation 或 awaiting_user_acceptance；
   lifecycle 保持 active，只有 $summary accept 可结算为 completed。
4. 真实范围或风险变化时，按主入口处理受影响的授权与预算；
   修复失败计数不因阶段切换而清零。

## 3. 试用与反馈

1. 交付实际改动、需求卡、验证证据、试用路径和剩余风险。
   项目未要求 structured/prepared 记录时，不为交付新增该类记录。
2. 当前需求内的小 Bug 修复后更新同一需求包，
   仅重验受影响部分；独立 Bug 按项目文档规范另记。
3. 新需求或超出原范围的改动进入新的确认与开发流程。
   用户试用和验收不能由实现者自证。
