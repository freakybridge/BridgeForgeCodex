# Codex 旧骨架受控接入

仅在根 skill 判定为 `adopt` 后读取。

1. 只运行 `bridgeforge project-sync --mode adopt` 生成 latest current-only rebuild 计划。
2. 仅允许“无合法版本戳但已有骨架资产”的项目进入；双戳、非法戳必须零写阻断，禁止猜测或自动补戳。
3. 主对话必须只读盘点 rules、hooks、AGENTS 项目区、Skills 与 legacy `.codex/memory/`；Rule / Memory 进入 `project-asset-migration.md` 的逐文件迁移，其他项目资产进入 `PreservationManifest`。
4. 所有需要用户决定的项目资产必须逐项确认；确认前零写入。最终 apply 传入已确认的迁移 manifest、`PreservationManifest` 与 `--confirmed-preservation-manifest`，但不得在既有逐项确认后追加一次清理确认。
5. 散落 Hook、非 canonical 命令或无法闭合的目录必须零写阻断。主对话只能把整理显式分派给 `implementation-worker`；它只能先在临时副本或受控前置步骤中整理为 `.codex/hooks/project_XXXX/entrypoint.rs` 自包含目录并闭合 `.codex/hooks.json`，然后重新规划和确认。
6. 准备 apply 时返回根入口并读取 `references/transaction.md`。

发现 `.claude/` 或 `CLAUDE.md` 时仅提示“Claude 骨架已停止支持”，不得读取、迁移、删除；`.codex/rules/*.md` 与 `.codex/memory/**` 属于本产品明确接管的迁移输入，不适用该禁读边界。
