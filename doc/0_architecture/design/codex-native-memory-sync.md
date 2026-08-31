# Codex 原生 Memory 同步架构

## 结论

Codex 原生 `~/.codex/memories/` 由 Codex 官方机制生成和注入。BridgeForgeCodex 不读取其
内部语义、不创建或编辑正文，只在用户明确启用后把整棵目录作为不透明字节快照同步到固定
私有 GitHub 仓库，用于多台受信电脑之间的最终一致恢复。

项目 `.codex/memory/` 不属于本架构。新骨架不得创建、注入、检索或写入项目 Memory；下游
已有目录由 `$bridgeforge-codex` 逐文件迁往其他项目资产；确认后的迁移与删除属于同一事务。

## 组件与数据流

```text
Codex 官方生成/读取 ~/.codex/memories/
  -> 用户级生命周期 Hook 只登记 pending
  -> wrapper 从当前 Git 项目解析并验证 .venv
  -> 启动或复用单一隐藏 worker
  -> scripts/codex_memory_sync.py 读取 opaque bytes 与 manifest
  -> 以 last-synced commit 做逐路径三方合并
  -> 以远端 HEAD 为父创建普通 commit 并 fast-forward 推送
  -> 其他电脑在下一次 worker 中合并或恢复
```

- `scripts/codex_memory_sync.py` 是同步实现；用户级 Hook 只负责触发，不解释 Memory 正文。
- Windows `commandWindows` 只调用受管 PowerShell wrapper `codex_memory_sync_hook.ps1`，并以
  `-NoProfile -NonInteractive -WindowStyle Hidden` 启动；wrapper 动态解析当前 Git 根和项目
  `.venv`，禁止把项目 Python 绝对路径持久化到用户配置。旧 `cmd.exe` wrapper 只用于识别并
  迁移历史 handler，不是当前正式入口。
- 用户级 Hook merge 必须保留第三方 handler；BridgeForge 只能替换内容完全匹配的受管旧
  handler，遇到人工漂移时 fail-closed。

## Git 与合并合同

- Memory 文件按 opaque bytes 计算逐文件 hash 和整树 digest；禁止依赖内部 schema。
- 临时读取仓库和发布仓库都必须关闭 `core.autocrlf`，并禁止 attributes、clean/smudge 或
  其他换行转换改变 LF/CRLF。
- 内容无变化必须 no-op；内容变化必须形成以远端 HEAD 为父的普通 commit，禁止 parentless commit 或 force-push 覆盖历史。
- `last-synced.commit` 是三方基线：不同路径双机修改自动合并；同路径只有单边修改采用修改版；同路径双边修改停止并保存 local / remote 两份。
- 旧 parentless 历史首次没有可信基线且两侧均变化时形成 bootstrap conflict，禁止猜测整树新旧。
- 冲突形成后若远端 HEAD 发生变化，只有新远端内容逐字节等于冲突包中的 captured local 时，才允许把已确认决议重放到新 HEAD；任何其他变化必须停止并重新取证。
- 任一正常 `push`、`restore`、`merge` 或 `noop` 完成后必须清除过期 active conflict；冲突证据包继续保留用于审计。
- 冲突形成后若远端 HEAD 发生变化，只有新远端内容逐字节等于冲突包中的 captured local 时，才允许把已确认决议重放到新 HEAD；任何其他变化必须停止并重新取证。
- 任一正常 `push`、`restore`、`merge` 或 `noop` 完成后必须清除过期 active conflict；冲突证据包继续保留用于审计。
- 本地目录不存在且远端是合法空 manifest 时返回 `noop`、清除对应 pending、更新收据，
  但不创建空的 `~/.codex/memories/` 目录。
- 本地有文件而远端为空快照时不得静默删除本地内容；损坏 manifest、digest 不一致、symlink、
  junction 或 reparse point 必须 fail-closed。

## 授权、健康与失败语义

- 未获用户明确同意时，禁止启用开关、创建仓库、安装同步 Hook 或写入拒绝之外的配置。
- 每次同步必须验证固定 GitHub 仓库身份和 private 状态；`gh` 不可用或登录失效时允许无感复用系统 Git 凭证访问 GitHub API，凭证禁止落盘、回显或进入错误信息。
- 每次同步必须验证固定 GitHub 仓库身份和 private 状态；`gh` 不可用或登录失效时允许无感复用系统 Git 凭证访问 GitHub API，凭证禁止落盘、回显或进入错误信息。
- `hookInstalled=true` 只证明配置存在；只有当前 handler revision 真实登记 pending / worker 收据和同步健康共同成立，才能报告 runtime 健康；`busy` 不等于成功。
- SessionStart、Stop 与 SessionEnd 只登记需求，单 worker 合并重复触发。死亡 PID 与受管临时目录可自动验证并自愈；五分钟未完成必须进入 `degraded` / `failed`。
- 同一失败或冲突只在下一次 SessionStart 或 `$bridgeforge-codex` 告警一次；状态未变化时禁止重复输出。同步失败
  不得阻止 Codex 官方 Memory 的生成、注入或正常会话。
- 合法空快照、非空快照、损坏远端、换行保持、并发 lease、wrapper 入口和第三方 Hook 保留
  都必须由隔离测试覆盖；真实安装或 GitHub 状态只能由真实 runtime 收据证明。

## 不支持范围

- BridgeForge 不创建、编辑、总结、分类或整理原生 Memory 正文。
- 禁止把原生 Memory 与项目 `.codex/memory/` 合并、junction 或逐文件拼接。
- 加密与安全擦除不属于当前模型；需要时必须另开设计。

实现历史和现场收据见
`doc/1_delivery/memory-rule-organization/requirements_2026-08-14_memory-governance-native-sync.md`、
`doc/2_bugs/BUG-codex-native-memory-empty-snapshot-reconcile.md` 与
`doc/2_bugs/BUG-codex-desktop-native-memory-powershell-hook-not-entering-python.md`。
