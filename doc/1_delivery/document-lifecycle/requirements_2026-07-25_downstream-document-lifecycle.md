---
lifecycle: active
validation_status: in_progress
date: 2026-07-25
---

# 下游文档生命周期重构确认卡

## 目标

让所有 BridgeForge 下游项目使用统一、可归档的文档生命周期：系统架构、需求交付、Bug 修复和外部资料各有唯一位置。

## 已确认规则

- `doc/` 固定为 `0_architecture/`、`1_delivery/`、`2_bugs/`、`3_reference/`、`4_archive/`。
- `1_delivery/` 支持 `flat` 与 `milestone` 两种布局，由 `doc/README.md` 的 `delivery_layout` 明确声明；禁止混用。
- `$confirm` 的确认卡承担需求论证，不新增 `decision.md`；正式 debate 跟随所属 delivery topic。
- 已完成 delivery 与已解决 Bug 经用户确认后归档；既有下游升级必须展示迁移清单，禁止静默移动。
- `templates/doc/README.md` 是生命周期字段与允许值的产品单一事实源；`lifecycle` 与 `validation_status` 分离，旧 `status` 只作迁移证据。

## 验收

- 两套模板、共享 skill、init/update 手册和相关归档/快照 hook 不再依赖旧文档路径。
- 下游可根据 `delivery_layout` 解析确认卡与讨论记录路径。

## IA-11 生命周期合同实施记录（2026-08-30）

- Template 与工厂 `doc/README.md` 已增加统一生命周期合同，并把当前交付入口改为只认 `lifecycle: active`。
- `$confirm`、`$develop`、`$summary`、`$archive-scan` 及确定性扫描器按同一合同衔接；历史需求卡留待后续逐项分类，不在本阶段批量猜测。
- 本卡保持 active，直至历史状态迁移、归档候选与 IA-11 验收全部完成。
