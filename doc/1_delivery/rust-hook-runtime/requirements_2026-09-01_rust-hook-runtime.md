---
lifecycle: active
validation_status: awaiting_user_acceptance
delivery_size: L
delivery_budget: 4h / 100k estimated new tokens / 3 subagents / 3 validation rounds
token_measurement: unavailable
source_bug: doc/2_bugs/BUG-windows-codex-hooks-open-visible-terminal.md
next: develop
---

# Rust Hook Runtime 需求卡

> 2026-09-02 范围演进：本卡是先行 Hook 迁移的历史确认范围；后续
> [Rust-only 需求卡](../rust-only-bridgeforge/requirements_2026-09-01_rust-only-bridgeforge.md)
> 已将非 Hook 工具、Native Memory 与测试全部迁入 Rust。本卡中的 Python 保留范围和
> 迁移期 Python 验证命令不再是当前操作合同；Hook 行为与安全要求继续有效。

## 原始需求摘要

用户要求将 BridgeForgeCodex 项目骨架当前注册的 6 个 Hook 及其完整触发链迁移到
Rust。最终 Hook 运行时不得启动 Python，Windows GUI 启动时不得出现可见控制台窗口。
Rust 源码随骨架下发，由 `init`、`adopt` 或 `update` 在写入前使用 Cargo 编译；不安排
Python/Rust 人工对比体验，验证通过后直接启用 Rust 并删除旧 Python Hook 源码。

本卡由 `$develop` 调用 `$confirm` 形成，后续交接 `$develop` 实施与独立审计。

## 目标与非目标

### 目标

1. 保留项目骨架 5 类事件、6 个受管 handler 及其顺序、输入、输出、退出码、timeout 和
   fail-closed 语义。
2. 以一个受管 Rust runtime 承载 dispatcher 与当前 Hook 工作模块的完整行为；Hook 进程树
   不得启动 Python。
3. Windows 入口必须可验证地无可见控制台窗口；非 Windows 入口继续可用。
4. Rust 源码由产品管理；项目同步事务在 apply 前构建并验证目标平台二进制，失败时零产品
   写入或完整回滚，禁止留下半更新状态。
5. 通用能力同步到 Template、工厂 dogfood、受管资产合同、fixture、测试和产品文档。

### 非目标

- 不把 BridgeForgeCodex 同步器、安装更新、发布工具、测试或其他非 Hook Python 工具迁移
  到 Rust。
- 不迁移用户级 Native Memory Hook 或个人小屏状态 Hook。
- 不保留旧 Python Hook 作为运行时 fallback 或长期双实现。
- 不在本轮写入真实下游项目、自动提交、推送或发布。

## 规模与预算

- 规模：L。该改动替换 Hook 运行时、构建交付、受管资产和跨平台启动合同，属于产品架构迁移。
- 上限：4 小时；100k 新增 token 估算，平台无可靠计量器，标记 token 未实测；最多 3 个
  子 agent；最多 3 轮完整验证。
- 超过时间、agent、验证轮次或确认范围任一上限前必须停止，请求扩大预算或缩小范围。

## 迁移前已核实事实（2026-09-01 历史基线）

- `templates/hooks.json` 当前包含 `PreToolUse`、`PostToolUse`、`PostCompact`、`Stop` 和
  `SessionStart` 5 类事件，共 6 个受管 handler。
- 6 个 handler 当前都通过项目 `.venv` 启动 `hook_dispatcher.py`；dispatcher 路由到 16 个
  唯一 Python 工作模块。
- 受管 Hook 目录当前共有 19 个 Python 文件，其中 dispatcher 与 16 个工作模块位于实际
  Hook 触发链，另有 2 个受管辅助检查文件。
- Hook 链直接复用 `project_runtime.py`、`hook_config_policy.py` 与 `archive_scan.py` 的部分
  能力；Rust 实现必须吸收所需行为，不能在 Hook 内反向调用这些 Python 文件。
- 当前模板同时声明 Windows 和非 Windows 命令；替换必须维持跨平台能力。
- 仓库当前没有根 `Cargo.toml`；本机已核实为 Rust/Cargo 1.94.1、Windows x64 MSVC。
- 当前工作区已有 7 处与本需求无关的用户修改，涉及 header-upgrade 文档、`doc/README.md`、
  同步器和测试；必须原样保留并在重叠处兼容。

## 已确认规则

### Runtime 与构建

- Hook 运行时唯一实现为 Rust；成功迁移后，受管 handler 禁止调用 Python、PowerShell 包装器
  或旧 `.py` Hook。
- Rust 源码随公共骨架下发；`init`、`adopt`、`update` 使用目标机器的 Cargo 构建本机二进制。
- Cargo 缺失、版本不满足、构建失败、二进制自检失败或目标路径不安全时，事务必须在受管
  产品写入前阻断或完整回滚。
- 构建只发生在安装或升级阶段；日常 Hook 触发不得调用 Cargo、rustc 或 Python。
- 此先行阶段只退役项目 Hook 的 Python 依赖；后续 Rust-only 阶段已退役其余骨架 Python
  流程，当前不再使用项目 `.venv` 运行 BridgeForge 工具。

### 行为与兼容

- 保持原 6 个 handler id、事件、matcher、执行顺序和 ownership 合同，避免下游配置产生
  重复 handler 或未知漂移。
- stdin 必须按 UTF-8 JSON 读取；stdout 只输出兼容的单一 Hook 响应；诊断写 stderr；退出码、
  timeout、阻断与继续执行语义必须与现行合同等价。
- `apply_patch` 虚拟编辑展开、串行 route、首个失败、SessionStart 聚合失败与上下文合并等
  dispatcher 语义必须保留。
- Windows 二进制必须在 GUI、Codex Hook 和后台非交互启动下无可见控制台，同时保留 stdin、
  stdout、stderr 和退出码；不得以吞掉输出换取隐藏窗口。

### 清理与传播

- 验证通过后删除 Template 与 dogfood 中旧 Python Hook 源码，受管 manifest 同步删除旧资产
  并登记 Rust 源码、构建产物合同和新入口。
- 旧 `.venv` Hook runtime 需求最初由本卡部分替代，随后已由 Rust-only 需求完整替代；
  旧 Python 脚本与用户级 Native Memory Python 入口只作为历史迁移记录保留。
- 本改动属于通用产品能力，必须 bump 根 `VERSION`，在 `CHANGELOG.md` 使用 `[product]`，并
  同步 Template 与本仓库 dogfood 镜像。

## 输入输出映射

| 现有入口 | Rust 目标 | 必须保持 |
|---|---|---|
| `pre-tool` | Rust dispatcher 的同名 route | shell/edit 分类、逐文件虚拟事件、阻断语义 |
| `post-edit` | Rust dispatcher 的同名 route | encoding 先行、后续检查顺序、首错退出 |
| `post-shell` | Rust dispatcher 的同名 route | 测试收据更新与失败语义 |
| `post-compact` | Rust dispatcher 的同名 route | snapshot 行为与输出 |
| `stop` | Rust dispatcher 的同名 route | snapshot 行为与输出 |
| `session-start` | Rust dispatcher 的同名 route | before/after 顺序、错误聚合、状态上下文 |

## 拟修改范围

- 新增 Rust workspace/crate、跨平台 Hook binary 入口、模块化规则实现及 Rust 单元/集成测试。
- 修改 `templates/hooks.json` 与 `.codex/hooks.json`，让 6 个 handler 直接调用已构建 Rust binary。
- 扩展 `scripts/bridgeforge_codex_project_sync.py` 的 plan/apply 事务，使其在受管资产写入前完成
  Cargo 构建、二进制自检、平台目标解析和失败回滚。
- 更新 Template/dogfood、`templates/managed-skeleton.json`、manifest 重建器及相关 fixture。
- 迁移现有 Hook 测试向量并补充零 Python 进程、Windows 无窗口、跨平台命令和失败原子性测试。
- 同步当前架构、旧 runtime 合同、源 Bug、`VERSION`、`CHANGELOG.md` 与 `doc/README.md`。

## 验收标准

1. 6 个受管 handler 全部直接指向 Rust binary，配置与进程树均不出现 Hook Python 入口。
2. 原 dispatcher 和工作模块的代表性成功、阻断、错误及边界 fixture 在 Rust 实现中保持等价。
3. Windows 非交互启动无可见控制台，且 stdin、stdout、stderr、退出码和 timeout 实测有效。
4. 非 Windows 命令与目标产物合同通过自动测试；未执行真实非 Windows runtime 时明确标为
   未验证，不以 Windows 结果代替。
5. init/adopt/update 在 Cargo 缺失、构建失败、自检失败和同步漂移时零写入或完整回滚；成功
   后重复 plan/update 为稳定 no-op。
6. Template/dogfood、manifest、ownership、VERSION、CHANGELOG、文档和 fixture 一致；旧 Python
   Hook 资产从受管范围及源码中删除。
7. Rust 定向测试、Hook 集成测试、完整 factory unittest、downstream fixture、manifest
   `--check`、mirror、metadata、project structure、instruction source 和 `git diff --check` 通过。
8. 独立 agent 复核等价性、Windows 进程行为、同步事务和遗漏风险，Blocker/High 闭合。

## 合理假设与风险

- Cargo 成为安装/升级 BridgeForgeCodex 骨架的新增前置依赖；没有 Cargo 的下游将被明确阻断。
- Windows GUI subsystem 与重定向标准流能否同时满足合同必须以真实进程测试证明，不能只靠
  编译选项推断。
- 非 Windows 真实 runtime 当前可能不可用；自动合同测试不能冒充真实平台验证。
- 现有 Hook Python 实现是迁移期的行为事实源，但不提供用户人工双跑入口；最终以 Rust 单实现
  交付并依靠 Git 历史回退。
- 当前 dirty 文件属于用户，任何重叠修改必须增量合并，禁止 reset、restore 或覆盖。

## 自动化边界

- 允许在本仓库新增/修改源码、测试和文档，并运行 Cargo、项目 `.venv` 测试与临时 fixture。
- 允许使用临时目录模拟下游和失败场景；禁止写真实下游项目或真实用户级 Hook 配置。
- 禁止自动 `git add`、commit、push、发布、删除用户改动或执行破坏性 Git 操作。

## 实施记录

- 2026-09-01：用户确认按本卡开工；进入 `$develop` L 级交付与 `$collab` 只读研读阶段。
- 只读研读与拟拆分见 `collabs_2026-09-01_rust-hook-runtime.md`。研读确认生成二进制需要
  新的受管资产合同；原 2-agent 预算不足以同时满足 implementation 与独立 review，等待预算选择。
- 用户批准预算升级为最多 3 个子 agent、100k 新增 token 估算；4 小时和 3 轮验证上限不变。
- 已新增并同步 Rust crate、5 类事件/6 个受管 handler 的直接二进制注册、Windows GUI
  subsystem 与子进程 `CREATE_NO_WINDOW`、事务构建/回滚/收据合同、fresh-clone dogfood
  bootstrap、worktree pre-commit Cargo 自举，以及 Template/dogfood 镜像。
- 旧 `templates/hooks/*.py` 与 `.codex/hooks/*.py` Hook 源已退役；仍有用途的三个 pre-commit
  Python 检查迁至 `.codex/scripts/`，不再属于 Codex Hook 运行链。
- 独立审计发现的 index 生成物误校验、指令源语义缺口、目录链接越界、子进程窗口、首失败
  顺序、收据伪成功/递归截断、归档计数与 worktree 二进制缺失均已修复并回归。
- 用户后续确认统一目录语义：Rust 源码改用 `templates/hooks/` 与 `.codex/hooks/`，旧
  `hook-runtime` 目录和 `codex.hook-runtime*` 资产 ID 退役；`hooks.json` 继续只承载注册，
  `.codex/hooks/project_*/` 继续承载项目自有扩展。

## 验证记录

- `cargo fmt --check`、`cargo test --locked`：通过；Rust crate 2 项单元测试通过。
- `.venv\Scripts\python.exe -B -m unittest scripts.tests.test_rust_hook_runtime
  scripts.tests.test_generated_hook_build_transaction scripts.tests.test_current_baseline_release
  scripts.tests.test_codex_hook_single_source`：55 项通过；覆盖 PE GUI subsystem、stdio、
  `CREATE_NO_WINDOW`、guard 顺序、junction 越界、迁移后检查、snapshot、收据和生成事务。
- 七模块聚焦组合首跑 155 项，154 项通过；唯一失败为新 worktree 缺 ignored 二进制。加入
  pre-commit Cargo 自举和显式 worktree 根后，失败用例单独复测通过（1/1）。
- `.venv\Scripts\python.exe -B scripts/tests/run_downstream_fixture.py`：5/5 通过，覆盖 init
  幂等、旧项目重建、漂移零写入、项目资产 gap 和事务迁移。
- `build_rust_hook_runtime.py` 返回 `unchanged: 1 generated asset(s) verified`；manifest
  `--check` unchanged；current baseline 1.8.0 通过；config-health strict、instruction source、
  project structure、skill metadata 与 `git diff --check` 均退出 0。
- 完整 factory unittest 首轮共 398 项，暴露的四个本任务失败均已在上述聚焦回归覆盖；另有
  一个 Native Memory 测试受 sandbox Git ownership 影响，属于本卡明确非目标，未以修改
  Native Memory 消除。修复后未再耗时完整重跑 398 项，因此不宣称完整 suite 最终全绿。
- Windows 无窗口已有 PE subsystem=GUI 与所有 Rust 子进程 `CREATE_NO_WINDOW` 的自动证据；
  尚未在真实 Codex Desktop 肉眼观察一次“无弹窗”。Linux/macOS 只有平台合同，未做真实
  runtime smoke。这两项等待用户试用/相应平台验证，不以 Windows 自动测试冒充。
- 目录统一回归：Rust/生成事务/注册/Skill/指令源等聚焦测试 78 项通过；current update
  幂等与新 worktree pre-commit 2 项通过；downstream fixture 5/5 通过。额外验证受管
  `.codex/hooks/src` 不会被误判为项目 Hook，且 `.codex/hooks/project_*` 仍可共存。
