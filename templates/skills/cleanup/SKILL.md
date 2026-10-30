---
name: cleanup
description: 显式调用后清理当前项目的运行产物、旧版本产物及编译缓存，保留本机配置、下载、依赖、当前程序和对应收据；仅适用于已采用三目录合同的项目。
user_invocable: true
argument: 可选 preview，只读查看清理范围
---

# cleanup — 项目产物清理

显式调用本 Skill 即授权执行约定范围，不逐项再次确认。只操作当前项目，不清理用户级安装、系统缓存或其他项目。

1. 从当前项目 AGENTS / INSTALL 确定受管工具入口和准确项目根。消费项目通常使用 `.deps/bridgeforge/bin/bridgeforge.exe`；工厂采用其项目专用入口。
2. 直接执行受管入口的 `cleanup --root <项目根>`。用户要求预览时追加 `--dry-run`，不随后自动执行删除。
3. 由工具核验三目录合同、迁移状态、保护边界、活动进程与文件占用；工具缺失、不支持 cleanup 或返回 blocked 时，报告具体缺口，不改用 shell 递归删除、不自动迁移旧布局或杀进程。
4. 按返回结果报告 completed / partial / blocked、实际删除数量、保留范围和失败路径。逻辑删除字节不是实际磁盘释放量；部分完成不宣称已清空。

范围由受管三目录合同判定：完整保留 `.runtime/config/`、`.runtime/download/` 和 `.deps/`；清空其余 `.runtime/` 内容；`.build/` 仅删除可确认的旧版本产物及编译缓存，保留当前程序和对应收据。项目在 `.codex/artifact-layout.json` 中用精确输出路径的 `cleanup: keep/cache/history` 声明保留、编译缓存或历史产物；保留声明优先。未声明清理角色的业务输出和无法确认归属的文件保留并报告，不按修改时间猜测。根外文件不参与清理，不把任意层级同名目录当作豁免。

使用当前项目工具完成并返回，不改用外部清理执行器，不自动重建工具，不往已清空目录写收据。

$ARGUMENTS
