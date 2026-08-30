---
lifecycle: active
validation_status: awaiting_user_acceptance
scale: M
date: 2026-08-26
source: develop -> confirm
---

# Windows Hook UTF-8 项目 Memory 加载确认卡

## 原始需求摘要

修复 Windows GBK 环境下 bridgeforge-codex 项目 memory 在 SessionStart 期间无法进入模型上下文的问题，并使用用户点名的四个下游项目进行加载验证。

## 目标

- 由公共 Hook dispatcher 为所有 Python 子步骤强制提供 UTF-8 运行环境。
- 不依赖用户手工设置 `PYTHONUTF8` 或 `PYTHONIOENCODING`。
- 在 `PYTHONUTF8=0` 的外层环境中，项目 memory 仍能形成有效的 SessionStart `additionalContext`。
- 以四个点名下游项目的当前 HEAD 创建隔离测试 worktree，验证真实项目 memory 内容。

## 不做

- 不修改项目 memory 正文或转换其文件编码。
- 不修改用户级 Codex 配置。
- 不修改四个点名下游的现用检出目录或业务代码。
- 不提交或推送工厂、下游改动。

## 规模与预算

- 规模：M。核心逻辑集中，但涉及公共 Template、工厂 dogfood、发布元数据、回归测试和四个真实下游验证面。
- 预算：45 分钟；约 20k 新增 token（平台无可靠计量，未实测）；最多 1 个独立审计 agent；最多 2 轮验证。
- 超预算停止点：需要扩大到用户级 Native Memory、改变 Hook 注册合同、修改下游业务资产，或第二轮修复后仍不能闭合真实下游验证。

## 已核实事实

- 公共 `hook_dispatcher.py` 使用 UTF-8 解码父进程捕获的子进程输出，但没有给子进程设置 UTF-8 环境。
- `memory_context.py` 以 `ensure_ascii=False` 输出完整项目索引；Windows GBK 无法编码索引中的 `🔍`、`²` 等字符。
- BridgeForgeCodex、StratusAgent 主检出、StratusAgent `4d74` worktree、CausisRiskSuite 和 BridgePersonalAssist 均可在 `PYTHONUTF8=0` 下复现 `UnicodeEncodeError`。
- 四个点名下游均有项目 `.venv`、项目 memory 脚本、dispatcher，且当前骨架版本为 `1.5.2`。

## 已确认规则

- 修复位于骨架公共 dispatcher，不要求下游 memory 避开特殊字符。
- Template 是公共产品事实源；工厂 `.codex` dogfood 必须同步。
- 真实下游写入只发生在专用临时测试 worktree，现用目录保持不变。
- 下游验证不包含 commit 或 push。

## 拟修改

- `templates/hooks/hook_dispatcher.py`：集中设置子进程 UTF-8 环境。
- `.codex/hooks/hook_dispatcher.py`：同步工厂 dogfood。
- `scripts/tests/test_codex_hook_single_source.py`：增加 GBK 外层环境回归。
- `VERSION`、`CHANGELOG.md` 与派生 manifest：发布 patch 版本并闭合产品传播。
- 本确认卡、Bug 记录与 `doc/README.md`：记录需求和六类验收证据。

## 验收

1. 定向测试证明 dispatcher 在外层 `PYTHONUTF8=0` 时仍让子脚本使用 UTF-8，并保留非 GBK 字符的 `additionalContext`。
2. 工厂 dogfood、manifest、结构、mirror、fixture 和完整自动测试通过。
3. 基于以下四个项目 HEAD 的隔离测试 worktree 均能在外层 `PYTHONUTF8=0` 时运行 SessionStart dispatcher，退出码为 0，输出包含 `[project-memory index]`：
   - `D:\Quant\StratusAgent`
   - `D:\Quant\CodexWorktree\4d74\StratusAgent`
   - `D:\Quant\CausisRiskSuite`
   - `D:\Quant\BridgePersonalAssist`
4. 独立审计核对实现、传播、回归和未验证边界。

## 合理假设与风险

- 本轮以 dispatcher 的真实 SessionStart JSON 输出作为 Hook 加载收据；若无法启动新的 Codex Desktop 任务，则“桌面对话已实际接收”单独标为未验证。
- 下游临时 worktree 必须通过官方项目同步器应用本地工厂模板，禁止手工复制受管资产。
- 临时 worktree 清理只针对本轮创建且已核对的路径。

## 实施记录

- `templates/hooks/hook_dispatcher.py` 与 `.codex/hooks/hook_dispatcher.py` 在启动所有 Python 子 Hook 前复制父进程环境，并固定 `PYTHONUTF8=1`、`PYTHONIOENCODING=utf-8`；父进程其余环境保持不变。
- `scripts/tests/test_codex_hook_single_source.py` 新增回归：外层显式 `PYTHONUTF8=0` 且删除 `PYTHONIOENCODING`，子 Hook 仍能输出 `🔍²`，并回报收到的 UTF-8 环境。
- 产品版本由 `1.5.2` 升至 `1.5.3`；`CHANGELOG.md`、Template manifest 与 dogfood manifest 已同步。
- 四个下游只在本轮专用临时 worktree 中应用 `1.5.3`，验证后已移除；四个原检出目录均保持 `changes=0`。

## 验证记录

- 基线：四个下游在 `PYTHONUTF8=0` 下均复现 `UnicodeEncodeError`。
- 定向回归：`python -B -m unittest scripts.tests.test_codex_hook_single_source.HookSingleSourceTest.test_dispatcher_forces_utf8_for_child_hook_when_parent_utf8_is_disabled`，`Ran 1 test`，`OK`。
- manifest：`python -B scripts/rebuild_shared_skill_manifest.py --check`，回报 `unchanged`。
- 工厂 fixture：`python -B scripts/tests/run_downstream_fixture.py`，`current-init-idempotent`、`old-project-confirmed-rebuild`、`current-drift-zero-write-block` 全部通过。
- 四个隔离下游均以外层 `PYTHONUTF8=0` 运行实际 `session-start` dispatcher，退出码均为 0，均包含 `[project-memory index]`，均无 `UnicodeEncodeError` 和 `PYTHONUTF8: OFF`：StratusAgent 主检出上下文 5667 字符；StratusAgent `4d74` 上下文 5629 字符；CausisRiskSuite 上下文 3231 字符；BridgePersonalAssist 上下文 5016 字符。
- 当前 BridgeForgeCodex 桌面对话在修复后的真实 SessionStart 中实际收到 `[project-memory index]`；这是桌面注入证据，不只是脚本输出。四个下游各自新建桌面对话的 UI 图标未逐一验证，不能据此宣称四个 UI 图标均已出现。
- 完整自动测试：首次全量运行 `Ran 288 tests in 460.931s`，唯一失败是沙箱用户触发 Git `dubious ownership`，另有 `skipped=1`；该唯一失败项改由真实 Windows 用户单独复跑，`Ran 1 test`，`OK`。这证明 288 项产品断言全部通过，但不冒充第二次无条件全量绿灯。
- 发布硬闸定向组：dogfood、项目结构、mirror、skill metadata 与版本单一事实源共 `Ran 34 tests`，`OK`。
- 独立审计：额外复跑 59 项相关测试、manifest `--check` 和 `git diff --check`，全部通过；结论为无阻塞问题。剩余边界仅是未在四个下游逐一新建 Codex Desktop 对话观察 UI memory 图标。
