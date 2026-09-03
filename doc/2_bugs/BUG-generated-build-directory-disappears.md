---
lifecycle: active
validation_status: in_progress
---

# 骨架生成程序构建时临时路径失效

1.12.2 在真实 Assist 升级的骨架 Hook 编译阶段返回路径不存在；项目未安装。直接失败点为 Cargo 编译时间戳文件，根因尚未确认，不等同于 Assist 配置错误或已证明的外部清理。

范围、预算与证据统一见[修复确认卡](../1_delivery/project-rust-hooks/requirements_2026-09-03_build-directory-failure.md)。缺少源码修复、产品传播、dogfood、fixture、真实下游与 runtime 终态证据前保持开放。

2026-09-03 诊断：原版 1.12.2 不写入计划完整构建成功，11 项并发回归连续 20 次通过；独立审计未找到本案确定性代码缺陷。WPR 权限不足，普通目录事件输出截断且无进程身份。根因未确认，已按确认边界暂停修复与发布，未更改 Assist。
