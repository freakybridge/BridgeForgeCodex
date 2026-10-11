# 项目待办总表

本表记录正式确认的原需求与独立后续事项；已有详细文档仍是需求与验收事实源，不迁移无关历史待办。编号和历史保留，状态/归属以统一合同为准。

| 编号 | 日期 | 内容 | 状态 | 归属 | 关联文档 |
|---|---|---|---|---|---|
| TODO-001 | 2026-10-08 | 正式发布后执行用户级受管分发，确认新 summary 生效及 snapshot/resume 原版安装副本退役；隔离安装已经通过，真实用户级安装尚未执行 | 待执行 | 后续事项 | [来源：Summary 交付](../1_delivery/summary-workflow/requirements_2026-10-08_summary-workflow.md) |
| TODO-002 | 2026-10-08 | 在另一台机器同步仓库，按 summary switch 交接文档实际接续，核验相对路径、授权与本机缓存缺口 | 待验证 | 后续事项 | [来源：Summary 交付](../1_delivery/summary-workflow/requirements_2026-10-08_summary-workflow.md) |
| TODO-003 | 2026-10-08 | 普通 git-sync 按用户输入到最终回复的完整口径复测耗时；不得以 CLI 内部计时替代 | 待验证 | 后续事项 | [来源：轻量发布交付](../1_delivery/git-sync-performance/requirements_2026-10-08_prepared-release.md) |
| TODO-004 | 2026-10-09 | refactor-scan 正式分发后的真实业务项目验证仍待完成；2026-10-11 工厂实践已扩展至 R01–R06 并验收，R05 完成等价重构，R06 完成基线并采纳暂缓优化结论 | 待验证 | 后续事项 | [来源：Skill 验收](../1_delivery/refactor-scan/requirements_2026-10-09_refactor-scan.md)、[R01–R04](../1_delivery/project-map-refresh/requirements_2026-10-11_R04.md#累计验收与发布授权)、[R05](../1_delivery/project-sync-content/requirements_2026-10-11_R05.md#用户验收与发布授权)、[R06](../1_delivery/memory-snapshot-baseline/requirements_2026-10-11_R06.md#用户验收与结论) |
| TODO-005 | 2026-10-09 | 正式发布后受管安装新版 develop，并在真实开发中核验 Q1–Q5 与实际耗时；当前仅完成规则重构、审计和工程检查，未验证安装后的行为与提速；先安装再验证（原组合状态为待执行／待验证） | 待执行 | 后续事项 | [来源：develop 交付](../1_delivery/develop-demand-discovery/requirements_2026-10-09_validation-refactor.md) |
| TODO-006 | 2026-10-11 | R07：统一 TODO 增改查权限、七个状态与归属，统一正式需求、TODO 与归档判断；完成条件以需求卡为准 | 进行中 | 原需求 | [需求：R07](../1_delivery/todo-lifecycle/requirements_2026-10-11_R07.md) |
| TODO-007 | 2026-10-11 | R08：严格 JSON 解析器归位与消费者迁移已完成；工程检查和独立审计通过，2026-10-11 用户验收；真实下游、自然 Hook 和用户级安装未验证 | 已完成 | 原需求 | [需求：R08](../1_delivery/shared-json/requirements_2026-10-11_R08.md)、[验收：TODO-007](../1_delivery/shared-json/requirements_2026-10-11_R08.md#用户验收与发布授权) |
| TODO-008 | 2026-10-11 | 发布累计 R07+R08：受管 release 预览 2.2.2 → 2.3.0；实际 fetch origin 因 SSL_ERROR_SYSCALL 失败，尚无提交/推送，版本仍 2.2.2；具体网络根因未确认，网络恢复后使用同一受管入口继续 | 待执行 | 后续事项 | [来源：发布失败现场](../1_delivery/shared-json/requirements_2026-10-11_R08.md#正式发布结果) |
