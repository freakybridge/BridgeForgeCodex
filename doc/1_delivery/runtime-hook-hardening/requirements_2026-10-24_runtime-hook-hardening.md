---
lifecycle: completed
validation_status: verified
---

# 工厂运行时准备与 Hook 正确性、性能

## 来源、授权与任务

用户在 2026-10-24 指定 autopilot 接管两个本地 BridgeForge 对话，并在本对话批准“预算上调至1000万，开始”。此前展示的整包范围、独立审计及禁止项一并继承。唯一任务包保持在本卡。

| 编号 | 来源 | 目标与决定 | 状态 |
|---|---|---|---|
| T1 / TODO-023 | `codex://threads/01a1697e-3bf7-7850-9dfd-652bbb1f7a3d` 补充 Agent 专业术语偏好 | 工厂在能力检查前通过锁定 Cargo 启动当前源码 CLI，按需准备 CLI、Hook 和收据；复用缓存、安装事务，不增加 Git Hook | 已验收 |
| T2 / TODO-024 | `codex://threads/01a1696f-7617-7413-a0d2-f8554d662274` 梳理未审批 Hook 的用途 | 先修路径防护、测试收据、启动配置，再优化扫描；定向验证与独立审计 | 已验收 |

两来源导入时 host=local、cwd=D:\Quant\BridgeForge、status=idle；第二来源最后实施回合 interrupted，未产生代码改动。当前 main@a83732cdd50d19aa7352e777d9299e2292d2041f。原有 templates/user/AGENTS.md 与 user-agents-manifest.json 的专业术语偏好改动保留，不归为本轮成果；该改动沿用发布后由 bridgeforge-codex 受管更新用户配置的既有机制。原发布授权不扩展到本批。

## 规模、预算与运行

L 级整包：涉及 Shell 判定边界、安装事务、生命周期与性能对照。主对话顺序实施，最多一名 review-auditor 只读独立审计。原生 Goal 总上限 10,000,000 token，覆盖本包调研、主/子 Agent、实现、审计、验证、修复与交付，不按阶段清零。预计工作时间 4–7 小时，仅估算无时间硬限制；预算是上限而非消耗目标。

周额度粗估沿用启动确认中明确标注的历史参考 0.152 个百分点/10k token：10,000,000 token 对应约 152.0%。来源为本项目既有交付卡记录的 provisional 系数；未核实当前配置适配，非官方配额或实际扣减。本次预算扩大不代表账户余额可覆盖。Goal 主/子计量覆盖尚未实测，不声称精确整包计量；在途调用和收尾可能超出停止阈值。

预检：原生 get_goal/create_goal 可用；Cargo 1.94.1、rustc 1.94.1 可调用；旧 CLI self-test=3.1.0。Hook 注册表静态核对为五个事件入口；原生自然触发、构建与写入许可尚未实测。保持机器和客户端可访问，不创建定时任务或后台守护器。

## 实施范围与传播四问

1. T1 的准备能力进入 templates/hooks Rust 产品源；工厂调用策略进入根 AGENTS 项目专区和维护文档。git-sync Skill 只增加项目声明入口的通用衔接。
2. T2 是产品通用 Hook 修复，进入 templates/hooks/src；scripts/tests 为工厂测试。源码、Skill 和派生 manifest 同步 dogfood。
3. 实施不改 VERSION。后续单独授权 release 时按实际产品、仓库和元文档改动记录 CHANGELOG。
4. 工厂不运行下游 project-sync adopt/apply，不写骨架版本标记；不修改真实下游、自有 Hook、全局权限、用户级配置或 Memory。

文件边界：templates/hooks/**、skills/git-sync/**、对应 .codex 镜像、scripts/tests/**、相关 manifest、AGENTS 项目专区、INSTALL.md 及既有相关设计文档。

顺序：保留基线 → 正确性修复 → 运行时准备 → 性能优化 → 独立审计及必要修正 → 最终受管构建、定向集成和交付。

## 验收与风险

| 改动 | 验收场景 |
|---|---|
| 路径防护 | 外部复制读取允许；相对外写、绝对外写、UNC、移动外部源阻断；含空格、链接、不同工作目录；已识别写入但目标不明时明确阻断，普通未知命令仍由平台控制 |
| 测试收据 | 字符串/注释/源码搜索不产生收据；直接测试执行正确记录；复合、异步、中断、管道不把 Shell 成功当测试成功 |
| 启动配置 | 保留 local/global/worktree 有效 hooksPath 和默认目录有效 Hook；查询失败零写入；无配置的新项目兼容初始化 |
| 编辑性能 | 普通编辑按目标检查，多文件补丁全覆盖；指令/合同修改触发完整检查；原先忽略目录入口剪枝；无持久化缓存 |
| 运行时准备 | 旧/缺失 CLI 引导；源码/lock/配方/EXE/收据变化识别；健康重复运行不改产物与收据；构建及安装失败、输入漂移、并发和文件占用；check 零写入 |
| 传播 | 模板/dogfood/manifest 一致；隔离首次安装与升级保持项目资产；不执行真实下游 |

性能在相同 release 配置、数据与机器上交替测量新旧产物，首次调用单列，至少 20 个预热后样本，报告中位数、P95、最大值及可取得的扫描数。普通编辑中位数降低至少 60% 为目标；普通 pre-tool 不明显退化。准备和清理不计入 Hook 耗时。优化不改变索引新鲜度和提交前硬闸；普通编辑不再立即报告无关存量问题是明确批准的时机变化。

按影响选择定向单元、隔离集成、性能对照与工厂必要检查，不默认完整全仓回归。最多两轮实质修复验证；工具/权限重试不算新轮次。验证覆盖不足或新风险需要扩大时先说明依据。连续两次实质修复失败、完整 Shell 解析器/新依赖需求、注册或自有接口变化、事务能力不足、预算不足时停止受影响部分并保存证据。

## 交付边界与进度

交付本地未提交改动及验证证据，用户验收留在本对话；不暂存、提交、推送、发布。源码、产品传播、dogfood、fixture、真实下游、自然 runtime 六类分别报告。真实下游和自然生命周期没有证据时标未验证，不模拟原生事件成功。

2026-10-24：已建立原生 Goal，完成启动预检；开始记录与实现，验证尚未开始。

2026-10-24 实施进展：T1 增加 factory build-assets 按需准备与只读检查，复用现有生成构建/缓存、ProjectLock、Git common-dir 锁和共享安装回滚；T2 增加有限命令识别、真实 Git 配置保护、目标编码检查与目录剪枝。首轮独立审计指出 Git 带值选项、嵌套 Shell 工作目录、纯字符串测试收据三个 P2，已修复并补例。Hook 单元集 32 passed / 2 subprocess fixtures ignored；factory_runtime 首批 3 项通过，正在补锁/漂移/占用场景与实际构建、集成和性能对照。上述是阶段证据，不代表整包完成。

## 实施证据与交接（2026-10-24）

### 最终行为与文件

| 任务 | 主要文件 | 实际效果 |
|---|---|---|
| T1 | `templates/hooks/crates/bridgeforge-core/src/factory_runtime.rs`、`generated_assets.rs`、`project_sync.rs`、`git_sync.rs`、`git_sync_plan.rs`、CLI `main.rs` | 独立的工厂准备入口；产品源/dogfood/manifest 身份校验；共享缓存、构建、锁与安装回滚；健康复用和零写入 check |
| T1 调用 | 根 AGENTS 项目专区、INSTALL、README、`skills/git-sync/**` 及镜像 | 准备先于能力检查；最终宿主完成准备后才生成发布分析；只读路径不启动 Cargo |
| T2 | `templates/hooks/src/shell.rs`、`guards.rs`、`post.rs`、`session.rs`、`lib.rs`、`util.rs` 及镜像 | 支持的命令按语义识别写目标和测试程序；保留已有 Git Hook 配置；本次编辑目标检查与指令遍历入口剪枝 |
| 定向验证 | `scripts/tests/unit/core_factory_runtime.rs`、`hook_hardening.rs`、`hook.rs`，`scripts/tests/src/hook_guards.rs`、`hook_hardening_performance.rs` | 路径、收据、配置、安装事务、剪枝/读取计数及可重跑配对 benchmark |

`sync-docs` 已更新既有用户结果合同、项目操作指南和安装说明。Map 的宽泛映射不足以说明具体变化，本次结合实际 diff 和有界文档搜索定位。没有新增第二份设计合同；没有新增依赖或改变 Cargo.lock、VERSION、CHANGELOG、Hook 注册表、自有 Hook。

### 已执行的定向验证

以下 `H` 指 `cargo test --locked --config scripts/tests/factory-cargo.toml --manifest-path .codex/hooks/Cargo.toml`；`F` 指 `cargo test --locked --manifest-path scripts/tests/Cargo.toml`。测试均使用 `-- --test-threads=1`，完整路径名的单项另加 `--exact`。

| 命令/检查 | 结果与覆盖 |
|---|---|
| `H -p bridgeforge-hook --lib` | 32 passed、0 failed；2 个 ignored 为父测试启动的子进程 fixture。覆盖既有路由、Map、编码 index 和本次修复 |
| `H -p bridgeforge-hook hardening` | 内嵌引号修复后 7 passed；无更改部分复用上一行证据 |
| `H -p bridgeforge-core factory_runtime` | 5 passed；check 零写入/零进程、健康重复运行、错误收据部分重建、源码/lock/配方/EXE 漂移、构建/安装自检失败、锁竞争、外部改动保留和 Windows 文件占用回滚 |
| `H -p bridgeforge-core git_sync::tests::factory_` | 8 passed；共享复用提取不改变版本/资产身份、失败回滚与只读缓存边界 |
| `H -p bridgeforge-core runtime::tests::generated_binary_can_replace_its_loaded_image_and_restore` | 1 passed；实际加载中 EXE 的替换及恢复原语 |
| `F build_provenance` | 12 passed；构建输入/配方/收据、无新产物、共享依赖、旧收据退役、项目锁及回滚 |
| `F hook_guards` | 9 passed；1 个旧断言仍要求普通代码补丁报告缺少 AGENTS。按已批准的新触发时机修正为 0 后，仅复测该场景通过；Map 失败、返回码及已完成编辑不回滚的断言保留 |
| `F tests::managed_manifests_are_current_and_python_free` | 1 passed；当前清单及 Skill 分发登记 |
| `F runtime_flows::prepared_public_artifacts_are_shared_and_update_preserves_project_hooks` | 1 passed，约 140 秒；真实受管构建和两个隔离项目的首次安装、升级、自有 Hook 保留 |
| 当前 CLI 的 `check baseline`、`check skill-metadata`、`check factory-version`、`manifest --check`、`check todo`、`check project-structure`、`check instruction-source` | 全部退出 0；baseline clean、版本 5.0.1、manifest changed=false；结构仅保留既有归档/排除目录提示 |

最初的旧测试断言失败与 ENV_LOCK poison 级联已区分，未作为多个业务缺陷处理。build_provenance 与 baseline 曾在 sandbox 的临时受保护路径收到 os error 5；相同命令提升后通过，没有改 ACL、绕过锁或削减断言。预先存在的 release.rs 三处 dead_code 警告和测试增量缓存 hardlink 降级警告未改动，不影响成功退出。

### 实际本地运行时

1. 当前源码 `cargo run --locked --release --manifest-path .codex/hooks/Cargo.toml --bin bridgeforge -- manifest --root .` 成功，重建模板、dogfood 与分发清单。
2. 按 INSTALL 的 `build-assets --factory` 引导成功：`status=prepared`、`prepared=2`、`reused=0`、`written_files=4`。当前 CLI 自检 version=5.0.1，包含 `factory-runtime-prepare-v1`；CLI/Hook 的 `self-test --json` 都成功。
3. 当前 CLI 的 `build-assets --factory --project-root . --check` 返回 current、read_only=true、self_test_executed=false。随后正常准备返回 current、reused=2、prepared=0、written_files=0。两步合计约 1.70 秒；四个产物与收据的 SHA-256 和 LastWriteTimeUtc 全部不变。

当前源码树摘要 `156678ffa1132a29c540ecbaf48dc9bf7fb902e1aca380432b39963ff37e754e`；CLI SHA-256 `091e51e0e9d0676d99acc736b65e4639c4675876ff7c25f1e435d1e9f6b84f4e`；Hook SHA-256 `6e6a218d7cd913aae006724328d28a7b7dc0143657e45241483f85c5693a3048`。真实构建收据在 `.codex/bin/build-receipt-cli.json`、`build-receipt-hook.json`，均为本地忽略产物。

### 最终性能对照

其它构建与测试结束后串行执行 `F hook_hardening_performance::paired_hook_release_benchmark -- --ignored --exact --nocapture --test-threads=1`。用 `BFC_BENCH_OLD`、`BFC_BENCH_NEW` 指向相同 release 配置的新旧二进制，`BFC_BENCH_REPORT` 指向 `.runtime/runtime-hook-hardening/performance-final.json`。该项通过，夹具准备和清理不计入单次 Hook 时间；测试总耗时约 27 秒。

基线是本轮修改前从 5.0.1 源码构建的 Hook，不是原来落后的安装产物；SHA-256 为 `67c7a680538e52a0570e36a02944ba10e2ea0b9b6891e604876cb3d331e3b28d`。两种程序使用相同机器、300 个约 16KB 文本文件、3,000 个原本排除目录内文件。首次调用单列，额外预热 2 次，之后 20 个交替顺序配对样本；每次 stdout/stderr 与原程序一致。首次调用不表示清空操作系统文件缓存后的冷启动。

下表单位为毫秒，旧/新分别展示；P95 使用 20 样本的第 19 个顺序统计量。

| 场景 | 首次 旧/新 | 中位数 旧/新 | P95 旧/新 | 最大值 旧/新 | 中位数降低 |
|---|---|---|---|---|---|
| 普通 pre-tool | 57.14 / 54.79 | 54.11 / 53.28 | 57.60 / 59.11 | 60.00 / 60.26 | 1.54% |
| 普通单文件编辑 | 290.92 / 54.32 | 250.20 / 52.26 | 271.38 / 54.45 | 271.73 / 54.49 | 79.11% |
| 指令文件编辑 | 248.66 / 72.41 | 250.66 / 74.44 | 303.63 / 77.25 | 313.83 / 77.87 | 70.30% |
| 10 文件补丁 | 261.90 / 73.79 | 265.01 / 73.91 | 272.85 / 76.24 | 280.28 / 78.08 | 72.11% |

普通编辑达到至少 60% 的目标；pre-tool 中位数基本持平，P95 约增加 1.5ms、最大值约增加 0.3ms，没有观察到明显退化。结论限于该夹具和本机，本次没有对全部真实下游或 Codex 整体响应延迟作性能保证。首轮含其它测试活动的结果保存在 `performance.json`，最终采用串行复测结果。

### 六类证据与交付边界

| 类别 | 状态 |
|---|---|
| 源码 | 定向单元、失败路径与独立审计完成 |
| 产品传播 | 清单重建/校验和隔离首次安装、升级通过；尚未正式发布或分发用户级安装 |
| dogfood | 模板镜像及本仓库 baseline clean；实际当前产物已准备 |
| fixture | 上述路径/配置/事务/加载 EXE/集成/性能夹具通过 |
| 真实下游 | 未执行，未写入，保持本轮禁止边界 |
| runtime | 当前 CLI/Hook 自检、手工事件路由集成与健康复用通过；新会话的五种自然生命周期全链路未完整实测 |

本包约定开发和验证工作已完成，无必需工程阻塞。用户验收、Git 保存和发布仍独立：保持 lifecycle=active、awaiting_user_acceptance；TODO-023/024 不由本次 summary switch 自动结算。路径识别仍是支持范围内的有限解析，未知程序交平台权限控制，不声称能静态证明任意 Shell 的副作用。

### 独立审计与经验

授权的唯一 `review-auditor` 只读复核本轮差异，发现并推动修复了 Git 带值选项、嵌套 Shell 工作目录、纯字符串收据、末尾换行/注释、PowerShell 纯字符串写命令误报和内嵌引号漏判。最终限定复核无剩余已确认问题；测试、性能和实际产物由主对话执行，未归为审计者亲测。

保留的工程经验：不能用全命令字符串命中推断执行；PowerShell 词首引号与词内引号语义不同，Bash 的引号程序名也不同；Shell 整体退出码不能归给复合命令中的测试；Git 配置查询失败不是缺失；安装事务只回滚能证明属于自身的写入。优化通过计数断言证明普通编辑读取 1/10 个目标、不扫描指令，新增 120 个忽略子树文件不增加指令遍历项数；未依靠持久化缓存掩盖扫描。

当前任务未验收，无本主题归档候选；不新增 Rule/Memory 建议，已有职责红线已覆盖。Memory 的有界初查未提供本次有效补充事实，交接依据本次实际证据。

### 续接状态

仓库 `D:\Quant\BridgeForge`，分支 main，基准 HEAD `a83732cdd50d19aa7352e777d9299e2292d2041f`（不是最终提交）。保留原有专业术语偏好两个文件的改动；本批为本地未提交修改，不暂存、提交、推送、发布或更新真实下游。依赖 Windows 和 Rust/Cargo 1.88+；本次实测 1.94.1。

原生 Goal 归当前对话 `01a16999-4185-71a0-beb1-ddee959703c4`，预算 10,000,000；交付前最后采样为 active、累计 483,005、剩余 9,516,995 token。该采样不是新的预算起点，完成状态和最终累计以原生 Goal 及最终交付为准。唯一审计 Agent 及全部测试进程已结束；实施、隔离传播和串行性能复测已完成。当前后续动作是用户在本对话验收；仅明确 `$summary accept` 时结算相关 TODO，Git 保存仍需另行授权。

忽略的 `.codex/bin`、`.runtime/runtime-hook-hardening/` 性能数据/旧基线、Cargo target/cache 不随 Git 携带。换机后按 INSTALL 恢复并核对，不以此文代替构建收据；不假设原进程、Goal 或剩余额度自动恢复。接续提示：读取 `doc/1_delivery/runtime-hook-hardening/requirements_2026-10-24_runtime-hook-hardening.md`，先核对当前分支、代码与文档差异，再在已有授权范围内从下一步继续；缺失证据如实标明。

## 用户验收与发布授权（2026-10-24）

用户在当前对话明确调用 `$summary accept` 和 `$git-sync release`，接受本卡约定范围内的 T1 / TODO-023、T2 / TODO-024 交付，并单独授权当前累计改动的受管发布。两项 TODO 结算为已完成；未修改其它 TODO，未新增推测待办。

验收复用上文定向测试、隔离首次安装/升级、实际产物、自检、健康复用、独立审计和串行性能证据，不为总结重新运行工程验证。真实下游、用户级自然分发和新会话自然生命周期全链路仍未验证；该边界不因验收变成通过。此前“待验收”和“未获发布授权”是阶段记录，以本段最新决定为准。

原生实施 Goal 已 complete，最终使用 488,818 / 10,000,000 token，约 50 分钟。本段验收与后续 Git 发布为新的明确指令，不重开或重置旧 Goal。正式发布版本、提交和远端结果以本次受管 release 收据及最终回复为准，不提前记录发布成功。

本主题符合归档候选条件，但本次不归档或移动文件；需要整理时另行调用 `$archive-scan`。既有职责与安全规则已经覆盖本次经验，不新增 Rule/Hook/AGENTS/Memory 建议。
