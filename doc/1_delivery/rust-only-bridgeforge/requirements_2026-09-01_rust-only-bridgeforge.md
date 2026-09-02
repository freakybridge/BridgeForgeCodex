---
lifecycle: active
validation_status: awaiting_user_acceptance
delivery_size: L
delivery_budget: 12h / 180k estimated new tokens / 3 subagents / 6 validation rounds
token_measurement: unavailable
next: develop
---

# BridgeForge 全面退役 Python 需求卡

## 原始需求摘要

用户要求在现有 Rust Hook 迁移基础上，让 BridgeForge 工厂及其下发的 Codex 公共骨架
整体退役 Python。最终仓库不含 `.py`，安装、更新、Hook、pre-commit、Git 同步、批量升级、
Native Memory 同步和测试均不依赖 Python 或项目 `.venv`。

本卡由 `$develop` 调用 `$confirm` 形成，后续交接 `$develop` 分阶段实施与独立审计。

## 目标与非目标

### 目标

1. 建立共享 Rust 核心库与统一 CLI，承接现有非 Hook Python 生产、维护和验证入口。
2. 保持现有配置、输入输出、退出码、错误语义、事务回滚、ownership、并发与安全边界等价。
3. 保持当前 Windows、Linux、macOS x86_64 合同；目标机器允许安装 Rust/Cargo 并本地构建。
4. 迁移完成后删除 BridgeForge 仓库全部 `.py`、Python 启动包装和 `.venv` 硬依赖。
5. 同步 Template、工厂 dogfood、Skills、manifest、fixture、测试、版本和产品文档。

### 非目标

- 不移除 Rust/Cargo 依赖，不改为发布平台预编译包。
- 不修改下游项目自身的业务 Python 代码或依赖。
- 不削弱既有安全检查、事务、回滚、错误码、跨平台或无窗口要求。
- 不在本轮自动 commit、push、发布、写入真实下游或同步真实远端 Memory。

## 规模与预算

- 规模：L。改动覆盖运行时、项目同步事务、Git、批处理、用户级生命周期 Hook、并发同步和
  全部测试，属于跨子系统架构迁移。
- 上限：12 小时；180k 新增 token 估算，平台无可靠计量器，标记 token 未实测；最多 3 个
  子 agent；最多 6 轮完整验证。
- 预计超过时间、agent、验证轮次或确认范围任一上限时必须停止，请求扩大预算或缩小范围。

## 迁移前已核实事实（2026-09-01 历史基线）

- 当前仓库共有 56 个 Python 文件：下游 Template 脚本 10 个、工厂 dogfood 脚本 11 个、
  工厂根工具 6 个、工厂 batch Skill 脚本 1 个、测试与 fixture 28 个。
- 当前 pre-commit 仍用项目 `.venv` 运行 instruction source、current baseline、project
  structure、skill metadata、factory version 与 manifest 检查。
- `$git-sync`、`$archive-scan`、`$bridgeforge-codex` 和 batch Skill 仍直接调用 Python。
- 项目同步器承担 current-only baseline、aggregate fingerprint、gap、生成资产、事务写入、
  回滚和收据；迁移不能以内联或删减功能替代。
- Native Memory 同步承担长期授权、用户级 Hooks ownership、隐藏 worker、私有 GitHub
  验证、锁、pending、三方合并与冲突恢复，必须作为独立迁移包处理。
- Rust Hook 已声明 Windows、Linux、macOS x86_64 目标，并在目标机器通过 Cargo 构建；
  用户确认继续采用该交付方式。
- 当前工作区包含尚未提交的 Rust Hook 迁移和既有用户修改；实施必须增量兼容，禁止 reset、
  restore 或覆盖。

## 已确认规则

### 运行与交付

- Rust Hook 保持独立、窄职责；新增统一 `bridgeforge` CLI 承接非 Hook 命令，共享纯 Rust
  核心能力，禁止把全部工厂功能塞进 Hook 事件入口。
- 目标机器允许使用 Rust/Cargo 本地构建；Cargo 缺失、版本不满足或构建失败时必须明确阻断。
- Windows GUI、Codex Hook 和后台任务启动必须无可见控制台，并保持标准流、退出码和 timeout。
- Python 实现仅可在迁移期作为差分事实源；对应 Rust 验收完成后必须从生产链删除。

### 等价与清理

- 每个迁移包必须先冻结现有输入、输出、JSON、退出码、错误文案和文件副作用，再做差分验证。
- Python 测试在迁移期临时保留；同等 Rust integration tests 覆盖且差分通过后才能删除。
- 最终 BridgeForge 仓库必须零 `.py`，不得保留 Python fallback、兼容 wrapper 或隐藏入口。
- 历史文档和 CHANGELOG 可以保留“Python”事实描述，但当前操作文档不得再要求 Python 或
  `.venv`。

## 输入输出映射

| 当前入口 | Rust 目标入口 | 必须保持 |
|---|---|---|
| Python 检查脚本 | `bridgeforge check ...` | 检查结果、JSON、退出码、fail-closed |
| `codex_git_sync.py` | `bridgeforge git-sync` | fetch、同步判定、autostash、提交推送与终态收据 |
| `archive_scan.py` | `bridgeforge archive-scan` | 候选、生命周期、索引与用户确认边界 |
| `bridgeforge_codex_project_sync.py` | `bridgeforge project-sync` | plan/apply、指纹、gap、事务、回滚、收据 |
| `batch_control.py` | `bridgeforge batch` | 串行顺序、锁、恢复、共同问题与终态摘要 |
| `codex_memory_sync.py` | `bridgeforge memory-sync` | 授权、ownership、隐藏 worker、Git、锁、合并与冲突 |
| Python unittest/fixture | Rust integration tests | 代表性成功、失败、边界与原子性行为 |

## 拟修改范围与顺序

1. 建立 Rust 公共底座：共享 core、统一 CLI、稳定收据和差分测试框架。
2. 迁移下游检查、pre-commit、`$git-sync`、`$archive-scan`，移除下游 `.venv` 运行硬闸。
3. 迁移工厂 project-sync、asset migration、manifest/build、factory version 与 batch。
4. 独立迁移 Native Memory 同步和用户级生命周期 Hook 启动链。
5. 将 28 个 Python 测试与 fixture 迁移为 Rust integration tests。
6. 删除全部 Python 文件和入口，更新 AGENTS、README、INSTALL、Skills、manifest、VERSION、
   CHANGELOG、架构文档、Template 与 dogfood。

## 验收标准

1. `rg --files -g '*.py'` 在 BridgeForge 仓库返回零文件。
2. 无 Python、无 `.venv` 的干净环境可以完成初始化、更新、Hook、pre-commit 和日常 Skills。
3. project-sync 的 init/adopt/update、check/dry-run 零写、aggregate fingerprint、gap、生成资产、
   drift 阻断和事务回滚保持等价。
4. Git 同步与 batch 的 clean/no-op、ahead/behind、autostash、失败停止、恢复和终态收据保持等价。
5. Native Memory 的授权、私有远端验证、隐藏 worker、并发锁、pending、三方合并与冲突恢复通过。
6. Windows 无可见控制台且标准流、退出码、timeout 有真实证据；非 Windows 未做真实 smoke 时
   必须明确标为未验证。
7. `cargo test`、factory dogfood、完整 fixture、manifest check、project structure、mirror、metadata、
   instruction source 与 `git diff --check` 通过。
8. 独立审计确认没有遗漏 Python 执行入口、功能降级或用户改动覆盖。

## 合理假设与风险

- 当前 Rust Hook 交付先形成稳定基线，再在其上推进全面迁移；不得把未收口状态误当历史基线。
- project-sync 与 Native Memory 是最高风险模块，必须分包迁移并各自取得差分与失败原子性证据。
- Python 实现与测试同时删除会失去等价参照，因此清理只能发生在对应 Rust 测试闭环之后。
- 非 Windows 真实 runtime 可能不可用；静态或模拟结果不能冒充真实平台验证。

## 自动化边界

- 允许修改本仓库源码、测试和文档，运行 Cargo、迁移期项目 `.venv` 测试与临时 fixture。
- 允许在临时目录模拟下游、Git 远端、Memory、并发和失败场景。
- 禁止自动 `git add`、commit、push、发布、破坏性 Git 操作、真实下游写入和真实远端 Memory 写入。

## 实施记录

### 2026-09-02 下发前十二项问题修复

- 用户在完整问题排查后授权“你来处理吧”，本批处理已列明的十二项分发、迁移、Memory 和文档不一致问题；沿用本卡范围与外部写入边界。
- 通用实现和 Skill 进入 Template / skills 并同步 dogfood；产品版本升为 1.8.2，最低 Rust/Cargo 明确为 1.88，更新 CHANGELOG。未写工厂骨架版本戳。
- 修复和验证明细见同 topic 协作记录的“下发前修复”。真实下游、真实 Codex 生命周期、真实跨电脑 Memory 同步与用户验收仍单独保留为未验证，不沿用早期通过结果代替本批验证。

### 2026-09-02 剩余工厂审计修复授权

- 用户确认一并修复 H5–H8、M1–M4，并明确“开始吧”；沿用本卡的目标、非目标、事务等价与验证边界，不重建第二份需求。
- 本批规模 L：涉及项目同步、Git、Hook 状态、Memory 恢复、测试分布和版本配置；本批独立预算为 3 小时 / 60k 新增 token 估算（未实测）/ 最多 3 个子 agent / 3 轮验证。超过任一上限前停止并请求调整。
- 顺序路径：H7 的测试迁移涉及其他各修复模块，无法与产品修复进行文件不重叠的独立并行；只读研读由 light-explorer 执行，随后 implementation-worker 顺序修复，最后 review-auditor 独立复核。主对话负责串联、机械镜像、版本、manifest、文档与最终验收命令。
- 已核实：project-sync apply 缺少事务全程排他锁；Git stash/pull/pop 早于现有锁；Hook 测试收据写失败被吞掉；Memory 恢复在过滤校验后复制全树；工厂版本同步未包含 scripts/tests/Cargo.toml；产品 Rust 源中仍带测试。
- H5/H6 验收：同一资源的第二个事务在首次写入前阻断，锁覆盖漂移重算、写入、回滚和退出；Git 覆盖 fetch/stash/pull/pop/commit/push，保留现有失败与 autostash 收据语义。
- H7/H8 验收：测试实体全部进入 scripts/tests，保留已有断言并补 Hook 生命周期/异常场景；版本发布同步测试 manifest/lock，Template/dogfood 一致。
- M1/M2/M3/M4 验收：状态保存失败可观察且不报告虚假成功；已知旧收据只在受管更新事务内安全退役；恢复仅写经 manifest 认可的快照文件；当前操作资料不再要求 Python/.venv，历史事实原样保留。
- M1 的“Windows rename 不能覆盖”此前并未证实，不将假说当 Bug；先以测试验证，再决定是否需改动。
- 工厂内 fixture 可模拟并发、失败和远端；禁止真实下游/用户级配置/真实 Memory/真实远端写入，禁止提交与推送。
- 本批验证记录见同 topic 协作记录；实现前尚无本批通过结论。

- 2026-09-01：用户确认完整需求卡；等待 `$develop` 最终开工选择。
- 2026-09-01：用户确认开始开发，并将子 agent 上限由 2 个扩大为 3 个；其余预算不变，
  `validation_status` 进入 `in_progress`。
- 2026-09-01：`light-explorer` 完成只读依赖拓扑、测试覆盖和 dirty 重叠研读；形成
  `collabs_2026-09-01_rust-only-bridgeforge.md`，等待用户确认拆分后启动实现。
- 2026-09-01：建立 `bridgeforge-core` 与 `bridgeforge-cli` Rust workspace，迁移 Hook、检查、
  project-sync、Git sync、release、archive、batch、Native Memory、manifest 和生成资产构建链。
- 2026-09-01：删除全部产品/测试 `.py` 与 Python 启动包装，pre-commit、Skills、机械同步 Agent、
  Template 和 dogfood 全部切换到 Rust；业务项目自身的 Python 依赖仍由项目所有。

## 验证记录

- `cargo test --locked --manifest-path templates/hooks/Cargo.toml --workspace -- --test-threads=1`：
  69 项通过；`.codex/hooks` dogfood 同命令另 69 项通过；覆盖 Hook、真实生成资产 init、破坏式重建、
  前瞻基线与版本戳最后写、同步事务回滚、linked worktree、split index、索引并发漂移、真实
  commit hook 拒绝、Git push 前后身份漂移、跨 contract ownership 分类、adaptation 收据生命周期、
  Git push/autostash 失败、Memory 本地远端、三方冲突、安全重放、持久健康度和 batch 串行状态机。
- `cargo test --locked --manifest-path scripts/tests/Cargo.toml`：6 项通过；覆盖零 `.py`、
  Template/dogfood 镜像、manifest、pre-commit/updater/Agent/Skills 零 Python 入口。
- `.codex/bin/bridgeforge.exe build-assets --project-root .`：Windows Hook 与 CLI release 构建、
  自检和收据写入通过；最终 Hook SHA-256 为 `a6d9157b...86ef5f28`，CLI 为
  `0c3e9a08...8d8a2d56`，共同 source tree SHA-256 为 `66221540...9542ef7c`。
- 临时 Git index 执行 `.githooks/pre-commit`：worktree/index baseline、project structure、
  skill metadata、factory version、manifest 全部通过，未改实际暂存区。
- 基线硬闸新增 exact schema、重复 JSON key、Markdown ownership projection、Hook 完整身份集合、
  通用 JSON deep-subset、AGENTS 双区和 Git `check-attr` 最终语义反例；Release 新增 HEAD timeout
  fail-closed、同/跨合同无效基线和 `.gitattributes` adoption 反例，定向测试 17/17 通过。
- 真实下游写入、真实 GitHub Native Memory 写入与非 Windows runtime smoke 未执行，按非目标和
  平台边界保留为未验证。
- 2026-09-02：`review-auditor` 最终只读复核结论为“无阻断”；未发现新的 Python 运行入口、
  用户定制覆盖风险或 Blocker/High，需求进入 `awaiting_user_acceptance`。

- 2026-09-02：用户授权修复真实 git-sync 自动升版后运行收据漂移；实现、独立复核及本批验证证据见同目录协作记录的“git-sync 自动发布事务修复（1.8.3）”。本轮不代表用户验收，不提交或推送工作仓库。

### 2026-09-02 十七项架构扫描修复

- 用户在阅读本轮完整架构扫描结论后明确授权“你直接一起修了吧”；本批以已经列明的十七项为范围，沿用本卡的完整性、失败语义和外部写入边界。
- Memory 内部逻辑由 implementation-worker 在既有独立包边界内修复，主对话负责其余实现、公共入口、传播和文档；review-auditor 独立读取最终实现。
- 产品版本升为 1.8.4。通用能力进入 Template / skills 并镜像 dogfood；工厂批次流程只保留在工厂 Skill。禁止提交、推送、写真实下游或实际原生 Memory。
- 每项修复、对应回归及六类交付证据见同目录协作记录的本批章节。

- 本批最终结果：十七项修复、文档与 1.8.4 产品/dogfood 同步完成；两套 workspace 各 91 项通过，完整工厂 70 项通过，最终加强的进程模块另 5 项通过，独立复核无剩余阻断。命令、收据及首轮失败修正见协作记录。保持待用户验收；真实下游、跨电脑 Memory、真实 Codex 宿主和非 Windows runtime 未验证。
