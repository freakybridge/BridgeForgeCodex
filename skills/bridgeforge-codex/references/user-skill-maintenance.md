# 用户级受管 skill 维护

- 活跃目录：`~/.codex/skills/`。
- 活跃账本：`~/.codex/bridgeforge-codex-managed.json`。
- 完整产品 home：`~/.bridgeforge-codex/`。
- Codex 薄入口：`~/.codex/skills/bridgeforge-codex/`，只含入口、references 与 bootstrap updater。
- 仅获准维护后运行薄入口的 `scripts/bridgeforge_codex_shared_update.ps1`，每轮最多一次；只读诊断禁止触发更新或事务恢复。
- updater 只处理 `bridgeforge-codex-manifest.json` 登记的 Codex skills；第三方目录不得修改。
- source 必须来自 GitHub `freakybridge/BridgeForgeCodex` 的 `main` 并逐文件验 hash。
- 产品 home、用户级 `.codex/bin/bridgeforge.exe`、skill stage/swap 与 ledger 必须由同一持久日志决定提交或回滚；恢复时核对组件路径和原始内容哈希。
- 统一提交前失败必须回滚全部组件；提交后备份清理失败不得单独回退 CLI。收据 `cleanup_pending=true` 表示分发已提交、旧备份待清理，后续维护先依据同一日志完成清理，禁止手工删除活动日志。

旧 `$bridgeforge`、旧 Codex/Claude ledger、`.bridgeforge` home 与旧 Claude Skill 已退役，禁止读取、接管或删除；需按安装说明重新安装。正式 Skill、hash 与 ledger 一致性仅由当前 updater 维护。
