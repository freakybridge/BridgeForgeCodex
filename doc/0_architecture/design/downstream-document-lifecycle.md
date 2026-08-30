# 下游文档生命周期设计

## 单一事实源

每个下游项目的 `doc/README.md` frontmatter 声明 `delivery_layout: flat | milestone`。所有 skill 必须读取该声明；未声明、混用或迁移中时停止并要求用户决定。

Template `templates/doc/README.md` 是生命周期字段、允许值和转换红线的产品单一事实源；安装后的项目 `doc/README.md` 是该合同与项目索引的当前投影。Skill 只引用该合同，禁止各自定义另一套状态词。

`lifecycle` 只表达事项是否仍属当前工作，`validation_status` 只表达验证进度。旧 `status` 与正文状态在完成迁移前仅作证据；缺少 `lifecycle` 的事项必须标为 `unclassified`，禁止猜成 active、completed 或 superseded。

## 目录职责

- `0_architecture/`：系统边界、数据流、接口与 ADR。
- `1_delivery/`：需求确认、计划、验收和正式 debate；flat 为 `<topic>/`，milestone 为 `<M>/<topic>/`。
- `2_bugs/`：Bug 的发现、复现、根因、修复、验证与回归。
- `3_reference/`：外部资料与来源信息。
- `4_archive/`：按原交付层级保存已完成 delivery，按 `bugs/` 保存已解决 Bug。

## 生命周期写入者

- `$confirm` 创建需求卡并写入初始生命周期。
- `$develop` 只推进验证状态，不负责验收关闭。
- `$summary 同意验收` 在全部条件满足时把当前事项结算为 completed。
- `$archive-scan` 只在用户确认移动后写 archived；被替代事项必须保留 `superseded_by`。

## 迁移

新项目初始化时选择布局。已有项目升级时仅输出旧 `1_plan/`、`2_pending/`、`3_design/`、`9_reference/` 的迁移提案；必须逐项展示 `git mv` 目标并等待用户确认。
