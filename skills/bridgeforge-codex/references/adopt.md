# Codex 旧骨架受控接入

仅在根 skill 判定为 `adopt` 后读取。

1. 只运行 `bridgeforge_codex_project_sync.py --mode adopt` 生成 destructive rebuild 计划。
2. 只有任一合法文件名下可识别的 `<1.4.31` 单戳允许进入；缺戳、双戳和非法戳必须零写阻断，禁止猜测来源或自动补戳。
3. 主对话必须先把 rules、hooks、AGENTS 项目区、Skills 与 legacy `.codex/memory/` 的只读扫描显式分派给 `review-auditor`，再展示完整 `PreservationManifest`。legacy Memory 必须原样保留并报告待迁移，adopt 不得语义读取或直接删除。
4. 所有用户决策项必须逐项选择 preserve 或 delete；逐项确认可以组成本轮唯一确认，但最终 apply 仍必须同时传 `--confirmed-preservation-manifest` 与唯一一次 `--confirmed-risk`。
5. 散落 Hook、非 canonical 命令或无法闭合的目录必须零写阻断。主对话只能把整理显式分派给 `implementation-worker`；它只能先在临时副本或受控前置步骤中整理为 `.codex/hooks/project_XXXX/entrypoint.py` 自包含目录并闭合 `.codex/hooks.json`，然后重新规划和确认。
6. 准备 apply 时返回根入口并读取 `references/transaction.md`。

发现 `.claude/` 或 `CLAUDE.md` 时仅提示“Claude 骨架已停止支持”，不得读取、迁移、
删除。
