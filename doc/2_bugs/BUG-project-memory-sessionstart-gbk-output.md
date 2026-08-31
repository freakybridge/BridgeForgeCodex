---
lifecycle: superseded
validation_status: verified
superseded_by: ../1_delivery/project-memory-retirement/requirements_2026-08-30_project-memory-retirement.md
---

# BUG：Windows GBK 阻断项目 Memory SessionStart 注入

## 状态

已修复，等待用户在四个下游的 Codex Desktop 新对话中观察 UI memory 图标。源码、产品传播、dogfood、fixture、真实下游 dispatcher、当前桌面 runtime 和独立审计均已有正向收据。

## 症状与根因

项目 Hook 已运行，但 SessionStart 上下文缺少 `[project-memory index]`。dispatcher 只指定父进程如何解码已捕获输出，没有给 Python 子进程设置 UTF-8 环境；子进程在 Windows GBK 下输出含非 GBK 字符的项目索引时抛出 `UnicodeEncodeError`。

## 修复边界

在公共 dispatcher 集中固定子进程 UTF-8 环境，并同步 Template、dogfood、测试和产品版本；不修改 memory 正文或用户级配置。

## 关闭证据

- 源码：Template 与 dogfood dispatcher 均为子 Hook 固定 `PYTHONUTF8=1`、`PYTHONIOENCODING=utf-8`；定向单测在外层 `PYTHONUTF8=0` 时证明子 Hook 能无损输出 `🔍²`，`Ran 1 test`，`OK`。
- 产品传播：根 `VERSION` 已升至 `1.5.3`，`CHANGELOG.md` 已登记 `[product][repo][meta]` 修复，Template 与 dogfood manifest 已重建且 `--check` 回报 `unchanged`。
- dogfood：`.codex/hooks/hook_dispatcher.py` 与 Template 同步；当前 BridgeForgeCodex 桌面对话在修复后的真实 SessionStart 中收到 `[project-memory index]`。
- fixture：`scripts/tests/run_downstream_fixture.py` 的 `current-init-idempotent`、`old-project-confirmed-rebuild`、`current-drift-zero-write-block` 全部通过。
- 真实下游：基于四个点名项目当前 HEAD 建立临时 worktree，经官方同步器升级至 `1.5.3`；在外层 `PYTHONUTF8=0` 时，StratusAgent 主检出、StratusAgent `4d74`、CausisRiskSuite、BridgePersonalAssist 的实际 SessionStart dispatcher 均退出 0、包含 `[project-memory index]`、无编码错误。四个原检出目录均为 `changes=0`，临时 worktree 已清理。
- runtime：当前 BridgeForgeCodex 有真实桌面注入收据；四个下游有真实 dispatcher 加载收据。尚未为四个下游逐一新建 Codex Desktop 对话检查 UI 图标，因此该 UI 表现明确标为未验证。
- 完整自动测试：全量 `288` 项只有 Windows 沙箱 Git 所有权检查导致的单项失败；同一项以真实 Windows 用户复跑通过。另行执行发布硬闸定向组 `34` 项，全部通过。
- 独立审计：只读审计额外复跑 `59` 项相关测试、manifest `--check` 与 `git diff --check`，全部通过；无阻塞问题。
