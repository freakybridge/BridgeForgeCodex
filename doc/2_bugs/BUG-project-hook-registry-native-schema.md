---
lifecycle: active
validation_status: awaiting_user_acceptance
---

# 项目 Hook 登记字段不符合 Codex 原生配置格式

## 事实与根因

Assist 的 `.codex/hooks.json` 含有顶层 `bridgeforgeProjectHooks`，Codex 报 unknown field，期望 `description` 或 `hooks`。1.12.0 骨架的说明和 Rust 读取器共同要求此位置，属于通用登记合同错误，不是 vault 业务程序或用户级信任状态的问题。

## 修复与范围

登记迁到项目所有的 `.codex/project-hooks.json`；原生执行命令留在 `.codex/hooks.json`。升级原子迁移旧登记，冲突和输入漂移停止；运行基线、暂存区基线、legacy 迁移及重建保留同步适配。

本轮范围、预算、验收与六类证据统一见[确认卡](../1_delivery/project-rust-hooks/requirements_2026-09-03_hook-registry-compatibility.md)。用户级 3 项待审核不在此次修复范围。未发布和未正式更新 Assist 前，不宣称原截图告警已经消失。
