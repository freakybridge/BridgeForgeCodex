# Codex 原生 Memory 同步架构

## 结论

Codex 原生 `~/.codex/memories/` 由 Codex 官方机制生成和注入。BridgeForgeCodex 不读取其
内部语义、不创建或编辑正文，只在用户明确启用后把整棵目录作为不透明字节快照同步到固定
私有 GitHub 仓库，用于单写入设备之间的最终一致恢复。

项目 `.codex/memory/` 不属于本架构。新骨架不得创建、注入、检索或写入项目 Memory；下游
已有目录只作为 `legacy_project_memory` 原样保留，必须经过扫描、人工确认和独立清理授权。

## 组件与数据流

```text
Codex 官方生成/读取 ~/.codex/memories/
  -> 用户级生命周期 Hook 触发受管 wrapper
  -> wrapper 从当前 Git 项目解析并验证 .venv
  -> scripts/codex_memory_sync.py 读取 opaque bytes 与 manifest
  -> 生成单一 parentless 整树快照
  -> --force-with-lease 更新私有 GitHub main
  -> 其他电脑按整套快照 reconcile
```

- `scripts/codex_memory_sync.py` 是同步实现；用户级 Hook 只负责触发，不解释 Memory 正文。
- Windows `commandWindows` 只调用受管 PowerShell wrapper `codex_memory_sync_hook.ps1`，并以
  `-NoProfile -NonInteractive -WindowStyle Hidden` 启动；wrapper 动态解析当前 Git 根和项目
  `.venv`，禁止把项目 Python 绝对路径持久化到用户配置。旧 `cmd.exe` wrapper 只用于识别并
  迁移历史 handler，不是当前正式入口。
- 用户级 Hook merge 必须保留第三方 handler；BridgeForge 只能替换内容完全匹配的受管旧
  handler，遇到人工漂移时 fail-closed。

## 快照合同

- Memory 文件按 opaque bytes 计算逐文件 hash 和整树 digest；禁止依赖内部 schema。
- 临时读取仓库和发布仓库都必须关闭 `core.autocrlf`，并禁止 attributes、clean/smudge 或
  其他换行转换改变 LF/CRLF。
- 每次远端状态是一份最新整树快照和单一 parentless commit，不做逐文件历史合并。
- 只支持单写入设备；写入使用 `--force-with-lease`，远端已变化时必须停止，禁止静默覆盖。
- 本地目录不存在且远端是合法空 manifest 时返回 `noop`、清除对应 pending、更新收据，
  但不创建空的 `~/.codex/memories/` 目录。
- 本地有文件而远端为空快照时不得静默删除本地内容；损坏 manifest、digest 不一致、symlink、
  junction 或 reparse point 必须 fail-closed。

## 授权、健康与失败语义

- 未获用户明确同意时，禁止启用开关、创建仓库、安装同步 Hook 或写入拒绝之外的配置。
- `hookInstalled=true` 只证明配置存在；只有当前 handler revision 的实际运行收据成功且
  `hookRuntimeVerified=true`，才能报告 Hook runtime 健康。
- SessionStart、Stop 与其他受管触发失败时只告警并保留 pending，后续触发重试；同步失败
  不得阻止 Codex 官方 Memory 的生成、注入或正常会话。
- 合法空快照、非空快照、损坏远端、换行保持、并发 lease、wrapper 入口和第三方 Hook 保留
  都必须由隔离测试覆盖；真实安装或 GitHub 状态只能由真实 runtime 收据证明。

## 不支持范围

- BridgeForge 不创建、编辑、总结、分类或整理原生 Memory 正文。
- 禁止把原生 Memory 与项目 `.codex/memory/` 合并、junction 或逐文件拼接。
- 多设备并发写入、加密与安全擦除不属于当前模型；需要时必须另开设计。

实现历史和现场收据见
`doc/1_delivery/memory-rule-organization/requirements_2026-08-14_memory-governance-native-sync.md`、
`doc/2_bugs/BUG-codex-native-memory-empty-snapshot-reconcile.md` 与
`doc/2_bugs/BUG-codex-desktop-native-memory-powershell-hook-not-entering-python.md`。
