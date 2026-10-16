---
lifecycle: active
validation_status: awaiting_user_acceptance
---

# git-sync 首次配置引导与 Cargo 隐式成员发现修复

最新情况（2026-10-17）：2.5.0 已按用户授权发布；StratusAgent 已升级到该版本，原隐式成员阻断已越过，但历史锁文件迁移兼容仍阻止独立发布。原因与证据见文末“发布后真实下游复核”，换机从[交接文档](handoff_2026-10-17_historical-lock-migration.md)继续。本卡不因此记为用户已验收。

## 用户目标、决定和授权

用户要求 git-sync 首次调用时提示选择配置，避免下游不知道如何使用已发布的新策略；已选择 A：升级到带引导的版本后，每个项目首次调用都提示，已有配置也展示并由用户确认。用户随后选择随项目保存，并明确“这个选择当然应该记住”。配置文件使用现有 `.codex/bridgeforge-version.json`，后续调用、换机、重新 clone 和普通版本升级不重复询问；只有用户主动更换策略才重新选择。配置缺失或损坏时不能假装已经记住，需报告真实状态。

2026-10-16 用户补充：“下游项目在配置上个版本 git-sync 时发生了错误，你看看。这个 bug 修复也要加入需求卡”。该指令明确授权只读调查、保存本卡、必要文档索引及原需求 TODO；尚未授权代码实施、提交、推送、发布、用户级安装或真实下游写入。此前 2.4.0 的实施与发布授权不延续到本次新增改动。

唯一需求卡为本文件。TODO-011 为首次配置引导，TODO-012 为 Cargo 隐式成员发现修复；两项可分别验证，纳入同一交付包。原 2.4.0 实现及工程证据见 [Rust 独立包发布](../git-sync-independent-packages/requirements_2026-10-16_rust-release.md)，不重写原交付历史或将本次新增项记成已经完成。

用户随后明确：“按需求卡开始实施，把首次配置引导和 Cargo 隐式成员发现的 Bug 一起做好，完成必要验证，先不要提交推送，也不要修改真实下游。”由此授权本卡代码、Skill、dogfood、分发合同、隔离测试和独立审计；承接本卡建议整体预算与安全实施默认。不安装用户级资产、不修改真实下游、不提交推送或发布。

## 规模、建议预算和停止点

按 M 级跨模块任务执行：复用既有受管发布事务、Rust 适配器和项目配置；新增引导状态及配置入口，并修复同一适配器的成员发现闭包。用户按本卡开工，承接展示的整体预算 120 分钟、30k 新增 token（估算，未实测）、一个 review-auditor 审计任务、最多两轮完整验证；预算包含本卡 Bug、定向复现、工厂强制检查及合理返工。中断恢复不重置累计预算；原审计会话若失效，仅恢复同角色的最终收口。当前没有可确认匹配当前模型的周额度系数，不虚构百分比。

未知 Cargo 语义影响范围、无法安全生成配置、项目自有 Hook/展示尚需改造、配置或输入漂移、预算预计不足时，仅停止相应动作并报告具体卡点。合法成员发现不得靠静默跳过包、放宽锁一致性或强行把独立 workspace 塞进上层 members 解决。

## 已确认行为与实施默认

| 编号 | 项目 | 合同 |
|---|---|---|
| 1 | 首次触发 | 新引导版本启用后，每个项目首次普通 git-sync 或 git-sync release 都先展示；已有配置也展示当前策略并确认 |
| 2 | 适用策略 | 保留当前规则；Rust 业务项目可选已支持的独立包升版。纯 Python 等项目不提供 Rust 独立包选项 |
| 3 | 业务识别 | 按业务 manifest、workspace 和项目结构判断；排除 BridgeForge 自带 Hook、依赖缓存等干扰，不能因安装骨架而误判 Rust 业务 |
| 4 | 持久化 | 选择与首次确认记录保存在项目 `.codex/bridgeforge-version.json`，随 Git 同步；不新建另一份选择事实源或本机确认清单 |
| 5 | 记住选择 | 换机、clone、后续调用及普通产品升级不重复问；不把产品版本号变化当成重新确认条件 |
| 6 | 原同步语义 | 普通同步仍不升版；显式 release 按项目规则执行；保留已有 Git、构建、Hook 和失败恢复保护 |
| 7 | 无配置项目 | 保留原有自动识别语义；确认记录落盘不能制造 manifests 缺失／空列表不合法等新发布阻断，尤其覆盖纯 Python 项目 |
| 8 | 继续原动作 | 技术默认：选择可安全落地时，经受管入口保存后继续原命令；选择或写入未完成时不擅自执行原同步或宣称已启用 |
| 9 | 配套适配 | 技术默认：若需修改项目自有版本 Hook、UI/日志版本来源，先展示接入方案，另行授权；不在 git-sync 中自动扩张为业务代码改造 |
| 10 | 非目标 | 不新增 Python 包版本管理，不扩展语言适配器，不把构建／业务测试变成日常 git-sync 前置，不自动改所有下游配置 |

前七项继承用户选择及现有行为边界；第八、九项是待整体开工确认的安全实施默认，未伪装成用户已经逐字决定。

## 已核实 Bug：合法隐式成员未被发现

### 来源和现象

来源对话：[确认 Cargo.toml 版本升级的编译影响](codex://threads/01a14421-0203-7633-9d00-2a5aa9e52c49)，2026-10-16 项目接入回合 `01a144ac-1df1-72f1-a2bf-324933b25e54`。已读取真实命令输出与工厂、下游相关 manifest；下游对话仍在进行，以下是读取时的证据，不代表其后续接入工作已经完成。

下游已安装 2.4.0，并尝试启用 Rust 独立包规则。实际 `git-sync --release-status` 返回：

```text
stratus/app-manager/Cargo.toml: local dependency stratus-common-research-input is not in registered workspaces: stratus/research-input/Cargo.toml
```

同一漏包也表现为：

```text
shared Rust input references unknown or duplicate package: stratus/research-input/Cargo.toml
```

只读现场核实：`stratus/Cargo.toml` 的显式 members 未列 `research-input`；`stratus/app-manager/Cargo.toml` 通过 `path = "../research-input"` 引用它；该包位于 workspace 目录内，没有独立 `[workspace]`，并继承所在 workspace 的 edition、rust-version 和 lints。它是 Cargo 合法自动纳入的本地成员，而非缺失 manifest。[Cargo workspace 规则](https://doc.rust-lang.org/cargo/reference/workspaces.html#the-members-and-exclude-fields)说明 workspace 目录内的 path 依赖会自动成为成员。

### 已知原因和修复范围

工厂事实源 `templates/hooks/crates/bridgeforge-core/src/release/rust.rs` 的 `load` 只从根 package 与显式 members 构建 packages，随后 `collect_dependencies` 要求所有 path 目标已登记。未递归发现隐式成员，因此在构图或 shared_inputs 校验前后阻断。2.4.0 文档曾将未登记本地包作为支持限制；真实项目暴露出该限制覆盖了 Cargo 的正常成员语义，本次作为产品能力缺口修复，不归咎于用户写错配置。

修复要求：

1. 从显式根和成员出发，按 Cargo 语义递归发现合法本地 path 隐式成员；覆盖 workspace 继承依赖、重命名、普通／构建／开发、可选和 target 条件声明，保持确定性、去重与循环终止。
2. 正确判定实际 workspace 归属。保留 exclude、独立 `[workspace]`、package.workspace 指针与跨已登记 workspace 的边界；外部路径包不能一律当成当前 workspace 成员，也不能静默加入可升版集合。
3. 当前工作树、历史发布基线和逐提交归属使用同一发现逻辑；首次迁移不能误报新增包，版本、依赖传播、shared_inputs 和锁文件校验必须能识别发现的合法成员。
4. 将发现时读取的 manifest、祖先及缺失状态纳入输入守卫；维持预览零写入、无联网、无 Cargo 解析进程的既有边界。
5. 产品源与 dogfood 同步；增加 Cargo 自身离线 metadata 对照夹具和回归测试，不使用 StratusAgent 名称或路径白名单修复通用逻辑。

### 对话中另见的接入问题

| 编号 | 观察到的报错 | 当前判断 |
|---|---|---|
| 1 | scripts/database 的本地策略依赖 etf510050-hypergamma 不在 registered workspaces | 属于另一条跨目录／workspace 归属证据；需查清实际根与配置，不直接归为 research-input 的同一缺陷 |
| 2 | 项目 version-sync Hook 报策略模板锁中 stale or duplicate local package stratus-clock | 属于下游 Hook／锁一致性诊断；不能通过放宽上游锁检查处理，根因及修复仍由下游接入流程核实 |

本卡将上述两项作为修复后重测时必须区分的诊断，不宣称它们已修复，也不授权在此修改真实下游的业务 Hook 或锁文件。

## 拟修改与传播四问

| 编号 | 落点 | 目的 |
|---|---|---|
| 1 | templates/hooks/.../release/rust.rs 及必要拆分模块 | Cargo 合法成员发现、实际归属、输入守卫 |
| 2 | templates/hooks/.../release/config.rs、git_sync.rs、CLI | 首次状态只读查询、受管配置写入、确认记录和原同步衔接；正式接口名由实现确定 |
| 3 | skills/git-sync/SKILL.md 及必要 reference | 对话内展示适用选择并记住，不启动可见交互控制台；入口继续满足行数硬闸 |
| 4 | .codex/hooks、受管 manifest | dogfood 镜像、资产 id/hash、能力自检及分发合同同步 |
| 5 | scripts/tests/** | 首次／重复、配置兼容、Python／Rust、隐式成员、历史与失败场景 |
| 6 | 本卡及 doc/README.md | 唯一需求和工程证据，不复制完整事实源 |

两项均为通用产品能力，进入 templates/ 与共享 skills/，同步自身 dogfood；StratusAgent 专属 Hook、窗口、ABI 等约束不下沉到公共产品。正式发布时需要产品版本与 [product] CHANGELOG，由另行授权的受管 release 生成；本轮只记需求，不预写版本，不对工厂执行 project-sync adopt/apply，不写骨架版本戳。

## 验收矩阵

| 编号 | 验收项 | 预期 |
|---|---|---|
| 1 | 旧 schema 1、现有 schema 2、无配置项目首次调用 | 都能展示准确当前规则及适用选项；确认前零写入和零 Git 同步 |
| 2 | 选择保存、重复调用、clone／换机、普通升级 | 从项目配置复用有效确认，不重复提示 |
| 3 | Python 业务项目含受管 Rust Hook | 不显示或误启用 Rust 独立包策略；落盘后仍保留原发布行为 |
| 4 | 配置写入失败、拒绝或未回答、输入漂移、未知版本 | 不伪称已确认或已启用，不执行依赖该选择的同步，不破坏原配置 |
| 5 | 隐式成员直接及递归 path 引用 | 与 Cargo 离线 metadata 成员集合对照一致；正常参与版本、反向传播、shared_inputs 及锁校验 |
| 6 | exclude、独立／嵌套 workspace、跨登记根、外部 path 包 | 实际归属正确，边界不被一律并入，未知情况明确阻断 |
| 7 | 历史基线、首次迁移和重复 release | 使用相同发现语义，不错判身份或漏包、不重复升版 |
| 8 | 普通同步、统一升版、独立升版、失败恢复 | 既有行为和保护不退化 |
| 9 | 工厂工程收口 | 定向测试、受管构建、完整 workspace／factory fixture、manifest／结构／镜像检查及独立审计 |
| 10 | 真实下游及 runtime | 仅在另行授权的测试 worktree 受管更新与验证；缺证据明确标未验证 |

## 当前进度

实现、独立审计及工程检查已完成，等待用户验收，尚未发布。主对话实现和整合，review-auditor 独立审计；未委派实现 Agent。中断前的构建进程句柄在恢复后失效，已通过当前产物自检及受管 baseline 复核 CLI/Hook 关联，不以无法回收的进程输出冒充测试终态。最终完整工程准备在恢复后连续执行并取得正式收据。

### 实际入口及配置合同

1. `git-sync --setup-status` 只读查看首次状态；已确认时走仅配置读取的快路。用户主动换策略时加 `--reconfigure` 才重新扫描并展示选项。
2. `git-sync --setup-plan --strategy current|rust_independent` 生成零写入计划；用它返回的完整 fingerprint 执行 `--setup-apply --strategy ... --setup-fingerprint ... --confirmed-choice`。原始 CLI 同步／release 在确认前返回 configuration-required，不调用 Git 同步；只读诊断和独立开发准备保持可用。
3. 保存字段为 `git_sync_setup: {schema: 1, confirmed: true, selection: "current"}`（selection 也可为 rust_independent）。记录不绑定产品版本号或机器路径，不维护本机副本。无配置项目使用 schema 1 的 manifest_policy=auto 保留原自动识别；已有配置及项目自有字段保留。
4. Rust 候选错误只影响该候选，不能阻断保留当前规则。已确认配置仍接受纯结构校验，损坏字段不能冒充正常记忆。候选切换先校验历史身份和累计未发布改动的归属；需要 Hook／产品版本来源／共享输入适配时明确阻断，不擅改业务。
5. 指纹覆盖所选策略、输出配置、HEAD、文件清单及候选追加读取的锁／祖先／缺失输入；apply 持锁重算并校验后原子写入。Rust 隐式发现与历史共用同一快照读取器。

### 首轮证据与经验

首轮 release 定向集 57 项中 56 passed、1 failed；失败来自迁移预检沿旧静态版本读取路径不识别合法单引号，已经改为 TOML 静态读取并使用候选 Rust 历史规则校验。其后首次引导修正集 8 passed、0 failed。追加的配置结构与中间历史校验、隐式成员真实 Git 历史覆盖已纳入最终完整检查；中断前 62 项定向集末尾未回收，不声明其完整通过。

原 review-auditor 指出的候选失败不阻断 current、schema 1 结构、完整计划指纹、候选历史迁移、schema 2 静态结构及累计历史覆盖均已修正。中断前源码阶段结论 passed、无未解决发现；恢复后只针对最终稳定差异与机械传播收口。

恢复后复审发现首次引导 reference 未在分发清单显式登记：受管 manifest 重建器只刷新既有条目 hash，不自动发现 reference。已补 `skills/git-sync/references/first-use.md -> references/first-use.md` 登记，并在既有分发一致性测试中断言每个平台均包含该文件。恢复审计最终 passed、findings=[]，绑定指纹 `sha256:2f191e68758fcd33a4febbd5ac507616fe153551f206c36c6ffec59224b43d73`；审计未运行测试，完整工程结果另记。

经验：版本传播声明图、workspace 成员归属与锁必需闭包是不同概念；发现合法 path 成员不等于把所有外部路径包并入本根。首次确认记录也不等于发布配置已兼容，保存前必须检验候选策略本身及相关历史。修复证据最终以工程收据为准，真实下游与自然 runtime 仍未验证。

### 最终工程证据（2026-10-16）

受管入口 `.codex/bin/bridgeforge.exe git-sync --prepare-release --message-file .runtime/first-use-message.txt --audit-file .runtime/first-use-audit.json` 返回 exit 0、`status=prepared`、`checks_passed=7`，隔离构建目标版本为 `2.5.0`。最终校验指纹与独立审计一致：`sha256:2f191e68758fcd33a4febbd5ac507616fe153551f206c36c6ffec59224b43d73`。

收据位于 `.runtime/bridgeforge-codex/release-preparation/current.json`；它是本机忽略产物，不作为仓库交付文件或发布授权。以下日志位于同目录的 `blobs/<hash>`，所有检查 exit_code 均为 0。

| 编号 | 检查 | 实际结果 | 日志 SHA-256 |
|---|---|---|---|
| 1 | baseline | 当前版本产物及工作区基线通过 | `660dbbbe86fc11be198f4536c4e29a9f726418558c971ed23b60b2722e83b15a` |
| 2 | skill metadata | 通过 | `2f4914ad120aad474ada903be1a4a2b10cd859eab34c0ef951d8ad6c4ca2efdd` |
| 3 | manifest --check | 通过，包含首次引导 reference 分发登记 | `4f26cd005522df1df06e7e991cad2699681b25283e63142caef1b1d6c07e26e5` |
| 4 | project structure | 通过 | `64d1f73a223dfb46f669cd6ff7282a62d6021a49a99d1869d0a66b5d87889497` |
| 5 | template/dogfood mirror | 源码全树逐字一致；1 passed | `224f4757462c4ca174d154416fe7f7565e13f670e7329cf125d5e92d248397a1` |
| 6 | 完整 workspace | 312 passed、8 ignored、0 failed | `c63fe124cc24677337e132fe0fcaa8adecb1ac19edaa18cae5588119e1f08acc` |
| 7 | 完整 factory fixture | 90 passed、6 ignored、0 failed | `814af5da5e068ba3d141b9f9d500e9cfdd33175aa32f835ba005b34b1ff5285b` |

workspace 命令为 `cargo test --locked --config scripts/tests/factory-cargo.toml --manifest-path .codex/hooks/Cargo.toml --workspace -- --test-threads=1`；factory 命令为 `cargo test --locked --manifest-path scripts/tests/Cargo.toml -- --test-threads=1`。两套完整集共 402 passed、14 ignored；镜像检查是 factory 集中的同一测试，不重复计入。忽略项包括显式启用的性能实验、辅助子进程、live log/外部授权夹具及特定运行入口，不计为通过。

本轮确认了 Python 业务含受管 Rust Hook 的识别、现有配置保留、确认持久化与主动重配、配置损坏及输入漂移阻断、候选历史预检；Cargo 直接及递归隐式成员与真实离线 metadata 对照、workspace 边界、历史归属及原发布行为均进入完整集。factory 中真实 CLI 发布、自动 runtime 构建、pre-commit 衔接和生成资产应用均在隔离临时仓库执行，不代表真实下游验收。

| 编号 | Bug 关闭证据维度 | 状态与边界 |
|---|---|---|
| 1 | 源码 | templates 通用实现完成，独立审计 passed、findings=[] |
| 2 | 产品传播 | Skill/reference、分发 manifest 与 hash 校验通过；未发布、未安装用户级资产 |
| 3 | dogfood | 镜像一致、当前版本受管产物 baseline 与完整 workspace 通过 |
| 4 | fixture | Cargo metadata 对照与完整 factory fixture 通过 |
| 5 | 真实下游 | 未修改，未验证；StratusAgent 现场接入问题没有据此宣称修复验收 |
| 6 | runtime | 隔离 CLI/runtime 测试与目标构建通过；用户级安装后的自然调用未验证 |

最终准备约从 22:44 到 23:27，耗时约 43 分钟；其中 workspace 检查收据约 22.1 分钟，factory 检查约 17.2 分钟，余下为基线、镜像、快照及目标构建。此处仅披露连续工程准备耗时，不冒充从用户开工到回复的端到端耗时。恢复前实施、审计及中断期间未取得完整计时，整体 token 未实测，不能宣称精确预算用量或周额度百分比。

在该工程准备结束时，尚未提交、推送、发布、安装用户级资产或写入真实下游，工作区 `VERSION` 为 `2.4.0`、HEAD 为 `d9da2bb64c37030c9188477f31e43522ed6a6cc0`，`2.5.0` 当时仅为隔离目标。随后发布记录见下节。生命周期保持 active，TODO-011/TODO-012 不自动结算，用户验收与后续发布另行判断。

## 发布后真实下游复核（2026-10-17 整理）

### 已发布部分与后续授权

用户随后显式调用 `$git-sync release`，并在本工厂首次引导中选择 1：保留当前统一升版。受管 setup 仅向 `.codex/bridgeforge-version.json` 添加 `git_sync_setup` 确认，原三项 manifests 保留。受管 release 将 `2.4.0` 升至 `2.5.0`，提交 `0e649b1486a88f3beb75af10fecff25019cea2a3`，消息为 `feat: 增加 git-sync 首次配置引导并修复 Cargo 隐式成员发现`，已推送 `origin/main`。独立终态核验工作区干净、HEAD 与 origin/main 相同、ahead/behind=0/0；产物复用 2、重建 0，工具内部同步 43672 ms，端到端未精确计量。原有 1 条 stash 保留。

用户后续询问下游仍认为未完全修复的原因，本轮获准只读调查并解释；没有因此实施新的修复或修改真实下游。当前 `$summary switch` 授权沉淀与交接，另有普通 `$git-sync` 授权保存这些文档，不代表新修复开工或用户验收，不升版、不结算 TODO。

### 已核实剩余阻断

来源仍为[确认 Cargo.toml 版本升级的编译影响](codex://threads/01a14421-0203-7633-9d00-2a5aa9e52c49)，最新复核回合 `01a1456a-57db-7440-86f1-60cfbe21a342`。对话证据显示下游骨架版本与受管工具 self-test 均为 `2.5.0`，具有 `git-sync-setup-v1` 与 Rust 独立包能力；不是版本未传播。下游已选独立包策略，确认记录尚未保存，不应要求重新选择。

主对话在真实 StratusAgent 再次只读执行 `git-sync --release-status`，返回 `status=blocked`，原始原因：

```text
strategies/template/blackswan/Cargo.lock: local package stratus-clock has missing, ambiguous, or stale version
```

| 编号 | 现场读取 | 实际数据 |
|---|---|---|
| 1 | `HEAD:.codex/bridgeforge-version.json` | schema 1，只登记 `stratus/Cargo.toml`、`scripts/Cargo.toml` |
| 2 | `HEAD:strategies/template/blackswan/Cargo.lock` | stratus-clock 锁记录 `1.50.0` |
| 3 | `HEAD:stratus/clock/Cargo.toml` 与 `HEAD:stratus/Cargo.toml` | 包继承 workspace version；有效版本 `1.51.0` |
| 4 | 当前 `strategies/template/blackswan/Cargo.lock` | stratus-clock 已修正为 `1.51.0` |

原隐式成员错误已越过；当前阻断是旧历史锁滞后，并非当前锁仍未修好。旧锁确有不一致，但该策略 workspace/锁并未纳入旧 schema 1 的声明合同；不能直接把新增独立发布合同倒套所有旧提交，也不能用修改当前文件或改写 Git 历史解决。

### 根因、独立复核与测试遗漏

`templates/hooks/crates/bridgeforge-core/src/release/independent.rs` 的 `historical_workspace`（2.5.0 第 32–45 行）使用传入的当前 RustConfig 调 `rust::load`，不读取该历史提交自身配置；`rust.rs` 第 515–516 行无条件执行 `validate_locks`，第 827 行要求本地锁记录与当时有效包版本完全一致。`validate_current` 第 64–65 行及中间提交回放第 144 行都进入这条路径。因此发布 boundary 和累计中间提交均可能受影响。

另一条历史发布合法性检查 `release_history.rs::verify_historical_manifests`（第 85–118 行）会读取历史配置，schema 1 只按当时 manifests 校验。两条历史路径使用了不一致的合同：旧规则认可的发布，在新策略回放时被新增锁校验拒绝。

按项目要求，`review-auditor /root/historical_lock_audit` 对本轮源码和 fixture 独立只读复核，确认 1 项 P1。审计未运行测试或构建；其源码结论独立取得，下游精确版本数据采用主对话现场证据。既有 `core_release.rs` 迁移 fixture 只有一个工作区，manifest 与锁始终一致为 `1.0.0`；迁移测试仅修改业务源码与配置，没有覆盖“旧合同未管理的额外 workspace，其历史锁滞后，当前修好后首次纳入”的场景。上一轮 402 项通过不能证明该遗漏场景通过，更不能据此把真实下游标为可发布。

修复方向尚待实施授权：仅对有证据的旧 schema 1、当时未纳入旧合同的 workspace/锁提供限定迁移兼容，同时保留包身份与有效版本守卫。当前锁、schema 2 历史锁仍应严格校验；不能全局关闭锁检查，不能静默丢包或退回统一升版。除 boundary 外还需覆盖累计中间提交。

应补正例：旧 schema 1 只管理 A/B，额外 C 的旧 boundary 或中间提交锁滞后；当前修好 C 后切到 schema 2 纳入 C，可受管预检。应补反例：当前锁滞后、已纳入 schema 2 的历史锁滞后、身份/有效版本漂移仍必须阻断。随后才能在另行授权范围内验证真实下游只读 release-status/release-preview；不得把只读预检当实际发布验收。

经验：共用成员发现算法，不等于所有历史时点共用当前校验合同；历史回放必须区分“事实归属”与“该时点已生效的规则”。测试不能只用全程一致的理想历史，应覆盖旧合同外的滞后数据及当前已修好的迁移状态。

### 剩余范围与交接边界

历史锁迁移尚未修复，未新增代码、fixture 或工程收据，也未对真实下游写入或执行发布。剩余修复与完整必要验证估计 90–120 分钟，仅为时间估计，不是已获实施授权或新的 token 预算。当前不新增 Rule/Hook/AGENTS 建议：问题有明确代码与测试落点，已有证据约束足够；原生 Memory 索引有界检索未命中该历史锁主题，未写入 Memory。当前关联 topic 仍有 blocker、未获用户验收，无归档候选。
