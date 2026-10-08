---
lifecycle: completed
validation_status: verified
---

# git-sync 显式发布

## 决定与授权

- 来源：对话 `01a119c7-91ed-7ee0-8f21-b30a6e4dd026` 及当前对话。
- 2026-10-08 用户确认：版本号规则写入计划，然后开始开发。
- 2026-10-08 用户随后明确调用 `$summary 同意验收` 与 `$git-sync 发布新版本`，授权本轮变更验收收口及向当前 origin/main 发布。发布前完整验证与独立审计正在执行；真实下游与用户级安装不在本次授权内。
- 初始开发范围：BridgeForge 上游 Skill、受管 Rust 入口、配置、测试、分发元数据与 dogfood；当时不提交、不推送。后续工厂发布授权见上条；始终不安装用户级 Skill、不写真实下游。
- 规模 M：同步与发布两个模块，保留已有事务及项目兼容策略；预算 45 分钟、约 20k 新增 token（未实测）、0 个子 Agent、最多 2 轮针对性验证。沿用前期调查，预算不因文件传播重新计算；超额前停止新增工作并说明剩余范围。现有本机周额度系数属于另一已记录模型，当前模型口径未核实，不伪造百分比。

## 用户行为与版本规则

1. 项目配置 `release_policy: explicit_release` 后，普通 `git-sync` 提交并同步，不自动改 VERSION、原生版本或 CHANGELOG。
2. `git-sync release` 映射到受管 CLI 的 `--release`，汇总上次发布以来的业务改动；工作区干净但存在未发布提交时仍可发布。
3. 累计改动取最高级别，只升一次：`!` / `BREAKING CHANGE:` 为 major；否则含 `feat` 为 minor；其余合法 `fix/perf/refactor/docs/chore` 为 patch。例如 `1.47.11` 加两个 fix 和一个 feat，发布为 `1.48.0`。major/minor 升级后低位归零。
4. 发布命令本身的消息不覆盖累计提交级别。当前未提交的业务改动使用经实际 diff 审查的提交消息参与判断。
5. 没有未发布业务改动不升版、不创建空提交；纯骨架升级不升业务版本。
6. 未配置或 `per_commit` 保持既有行为。原交付时工厂保留 per_commit；用户后续已确认工厂切换 explicit_release，当前策略与验证见[性能优化计划](../git-sync-performance/requirements_2026-10-08_git-sync-performance.md)。非法策略、不可判定的版本历史或业务提交格式停止发布，保留现场。
7. 原生 manifest 和 lock 按既有发布事务同步；根 VERSION 仍是版本事实源。
8. Skill 执行前展示版本依据；收据区分同步与发布，保留分支、commit、推送目标、实际 ahead/behind、工作区与 stash 结果。
9. 发布边界要求 VERSION 语义值增加、新增唯一对应 CHANGELOG 版本节、历史原生 manifest 一致；仅空白等格式修改不改变边界、不驱动版本升级。首次引入 VERSION 兼容为初始化基线。

## 实施计划

| 编号 | 优先级 | 范围 | 工作 |
|---|---|---|---|
| 1 | P0 | release.rs 与新增发布历史模块 | 项目策略、只读发布计划、可验证版本边界、逐提交所有权分类、累计 SemVer 和 CHANGELOG |
| 2 | P0 | git_sync.rs、CLI | `--release` 与只读预览；普通同步跳过升版，显式发布复用事务和失败回滚 |
| 3 | P1 | skills/git-sync/SKILL.md | 两种用户用法、配置兼容、版本预览、停止条件和收据 |
| 4 | P1 | scripts/tests/** | 普通同步、累计发布、干净发布、重复发布、骨架排除、兼容和失败回滚 |
| 5 | P1 | VERSION、CHANGELOG、manifest、dogfood | 产品新增能力，版本 1.23.0；同步受管镜像和生成合同 |

## 传播四问

产品层通用能力；源码进入 templates/ 与 skills/；需要 VERSION 和 `[product]` CHANGELOG；需要同步自身 dogfood。工厂专属规则不下沉，禁止对工厂运行 project-sync apply。

## 验证与风险

- 定向 Rust 测试：版本最高级别、多个已同步提交、当前业务修改、干净工作区、重复发布、非规范历史、手工 VERSION 改动、浅历史、纯骨架和混合所有权、事务失败恢复。
- 工厂检查：格式、源码镜像、skill metadata、manifest --check、factory-version、project structure；发布前全套检查另行执行。
- 发布边界不能假设存在 tag；从可验证的 Git VERSION 历史定位。历史不完整或不一致时停止，不能静默只取最后一次提交。
- 普通同步仍需运行既有 baseline/pre-commit；不承诺完全免编译，只消除自动升业务版本引起的额外编译。
- 真实 StratusAgent 升级与运行、用户级安装未授权、未验证。本次后续发布授权仅限工厂当前改动，不扩展到真实下游。

## 执行记录

- 2026-10-08 发布后用户要求将 Skill 用法改为 `git-sync release`；已更新入口参数与当前用法说明，CLI `--release` 和版本规则不变。此前用户授权的原话保留作历史记录。此更名为待发布产品改动，不代表已再次提交或安装；版本与正式 CHANGELOG 由下一次受管发布事务统一生成，无 Rust 镜像改动。
- 前期试改已撤回；开工前工作区干净。
- 已实现：策略读取、累计发布计划、逐提交所有权分类、最高 SemVer、CHANGELOG 分组、只读预览、CLI 能力标记、同步版本收据；Skill 在调用新参数前验证能力，避免旧 CLI 忽略参数造成副作用。
- 已验证：release 单元测试 16/16；git_sync 单元测试 21/21；补充 mixed region 与 merge 边界后 explicit_release 定向集 7/7。三个测试集共有 39 个不同测试通过。测试仓库与 bare remote 均在临时目录。
- 初次沙箱测试无法创建临时 `.git/config`，提权后通过；这是环境权限重试，无源码修复。
- 合并提交尚不自动归并：显式发布遇到未发布 merge/root 提交停止并指出 SHA；普通同步不因此受限。浅历史、非规范业务提交、未提交手工 VERSION 变化同样停止发布。
- 受管 build-assets 已完成 Hook/CLI release 构建、自检和实测收据安装。首次从待替换 CLI 本身运行时安装事务返回拒绝访问并回滚；改用同一官方二进制的已核对哈希临时副本重试成功，临时副本已删除。
- 真实 CLI 夹具初次被 baseline 拒绝（空 assets）；补齐合法受管资产后通过，产品代码未因该失败修改。两轮验证预算内完成，未运行全量回归或子 Agent，token 未实测。
- 当前：上游开发及定向验证完成，工作区未提交；等待用户试用验收。没有更新用户级 Skill 或真实下游。
- 验收发布阶段：用户已同意验收，关闭状态暂待发布前完整验证和独立审计。首次只读发布预览因开发阶段预写 VERSION=1.23.0、HEAD=1.22.0 被保护规则拒绝；需核对并撤回仅预生成的版本元数据，再由受管发布事务从已提交基线统一生成，不能绕过版本保护或再升第二次。
- 发布前独立审计发现 P1：仅按最近触及 VERSION 的提交定位边界会被等值空白修改截断历史。修复为跳过等值格式变化，真实升版必须有该提交新增的唯一 CHANGELOG 版本节和一致的原生 manifest；初次引入 VERSION 可作为兼容基线，降版、非法值、删除后重引入停止。纯 VERSION 等值格式变化不驱动发布。正在补边界回归和独立复审，修复前全量结果不充当修复后发布收据。
- review-auditor 对实际修复复审通过：原 P1 代码问题关闭，无新增发布阻断；独立 Agent 没有执行测试。主对话修复后 release 回归 21/21 通过。第一次重跑因旧测试 EXE 正在运行而发生 LNK1104，等待旧进程结束后原命令通过。
- 已核对并恢复仅本轮预写的 VERSION、CHANGELOG 版本节与三组 Cargo manifest/lock；工作区暂用 1.22.0 发布基线，功能源码保留，目标仍为受管事务生成的 1.23.0。修复前工厂 suite 在验证期间已失效并停止，重新完整验证，不将其作为最终发布收据。

## 开发阶段验证收据（发布审计修复前）

以下 Cargo 命令均在工厂根目录运行；需要 Git 写入的测试只写临时夹具及本地 bare remote。

| 编号 | 命令 / 检查 | 结果与断言 |
|---|---|---|
| 1 | `cargo test --locked --config scripts/tests/factory-cargo.toml --manifest-path templates/hooks/Cargo.toml -p bridgeforge-core release::tests --no-fail-fast` | 16 passed：累计 major/minor/patch、当前修改、历史分类、重复发布、非法输入、只读计划 |
| 2 | 同入口过滤 `git_sync::tests` | 21 passed：普通同步、历史发布、旧策略、失败回滚、既有 Git 安全路径 |
| 3 | 同入口过滤 `explicit_release_` | 7 passed，含新增混合 region 和 merge 阻断；与前两项有交集，共 39 个不同 core 测试 |
| 4 | `cargo test --locked --manifest-path scripts/tests/Cargo.toml factory_version_config -- --test-threads=1` | 2 passed：三个 Cargo manifest/lock 同步及既有合同检查 |
| 5 | 同入口分别过滤 `rust_source_is_identical_in_template_and_dogfood`、`managed_manifests_are_current_and_python_free` | 各 1 passed：全部源码镜像一致、生成清单无漂移 |
| 6 | 同入口过滤 `real_cli_explicit_release_preview_and_execution` | 1 passed：真实 CLI 能力标记；普通同步不升版；预览保持 HEAD/index/FETCH_HEAD/VERSION 不变；发布 1.0.0 → 1.1.0 且 clean/0/0；重复发布不产生新提交 |
| 7 | `.codex/bin/bridgeforge.exe self-test --json` | version=1.23.0，能力包含 `git-sync-explicit-release-v1` |
| 8 | `check skill-metadata`、`check project-structure`、`check factory-version`、`manifest --check` | 无错误；仅保留既有归档建议；版本 1.23.0 和 CHANGELOG 一致 |
| 9 | `.codex/bin/bridgeforge.exe check baseline --root .` | state=clean，包含源码、生成二进制与实测收据；该 clean 表示骨架基线，不表示 Git 工作区无改动 |
| 10 | `rustfmt --check`（本轮 Rust 文件）、`git diff --check` | 通过 |

共 44 个不同定向测试通过。生成源码树 SHA-256 为 `223618da47b00d49613da1bfdbea21b27930b8eaa546d15ad3152c8b947dffef`；CLI 二进制 SHA-256 为 `09c7273e8f4add1b429a92ed1ea7518017f8257dfb5371613462e556b3d218f7`。

截至开发阶段，上述收据未包含正式发布前全量 fixture/自动测试与独立审计、真实 StratusAgent 升级及启动编译影响、用户级 Skill 分发和真实远端同步。后续验收发布证据见下节，不将开发阶段的本地 1.23.0 准备状态当作发布收据。

## 验收发布阶段收据

本节为审计修复后的证据；上节保留开发阶段历史，不替代本节。

| 编号 | 验证 | 结果 |
|---|---|---|
| 1 | review-auditor 独立审计及修复复审 | 原 VERSION 格式变化截断历史的 P1 已关闭；实际源码复审通过，无新增发布阻断 |
| 2 | 修复后 release 定向回归 | 21 passed，0 failed |
| 3 | 模板完整 workspace：`cargo test --locked --config scripts/tests/factory-cargo.toml --manifest-path templates/hooks/Cargo.toml --workspace -- --test-threads=1` | CLI 14、core 167、Hook 25，共 206 passed、5 ignored、0 failed |
| 4 | dogfood 完整 workspace：相同命令使用 `.codex/hooks/Cargo.toml` | CLI 14、core 167、Hook 25，共 206 passed、5 ignored、0 failed |
| 5 | 受管 build-assets 与完整 baseline | 修复后源码、二进制和实测收据一致；发布准备基线为 1.22.0，baseline state=clean |
| 6 | 修复后 `git-sync --release-preview --message-file .runtime/git-sync-explicit-release-message.txt` | release-planned，factory，1.22.0 → 1.23.0；VERSION、CHANGELOG 与三组 Cargo manifest/lock 同一计划 |
| 7 | manifest --check、skill metadata、project structure、factory-version、格式与 diff 检查 | 通过；既有归档建议不作为阻断 |
| 8 | 完整工厂回归与 fixture：`cargo test --locked --manifest-path scripts/tests/Cargo.toml -- --test-threads=1` | 88 passed、3 ignored、0 failed；覆盖真实 CLI 发布与运行时修复、骨架真实初始化构建、升级及幂等事务 |

2026-10-08 按用户明确的“同意验收”完成本次上游交付收口。两套完整 workspace 与工厂 suite 均通过；条件忽略项包括由其它测试调用的子进程 helper，以及需要额外环境或真实项目授权的测试，未冒充已执行。真实 StratusAgent 升级/运行、用户级安装不在本次验收范围内，仍未验证。

本次授权发布目标为 origin/main，提交消息为 `feat: 分离日常同步与显式发布并按累计改动统一升版`；受管 `git-sync --release` 从 1.22.0 统一生成 1.23.0 并完成提交推送，实际 Git 成败、commit 与最终 0/0 以该次机器收据和对话交付为准，本文不预写成功结果。历史 `codex_git_sync_autostash` 保留，不纳入本轮提交。

当前 topic 已符合归档候选条件，但没有执行归档或改动其他 topic；需用户另行调用 `$archive-scan`。

记忆核对：只读核对既有受管 Git 入口、无变化不空提交、最终真实推送目标 0/0 与历史 stash 保留经验；这些已由当前工具、Skill 和项目规则承载，不新增 Rule / Hook / AGENTS.md 建议，不写原生 Memory。索引引用的旧 rollout 摘要路径已不存在，未将其视为本轮验证证据。
