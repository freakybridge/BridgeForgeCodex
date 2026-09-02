---
name: bridgeforge-codex
description: 在 Windows Codex 项目中初始化或事务更新 bridgeforge-codex 协作骨架，并维护受管用户级 skills。用户提到 bridgeforge-codex、Codex 骨架初始化或同步上游模板时使用。
user_invocable: true
argument: 仅支持无参数
---

# bridgeforge-codex

bridgeforge-codex 是 Codex-only 骨架维护入口，只接受无参数 `$bridgeforge-codex`，并且必须由主对话编排。旧 `$bridgeforge`、`switch`、Claude 项目维护和内部参数不属于公开命令面。

## 1. 刷新产品入口

仅支持 Windows；其他平台必须在下载或写入前停止。每轮先从 Codex 薄入口运行一次 updater：

```powershell
$BRIDGEFORGE_CODEX_ENTRY = Join-Path $env:USERPROFILE ".codex\skills\bridgeforge-codex"
$BRIDGEFORGE_CODEX_HOME = Join-Path $env:USERPROFILE ".bridgeforge-codex"
& powershell -NoProfile -ExecutionPolicy Bypass -File `
  (Join-Path $BRIDGEFORGE_CODEX_ENTRY "scripts\bridgeforge_codex_shared_update.ps1")
```

成功收据必须包含 `BRIDGEFORGE_CODEX_SHARED_UPDATE_RECEIPT`。随后重新读取 `$BRIDGEFORGE_CODEX_HOME\skills\bridgeforge-codex\SKILL.md` 并以新版本继续，本轮禁止再次刷新。

薄入口只是 Codex 可发现入口，完整产品只能来自包含 `templates/hooks/Cargo.toml` 与受管 `bridgeforge` 二进制的官方产品 home。updater 失败、产品 home 缺文件、非普通目录、工作树不干净或 origin 不匹配时，读取 [用户级受管 Skill 维护](references/user-skill-maintenance.md) 后停止；禁止从旧用户目录、本地 clone 或当前项目补文件，也禁止读取、迁移或删除旧 BridgeForge/Claude 遗留。

## 2. 判断模式并锁定 Rust 工具

在运行任何写操作前，只读检查版本戳并锁定唯一 `$MODE`：

1. `.codex/.bridgeforge_codex_version` 与 `.codex/.bridgeforge_version` 双戳或非法戳：零写阻断。
2. 恰好一个合法戳：`update`；版本只证明骨架身份，不选择历史升级代码。
3. 没有合法戳但已有 `.codex/`、`AGENTS.md` 或其他骨架资产：`adopt`。
4. 没有骨架身份的空白项目：`init`。

每轮只以刷新后的产品 home 为完整基线；禁止读取旧 manifest、旧 schema、旧 hash lineage 或逐版本迁移链来选择实现。只允许使用 updater 安装并自检通过的受管 Rust CLI：

```powershell
$BRIDGEFORGE = Join-Path $env:USERPROFILE ".codex\bin\bridgeforge.exe"
& $BRIDGEFORGE doctor --product-root $BRIDGEFORGE_CODEX_HOME --json
```

验证成功后锁定 `$BRIDGEFORGE`；本轮禁止切换到项目脚本、Python fallback 或其他本地 clone。Cargo、锁文件或受管二进制不健康时，必须先读取 [Rust runtime preflight](references/runtime-preflight.md)；未取得有效 runtime 收据不得继续。

## 3. 按需处理 Codex 原生 Memories

锁定 `$BRIDGEFORGE` 后运行一次状态检查；健康、pending、no-op 和已展示告警必须静默：

```powershell
& $BRIDGEFORGE memory-sync status --codex-home (Join-Path $env:USERPROFILE ".codex")
```

- `declined`：只记用户级 gap，禁止再次询问或改配置。
- 当前策略的 `approved + enabled + hookInstalled + hookRuntimeVerified`，且无未解决的 `degraded` / `failed` / `conflicted`：no-op。
- 其他授权、安装、runtime 或 disabled 状态：执行任何动作前读取 [Native Memory 状态处理](references/native-memory.md)。

Native Memory 的 safe/risk/gap 必须并入本轮唯一 accumulator；项目骨架更新禁止顺手执行完整 `reconcile`。

## 4. 读取唯一模式手册并生成计划

只读取当前模式对应的一份手册：

| 模式 | 必读手册 |
|---|---|
| `init` | [新项目初始化](references/init.md) |
| `adopt` | [旧骨架受控接入](references/adopt.md) |
| `update` | [当前骨架更新](references/update.md) |

然后只用已锁定的 `$BRIDGEFORGE` 运行 planner：

```powershell
& $BRIDGEFORGE project-sync --project-root . `
  --template-root $BRIDGEFORGE_CODEX_HOME --mode $MODE --output-format combined
```

plan 必须零写入。同步器 `machine` 区负责 fingerprint、safe、risk、gap、blocker 与一次性 `PreservationManifest`；`human` 区负责稳定的用户结果。blocker 必须立即停止；无 risk 的 current baseline 更新零确认。存在 risk 时，主对话只能把所有用户决策合并为本轮一次确认，确认前禁止执行 safe 或 risk 动作。

若 `asset_migration.source_count > 0`，必须读取 [legacy Rule / Memory 逐文件迁移](references/project-asset-migration.md)，在同一连续流程中逐源文件确认完整迁移包，再以 stdin 重新规划。迁移确认本身就是对应旧源删除授权；禁止另问清理确认。

## 5. Apply 与用户结果

准备 Apply 时必须用同一份内存中的迁移 manifest 重新生成 plan；fingerprint 漂移则零写停止并重新展示。只有 fingerprint 与用户选择仍有效时，才读取 [事务与回滚](references/transaction.md) 并按其唯一顺序执行；Apply 必须传入刚生成的 `--plan-fingerprint`，禁止人工 copy、merge、删除或写版本戳。

默认结果必须逐项展示同步器 `human` 区的“结论、待处理事项、下一步”；结论只能使用“已完成、无需处理、可直接执行、等待确认、未完成、已完成但仍有待处理项”六类固定中文状态。safe-only 计划必须显示“可直接执行”，禁止把零确认更新说成“等待确认”。禁止自行改写结论或直接倾倒 `machine` 区的 safe/risk/gap、fingerprint、asset ID、内部枚举与验证流水。只有同步器未覆盖用户追问的背景时，主对话才补充说明；补充说明不得改变同步器结论。

- 成功：说明当前骨架版本；只把本轮真实收据证明产生的未提交骨架文件计入数量，并把 `$git-sync` 作为需要保存到 GitHub 时的唯一下一步。
- no-op：说明当前版本和“本次操作已结束，无需继续处理”，不展示 planner、validator 或健康 Native Memory 明细。
- risk：用白话说明范围、影响、保留项和不可恢复边界，并且只问一个决策问题。
- blocker/失败：直接展示同步器 `human` 区确定的中文停止原因、写入/回滚状态与安全动作；未分类错误只能显示通用中文停止说明，原始异常只保留在 `machine` 技术收据中。禁止把状态码、blocker 列表、原始异常或 traceback 当结论。
- gap/advisory：只有影响当前结果、需要用户操作或会改变后续行为时才展示；等待真实生命周期事件验证时，只说明仍在等待正常使用完成验证。

工作区中无法证明由本轮产生的既有改动，禁止归为“本次升级产生”。需要核对字段级成功条件、Native Memory readiness、回滚或向用户解释技术证据时，读取 [内部技术收据](references/technical-receipts.md)；用户未追问时不得补发整份技术清单。
