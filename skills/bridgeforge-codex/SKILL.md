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

薄入口只是 Codex 可发现入口，完整产品只能来自包含 `templates/` 与 `scripts/bridgeforge_codex_project_sync.py` 的官方产品 home。updater 失败、产品 home 缺文件、非普通目录、工作树不干净或 origin 不匹配时，读取 [用户级受管 Skill 维护](references/user-skill-maintenance.md) 后停止；禁止从旧用户目录、本地 clone 或当前项目补文件，也禁止读取、迁移或删除旧 BridgeForge/Claude 遗留。

## 2. 判断模式并锁定项目 Python

在创建 `.venv` 或运行 Python 前，只读检查版本戳并锁定唯一 `$MODE`：

1. `.codex/.bridgeforge_codex_version` 与 `.codex/.bridgeforge_version` 双戳、非法戳，或已有骨架资产却没有可识别戳：零写阻断。
2. 恰好一个合法戳且版本 `<1.4.31`：`adopt`。
3. 恰好一个合法戳且版本 `>=1.4.31`：`update`。
4. 没有骨架身份的空白项目：`init`。

禁止根据旧合同、目录内容或文件名猜项目身份。已有 `.venv/Scripts/python.exe` 时，用它运行：

```powershell
& .\.venv\Scripts\python.exe -B `
  (Join-Path $BRIDGEFORGE_CODEX_HOME "templates\scripts\project_runtime.py") `
  validate --project-root . --executable .\.venv\Scripts\python.exe
```

验证成功后锁定 `$HOOK_PYTHON`，本轮所有 Python 命令只能使用该解释器。`.venv` 缺失、损坏、低于 3.11、不是 CPython 或路径逃逸时，必须先读取 [项目 Python preflight](references/runtime-preflight.md)；未取得有效 runtime 收据不得继续。

## 3. 按需处理 Codex 原生 Memories

锁定 `$HOOK_PYTHON` 后运行一次只读状态检查：

```powershell
& $HOOK_PYTHON `
  (Join-Path $BRIDGEFORGE_CODEX_HOME "scripts\codex_memory_sync.py") `
  status --project-root .
```

- `declined`：只记用户级 gap，禁止再次询问或改配置。
- 当前策略的 `approved + enabled + hookInstalled + hookRuntimeVerified`：no-op。
- 其他授权、安装、runtime 或 disabled 状态：执行任何动作前读取 [Native Memory 状态处理](references/native-memory.md)。

Native Memory 的 safe/risk/gap 必须并入本轮唯一 accumulator；项目骨架更新禁止顺手执行完整 `reconcile`。

## 4. 读取唯一模式手册并生成计划

只读取当前模式对应的一份手册：

| 模式 | 必读手册 |
|---|---|
| `init` | [新项目初始化](references/init.md) |
| `adopt` | [旧骨架受控接入](references/adopt.md) |
| `update` | [当前骨架更新](references/update.md) |

然后只用已锁定的 `$HOOK_PYTHON` 运行 planner：

```powershell
& $HOOK_PYTHON `
  -B `
  (Join-Path $BRIDGEFORGE_CODEX_HOME "scripts\bridgeforge_codex_project_sync.py") `
  --project-root . --template-root $BRIDGEFORGE_CODEX_HOME --mode $MODE `
  --output-format combined
```

plan 必须零写入。同步器 `machine` 区负责 fingerprint、safe、risk、gap、blocker 与一次性 `PreservationManifest`；`human` 区负责稳定的用户结果。blocker 必须立即停止；无 risk 的 current baseline 更新零确认。存在 risk 时，主对话只能把所有用户决策合并为本轮一次确认，确认前禁止执行 safe 或 risk 动作。

## 5. Apply 与用户结果

准备 Apply 时必须重新生成 plan；fingerprint 漂移则零写停止并重新展示。只有 fingerprint 与用户选择仍有效时，才读取 [事务与回滚](references/transaction.md) 并按其唯一顺序执行；Apply 也必须传入 `--output-format combined`，禁止人工 copy、merge、删除或写版本戳。

默认结果必须逐项展示同步器 `human` 区的“结论、待处理事项、下一步”；结论只能使用“已完成、无需处理、可直接执行、等待确认、未完成、已完成但仍有待处理项”六类固定中文状态。safe-only 计划必须显示“可直接执行”，禁止把零确认更新说成“等待确认”。禁止自行改写结论或直接倾倒 `machine` 区的 safe/risk/gap、fingerprint、asset ID、内部枚举与验证流水。只有同步器未覆盖用户追问的背景时，主对话才补充说明；补充说明不得改变同步器结论。

- 成功：说明当前骨架版本；只把本轮真实收据证明产生的未提交骨架文件计入数量，并把 `$git-sync` 作为需要保存到 GitHub 时的唯一下一步。
- no-op：说明当前版本和“本次操作已结束，无需继续处理”，不展示 planner、validator 或健康 Native Memory 明细。
- risk：用白话说明范围、影响、保留项和不可恢复边界，并且只问一个决策问题。
- blocker/失败：直接展示同步器 `human` 区确定的中文停止原因、写入/回滚状态与安全动作；未分类错误只能显示通用中文停止说明，原始异常只保留在 `machine` 技术收据中。禁止把状态码、blocker 列表、原始异常或 traceback 当结论。
- gap/advisory：只有影响当前结果、需要用户操作或会改变后续行为时才展示；等待真实生命周期事件验证时，只说明仍在等待正常使用完成验证。

工作区中无法证明由本轮产生的既有改动，禁止归为“本次升级产生”。需要核对字段级成功条件、Native Memory readiness、回滚或向用户解释技术证据时，读取 [内部技术收据](references/technical-receipts.md)；用户未追问时不得补发整份技术清单。
