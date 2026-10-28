---
lifecycle: active
validation_status: awaiting_user_acceptance
---

# 核心能力保留与非核心、过期实现退役确认卡

日期：2026-10-28。关联原需求：**TODO-045**。

## 1. 当前结论与授权

需求范围及整体预算已确认。确认卡落盘后，用户在本对话明确要求“开始吧”，已授权按本卡实施清理、必要验证和受管子包收口。**不提交、不推送、不发布。** 本卡已补充工程交付结果，尚未取得用户验收。

目标是在保留现行核心能力的前提下，整块退役已明确不需要的功能及其实现、入口、配置、分发登记和专属测试，并清理无当前消费者的辅助入口和重复代码。测试保留以有效行为和风险覆盖为标准，不要求维持旧测试数量，也不为旧测试继续维护作废实现。

用户的原始要求包括“希望清理掉足够多的冗余代码，只保留核心逻辑”。最终净减少行数分别统计生产源码、独立测试、Skill/说明、机器配置、源码投影与收口账本，不把文件搬迁算作删除，也不把镜像重复计算为独立功能清理量。

## 2. 已确认决定

| 编号 | 能力或范围 | 用户决定与处理边界 |
|---|---|---|
| C01 | 独立发布候选准备 | 退役 `--prepare-release`、`--development-status`、prepared 演练及其专用检查缓存、配置和专属测试。用户明确要求纳入退役，先不实施。 |
| C02 | 多项目批量骨架升级 | 整块退役工厂批量 Skill、批次控制实现、入口、登记引用和专属测试；保留单项目升级与 Git 保存。 |
| C03 | AGENTS 提案专项检查 | 退役 proposal 检查器、相关命令入口和专属测试。提案材料和关联 Bug 历史资料保留。 |
| C04 | 自动检索地图 | 保留地图生成、更新标记、Hook 调用，以及 find-doc / sync-docs 使用方式。 |
| C05 | 开发检查结果复用 | 保留正常 develop 的确定性检查复用及必要环境、输入、日志、产物校验；它与 C01 的专用缓存分开。 |
| S01 | 支持版本 | 仅维持骨架 **8.0.0 及以后**的兼容，不再支持更早项目的接入迁移。按骨架身份判断，不按业务 VERSION、子包版本或状态文件 schema 数字判断。 |
| S02 | 现行核心 | 保留新项目初始化、8.x 及后续项目的单项目升级和恢复、定制保护、现行 Skills 主流程、正常 develop / git-sync / release、语义版本管理、Native Memory、现有 Hook 与事务保护。 |
| S03 | 历史与资料 | 保留 8.x 及以后项目核验自身发布历史仍必需的只读解析、历史 hash、原始账本和项目资料。不因内容提及旧格式就删除。 |

本卡承接本对话的明确回答：C01 不需要；C02 Skill 完整退役；C03 含相关入口和专属测试退役；C04、C05 保留；只需保持 8.x.x 以后兼容。用户最后明确授权“把完整确认卡整理出来。先不实施，也不提交、推送或发布”。

### 版本身份与数据对应

| 对象 | 用途和处理规则 |
|---|---|
| `.codex/.bridgeforge_codex_version` 及经核验的骨架身份 | 判断是否达到 8.0.0 支持下限；不得修改旧戳、冒认新版本或通过删除戳绕过。 |
| 下游业务 `VERSION` | 继续表示业务主程序版本；小于 8.0.0 不代表骨架不受支持。 |
| 独立子包版本 | 保持各自版本与唯一来源，不能把 lifecycle 0.3.0 等合法子包当旧骨架清理。 |
| 原生配置/状态 schema 数字 | 按它在当前 8.x 能力中的实际用途判断；不以数字小于 8 为删除依据。 |
| 发布基线、历史配置、资产 hash 和收口记录 | 当前核验仍依赖时原样保留；不重切历史、重绑旧证明或清空账本。 |

合理假设：新项目初始化和未纳管项目的合法首次接入仍使用当前基线；它们与已识别为 <8 的旧骨架迁移分开判断。来源/身份不明时只读报告并停止相关写入，不以补版本戳或忽略旧配置来推定安全。

## 3. 基准、规模与预算

- 项目：`D:/Quant/BridgeForgeCodex`，分支 main，HEAD `64ee6ba`，主程序 8.0.1；lifecycle 已在上一轮本地收口为 0.3.0。
- 当前已有 R12–R15 的 16 个未提交文件变化，应原样保留。此前的实现、收口与验证不冒充本需求成果；本需求不回退、不重新结算此前事项。
- 规模：**L**。涉及多项现有能力退役、最低支持版本、公共 CLI/Rust 接口、测试合同、受管分发与历史边界，存在共享逻辑需要拆分。
- 已接受的整体执行上限：**3 小时 / 约 100000 token / 最多 1 个独立审查 Agent / 最多两轮验证**。包含调查、主/子 Agent、实施、合理修复、必要集成验证、受管构建、版本收口和收尾；token 未实测。
- 实施中用户明确接受追加 **90 分钟**，仅用于当前第二轮验证、受管收口及最终产物核验；累计时间上限为 4.5 小时，token、Agent、验证轮次及不提交推送发布的边界不变。
- 独立 CLI 复验未通过并完成独立研判后，用户询问剩余时间；已明确告知从当时起预计 60–90 分钟，用户再次要求“那就开始吧”。据此批准该收尾时段及一轮限定追加验证：3 个失败 CLI 用例和 3 个新项目首配场景；仅在证实缺陷时修复承诺保留的初始化行为，不扩大清理范围或全仓回归，仍只有原审查 Agent、100000 token 上限，不提交、推送或发布。
- 原第二轮最终返回 lifecycle 130 通过 / 22 失败、workflow 9 通过 / 7 失败后，已解释此前六项范围估计不完整。用户明确同意“一并完成这些失败项”，将这 29 项原有失败测试纳入同一次定向复验；90 分钟收尾上限不变，不重跑已通过项或全仓。factory 第二轮 8 项均通过。
- 最终构建出现 Windows 内存/分页额度不足，改为单任务构建后又在会话恢复时确认原进程已退出且没有完成收据。恢复命令未命中足够缓存，用户进一步批准“延长 30 分钟完成最后构建”；这次只扩展当前受管构建及终态核验，不修改生产逻辑、不新增回归或 Agent，提交推送发布边界不变。
- 预计消耗约 100000 token，占周额度约 1.5%。该百分比沿用本机现有暂定系数，仅为估计，不是实际扣减或官方保证。
- 本次确认、规划与后续 Skill 交接共用这一整体预算，不重新开始计时或额度。上一轮 R12–R15 的已结束工作不并入本轮成果。
- 预计超出预算、出现核心功能变化、保护规则冲突，或同一问题达到失败升级条件时，保留现场并暂停受影响部分；不静默扩额或删验证。
- 本轮落盘阶段：不启动子 Agent，不运行构建、业务测试、完整回归或下游 fixture。后续独立审查最多使用已接受的 1 名 Agent，沿用当前模型设置。

## 4. 已核实事实与必须保留的共享逻辑

1. C01 两个模块共 853 行，但不是 853 行都可以无条件消失。`source/workflow/src/git_sync.rs` 的 `ReleasePlanningInputs::capture` 仍调用 `preparation::raw_inputs`。该输入枚举/摘要参与普通 release 的计划复用和漂移保护，应归入现有 Git 快照职责后再删除 C01 模块，不建立另一套长期证书或缓存框架。
2. C02 生产模块 657 行，工厂专属 Skill 117 行，独立批次测试 177 行。当前 Skill 位于 `.codex/skills/bridgeforge-codex-batch/SKILL.md`；没有同名 `templates/skills` 源文件。只调整实际存在的登记，不创建或假定第二份 Skill。
3. C03 检查器 64 行；当前调用来自工厂 `check proposal` 入口和 `scripts/tests/src/lib.rs` 中的提案测试。该测试还含其他工厂结构断言，只删除专项部分或拆分保留其余覆盖。
4. 当前受管合同的 `compatibility_baseline` 为 1.8.6。`project_sync.rs` 中低于该值会进入 destructive rebuild，它**不是最低受支持版本的拒绝闸**。不能只改为 8.0.0 就宣称旧项目已被阻断。
5. `schema4/migration.rs` 同时处理旧配置迁移、首次配置、现行配置更新和基线修复。支持下限变化不授权删除整个文件；8.x 新项目初始化、当前配置变更及必要修复必须仍可用。
6. 当前正常发布仍会校验历史发布配置；`config.rs`、`polyglot.rs`、原生适配器或 relocation/hash 数据可能同时服务历史核验。必须按当前真实消费者区分旧实时迁移与只读历史解析。
7. C04 的 Hook 链路、C05 的 develop 复用、Native Memory 同步及原始字节保护均保持。C01 中出现的 prepared、cache 字样不能作为全局删除同名概念的依据；`build-assets` 和公共产物准备仍属于核心构建能力。

## 5. 拟影响文件清单

以下路径相对仓库根目录。它们是后续实施的影响清单，不表示本轮已经修改或删除。混合职责文件只调整命中范围；条件项须有消费者证据，不按文件名整删。

### 5.1 拟整文件退役

| 文件 | 当前体量 | 前置条件/用途 |
|---|---:|---|
| `source/workflow/src/release_preparation.rs` | 676 行 | 先迁出普通 release 仍使用的输入快照逻辑，再退役 C01。 |
| `source/workflow/src/release_check_cache.rs` | 177 行 | 仅 C01 专用缓存退役，保留 C05。 |
| `source/factory/src/batch.rs` | 657 行 | C02 批次状态控制退役。 |
| `source/factory/src/proposal_contract.rs` | 64 行 | C03 专项检查退役。 |
| `scripts/tests/unit/core_batch.rs` | 177 行 | 批次专属测试随能力退役；单项目行为覆盖由其原有测试保留。 |
| `.codex/skills/bridgeforge-codex-batch/SKILL.md` | 117 行 | 工厂专属 Skill 退役及引用清理。 |
| `.codex/development-checks.json` | 配置，不计代码量 | 随 C01 退役；先核对剩余消费者，不删除现行子包 development 检查清单。 |

### 5.2 确定需要调整的生产入口与边界

| 文件 | 拟调整内容 |
|---|---|
| `source/workflow/src/git_sync.rs` | 移除 C01 导出和 preparation 模块连接；保留并归位 release 输入快照、Git 事务、计划复用和漂移保护。 |
| `source/workflow/src/git_command.rs` | 清理准备/准备状态模式；8.x 客户端遇到已退役命令应明确拒绝，不得忽略参数后执行普通同步。 |
| `source/cli/src/lib.rs` | 清理 batch 转发、proposal 转发、C01 能力标签和 usage；保留 C04/C05 与公共产物准备。 |
| `source/factory/src/commands.rs` | 清理 batch/proposal 命令分派与批次适配函数，保持其他受管入口。 |
| `source/factory/src/lib.rs` | 移除 batch/proposal 模块导出。 |
| `source/workflow/src/project_sync.rs` | 基于真实骨架身份落实 <8.0.0 项目的只读拒绝；保留当前安装/升级事务。复核测试专用旧附加入口和已退役迁移分支。 |
| `source/skeleton/src/baseline.rs` | 协同支持下限合同校验，区分历史重建阈值和最低支持版本，保留资产/项目定制核验。 |
| `source/factory/src/manifest.rs` | 维护支持边界与退役资产的事实源/受管生成，确保旧 C01 文件不会残留为有效分发资产。 |

### 5.3 按消费者核实后局部调整的兼容与辅助文件

| 文件 | 允许清理与必须保留的边界 |
|---|---|
| `source/lifecycle/src/release.rs` | 剩余旧辅助导出、测试应用逻辑归位；生产仍需的原生验证与历史归属保留。 |
| `source/lifecycle/src/release_history.rs` | 收敛旧包装；保留 8.x 当前发布仍依赖的历史读取和版本连续性。 |
| `source/lifecycle/src/release/config.rs` | 分离旧实时配置入口与必要历史解析，不整文件删除。 |
| `source/lifecycle/src/release/polyglot.rs` | 只清理无当前消费者的旧实时分支；Rust/Python 原生适配、依赖和历史验证保留。 |
| `source/lifecycle/src/release/setup.rs` | 保留 8.x 首次配置/重配置；退役仅服务旧项目迁移的引导。 |
| `source/lifecycle/src/release/schema4/migration.rs` | 区分首次配置、现行重配置/修复与 <8 项目迁移，后者停止支持；不删除有效 schema 4 配置事务。 |
| `source/workflow/src/project_hook_migration.rs` | 只在证明仅服务 <8 项目后局部裁剪；8.x 项目已有 Hook 行为、状态验证和合法扩展保留。 |
| `templates/source-layout.json` | 逐条核验旧源路径/历史 hash 的消费者；保留 8.x 累计骨架差异与历史归属所需证明，添加本次明确退役的受管资产信息。 |

测试辅助如需迁出，优先归入现有 `scripts/tests/support/`。可能新增 `scripts/tests/support/release_plan.rs` 等专用辅助，须以确有复用为依据；搬迁行数不计净清理收益，不新建通用框架凑抽象。

### 5.4 测试与测试入口

| 文件 | 处理方式 |
|---|---|
| `scripts/tests/unit/core_git_sync.rs` | 删除 C01 专属准备/缓存场景；保留正常同步、发布复用、Git/暂存树及失败保护。 |
| `scripts/tests/unit/factory/git_sync.rs` | 删除 C01 专属断言；保留真实构建缓存、回滚、版本/产物与普通 release 保护。 |
| `scripts/tests/src/lib.rs` | 删除 proposal 检查部分，保留独立的工厂结构与 Rust 规范断言。 |
| `scripts/tests/unit/cli.rs` | 更新退役命令和能力声明断言，验证旧调用明确失败且无写入。 |
| `scripts/tests/support/git_repository.rs` | 清理只为 C01 存在的配置/辅助；复用有效通用 Git fixture，不混并不同业务历史。 |
| `scripts/tests/unit/core_release.rs` | 清理旧入口和旧迁移专属测试；保留当前空变化、骨架归属、历史连续性和漂移保护。 |
| `scripts/tests/unit/core_release_setup.rs` | 首次配置/当前配置仍测；<8 迁移改为支持下限拒绝场景。 |
| `scripts/tests/unit/core_release_schema4.rs` | 按现行配置合同审查迁移案例，保留当前配置、版本与事务语义。 |
| `scripts/tests/unit/core_package_development.rs` | 测试辅助消费者适配；保留子包升版、真实检查和失败回滚。 |
| `scripts/tests/unit/core_package_eol.rs` | 测试辅助消费者适配；保留 Git 证明的换行等价与真实变化阻断。 |
| `scripts/tests/unit/core_package_portability.rs` | 测试辅助消费者适配；保留 C05 和显式环境/交付物边界。 |
| `scripts/tests/unit/core_release_polyglot.rs` | 清理仅旧迁移的断言，保留当前多语言版本与依赖行为。 |
| `scripts/tests/unit/core_release_polyglot_scenarios.rs` | 同上，保留当前调用方与原生锁验证。 |
| `scripts/tests/unit/core_project_sync.rs` | 测试 <8.0.0 拒绝、8.0.0/8.x 保留支持、新项目初始化、漂移、项目保护与回滚。 |
| `scripts/tests/unit/core_baseline.rs` | 检查支持下限与原重建阈值区分、资产合同有效性。 |
| `scripts/tests/src/runtime_flows.rs` | 适配退役辅助入口及 8.x 支持边界，保留单项目事务和 C04 相关覆盖。 |
| `scripts/tests/src/factory_version_config.rs` | 测试辅助调用适配，保留当前主程序/子包版本分工。 |
| `scripts/tests/src/git_sync_runtime.rs` | 按实际 C01 入口/配置引用局部调整，保留正常 Git 收发。 |
| `scripts/tests/src/git_sync_polyglot_runtime.rs` | 若现有迁移 fixture 命中最低支持边界则调整；保留当前 Rust/Python 行为。 |
| `scripts/tests/src/distribution_regressions.rs` | 核对退役资产不再安装、清单与实际投影一致，其他分发断言保留。 |
| `scripts/tests/README.md` | 更新检查入口及已退役能力说明。 |

上述混合测试文件不是整文件删除目标。不能为了让测试通过只把样例中的业务版本改成 8.x，也不能删除一整个混合测试函数而丢失仍有效的保护。C04/C05 的既有测试作为保留能力的回归证据，未命中修改时不为本轮重写。

### 5.5 当前说明、Skill 与自身镜像

- `templates/skills/git-sync/SKILL.md`、`templates/skills/git-sync/references/schema4.md`：移除 C01 入口和可选 prepared 工作流，保留 release 与 develop 的现行分工。
- `.codex/skills/git-sync/SKILL.md`、`.codex/skills/git-sync/references/schema4.md`：由 dogfood 同步，保持单一源。
- `templates/skills/develop/references/completion-release.md`：取消 C01 可选准备导航，保留 C05 和必要验证。
- `templates/skills/bridgeforge-codex/SKILL.md`、`templates/skills/bridgeforge-codex/references/transaction.md`：说明 8.0.0 支持下限及旧项目拒绝边界，保留 C04。
- `doc/0_architecture/design/development-completion.md`、`doc/0_architecture/design/package-version-lifecycle.md`：取消 C01 默认之外的旧可选流程说明，明确现行能力与版本边界。
- `README.md`、`INSTALL.md`、`doc/README.md`：按实际内容更新当前入口、支持下限及导航；安装/恢复完整说明仍由 INSTALL.md 单独维护。
- 本确认卡与 `doc/0_architecture/TODO-INDEX.md`：仅记录本事项真实进度，不自动结算 TODO-042/043/044 或其他历史事项。

根及模板 AGENTS、C04/C05 的 Skill 正文、Native Memory 和现有 Hook 业务实现不作为默认改动目标；若核验发现必须修改受保护规则，先列原文、具体冲突和影响，只暂停受影响部分。

### 5.6 受管投影、机器合同和版本派生

- C01 的两个公共投影拟由受管退役删除：`.codex/hooks/source/workflow/src/release_preparation.rs`、`.codex/hooks/source/workflow/src/release_check_cache.rs`。
- 其他改动的公共 Rust 源文件同步到 `.codex/hooks/source/` 的同包路径；factory 私有代码不投影。
- `.codex/bridgeforge-version.json`：清除退役文件在检查输入中的登记，按实际新增文件补齐输入；保留现行子包 development 检查与 C05，不能把整份合同当 C01 配置删除。
- `source/factory/src/dogfood.json`、`bridgeforge-codex-manifest.json`：核实实际 Skill/资产登记并经受管流程调整，不维护第二份路由表。
- `templates/managed-skeleton.json`、`.codex/managed-skeleton.json`：经受管入口更新资产与退役合同，不手写 hash。
- 受影响 `source/*/Cargo.toml` 及对应公共投影、根 `Cargo.lock`、`.codex/hooks/Cargo.lock`、`scripts/tests/Cargo.lock`、`.codex/package-closures.json`：完成必要验证后由实际受管收口生成；本次已完成。
- 根 `VERSION` 和正式 `CHANGELOG.md` 本次不升版、不编造发布记录。正式 release 保持独立，当前没有发布授权。

## 6. 代码量基线与估算

统计基于本轮现有工作区，不把上一轮 R12–R15 的净减 87 行重复计入。行数包含空行和注释，采用文本行计数，最终与 Git diff 同口径分组核对。

| 类别 | 已核实当前体量 | 预计本需求净减少量 | 估算边界 |
|---|---:|---:|---|
| C01–C03 四个生产模块 | 1574 行 | 约 1450–1600 行 | 先扣除必须迁出的核心快照逻辑，再计配套入口清理和必要替代提示。 |
| 其他旧实时迁移、无消费者包装、重复辅助 | 尚未按可删除函数逐项定量 | 约 50–400 行 | 仅删除可证明不服务 8.x 当前行为或必要历史核验的部分；不能整体删除旧格式解析器。 |
| **生产源码合计** | — | **约 1500–2000 行** | 规划估算，不是已实施结果或必须凑齐的指标。 |
| 独立测试与测试辅助 | 批次专属 177 行；C01 名称相关 18 个函数区间约 842 行 | **约 500–1000 行** | 842 行候选中混有仍有效保护，不是全部删除目标；兼容场景、fixture 合并和新增边界测试相互抵扣。 |
| **实质代码合计** | — | **约 2000–3000 行** | 生产源码 + 独立测试；不含 Skill/文档、配置、账本和公共镜像。 |
| 批量 Skill | 117 行 | 约 117 行说明文本 | 单独计入 Skill 退役，不算 Rust 代码。 |
| 公共源码投影 | C01 两个投影对应 853 行，另有公共局部修改 | 另行统计 | 共享逻辑迁出会产生对应投影，不能简单把体量翻倍当独立收益。 |
| 清单、历史 hash、配置、收口账本 | — | 暂不估计净变化 | 为保留必要历史和追加收口记录，部分文件可能增长。 |

优先确保已确认退役功能被完整移除，不为达到行数删除有效功能、减少核心断言或重写历史。实施后报告实际删除/新增/净变化及估算偏差原因；若调查表明可清理量明显低于此估计，先给证据，不用额外取消核心功能补数。

## 7. 实施步骤与验证安排

| 阶段 | 工作 | 验证与停止点 |
|---|---|---|
| P0 | 固定现有 16 文件基线；列生产入口、8.x 消费者、旧测试与共享逻辑；逐项确定退役和保留。 | 任何仅凭关键词或文件名成立的删除不执行；历史证明或核心行为归属不清时暂停该项。 |
| P1 | 先归位核心 release 输入快照，再完整退役 C01–C03；更新实际分派、能力声明、Skill 登记与专属测试。 | 正常 develop / git-sync / release 仍通；退役命令不能被静默解释为另一种写操作。 |
| P2 | 落实 8.0.0 支持下限，清理旧实时迁移；保留首次初始化、8.x 重配置与必要历史读取。 | <8 项目在写入前拒绝；8.0.0、当前 8.x、新项目、业务版本低于 8 的合法项目均正确处理；不得通过删除版本戳绕过。 |
| P3 | 清理剩余无消费者辅助和重复 fixture，核对混合测试断言，更新当前说明与受管投影。 | C04、C05、Native Memory、Hook 和单项目流程的实际消费者与原有语义保持；历史资料不自动归档。 |
| P4 | 稳定源码后执行预定回归及独立复核，完成受管子包收口与最终产物核验。 | 最多两轮验证；跨模块风险用相关集成集覆盖，不机械全仓清零。不得使用已退役 C01 验证本需求完成。 |

本次验证从现有入口选择必要集合：

1. 受影响 workflow / lifecycle / factory / CLI 单元与定向回归；确认筛选实际执行非零用例。
2. 相关隔离 Git/分发/单项目升级集成：正常同步与发布、8.x 项目定制保护、旧版本拒绝零写入、漂移和失败恢复。
3. C04 地图以及 C05 检查复用的保留行为；Native Memory/Hook 若无实现变化则复用仍有效证据并做必要连接检查，不宣称真实双机或自然生命周期已验证。
4. manifest / dogfood / 受管资产退役一致性、当前说明链接与结构检查。
5. 实际受影响子包通过受管 develop 收口；公共接口与能力取消按实际不兼容影响判级。包级接受记录已绑定实际 fingerprint 和移除范围，保存在受管追加账本中。
6. 由当前源码的受管入口准备最终产物，并以相同任务范围复核无待收口项。源码稳定后合并构建，文档收尾不触发无依据的重复冷构建。

以上为确认时选定的验证计划，实施结果见第 10 节。真实下游操作不在本次授权内。

## 8. 验收标准与风险

| 编号 | 标准 |
|---|---|
| A01 | C01–C03 的有效入口、生产调用、能力广告和专属配置/测试清理完整，历史说明明确不适用于当前；未留下可误导 Agent 的有效兼容壳。 |
| A02 | 8.0.0 及以后项目、新项目初始化和当前版本配置流程保持有效；明确 <8 项目无写入拒绝，业务版本与骨架版本分开。 |
| A03 | 正常 develop / 普通 git-sync / release、版本历史、Git 保护和核心输入快照不回归；C04、C05、Native Memory 和现有 Hook 保留。 |
| A04 | 有效核心行为覆盖保留；作废场景的专属测试随实现退役，不按旧测试数量验收。 |
| A05 | 报告实际生产源码、测试、镜像、Skill/说明和元数据行数变化；搬迁、格式化和镜像不算独立清理成果。 |
| A06 | 受管投影、清单、版本/锁和收口记录一致；不改写既有账本或伪造证据。 |
| A07 | 工程实现、工程验证、用户验收、Git 保存和正式发布分别报告；未执行项明确列出。 |

主要风险：共享实现误删；把骨架 8.0.0 与业务/子包版本或 schema 数字混淆；误删 8.x 发布仍依赖的旧历史证明；为旧测试保留作废能力，或反过来删除混合测试的有效断言；受管删除未传播导致下游残留；把 C01 缓存与 C05/编译缓存混删。以上风险均有对应阶段检查和停止点。

若发现需要取消本卡明确保留的能力、修改 AGENTS 或 Hook 语义、重置发布基线/账本、删除项目资料、写真实下游、安装用户级工具或超预算，只暂停依赖该决定的部分并说明具体影响，不扩大既有授权。

## 9. 传播四问与交接

1. **层级**：C01 与支持下限是通用产品层；C02/C03 主要为工厂能力及自身配置；说明与本卡为元文档。
2. **通用/工厂边界**：公共能力进入 source / templates 并维护对应公共投影；工厂私有 batch / proposal 不下沉为公共模块。
3. **版本与 CHANGELOG**：本次已由受管流程升级受影响子包；主程序和正式 CHANGELOG 仅在另有明确 release 授权时处理，本次未写入。
4. **dogfood**：后续产品改动必须同步受管源码投影和实际登记的自身 Skill 镜像；不更新用户级安装，不写真实下游。

交接给 develop 的实施授权已由用户“开始吧”满足；沿用本卡范围、预算与现有决定，不重复询问 C01–C05 和支持版本选择。开工前已核对当前基准并在忽略目录保存改动前文件快照，当前状态为**工程交付完成，待用户验收**。

## 10. 本次落盘记录

确认阶段仅新增本卡，在 TODO 总表登记 TODO-045（待执行 / 原需求），并更新文档索引；当时未实施代码或版本变更。用户随后授权实施，当前进度如下。

必要的文档级只读核验为 `check todo --root .`、`check project-structure --root .` 和 `git diff --check`；它们不代表清理实现或业务测试通过。既有 R12–R15 文件按落盘前摘要核对，除本次获准追加导航的 doc/README.md 外保持一致，Git 索引未写入。

### 实施与验证进度（2026-10-28）

- 已移除 C01 发布准备与检查缓存、C02 批量升级实现和 Skill、C03 提案检查器的活动入口及专属测试。核心发布输入快照、提交保护、公共产物准备、C04 地图及 C05 开发检查复用保留。
- 已在单项目同步、配置初始化与开发/发布输入边界落实骨架 8.0.0 下限；新项目和业务版本低于 8 的场景保留。旧实时 schema 1–3 迁移不再执行；历史解析仅服务不可变发布证据，测试通过显式当前配置 proposal 建立新基线，不添加生产兼容壳。
- 公共源码退役经受管 manifest 传播，并校验退役目标、已知 hash、漂移和回滚。独立审查发现的无版本身份绕过、加锁前下限检查、退役目标校验问题均已修正。
- 按 `.runtime/core-only-cleanup/before/` 的本次开工快照，当前生产 Rust 净减 1771 行，测试及辅助净减 520 行，实质代码合计净减 2291 行；不含先前 R12–R15、公共投影、Skill、配置和账本。最终统计与完整文件清单见第 11 节。
- 第一轮相关 workspace 检查中 CLI 22 项、skeleton 40 项通过；其余失败已按旧活动夹具、清单未刷新及首次初始化问题归因并修正。第二轮及获准追加验证只复验相关失败与必要连接，未重新启动完整回归。
- 已通过：受管三产物准备、manifest 一致性、Skill metadata、project structure、退役 CLI 零写入 smoke、5 项地图/Hook 连接、3 项单项目同步事务。最终独立测试工程编译通过；保留的普通同步和 release 集成六场景全部通过。
- 第二轮日志位于 `.runtime/core-only-cleanup/`。独立 CLI 复验的 3 项失败已保留：两个发布夹具把无 baseline 的 schema 4 proposal 当作 live 配置并提交，第三个 source-relocation 夹具没有测试函数，受管收口正确拒绝零测试并回滚版本。首次配置同因连续修正失败后进入 escalate，由同一审查 Agent 独立研判，未继续盲修或自行启动第三轮。
- 用户已批准限定追加验证。三个新项目真实 CLI 探针确认：业务版本载体场景通过，独立根包与虚拟 workspace 均被错误的根 Cargo 版本一致性要求拒绝。生产修正只在无 live 配置、历史初始候选也没有配置时采用显式 product_manifests；已有历史配置和正式版本变化仍使用原校验，独立包继续核验不可变原生历史、锁文件与 lineage。该修正已获原独立 Agent 复核，并通过实际受管产物的六场景复验。
- 仍在执行的原第二轮另暴露历史基线修复等测试初始化残留。已统一其中七处当前 proposal 准备，保留历史共享来源、外置 Cargo、漂移拒绝和回滚断言；接口影响测试为新增的 b 包补上其登记 validation。这些 29 项在获得用户补充授权后完成定向复验，结果全部通过；未把静态修正当作测试通过。
- 受管 develop 已返回 development-closed，最终产物和相同任务范围预览核验通过；工程完成不代表用户已验收或已发布。未执行真实下游、用户级安装、真实双机 Memory 或自然 Hook 生命周期验收。
- Git 未暂存、提交、推送或发布；根 VERSION 与正式 CHANGELOG 不写入。另有来源不属于本任务的 `forge-simplification/plan_2026-10-28_command-paths.md`，保留且不计入本次成果。

## 11. 当前交付证据与实际文件清单

代码清理、选定工程验证、受管子包收口和最终产物核验均已完成，状态为等待用户验收。最终收尾在用户追加的构建时间内完成。

| 证据 | 实际结果 |
|---|---|
| CLI 与 skeleton 初轮 | CLI 22 项通过；skeleton 40 项通过，原有忽略项不算通过 |
| lifecycle 相关回归 | 第二轮 130 项通过；原 22 个失败项经修正取得 20+2 项通过，不重跑已通过项 |
| workflow 相关回归 | 第二轮 9 项通过；其余 7 项定向复验通过；初轮其他通过证据按未变行为复用 |
| factory 复验 | 8 项通过，包含构建复用、源/产物漂移、二进制占用与回滚 |
| 保留的地图/Hook 连接 | 5 项通过；沙箱环境下失败后按相同命令提升执行通过 |
| 六个真实 CLI 隔离场景 | 6 项通过：三种新项目首配、普通同步/发布、独立包发布、源码迁移后的发布预览；仅使用临时本地 Git remote |
| 独立测试工程 | 最终版本下 locked/offline no-run 编译通过；未宣称完整独立测试套件已执行 |
| 受管 develop | development-closed；task-selected-final-candidate；实际执行 5 组登记检查，git_written=false |
| 账本和 Git 保护 | 原 48 条记录逐项保持，只追加 5 条；Git 索引、根 VERSION 与 CHANGELOG 不变 |
| 最终受管产物 | prepared=3、written_files=6、self_test_executed=true；只读检查 current、reused=3、built=0；manifest changed=false；factory-version healthy=true |

最终任务预览为 task-development-current，affected_packages=[]、writes=[]、readiness_scope=task；不据此声明全仓 release 就绪。最终命令和原始输出保留在 .runtime/core-only-cleanup/。构建曾遇到标准库元数据异常和 Windows 1455；确认目标/标准库存在后，单任务构建最终成功，未更换工具链或修改分页文件。

受管版本结果：factory 0.2.0 → 0.3.0；lifecycle 0.3.0 → 0.4.0；skeleton 0.1.0 → 0.2.0；workflow 0.1.2 → 0.2.0。checks 仍为 0.1.0，仅追加依赖构建复核；主程序仍为 8.0.1。四个变化包因不兼容清理保持 0.x，未触发自动稳定版。

Rust 行数以本次开工快照为基线，包含注释和空行：生产新增 301 / 删除 2072 / 净减 1771；测试新增 1456 / 删除 1976 / 净减 520；合计新增 1757 / 删除 4048 / **净减 2291**。测试新增部分用于保留核心断言、显式当前配置夹具、首次配置边界及受管退役/回滚保护，不提供生产兼容壳。公共投影不重复算作独立代码收益；Skill、文档、版本锁和追加账本另计。

以下列出本任务涉及的 92 个仓库文件，含确认阶段记录和受管投影。排除上一轮 R12–R15 独有的两份文档/合同变动，以及来源不属于本任务的未跟踪 command-paths 计划。受管二进制及构建收据另位于忽略的 .codex/bin，不计文本代码行数。

| 文件 | 动作 | 类别 |
|---|---|---|
| `.codex/bridgeforge-version.json` | 修改 | 配置/受管元数据 |
| `.codex/development-checks.json` | 删除 | 配置/受管元数据 |
| `.codex/hooks/Cargo.lock` | 修改 | 公共投影 |
| `.codex/hooks/source/cli/src/lib.rs` | 修改 | 公共投影 |
| `.codex/hooks/source/lifecycle/Cargo.toml` | 修改 | 公共投影 |
| `.codex/hooks/source/lifecycle/src/release.rs` | 修改 | 公共投影 |
| `.codex/hooks/source/lifecycle/src/release/schema4/inputs.rs` | 修改 | 公共投影 |
| `.codex/hooks/source/lifecycle/src/release/schema4/migration.rs` | 修改 | 公共投影 |
| `.codex/hooks/source/lifecycle/src/release/schema4/plan.rs` | 修改 | 公共投影 |
| `.codex/hooks/source/lifecycle/src/release/setup.rs` | 修改 | 公共投影 |
| `.codex/hooks/source/lifecycle/src/release_history.rs` | 修改 | 公共投影 |
| `.codex/hooks/source/skeleton/Cargo.toml` | 修改 | 公共投影 |
| `.codex/hooks/source/skeleton/src/baseline.rs` | 修改 | 公共投影 |
| `.codex/hooks/source/workflow/Cargo.toml` | 修改 | 公共投影 |
| `.codex/hooks/source/workflow/src/git_command.rs` | 修改 | 公共投影 |
| `.codex/hooks/source/workflow/src/git_sync.rs` | 修改 | 公共投影 |
| `.codex/hooks/source/workflow/src/project_sync.rs` | 修改 | 公共投影 |
| `.codex/hooks/source/workflow/src/release_check_cache.rs` | 删除 | 公共投影 |
| `.codex/hooks/source/workflow/src/release_preparation.rs` | 删除 | 公共投影 |
| `.codex/managed-skeleton.json` | 修改 | 配置/受管元数据 |
| `.codex/package-closures.json` | 修改 | 配置/受管元数据 |
| `.codex/skills/bridgeforge-codex-batch/SKILL.md` | 删除 | Skill/镜像 |
| `.codex/skills/git-sync/SKILL.md` | 修改 | Skill/镜像 |
| `.codex/skills/git-sync/references/first-use.md` | 修改 | Skill/镜像 |
| `.codex/skills/git-sync/references/schema4.md` | 修改 | Skill/镜像 |
| `Cargo.lock` | 修改 | 配置/受管元数据 |
| `INSTALL.md` | 修改 | 说明/记录 |
| `README.md` | 修改 | 说明/记录 |
| `bridgeforge-codex-manifest.json` | 修改 | 配置/受管元数据 |
| `doc/0_architecture/TODO-INDEX.md` | 修改 | 说明/记录 |
| `doc/0_architecture/design/development-completion.md` | 修改 | 说明/记录 |
| `doc/0_architecture/design/package-version-lifecycle.md` | 修改 | 说明/记录 |
| `doc/1_delivery/core-only-cleanup/requirements_2026-10-28_core-only-cleanup.md` | 新增 | 说明/记录 |
| `doc/README.md` | 修改 | 说明/记录 |
| `scripts/tests/Cargo.lock` | 修改 | 测试/辅助 |
| `scripts/tests/README.md` | 修改 | 测试/辅助 |
| `scripts/tests/src/artifact_layout_runtime.rs` | 修改 | 测试/辅助 |
| `scripts/tests/src/distribution_regressions.rs` | 修改 | 测试/辅助 |
| `scripts/tests/src/factory_version_config.rs` | 修改 | 测试/辅助 |
| `scripts/tests/src/git_sync_polyglot_runtime.rs` | 修改 | 测试/辅助 |
| `scripts/tests/src/git_sync_runtime.rs` | 修改 | 测试/辅助 |
| `scripts/tests/src/lib.rs` | 修改 | 测试/辅助 |
| `scripts/tests/src/runtime_flows.rs` | 修改 | 测试/辅助 |
| `scripts/tests/src/schema4_fixture.rs` | 修改 | 测试/辅助 |
| `scripts/tests/support/current_fixture.rs` | 新增 | 测试/辅助 |
| `scripts/tests/support/git_repository.rs` | 修改 | 测试/辅助 |
| `scripts/tests/support/release_plan.rs` | 新增 | 测试/辅助 |
| `scripts/tests/support/schema4_agent.rs` | 修改 | 测试/辅助 |
| `scripts/tests/unit/cli.rs` | 修改 | 测试/辅助 |
| `scripts/tests/unit/core_baseline.rs` | 修改 | 测试/辅助 |
| `scripts/tests/unit/core_batch.rs` | 删除 | 测试/辅助 |
| `scripts/tests/unit/core_git_sync.rs` | 修改 | 测试/辅助 |
| `scripts/tests/unit/core_manifest.rs` | 修改 | 测试/辅助 |
| `scripts/tests/unit/core_package_development.rs` | 修改 | 测试/辅助 |
| `scripts/tests/unit/core_project_sync.rs` | 修改 | 测试/辅助 |
| `scripts/tests/unit/core_release.rs` | 修改 | 测试/辅助 |
| `scripts/tests/unit/core_release_polyglot.rs` | 修改 | 测试/辅助 |
| `scripts/tests/unit/core_release_polyglot_scenarios.rs` | 修改 | 测试/辅助 |
| `scripts/tests/unit/core_release_schema4.rs` | 修改 | 测试/辅助 |
| `scripts/tests/unit/core_release_setup.rs` | 修改 | 测试/辅助 |
| `scripts/tests/unit/factory/git_sync.rs` | 修改 | 测试/辅助 |
| `scripts/tests/unit/factory/public_artifact_cache.rs` | 修改 | 测试/辅助 |
| `source/cli/src/lib.rs` | 修改 | 生产源码/包元数据 |
| `source/factory/Cargo.toml` | 修改 | 生产源码/包元数据 |
| `source/factory/src/batch.rs` | 删除 | 生产源码/包元数据 |
| `source/factory/src/commands.rs` | 修改 | 生产源码/包元数据 |
| `source/factory/src/lib.rs` | 修改 | 生产源码/包元数据 |
| `source/factory/src/manifest.rs` | 修改 | 生产源码/包元数据 |
| `source/factory/src/proposal_contract.rs` | 删除 | 生产源码/包元数据 |
| `source/lifecycle/Cargo.toml` | 修改 | 生产源码/包元数据 |
| `source/lifecycle/src/release.rs` | 修改 | 生产源码/包元数据 |
| `source/lifecycle/src/release/schema4/inputs.rs` | 修改 | 生产源码/包元数据 |
| `source/lifecycle/src/release/schema4/migration.rs` | 修改 | 生产源码/包元数据 |
| `source/lifecycle/src/release/schema4/plan.rs` | 修改 | 生产源码/包元数据 |
| `source/lifecycle/src/release/setup.rs` | 修改 | 生产源码/包元数据 |
| `source/lifecycle/src/release_history.rs` | 修改 | 生产源码/包元数据 |
| `source/skeleton/Cargo.toml` | 修改 | 生产源码/包元数据 |
| `source/skeleton/src/baseline.rs` | 修改 | 生产源码/包元数据 |
| `source/workflow/Cargo.toml` | 修改 | 生产源码/包元数据 |
| `source/workflow/src/git_command.rs` | 修改 | 生产源码/包元数据 |
| `source/workflow/src/git_sync.rs` | 修改 | 生产源码/包元数据 |
| `source/workflow/src/project_sync.rs` | 修改 | 生产源码/包元数据 |
| `source/workflow/src/release_check_cache.rs` | 删除 | 生产源码/包元数据 |
| `source/workflow/src/release_preparation.rs` | 删除 | 生产源码/包元数据 |
| `templates/managed-skeleton.json` | 修改 | 配置/受管元数据 |
| `templates/skills/bridgeforge-codex/SKILL.md` | 修改 | Skill/镜像 |
| `templates/skills/bridgeforge-codex/references/transaction.md` | 修改 | Skill/镜像 |
| `templates/skills/develop/references/completion-release.md` | 修改 | Skill/镜像 |
| `templates/skills/git-sync/SKILL.md` | 修改 | Skill/镜像 |
| `templates/skills/git-sync/references/first-use.md` | 修改 | Skill/镜像 |
| `templates/skills/git-sync/references/schema4.md` | 修改 | Skill/镜像 |
| `templates/source-layout.json` | 修改 | 配置/受管元数据 |

### 辅助文本变化口径

以下按当前会话相对 HEAD 的差异统计，包含与 R12–R15 重叠的公共投影和账本变化；不与上述开工快照口径混加，也不算作独立生产代码收益。Skill/说明包含本确认卡。

| 类别 | 新增 | 删除 | 净变化 |
|---|---:|---:|---:|
| 公共投影 | 242 | 1311 | -1069 |
| Skill/说明 | 425 | 171 | +254 |
| 配置、版本锁与账本 | 1009 | 301 | +708 |

批量升级 Skill 的 117 行说明已删除；账本增长用于保留不可变历史及追加本次受管检查证据，不属于运行时代码。
