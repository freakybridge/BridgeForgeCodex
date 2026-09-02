# BridgeForge 全面退役 Python 协作记录

## 需求与预算

- 确认卡：`requirements_2026-09-01_rust-only-bridgeforge.md`
- 目标：BridgeForge 工厂及下发骨架整体退役 Python，最终零 `.py`、零 `.venv` 运行依赖，
  保持现有功能、安全、事务、并发、跨平台和错误语义等价。
- 当前预算：12 小时、180k 新增 token 估算（未实测）、最多 3 个子 agent、最多 6 轮验证。
- Agent 配额：`light-explorer` 1 个已用于只读研读；后续仅使用 `implementation-worker` 1 个
  和 `review-auditor` 1 个。

## 只读研读收据

- `light-explorer` 已核对 56 个产品/测试 Python 文件、三个文档区 Python 文件、Skill、
  pre-commit、project-sync、batch、Native Memory、测试覆盖和当前 dirty diff；未修改文件，
  未运行全量回归。
- 活跃生产链集中在 baseline/runtime/release、instruction/layout/skill、Git/archive、project-sync、
  manifest/batch、Native Memory 六组；`.codex/scripts` 是 Template dogfood 投影，不应单独重写。
- `hook_config_policy.py` 已无活跃调用方，可在 Rust config-health 覆盖证明后退役；
  `build_rust_hook_runtime.py` 由 Cargo/Rust 安装命令替代；`audit_user_allow.py` 仍属下发资产，
  未获功能退役授权，必须迁移。
- `doc/2_bugs/BUG-agents-ia/proposal/contracts/*.py` 仍被当前 proposal 当作验证器，不能当历史
  文字删除；必须迁移或在 proposal 正式关闭归档后处理。
- 当前 Rust Hook diff 与 project-sync、manifest、pre-commit、baseline 和测试高度重叠；这些
  共享集成文件固定由主 agent 增量处理，禁止分派给 worker。

## 固定接口合同

| 接口 | 合同 |
|---|---|
| Cargo 布局 | 保留 `templates/hooks/` 作为 Rust workspace 与 Hook 源；增加 `bridgeforge-core`、`bridgeforge-cli`，dogfood 镜像为 `.codex/hooks/` |
| 二进制 | Hook 为 `.codex/bin/bridgeforge-hook[.exe]`；统一工具为 `.codex/bin/bridgeforge[.exe]` |
| `CommandOutcome` | 统一退出码、stdout、stderr 与 JSON receipt schema，保持旧命令失败语义 |
| `ProjectContext` | 提供项目根、Git/index 读取及 Cargo/BridgeForge CLI 健康状态；禁止暴露 Python executable |
| `BaselineReport` | pre-commit、git-sync、project-sync、batch、release 共用同一结果与 transition proof |
| `AssetContract` | 统一 manifest、ownership、hash lineage、aggregate fingerprint、事务写入与回滚 |
| `ProcessRunner` | Git/Cargo/GitHub 统一 timeout、UTF-8、Windows 隐藏窗口、标准流与错误语义 |
| 构建方式 | 目标机器使用 Cargo 本地构建 Windows/Linux/macOS x86_64；不引入预编译下载链 |

## 拆分与依赖

| 阶段 | 负责人 | 唯一主要文件边界 | 依赖与并行关系 |
|---|---|---|---|
| S0 公共底座 | 主 agent | `templates/hooks/Cargo.toml`、core `lib.rs`、CLI `main.rs`、receipt/process 接口、Cargo lock | 必须最先串行，冻结上述接口 |
| A baseline/runtime/release | 主 agent | core 的 `runtime.rs`、`baseline.rs`、`release.rs` 及对应 Rust test | S0 后；与 F 并行 |
| B instruction/layout/skill | 主 agent | `instruction_source.rs`、`project_structure.rs`、`skill_metadata.rs` 及对应 Rust test | S0 后；与 F 并行 |
| C archive/audit | 主 agent | `archive_scan.rs`、`audit_user_allow.rs` 及对应 Rust test | S0 后；与 F 并行 |
| F Native Memory | `implementation-worker` | `memory_sync.rs`、`hooks_ownership.rs`、`memory_worker.rs`、对应 Rust test | S0 的 context/process 接口冻结后，与 A/B/C 并行；禁止修改共享 dispatcher/manifest/docs |
| D Git sync | 主 agent | `git_sync.rs`、`git_transaction.rs` 及对应 Rust test | A 完成后串行接入 |
| E project updater | 主 agent | `project_sync.rs`、`asset_migration.rs`、`hook_build.rs` 及对应 Rust test | A+B 完成后；与 F 后段可并行 |
| H proposal 合同 | 主 agent | `proposal_validate.rs`、`region_migration.rs` 及对应 Rust test/fixture | S0 后独立，但由主 agent 串行完成 |
| G manifest/batch/factory | 主 agent | `manifest.rs`、`batch.rs`、`factory_version.rs` 及对应 Rust test | D+E 完成后 |
| Z 总切换清场 | 主 agent | pre-commit、Skills、managed manifest、AGENTS/README/INSTALL、VERSION/CHANGELOG、Python 删除 | 所有功能包差分通过后统一执行 |
| R 独立审计 | `review-auditor` | 只读确认卡、协作记录、真实文件、`git diff` 与验证收据 | 全部实现与串联完成后；不得参与实现 |

## 串联边界

- CLI dispatcher、Cargo workspace/lock、两份 pre-commit、两份 managed skeleton、根 manifest、
  Skills、VERSION、CHANGELOG、README、INSTALL、AGENTS 固定由主 agent 修改。
- baseline Rust 等价前禁止切 pre-commit；git-sync、baseline、metadata 未完成前禁止切 updater；
  updater 和 git-sync 未完成前禁止切 batch。
- 对应 Rust integration tests 未闭环前禁止删除 Python 测试；全部生产入口切换前禁止删除
  `.venv` 或公共 CPython 红线。
- worker 不是独自在仓库中，必须保留当前 Rust Hook 与 header-upgrade 等既有 dirty 改动，禁止
  reset、restore、覆盖或修改边界外文件。

## 计划验证

- 每个功能包先运行对应 Rust 精确 test target，再运行原 Python unittest 作为迁移期差分裁判。
- project-sync、Git、Memory 分别覆盖成功、阻断、漂移、回滚、锁、并发和冲突场景。
- Z 清场后运行完整 `cargo test`、无 Python 下游 fixture、真实 pre-commit、factory dogfood、
  manifest/structure/mirror/metadata/instruction checks 与 `git diff --check`。
- 最终用 `rg --files -uu -g "*.py" -g "!.git/**" -g "!.venv/**"` 证明产品树零 Python；
  `.venv` 作为本地迁移工具只在所有差分验证完成后删除。

## 当前状态

- 只读研读：完成。
- 拆分确认：已完成。
- 产品实现：已完成并通过最终复验，等待用户验收。
- 独立审计：完成；最终结论“无阻断”，仅保留已声明的外部实测边界。

## 实际改动与验证记录

- 主 agent 已完成共享 Rust CLI/core、Hook、project-sync、Git sync、release、batch、Native
  Memory、manifest、factory tests 和产品清场；Template 与 dogfood Rust workspace 各 69 项、
  factory tests 6 项及真实临时索引 pre-commit 通过。
- `implementation-worker` 交付的 Native Memory 模块已由主 agent 串联，补充真实本地 bare Git
  push/restore、同路径三方冲突和显式 resolution 测试。
- `review-auditor` 多轮指出的前瞻基线/版本戳事务、Git 索引与仓库身份、Native Memory
  持久健康/告警、远端收敛后安全重放、baseline ownership trust anchor 与 Release HEAD
  分类阻断均已实现并取得反例测试；最终只读复核结论为“无阻断”。

## 2026-09-02 工厂审计 H3 / H4 修复

此节只记录 H3 / H4，不表示其他架构审计项已经关闭，也不替代真实下游或用户体验验收。

- 源码：共享进程执行器改为 Windows 挂起启动、加入 Job 后恢复；三路 I/O 并行计时，超时结束整个 Job，清理最多两秒。构建器实测受管输入，核验独立快照，每个资产独立输出目录并显式指定 Cargo binary，自检及批次结束前复核漂移。
- 产品传播：Template workspace、依赖锁、生成资产 manifest 与当前未提交的 1.8.0 CHANGELOG 已同步；未分发到用户级产品安装。
- Dogfood：自身源码镜像与两个正式 EXE 已更新；baseline 返回 clean，fingerprint 为 `sha256:f2b6b065b21bddc232f6673cb1c33b2dae84288f6290e4ca615363a2c2e698fc`。CLI / Hook 的实际 PE subsystem 分别为 3 / 2。
- Fixture：进程定向 5 项与来源收据定向 6 项通过；覆盖大输入/输出、非零退出、堵塞 stdin、父退出后后代占管道、陈旧声明、构建中输入/快照/产物变化、第二次 Cargo 成功但未产出目标时拒绝复用旧文件。原有大输出测试从产品源码迁入 scripts/tests，未删掉能力。
- 真实下游：未操作、未验证。
- Runtime：真实 Windows 子进程与正式构建的两个 self-test 通过；真实 Codex 宿主事件、桌面无闪窗体验、Unix 运行环境未验证。Unix 主动脱组和外部服务代为启动不属于进程组安全保证；构建收据不承诺机器环境完全可复现。

实际验证命令：

| 命令 | 结果 / 断言 |
|---|---|
| `cargo test --locked --manifest-path templates/hooks/Cargo.toml --workspace -- --test-threads=1` | 60 项通过 |
| `cargo test --locked --manifest-path .codex/hooks/Cargo.toml --workspace -- --test-threads=1` | 60 项通过 |
| `cargo test --locked --manifest-path scripts/tests/Cargo.toml -- --test-threads=1` | 最终整套 43 通过、0 失败、1 子进程辅助用例按设计忽略，244.04 秒；首轮并行于正式构建导致旧收据 fixture 失败，资产就绪后单项与整套重跑均通过 |
| `cargo clippy --locked --manifest-path scripts/tests/Cargo.toml --all-targets --no-deps -- -D warnings` | 通过；不代表整个 workspace 无既有 Clippy 警告 |
| `templates/hooks/target/debug/bridgeforge.exe build-assets --project-root D:/Quant/BridgeForgeCodex` | 正式构建、自检与 schema 2 实测收据成功 |
| `.codex/bin/bridgeforge.exe check baseline --root .` | clean，包含生成资产 |
| `.codex/bin/bridgeforge.exe manifest --root . --check` | changed=false |
| `.codex/bin/bridgeforge.exe check project-structure --root .` | errors / advisories 为空 |
| `.codex/bin/bridgeforge.exe check skill-metadata --root .` | issues / warnings 为空 |
| `.codex/bin/bridgeforge.exe check factory-version --root .` | 1.8.0 healthy |
| `git -c safe.directory=D:/Quant/BridgeForgeCodex diff --check` | 通过 |

独立复核由本轮已有的 `factory_architecture_audit` agent 执行；复核指出跨资产残留产物风险后，已增加独立产物目录与反例测试。最终结论：本轮 Windows / 工厂范围内 H3 / H4 无剩余 Blocker / High，可以关闭。未提交、未推送。

## 2026-09-02 剩余审计项顺序修复

- 授权与范围：同一需求卡“剩余工厂审计修复授权”，H5–H8 / M1–M4；用户已确认开始。
- 本批预算：3 小时 / 60k 新增 token 估算（未实测）/ 最多 3 个子 agent / 3 轮验证。05:00 UTC 记录开工预算；耗时包含研读、等待和重试，不把已有整仓迁移改动算作本批新增。
- 传播四问：H5/H6/H7/M1/M2/M3 为通用产品层修复，进入 Template 并同步 dogfood；H8 为工厂自身配置；M4 为元文档。沿用当前未提交 1.8.0 版本，更新对应 CHANGELOG 标签，不写工厂骨架版本戳。
- `light-explorer` 负责锁作用域、过滤恢复和测试迁移的只读调用链研读；`implementation-worker` 顺序处理产品及其测试，避免 H7 移动测试与其他模块修改冲突；`review-auditor` 最后独立复核。主对话负责文档、H8 配置/回归及统一镜像/manifest/构建验收。
- H8 配置已加入 scripts/tests/Cargo.toml；新增工厂 release plan fixture 验证三个 manifest 与对应 lock 同步，并断言 plan 零写入。
- M4 将旧 CPython 需求卡标为 superseded，保留历史内容并链接 Rust-only；先行 Hook 卡明确后续范围覆盖，不再宣称其他骨架流程仍需 Python。
- 第 1 轮验收准备中；H8/M4 定向 2 项已通过，其余尚未给出通过结论。
- H7 生命周期补查发现：Hook util::run_command 仍保留独立 spawn/poll/kill/wait_with_output 实现，session/post 调用未接入 H3 的 SystemProcessRunner。上轮 H3 关闭证据只覆盖 core 执行器，不能代表此分支；本批在既定 Hook 异常路径范围内统一薄适配并补回归，完成前不重复宣称 H3 全链闭合。
- 第 1 轮独立源码复核发现：M2 build-assets 已保护外部漂移，但 apply 删除旧收据后的失败回滚仍可能覆盖外部重新创建的文件。已退回 implementation-worker 增加原始字节重验、回滚冲突保护及反例；整体验收等待修复后进行。
- 后续复核确认同步器本地 writer 在 Windows 先删除旧文件再 rename，失败中间态与新的回滚预期核验冲突；改为复用共享原子替换入口，并要求实际替换失败时原文件保留的回归。两项复核发现都必须在最终全套前收口，不拿此前单项通过替代最终结果。

### 最终复验结果（第 2 轮）

- 源码：H5/H6 全程锁、H7 工厂专用测试注册、H8 版本配置、M1 写入失败传播、M2 旧收据事务退役与外部漂移保护、M3 manifest 白名单恢复、M4 历史合同状态均已落地。Hook 辅助命令也已接入共享执行器，补齐上轮 H3 漏掉的调用分支。
- 产品传播：通用 Rust 修复进入 Template，工厂专属配置/测试留在自身；沿用本次未提交的 1.8.0，CHANGELOG 已追加对应层标签。未安装用户级产品，未写工厂骨架版本戳。
- Dogfood：最终 Template 与自身镜像一致；两个正式 EXE 和 schema 2 收据已事务安装，旧 schema 1 build-receipt.json 已退役。baseline 为 clean，fingerprint 为 `sha256:32cbbfeb894a51bcbd5113c8c4b99826f7cbeec9c0aa373b3fca5de9a63068e9`。
- Fixture：两套工厂私有单测各 73 通过、2 子进程 helper 按设计忽略；原 54 个私有单元和 6 个 Memory 集成测试逐名比对无遗漏。完整工厂回归 56 通过、0 失败、1 helper 按设计忽略，261.99 秒；隔离下游真实安装后，无 scripts/tests 目录的 cargo test --locked --all-features --workspace 通过。
- 真实下游：未操作、未验证；临时下游 fixture 不替代真实项目验收。
- Runtime：正式构建及自检通过，实际 PE subsystem 为 CLI=3 / Hook=2；Windows 普通进程、文件共享锁和原子替换失败路径已实测。真实 Codex 宿主事件、桌面无闪窗体验、非 Windows 和真实 Memory/GitHub 未验证。

本批独立复核由 `factory_architecture_audit`（review-auditor）完成，发现的两条 High 均已修复并用最新测试程序复测关闭；独立短测共 45 项通过，未发现本批遗留 Blocker / High。此结论不等于不存在其他未发现问题。

| 实际命令 / 收据 | 结果 / 覆盖 |
|---|---|
| `cargo test --locked --config scripts/tests/factory-cargo.toml --manifest-path templates/hooks/Cargo.toml --workspace -- --test-threads=1` | 最终 73 通过、2 helper 忽略；覆盖所有迁出的私有测试 |
| 同命令的 `--manifest-path .codex/hooks/Cargo.toml` | 最终 73 通过、2 helper 忽略 |
| `cargo test --locked --manifest-path scripts/tests/Cargo.toml -- --test-threads=1` | 56 通过、0 失败、1 helper 忽略；包含无工厂测试目录的下游真实 Cargo 安装/构建/测试、事务与回滚、真实本地 Git 与 Memory fixture |
| 三个 Template 单元测试 exe `--list --format terse` 与原名单逐项比对 | 原 54 项无遗漏；Memory 6 项全部登记在 factory 测试程序 |
| `cargo clippy --locked --manifest-path scripts/tests/Cargo.toml --all-targets --no-deps -- -D warnings` | 通过；不代表整个产品 workspace 无既有警告 |
| `cargo fmt --manifest-path … --all --check`，分别指定 Template、dogfood、factory manifest | 三套通过 |
| `templates/hooks/target/debug/bridgeforge.exe build-assets --project-root D:/Quant/BridgeForgeCodex` | built，两资产独立构建、自检、安装；源树 `sha256:2064b2f8cf1b02ed1783dcd3260d09c3b1576227c2b6d03112bd7aa4f74f9dd1` |
| `.codex/bin/bridgeforge.exe check baseline --root .` | clean，包含当前生成资产；旧 build-receipt.json 不存在 |
| `.codex/bin/bridgeforge.exe manifest --root . --check` | changed=false |
| `templates/hooks/target/debug/bridgeforge.exe check project-structure --root .` | errors=[]；仅旧 CPython 主题的归档候选 advisory，未归档 |
| 同 CLI `check skill-metadata --root .` / `check factory-version --root .` | issues/warnings=[]；1.8.0 healthy |
| `git -c safe.directory=D:/Quant/BridgeForgeCodex diff --check` | 通过 |

最终二进制 SHA-256：Hook `d4fa5099036542ffd5e1ea6b2f31966a2d4227d790fe10a0bf9299d2e21a44de`；CLI `a56c3689e533b01758fd3717b370bc7a4503159a3e18c35403e9b08cf8b33ec0`。未提交、未推送。

本批 H5–H8 / M1–M4 在约定工厂范围内完成修复和最终验证，需求卡恢复 awaiting_user_acceptance，未代替用户确认验收。实际参与 3 个子 agent、2 轮主侧验证（完整工厂套件在最终轮执行）；耗时约 53 分钟，token 未实测。外部实测边界继续保留，旧 CPython 主题仅标记可归档，没有擅自归档。

## 2026-09-02 下发前修复（1.8.2）

用户授权处理前一轮列出的十二项问题。主对话实现；`fix_docs_map`（light-explorer）只读核对文档映射，`rust_docs_audit`（review-auditor）独立复核。没有运行真实项目安装、用户级更新或 Git 提交推送。

| 原问题 | 已实施修复 | 对应证据 |
|---|---|---|
| 1. 空项目 doctor 错查下游 Cargo | 明确 `--product-root`，验证产品版本、工具链、锁和实际 CLI | product_preflight 回归；产品 CLI smoke |
| 2. 文档承诺的复合迁移目标被拒绝 | AGENTS 项目区、hooks.json、doc/README 经各自渲染器与最新公共基线组合 | composite_migration 成功与回滚回归 |
| 3. 同版本修复因无戳写入失败 | 允许戳不变的已确认修复，最终核验唯一版本戳与完整基线 | current_version_repair 回归 |
| 4. 旧戳遗留、双戳身份不明 | 先验证单戳身份，旧戳同事务退役，双戳/非法/较新版本拒绝 | legacy_identity 回归 |
| 5. 旧 manifest/hash 被当作删除授权 | 不解释旧合同；未知普通文件逐项确认保留/删除 | old_manifest_never_proves_deletion_ownership 回归 |
| 6. 声明 Rust 1.85 与语法不符 | 工厂及产品最低版本修正为 1.88，doctor 显式核验 | 工具链拒绝回归；本机 stable 实际构建；1.88 本机实编未验证 |
| 7. Git 凭证降级未实现 | gh 查询失败后读取既有 Git 凭证，仅传入子进程环境并重试私有性校验 | 私有通过/非私有拒绝/无凭证泄露测试替身 |
| 8. 生命周期同步模型不一致 | 三事件仅排队并复用隐藏 worker，消费同步期间和释放边界的后续 pending | 三事件正式配置测试、queue drain 回归 |
| 9. 恢复步骤核验旧二进制 | 改为正式更新器构建、自检新产物、事务替换，下轮重新进入 | runtime-preflight 与真实产品调用链核对 |
| 10. JSON 收据与状态文档过期 | 对齐当前 plan/apply/Memory 字段及失败出口 | technical-receipts 与实际结构体/回归核对 |
| 11. 旧 Rule 自动退役与 schema 3 文档残留 | 逐源迁移授权、schema 4、可信产品基线修复边界 | 原生指令、同步与反哺设计文档核对 |
| 12. README 全局禁读 Memory | 区分不透明整树同步与 summary 按需只读建议 | README / AGENTS / summary 合同核对 |

独立复核补出的组合问题也已修复：删除旧 Hook 后最终注册不再复活；说明字段不能冒充实际 Hook handler；worker 的预留/登记/释放与 pending 共用队列锁。进一步发现过期锁回收的删除竞态，改为 Windows 独占文件句柄 / Unix flock，锁文件保留且进程退出自动释放，用户 Hook 配置复用同一机制。Windows 接口依据 [Rust OpenOptionsExt](https://doc.rust-lang.org/std/os/windows/fs/trait.OpenOptionsExt.html#tymethod.share_mode)，Unix 依据 [flock](https://www.man7.org/linux/man-pages/man2/flock.2.html)。

新增 CLI 测试最初三次失败分别暴露 ledger 缺失、测试配置不全、生产授权适配器把目录当文件。前两次为 fixture 问题，第三次为真实生产缺陷；经 `$escalate` 独立诊断后修正统一适配器为 `state/remote.txt`，fixture 改用正式 `configure`，并验证远端漂移拒绝。未通过删除断言或降低授权门槛使测试通过。

### 本批验证记录

- Template 工厂私有测试：74 通过、0 失败、2 个子进程 helper 按设计忽略；使用 `cargo test --locked --config scripts/tests/factory-cargo.toml --manifest-path templates/hooks/Cargo.toml --workspace -- --test-threads=1`。
- 新增分发回归：11 通过、0 失败；使用 `cargo test --locked --manifest-path scripts/tests/Cargo.toml distribution_regressions:: -- --test-threads=1`，包括预置死亡 worker 与过期 queue 锁的 8 启动器竞争。
- Dogfood 使用同一带配置的命令、manifest 改为 `.codex/hooks/Cargo.toml`：74 通过、0 失败、2 helper 按设计忽略。
- `.codex/hooks/target/release/bridgeforge.exe build-assets --project-root .`：built，两资产独立 release 构建、自检和 schema 2 收据成功。源树 `sha256:388cbebc2df8f4968c1db379967e62a90acdf4aa06fda21004143f3c4e84f23f`；Hook 二进制 `sha256:48cbed50e3e6dfb077bf7a145352a37668b44e48c863a0b1a18549078ca7f8fd`；CLI 二进制 `sha256:749e475447ab5967e58e8d55f119f42687d509e0f3241862132ad651264ffc04`。
- `.codex/bin/bridgeforge.exe check baseline --root .`：clean，1.8.2，fingerprint `sha256:ac8441bd63099442eeadafa0617da41cccaba88e6f34032f444b514d3ced2814`。`manifest --root . --check` 返回 changed=false；`check factory-version --root .` 返回 healthy=true；`check skill-metadata --root .` 无 issues/warnings；`check project-structure --root .` 无 errors，仅保留既有 CPython 主题归档候选提示。
- 从新建空目录执行正式 CLI `doctor --product-root D:/Quant/BridgeForgeCodex --json`：成功读取产品 workspace，工具链 cargo/rustc 均为 1.94.1；调用后空目录仍零文件。CLI 与 Hook 的 `self-test --json` 均返回正确身份；先前省略 `--json` 的人工调用不属于有效自检证据。
- 源码与 dogfood 逐字镜像差异为空，`cargo fmt --manifest-path scripts/tests/Cargo.toml --all --check` 和 `git -c safe.directory=D:/Quant/BridgeForgeCodex diff --check` 通过。
- `rust_docs_audit` 最终限定范围独立复核通过，未发现新的必须修改问题；复核只读检查源码、测试与文档，不把主侧测试说成独立重复测试。
- 完整工厂命令 `cargo test --locked --manifest-path scripts/tests/Cargo.toml -- --test-threads=1`：首轮 66 通过、1 失败、1 子进程 helper 按设计忽略，266.13 秒。唯一失败是旧更新样例没有版本戳；补齐当前版本身份、保留原内容保护断言后，`cargo test --locked --manifest-path scripts/tests/Cargo.toml runtime_flows::project_sync_real_contract_preserves_project_owned_zones_rows_and_hooks -- --exact --nocapture` 1 通过。67 个主测试场景均已有通过证据，未宣称修正后再次整套运行。中途一次编译漏借用已更正；一次并行链接被仍在运行的测试 EXE 占用，等待整套退出后复测成功。
- 首轮通过的真实安装场景覆盖：两个资产由 Cargo 实际构建、临时项目 init/apply、二进制哈希与完整 baseline 校验，以及不含工厂 `scripts/tests` 的下游 `cargo test --locked --all-features --workspace`。它证明隔离 fixture 安装，不替代真实下游项目。
- 真实下游、真实 GitHub / 跨电脑 Memory、真实 Codex 事件与桌面无闪窗体验、非 Windows runtime、Rust 1.88 精确工具链实编：未验证。

本批十二项问题及独立复核补出的相关缺陷已在工厂范围内处理完成，版本为 1.8.2；保持待用户验收，未提交、未推送、未分发用户级产品。

### 可选源码与文档映射

本仓库尚无 `.codex/sync-docs.map.md`；本次以实际源码查找已有文档，未自动新建映射。候选：`templates/hooks/crates/bridgeforge-core/src/project_sync.rs` 与 `asset_migration.rs` → `doc/0_architecture/design/codex-project-sync.md`；`templates/hooks/crates/bridgeforge-core/src/memory/**` → `doc/0_architecture/design/codex-native-memory-sync.md`。是否补充此映射，留待用户单独采纳，不影响本批修复完成。

## 2026-09-02 git-sync 自动发布事务修复（1.8.3）

此前真实 git-sync 在 fetch 成功后被 pre-commit 报 `generated asset receipt drifted: codex.hooks` 阻断：自动升级 Cargo 版本并刷新清单后，没有同步重建运行产物与收据。旧结论仅覆盖静态基线与分段测试，没有覆盖自动升级后的完整提交链。本次按用户“开始修”授权修复，不对工作仓库提交或推送。

传播四问：属于通用产品实现，进入 Template 并同步 dogfood；工厂分支中的自动构建不改变下游业务版本规则；版本提升至 1.8.3，CHANGELOG 标记 product；相关设计、安装说明和 git-sync Skill 同步更新。

实现：发布计划在渲染前记录输入字节；在仓库外快照中投影新版本、清单、锁定 Cargo 构建、自检与实测收据；完整计划准备后复核输入、目标、源码集合、仓库身份和 index，随后单事务安装。完整 baseline 与真实 pre-commit 保持开启。Windows 当前运行映像移入被忽略的受管缓存，后续同步按名称和内容哈希清理，保持当前进程继续执行。提交前失败恢复全部自动资产与原 index。

独立复核：review-auditor 发现并发修改 CHANGELOG 的时间窗口，已通过 FileReleasePlan.inputs 补齐渲染前快照及后续复核；对应 release-drift 用例已通过。复核者再次只读检查确认该 P1 关闭，其余限定范围无新增必改项；未把主侧测试算作独立重跑。

本批验证记录如下；真实下游、真实 GitHub、跨电脑 Memory 和用户桌面视觉体验仍属于未验证边界。

### 本批已取得的验证证据

- `cargo test --locked --config scripts/tests/factory-cargo.toml --manifest-path templates/hooks/Cargo.toml --workspace`：77 通过、0 失败、3 个子进程 helper 按设计忽略。使用 `.codex/hooks/Cargo.toml` 的 dogfood 完整命令同样为 77 通过、0 失败、3 helper 忽略。覆盖构建失败零发布写入、源码和发布渲染输入漂移拒绝、真实 Git commit hook 拒绝后的版本/二进制/收据/index 恢复，以及 Windows 子进程替换自身运行映像、继续运行、恢复原字节和缓存清理。
- 实际运行的完整工厂命令为 `cargo test --locked --manifest-path scripts/tests/Cargo.toml -- --test-threads=1 --nocapture`。其中 `git_sync_runtime::real_factory_cli_sync_builds_new_runtime_and_commits_through_precommit` 已通过：隔离副本由 1.8.3 自动升级至 1.8.4；真实 CLI 从受管路径运行并替换自身；真实 pre-commit 接受；worktree/index baseline、自检新版本和 manifest 校验成功；本地 bare 远端最终 0/0，工作树 clean。没有访问真实 GitHub 或提交当前工作仓库。完整结果为 68 通过、0 失败、1 个子进程 helper 按设计忽略，耗时 477.40 秒；包含真实生成资产 init/apply 和之前修正过的项目定制保留场景。
- `.codex/hooks/target/release/bridgeforge.exe build-assets --project-root .`：built，源树 `sha256:42957441675e6fb47ab15b506ab6f4d8b214517e6b0262404c79e68d5efa465f`；Hook `sha256:f5b267aeac1057ec96f10f5fcdb99c20b46d1b33ab4c1d16d386bd3f745036f2`；CLI `sha256:365ca6f1d000cfb495efd7aac3072bc064d33c162ca9469a811d0615cd75f7a0`。均来自锁定 Cargo 构建、实测自检和 schema 2 收据。
- `.codex/bin/bridgeforge.exe check baseline --root .`：clean、1.8.3，fingerprint `sha256:b0e312d14e70b0fcb5d92ddc87a221b5cffa12d6e94f9846ea174e4e5577c4f3`。同一 CLI 的 `manifest --root . --check` 为 changed=false；`check factory-version --root .` 为 healthy=true；`check skill-metadata --root .` 无 issues/warnings；`check project-structure --root .` 无 errors，仅保留既有旧交付归档候选。
- Template 与 dogfood 源码逐字差异为空；`cargo fmt --manifest-path scripts/tests/Cargo.toml --all --check` 与 `git -c safe.directory=D:/Quant/BridgeForgeCodex diff --check` 通过。

### 六类证据与剩余边界

| 证据类别 | 本批结果 |
|---|---|
| 源码 | 新发布输入快照、完整构建计划和运行映像替换已落地，针对性回归通过 |
| 产品传播 | Template / 共享 Skill、VERSION 1.8.3、CHANGELOG 与 manifest 已同步；未安装用户级产品 |
| Dogfood | 源码逐字镜像、77 项测试、完整运行基线与版本硬闸通过 |
| Fixture | 工厂完整 68 项通过；真实 CLI 自动升版、真实 pre-commit、真实本地 Git commit/push 和 0/0 验证通过 |
| 真实下游 | 未执行用户下游项目更新，仍未验证 |
| Runtime | Windows 自替换/恢复、真实锁定构建、自检及完整同步通过；真实 GitHub、跨电脑、非 Windows、精确 Rust 1.88 与用户桌面视觉体验未验证 |

本次修复完成，保持待用户验收；当前工作仓库未 commit/push，HEAD 和已有 stash 保留，暂存区为空。隔离回归中的 1.8.4 不影响当前工作仓库的 1.8.3。

## 2026-09-02 十七项架构扫描修复（1.8.4）

### 范围、传播与授权

- 用户明确授权将本轮扫描发现的十七项一起修复；按现有需求卡和独立 Memory 包边界执行，不提交、推送、发布或写真实下游/用户配置/原生 Memory。
- 传播四问：通用实现属于产品层，进入 Template / skills 并同步本仓库 dogfood；批次流程为工厂专属，只更新工厂 Skill；版本升为 1.8.4 并记录 CHANGELOG；不写工厂骨架版本戳。
- implementation-worker 负责 Memory 三个实现文件与两个测试文件；主对话负责共享入口、其余实现与整合；light-explorer 定位文档；review-auditor 独立核查当前源码。

### 修复与针对性回归

| 编号 | 已修复问题 | 回归依据（scripts/tests 下） |
|---|---|---|
| 1 | 冲突后新增本地内容被旧树覆盖；发布/恢复及移走旧树后复核，保留原树备份 | unit/core_memory_remote.rs：conflict_resolution_blocks_new_local_files_without_publishing、automatic_merge_preserves_concurrent_local_writes_before_and_after_push、replacement_retains_old_tree_for_writers_with_open_handles |
| 2 | 显式 remote 绕过授权与自定义目录越过授权范围 | unit/cli.rs：explicit_memory_parameters_cannot_bypass_consent_or_scope |
| 3 | Git attributes/filter/ignore 改变快照原始字节 | unit/core_memory_remote.rs：snapshot_blobs_ignore_attributes_filters_ignores_and_line_endings |
| 4 | 删除 project_a 误删相似前缀 Hook 注册 | unit/core_project_sync.rs：hook_removal_matches_path_boundaries_and_platform_commands；src/distribution_regressions.rs 连续升级 |
| 5 | 用户级更新收尾失败只回退 CLI，形成混装 | shared_transaction.ps1；src/distribution_regressions.rs：shared_bundle_commit_keeps_components_consistent_when_old_image_is_running |
| 6 | 新目录折叠导致全量暂存保护漏掉 .env | src/security_guards.rs：bulk_add_detects_sensitive_files_inside_untracked_directories |
| 7 | 旧批次关闭释放新批次锁，state 目录分裂锁作用域 | unit/core_batch.rs：state_machine_is_serial_and_stops_on_repeated_common_issue 中的 owner、重复 close 与工厂锁断言 |
| 8 | 过期批次互斥锁先检查后删除，误删新持有者 | file_lock.rs 使用系统持有句柄且不 unlink；unit/core_file_lock.rs 真实进程互斥 |
| 9 | 仅按 upstream 判定，实际 push 目标未同步却返回 synced | unit/core_git_sync.rs：distinct_push_target_receives_commits_even_when_upstream_has_parity，两个真实本地裸仓库 |
| 10 | 提交编码检查读取工作区而非 index | unit/hook.rs：precommit_encoding_checks_index_not_worktree，覆盖正反向部分暂存与乱码 |
| 11 | 运行中的测试被推断为成功 | unit/hook.rs：asynchronous_test_receipt_never_infers_success，含子命令 exit code 0 反例 |
| 12 | 带注释/引号的 TOML 表重复写入 | unit/core_memory_user_config.rs：annotated_quoted_and_inline_tables_are_edited_without_duplication、invalid_or_wrong_shaped_toml_is_rejected_without_writing |
| 13 | Memory 已成功收敛但保留 active-conflict | unit/core_memory_remote.rs：successful_convergence_clears_only_active_conflict_marker |
| 14 | Hook 删除留下空目录阻断下次升级 | unit/core_project_sync.rs：retired_hook_directories_rollback_with_their_files；src/distribution_regressions.rs 连续升级 |
| 15 | ProjectLock/GitLock 强制退出后永久阻塞 | unit/core_file_lock.rs：os_lock_excludes_contenders_and_recovers_after_process_death；原 project/Git 排他测试 |
| 16 | batch retry 可清除 common issue，绕过 restart 修复见证 | unit/core_batch.rs：共性故障 retry 拒绝且状态字节不变 |
| 17 | 下游 Skill 门禁默认扫描根 skills 而非 .codex/skills | unit/cli.rs：default_metadata_gate_checks_native_project_skills |

### 已取得的阶段证据

- Template workspace 首次完整回归：91 通过、0 失败、4 个子进程 helper 忽略；后续升为 1.8.4 并补第 11 项反例，另跑该定向测试通过。最终整体验证见下节，不以阶段测试替代最终产物验证。
- 分发定向回归的连续升级用例已通过；共享事务脚本实际验证提交前一起回滚，以及运行中旧 exe 阻止清理时保留所有新组件，进程退出后按日志完成清理。Rust 启动该脚本的模块加载差异已由测试显式导入系统模块解决，定向重测通过。
- 初始新测试两处前置假设已修正：配置测试先建立合法受管账本；Windows 打开子文件时目录 rename 被系统拒绝，测试改为验证安全阻断及关闭句柄后重试。没有把测试环境失败归为生产数据丢失。
- 独立 review 首次发现第 11 项仍可能从运行中日志提取子命令成功码；主对话修复并补同一反例，最终限定复核结论为“未发现剩余阻断”。review 为只读源码/断言审查，测试执行证据由主对话提供。

### 最终六类交付证据

- 源码：十七项已逐一实现并完成独立复核。
- 产品传播：1.8.4、Template / skills 与 CHANGELOG、manifest 已同步；两个实际 EXE 经锁定 release 构建、自检及事务安装。未安装用户级产品。
- dogfood：受管源码及工厂 Skill 已同步；Template 和 dogfood 最终完整测试各 91 通过、0 失败、4 个子进程 helper 按设计忽略；实际运行 baseline clean。
- fixture：完整工厂重跑 70 通过、0 失败、1 个子进程 helper 按设计忽略，334.09 秒；覆盖临时本地 Git/Memory、Hook、下游安装、真实 CLI 自动升版与 pre-commit、本地 Git 提交推送、进程终止和分发事务。
- 真实下游：未执行，不声称已验证。
- runtime：Windows 隔离进程与本地 CLI/Hook 有实跑证据；真实 Codex 生命周期、真实 GitHub/跨电脑 Memory 和非 Windows 实机均未验证。

### 剩余边界

Memory 恢复会保留 memories 同级 before-sync 原树备份，不自动清理；它用于保全同步期间的晚到写入，不作为新的 Memory 内容来源。用户级更新已提交但旧 exe 被占用时会返回 cleanup_pending，下一次维护先恢复清理；不因此回退单个组件。

### 本批最终命令与收据

| 实际命令 / 收据 | 结果 / 覆盖 |
|---|---|
| cargo test --locked --config scripts/tests/factory-cargo.toml --manifest-path templates/hooks/Cargo.toml --workspace -- --test-threads=4 | 91 通过、0 失败、4 helper 忽略；CLI 8、core 72、Hook 11；日志 .runtime/fix17/template-tests.log |
| 同命令，manifest 改为 .codex/hooks/Cargo.toml | 91 通过、0 失败、4 helper 忽略；日志 .runtime/fix17/dogfood-tests.log |
| templates/hooks/target/release/bridgeforge.exe build-assets --project-root . | built，两个资产独立锁定构建、自检、安装；日志 .runtime/fix17/build-assets.log |
| .codex/bin/bridgeforge.exe check baseline --root . | clean，project_version=1.8.4；运行资产和源码均匹配 |
| 同 CLI：manifest --root . --check；check factory-version --root .；check skill-metadata --root .；check project-structure --root . | changed=false、healthy=true、issues/warnings=[]、errors=[]；仅既有旧交付归档候选提示 |
| 临时 GIT_INDEX_FILE 执行 D:/Program Files/Git/usr/bin/sh.exe .githooks/pre-commit | exit 0；worktree/index baseline、指令、配置、编码、结构、metadata、版本、manifest 硬闸全部通过；日志 .runtime/fix17/precommit.log |
| git -c safe.directory=D:/Quant/BridgeForgeCodex diff --check | 通过 |

共同源树 SHA-256：ddbd6b9e73a2b9a5ba20d34e0e01a9d2284b24c21e7bba1738ba6be644cc582e；Cargo.lock：374998b5e440a042d4161a73b46a08d8c39e94cdf135430a79ff49f3e1d4fdcc。Hook 二进制：7b4923091fee395e2d2cbc59136bce7b88239356c9609e1b6cf4d2fc2714523d；CLI 二进制：920b2b65448d684e917e90e6feab8510b502fc8dce6727d46cbccec9c75175d9。工厂 baseline fingerprint：787944250caaa369fbf88bde5b78aa6045de9d337de83fb7d8d66f62b5b1afc7。

完整工厂首轮为 66 通过、4 失败、1 helper 忽略，287.20 秒（.runtime/fix17/factory-tests.log）。其中三项是测试仍假设锁文件被删除、或最小工厂没有忽略 .runtime，已改成验证句柄释放后可再次构建，并为两个 fixture 补官方目录忽略规则；新增不同 state 目录不能启动第二批次的反例。后代进程测试在并行构建时未在 800ms 内写出 PID，原测试单独运行 1.63 秒通过；给双进程启动留 3 秒、总退出上限 6 秒，仍要求睡眠 8 秒的 leaf 被提前终止，PID 消失和 leaf-ready 断言保持。最终完整重跑结果另记，不将首轮失败算为通过。

环境性重试：运行中的工厂测试 EXE 导致并行链接 LNK1104，等待退出后才重新编译；初次裸 sh 不在 PATH，改用已核实的 Git shell 绝对路径后真实 pre-commit 通过。前述失败调用均不列为通过证据。实际 .git/index 的 SHA-256 在验证前后保持 5249b1a31bb49d56acb36b8a85dba01258b03d771962f9b17006ef9fde1d6eaf，没有提交或推送当前工作仓库。

最终测试适配的独立复核：review-auditor 确认第二次实际构建和跨 state 互斥没有削弱原证明；指出进程总时限必须覆盖 PID 等待，已在终态检查后补同一 6 秒总上限，再次只读复核确认缺口关闭。其运行证据以最终 process_runtime 模块重跑为准。

最终完整工厂命令 cargo test --locked --manifest-path scripts/tests/Cargo.toml -- --test-threads=4：70 通过、0 失败、1 helper 忽略，334.09 秒；日志 .runtime/fix17/factory-tests-final.log。包括首轮四个失败场景，以及两个真实锁定构建场景：下游 init/apply 与实际 CLI 从 1.8.4 自动升至 1.8.5、经过真实 pre-commit 和本地远端同步。该隔离测试版本不改当前工厂的 1.8.4。完整套件启动后仅加强了后代终态总时限断言，未改产品代码，另行重跑整个 process_runtime 模块确认最终断言。

最终增强断言验证命令 cargo test --locked --manifest-path scripts/tests/Cargo.toml process_runtime:: -- --test-threads=4：5 通过、0 失败、1 helper 忽略，6.04 秒；日志 .runtime/fix17/process-final.log。包含 stdin 超时、标准流/退出码、父进程提前退出后的后代终止，以及终态验证总耗时上限。

本批十七项修复在约定工厂范围内完成，需求卡恢复 awaiting_user_acceptance；未代替用户验收。真实下游、真实 Codex 生命周期、GitHub/跨电脑 Memory、非 Windows、Rust 1.88 精确工具链实编和桌面视觉体验仍未验证。HEAD 保持 106b87b48b0ddb37f266cf86d21826d99022907c，当前仓库未提交、未推送，实际暂存区不变。
