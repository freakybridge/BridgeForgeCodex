---
lifecycle: active
validation_status: awaiting_validation
---

# develop 按本次任务收口与验证复用

关联 TODO-043。日期：2026-10-27。本卡为本次需求、决定、实施授权、验收范围和后续结果的唯一记录；不是已经生效的 Skill 或机器合同。

最新讨论与已执行边界见第 14 节：方案 A 已写入本仓库 AGENTS 项目级专区，暂停自动正式收口；原有实施和失败记录保留为历史事实，F1–F5 最小修复及工具刷新仍是待执行方案，未验收、未发布。

## 1. 原始需求、决定与授权

用户要求结合两个正在执行的对话、现有 develop Skill 和社区项目，评估 develop 的合理性，针对难以收口提出专项方案，并形成完整 develop 方案；最初仅授权只读讨论。用户随后明确选择“采用按本次任务收口”，并明确授权：“认可这套方案，按‘本次任务收口’的原则完善并落盘正式需求卡和 TODO，然后开始开发”。

已确认的核心规则：只要求本次改动及必要依赖的完成条件满足；已经存在、且不影响本次功能、安全和必要验证的问题单列报告，不阻断本次开发完成。直接影响上述结论的问题仍阻断。全仓诊断与正式 release 的累计范围检查继续保留，不能用本次完成掩盖相关发布缺口。

授权覆盖当前 Forge 项目的需求卡、TODO 和索引、通用 develop Skill 及必要 reference/合同、受管 Rust 能力、必要测试、工厂投影和 dogfood、必要受管构建与受影响子包开发收口。需求写入和开发授权已经有效，实施预算尚待一次性确认，不重新询问是否实施。

不包含：提交、推送、正式发布、用户级安装分发、真实下游写入、终止其他对话的验证事务、修改其他项目或直接修复 Causis 的 Qt 业务缺陷。未获得的真实下游、自然运行时、用户试用和验收不能用离线测试替代。

## 2. 规模、预算与当前实施位置

L 级：涉及通用工作流、任务范围与依赖边界、检查证据复用、最终候选及事务恢复，不只是修改 Skill 文案。规模按逻辑变化判断，不因模板镜像或清单机械传播加档。

首次暂定建议为 10 小时、600,000 新增 token（估算、未实测）、最多 1 个 review-auditor 子 Agent、最多 2 轮完整验收闭环。用户质疑该建议是否仍按旧 develop 设计计算；主对话确认它主要采用旧流程历史成本加保守余量，缺少分项核算，已撤回这组建议，不能记为用户批准、合理预计耗时或当前有效实施上限。

重新估算须分开列出按新方案必须完成的实现与验证、仍需保留的旧工具保护、可消除的重复工作和未知成本。不能将旧流程历次失败、全仓待处理包或重复构建直接累计为新预算。机器等待时间不直接换算为 token；token 未实测，不从编译分钟数推算模型用量。本机暂定周额度系数的模型/速度适用性未核实，不将原 9.1% 参考值当作本任务已经确认的额度预测。

有效预算：用户在重新核算后明确“只能给你一个小时开发”，本轮按 60 分钟硬上限执行；起点 2026-10-27 14:51:14（Asia/Shanghai），截止 15:51:14。8 小时/160,000 token 的重新核算建议同样没有获准，不记为有效预算；没有获准的独立 token 上限或周额度预测。主对话优先完成任务范围能力、Skill/合同及必要定向验证，只使用一个只读独立审查 Agent。完整原需求不因时间限制自动改成已完成；截止时停止新增工作，如实交付实现、验证和未完成项，不追加预算确认。相同问题连续两次实质修复失败仍沿用既有升级规则。

原目录 `D:/Quant/BridgeForgeCodex` 的 TODO-042 正在执行受管开发事务，仍要求输入冻结。本次通过受管工具建立隔离 worktree `D:/Quant/CodexWorktree/develop-task-closure/BridgeForgeCodex`，基点 `8fdfd2f57fb1e539633d10bd01bd665eae4aad8b`，只先落本卡和 TODO/索引。为保留文档链接与编号，复制原目录 TODO 总表和 TODO-042 需求记录作为只读来源快照；这些内容不是本次实现成果。不得在 TODO-042 运行时改动其原目录、复制半完成版本事务作为开发基线，或将两项任务的证据相互认证。

本轮为在一小时内推进，在隔离目录接续 TODO-042 已冻结的 Rust/测试源码快照，原样保留在 `.runtime/task-closure/upstream/` 并记录文件摘要；未复制候选版本、Cargo.lock 或收口账本，不把其未完成验证算作通过。最终受管收口仍须核对前置稳定版本与收据；前置未完成时不得借本次工具写入认证。原目录与其运行中事务保持不动。

## 3. 已核实事实和问题归因

1. 当前 develop 已要求继承授权与预算、按风险验证、复用有效证据，并分别报告实现、工程验证、用户验收和 Git/发布；这些原则保留。
2. 当前 `completion-release.md` 明确受管 apply 覆盖全部待处理包，不能只收口其中一部分。`development.rs` 的 pending 比较已有包证据，execute 对待处理包逐项实际运行检查；受管恢复后的有效检查复用能力不足。
3. Causis 的 36 包迁移本身有广泛验证依据。其记录显示首轮 36 包成功，apply 2252 秒，之前两次预览 459/474 秒；安装同步后 36 包因 environment 分项变化重新进入验证范围。去重测试结果为 2284 通过、1 项真实样本跳过。后续 Qt 原生退出是真实失败，不能为收口放行，也不能将环境变化本身当成原生崩溃根因。
4. 发布证据对话在修改产品代码之前已有七包因环境等历史证据差异待处理；审计补改与候选准备交错导致清单再次失效，后期发现真实快路径保护缺口后受管恢复并重验。保护缺口必须修，重检查的重复成本应单独解决。
5. 同一测试文件在不同依赖或安装组合下执行不当然重复。318 次文件执行对应 172 个不同文件，不能直接删除差额；仅允许复用已证明输入与执行条件等价的检查。

来源：[重构对话](codex://threads/01a1763c-a0ae-77a1-8d1d-a2ac6e07b482)、[发布证据对话](codex://threads/01a17acb-7635-76f2-9d14-5d1059ece457)、[既有 develop 重构](requirements_2026-10-09_validation-refactor.md)、[跨电脑证据与快速发布](../package-version-lifecycle/requirements_2026-10-27_cross-machine-release.md)、[工厂开发收口](../../0_architecture/design/development-completion.md)。对话和记录为已读快照，最新执行状态在接续时核验。

## 4. 社区借鉴与边界

| 来源 | 采纳方向 | 不照搬的内容 |
|---|---|---|
| [OpenSpec 工作流](https://github.com/Fission-AI/OpenSpec/blob/main/docs/workflows.md) | 以具体变更定义完成范围；区分完整性、正确性与设计一致性；范围实质改变时拆出新变更 | 归档允许警告不等于 Forge 可以放行必需验证失败 |
| [Spec Kit implement](https://github.com/github/spec-kit/blob/main/templates/commands/implement.md) | 开工前明确 Done When，按本次任务、规格和测试逐项判断 | 不引入整套初始化与文档体系，不添加无关清单 |
| [Superpowers executing-plans](https://github.com/obra/superpowers/blob/main/skills/executing-plans/SKILL.md) | 任务完成证据、决定与恢复点；重要问题和可延后问题分开 | 不照搬每任务提交、全套测试和额外审查；旧记录不可直接冒充最终候选已验证 |
| [Trail of Bits 仓库规则](https://github.com/trailofbits/codex-config/blob/main/AGENTS.md) | 按实际修改内容选择对应检查 | 配置仓库轻检查不替代版本事务和必要运行验证 |

上述页面与部分源码已只读核验，没有安装或运行第三方项目，没有性能优劣实测；更详细的旧调研见[社区对标](../../3_reference/skeleton-benchmark_2026-10-24.md)。

## 5. 需求与完成合同

### R1 本次任务范围与全仓诊断分离

本次完成范围由已授权用户目标、实际变更和必要依赖共同确定，不能仅由 Agent 自选包名排除真实影响。任务起点须记录既有改动与证据状态；依赖、共享构建/检查输入、公开接口或包生命周期影响范围仍由机器核对。无关历史问题单列，不自动并入开发。

同一包混有未审查或未授权的其他内容时，不允许只认证若干修改行后宣称整包有效。明确合并范围或完成安全隔离后才能认证。全仓诊断保留原有全仓语义；不得将旧 `nothing-to-close` 偷换为局部成功。任务范围的机器输入、能力标识与收据需要显式设计和验证，不提前编造已存在的 CLI 参数或状态。

### R2 区分证据结论与失效范围

区分源码/依赖/检查合同证明、构建产物身份、安装身份和针对实际运行产物的验证。变化报告必须说明受影响的检查及补验理由；不能仅因整体环境摘要不同否定全部源码检查。真实编译器、依赖、配置、原生环境和明确运行条件变化仍使相关证据失效。

沿用 TODO-042 的显式跨电脑环境合同和证据/本地产物分离，不重复建立另一套身份算法。无法界定的环境不承诺可移植或可复用；新生成的二进制不自动继承旧二进制运行验证。产物同时是其他检查输入时必须传播失效。

### R3 检查级证据复用与恢复

由受管工具保存检查身份、命令、最终候选版本及输入、有效依赖、必要工具/环境、产物和完整结果/日志关联。只有受管执行产生且完整成功、所有必要身份仍等价、证据文件可核验的检查才可复用。不得接受手工 passed、任意外部日志或只匹配测试文件名作为认证。

失败、超时、中断、缺输出、需要实际副作用或无法证明等价的检查不缓存为成功；恢复时检查缺失产物和最终候选的依赖图。回滚版本与账本的原子性保留，已通过检查的证据在输入等价时允许继续利用，不能把部分缓存当作整项收口成功。

### R4 最终候选与防止收尾自失效

必要定向验证和适用审查修正完成后再准备最终候选，避免构建期间继续补改源码。候选版本、原生依赖/锁及必要投影进入同一受管事务，验证对应最终候选。需要安装验证时优先在隔离受控环境中验证该候选；必须在收口后才执行的安装或部署，只增加对应身份/运行补验，真实相关变化仍按影响失效。

同一候选、同一有效输入与同一检查不因阶段包装重复执行。1.0.0 稳定检查、最终依赖回退或版本变化必须重新核对影响范围，不能复用升版前不适用的结果；纯证据刷新不重复升版。

### R5 完整 develop 编排

| 阶段 | 行为与进入下一阶段的条件 |
|---|---|
| D1 接收任务 | 复用需求、授权、规模与预算，仅补关键缺口 |
| D2 定义完成条件 | 明确用户结果、必要验证、交付物、版本处理和不包含事项；S 级短说明，M/L 复用唯一需求卡 |
| D3 核对起点 | 区分本次变更、既有状态、无关历史阻断和可复用证据 |
| D4 实现与定向验证 | 优先验证本次行为和失败路径，不自动运行全量集 |
| D5 适用审查与修正 | 本次必需修复清楚，源码达到准备最终候选的状态；晚发现的真实缺陷仍处理 |
| D6 准备最终候选 | 版本、依赖、必要产物和安装验证对象明确，采用受管入口 |
| D7 验证与收口 | 执行或核验有效检查证据，版本与账本原子提交到文件事务，不进行 Git 提交 |
| D8 最终复核与交付 | 检查本次完成结论仍有效；报告无关历史问题，不以全仓清零替代本次判断 |

本次开发完成 = 需求满足、必需验证有效、适用审查完成、交付物对应最终内容、适用的子包收口完成。用户验收、Git 保存和正式发布分别记录，不互相替代。

### R6 新问题、预算与停止

本次引入的回归或违反完成条件的问题必须修；既有但直接妨碍本次功能、安全或必要验证的问题明确阻断，范围/预算实质扩大时只确认该部分。无关历史缺陷、优化和润色单列报告，不自动修改或创建 TODO。原因不明的环境或测试失败保留现场，限定结论，不当作通过或默认归因生产代码。

规模、预算、修复失败计数和 Agent 触发规则继续引用既有事实源，不在 develop 复制阈值或增加审批卡。重检查前报告所选范围、成本依据和可复用项；已有授权且预算充足时继续，不逐步骤重复确认。

### R7 兼容与发布边界

旧账本、正式发布边界和身份历史保持；新增能力须显式识别，旧工具不冒充支持任务范围收口或检查复用。普通 git-sync 不要求先完成本任务；未完成写事务的保护保留。release 继续按正式发布以来的累计实际影响核验，不补跑 develop，也不因单次任务已完成就忽略累计相关缺口。

## 6. 实施任务与文件边界

| 编号 | 优先级 | 主要范围 | 目标与验证 |
|---|---|---|---|
| T1 | P0 | templates/skills/develop/SKILL.md、references/completion-release.md | 完成条件、范围、阶段顺序和阻断处理；规则场景审查 |
| T2 | P0 | source/lifecycle/src/release/schema4/development.rs、configuration.rs、analysis.rs 及 CLI/工厂直接调用方 | 显式任务范围、必要依赖与全仓诊断分离；正反范围 fixture |
| T3 | P0 | development.rs、development_inputs.rs、既有环境身份和产物链路 | 检查级受管证据复用、恢复及细分失效；篡改/缺失/依赖变化等负例 |
| T4 | P0 | development 候选事务及 source/factory/src/manifest.rs 的必要调用边界 | 最终候选与安装/产物验证顺序；版本与账本一致、回滚和稳定候选场景 |
| T5 | P1 | develop/references/ml-delivery.md、适用的 agent-execution.md、templates/skills/git-sync/references/schema4.md、工厂开发收口文档 | 单一事实源、状态和恢复点，合同与实现一致；不删除既有必要红线 |
| T6 | P1 | scripts/tests/unit/core_package_development.rs、core_package_portability.rs、相关 CLI/factory fixture、既有分发行为用例 | 场景回归、同条件成本对照、独立审计、受管构建和本次范围收口 |

传播四问：通用产品能力进入 source/，共享 Skill/合同进入 templates/；工厂专属内容只留工厂文档与实现，本卡为元文档；实际受影响子包验证后由受管 develop 收口，根 VERSION 与正式 CHANGELOG 留待单独 release；按真实登记同步公共投影、manifest、dogfood 和引用。当前共享 develop 的安装源是 templates/skills/develop，不创建不存在的 .codex/skills/develop 镜像，不手工覆盖用户级安装副本。

## 7. 验收与验证策略

| 编号 | 场景 | 通过标准 |
|---|---|---|
| A1 | 单包任务与无关历史证据失效并存 | 本次范围可正确完成，历史问题可见；不会认证无关变化 |
| A2 | 底层依赖、共享输入、接口或生命周期变化 | 真实必要范围进入验证，不允许任务选择绕过影响 |
| A3 | 安装身份变化而源码及有效测试输入等价 | 定位安装补验，不无差别重跑全部源码测试；真实环境变化仍正确失效 |
| A4 | 后项失败、受管恢复并重启相同候选 | 前项完整有效结果可复用，后项真实重验；失败事务不留下半套认证 |
| A5 | 审查后局部修正、版本或依赖图变化 | 仅适用证据复用；受影响检查执行；重复收口和纯证据刷新不重复升版 |
| A6 | 失败、超时、缺日志/产物、伪造或篡改证据 | 明确拒绝复用与完成，不用 warning 或旧成功掩盖 |
| A7 | 同一包混入其他改动、任务范围不完整 | 明确阻断或要求范围整理，不凭局部完成认证整个包 |
| A8 | 旧账本、旧工具、普通同步和累计 release | 历史链不改写，能力缺口明确；普通同步与发布边界保留 |
| A9 | 最终复核、新发现问题、未包含真实环境验收 | 本次完成与全仓诊断区分，真实必需失败仍阻断，未验证如实报告 |
| A10 | 同条件成本对照 | 记录分阶段耗时、执行/复用检查数和构建数；等价有效输入不因阶段重复执行，不虚报固定总耗时改善 |

先选择覆盖上述风险的定向单元、事务/CLI 集成与必要 fixture，再按剩余风险决定是否扩大；不因新收口方案自动执行全部 workspace 和完整 fixture。使用现有锁定 Rust/受管入口，不新增脚本包装器。完整回归必须说明定向覆盖不足的具体理由及预算。构建和投影只按实际输入变化执行，复用有效缓存。

独立 review-auditor 在预算确认后读取真实需求、代码和 diff，覆盖任务范围绕过、错误证据复用、候选/产物身份、失败恢复与 release 消费者；审计不代替实际测试。针对审计修正重验实际影响部分，不以无限审计代替完成。Skill 行为场景与静态文件检查分开报告，未运行真实模型行为测试不称其通过。

## 8. 实施记录与交接

- 2026-10-27：用户确认按本次任务收口，并授权正式需求卡/TODO 与后续开发；读取 confirm、develop、TODO 合同及现有机器合同。新增 TODO-043，与已验收 TODO-021/TODO-037、待执行 TODO-005 及实施中 TODO-042 分开，不改写其状态或重新宣称旧问题已验收。
- 初始准备时原目录仍有运行中的 TODO-042 事务，本次创建受管隔离 worktree 保护其冻结输入；该准备阶段只复制文档，未复制候选版本或产品源码。后续一小时开发接续冻结源码的边界见第 2 节，不能把两个阶段混为一谈。
- 初始交接条件为实施预算与前置基线；用户随后给出一小时硬上限，已按第 2 节进入隔离开发。当前实现、验证与未完成项统一以下节的最新结果为准。
- 初始落盘轻量核验：`git diff --check` 和 `check todo --root D:/Quant/CodexWorktree/develop-task-closure/BridgeForgeCodex` 均退出 0，TODO `errors=[]`、43 条记录；当时 TODO-043 为“待执行/原需求”、卡为 `active/not_started`。这些是落盘当时的历史状态，不能覆盖后续实施记录。
- 预算讨论：用户未批准首次 10 小时/600,000 token 建议，主对话已撤回并转为只读分项核算；不能把用户质疑记为批准。已有文档落盘与开发授权不撤回，不因预算讨论新增任务或 Goal。
- 2026-10-27 14:51:14：用户给定一小时硬上限，已开始隔离开发。先实现 `task-scoped-development-v1` 与 --packages，保留全仓旧入口；受管检查级跨事务复用尚未实现，不在 Skill 或收据中声称生效。
- 中间验证：首轮 task_development_ 的 4 项定向测试全部通过，真实测试执行 221.63 秒（编译另计）。独立审查发现依赖 split 端点/调用方闭包遗漏和重复末尾缺值参数问题；已补修正及回归，最终验证待实际结果，不用首轮结果认证修正后的候选。

## 9. 一小时开发结果（2026-10-27）

### 实际实现

1. 新增 `task-scoped-development-v1`：公共 CLI 和工厂直接入口支持 `--packages`，选择、有效依赖范围与计划指纹绑定；范围闭包覆盖消费者、依赖和相关身份变迁。无关包证据不作为任务成功依据，旧全仓入口保持原语义。任务无待处理项为 `task-development-current`，不是全仓 `nothing-to-close`。
2. 新增 `resumable-development-checks-v1`：显式 environment 且 `reusable: true` 的确定性检查可保存和复用同机完整候选的成功结果。绑定实际工具二进制、完整候选/计划、输入、目录、严格环境和 Git 身份，保存完整输出与产物摘要；输入变化、产物缺失或记录/日志损坏实际重跑。失败事务只保留已通过检查的缓存，不提前写部分收口账本。
3. 缓存为保守能力：默认关闭；不支持跨主机检查缓存或局部修改的最小失效；启用缓存的事务中声明产物必须全部被 Git 忽略，tracked/unignored 产物在候选写入前明确拒绝，应关闭 reuse 走原流程。未自动修改任何真实项目检查清单以开启缓存。
4. develop 主入口、收口 reference、M/L 状态、共用 schema4 合同和工厂收口说明已更新，主入口 120 行。由既有受管 manifest 工具生成公共源码投影、清单和 dogfood；没有新建 Skill、Agent、Hook 或独立工作流框架，没有手工覆盖用户级 Skill。

### 实际验证与覆盖

| 证据 | 结果与边界 |
|---|---|
| 三包定向 `cargo test --locked --offline --config scripts/tests/factory-cargo.toml --manifest-path Cargo.toml -p bridgeforge-lifecycle -p bridgeforge-cli -p bridgeforge-factory --lib task_development_ --target-dir .runtime/source-workspace-refactor/cargo -- --test-threads=4 --nocapture` | CLI 1、factory 1、lifecycle 12，共 14 项通过；lifecycle 实际执行 588.78 秒。日志 `.runtime/task-closure/verified-task-tests.log`。覆盖任务范围、split 消费者、两入口错误参数、缓存接续、源码/产物/日志/Git/本机环境变化和失败不缓存。此集在最后追加 tracked 产物预览限制之前执行；追加局部分支后只补下列负例和缓存正例，未受影响结果按差异审查复用 |
| CLI 能力与写入参数保护（复用已编译测试二进制） | 1 项通过，日志 `.runtime/task-closure/cli-capabilities.log` |
| 旧证书跨骨架升级兼容 | 1 项通过，162.50 秒，日志 `.runtime/task-closure/legacy-compatibility.log`；没有将默认 false 写入旧证据序列化 |
| 真实 Cargo 检查最终 manifest/lock | 1 项通过，75.32 秒，日志 `.runtime/task-closure/final-candidate-cargo.log`；只验证隔离 fixture，非真实下游 |
| 审查补充的 tracked 产物限制 | 1 项通过，54.66 秒，日志 `.runtime/task-closure/tracked-output-test.log`；在预览拒绝，候选 manifest 未变、检查调用 0、没有收口记录 |
| 修正限制后的缓存正例补验 | 1 项实际通过，203.95 秒，最终源码编译出的测试二进制证明恢复时首项检查没有重跑。日志 `.runtime/task-closure/final-cache-positive.log`；不重复累加进上述不同测试计数 |
| 调试工具构建与实际 self-test | 最终 CLI、factory 调试构建成功；两个工具的上述两项能力均为 true，记录 `.runtime/task-closure/final-cli-self-test.json`、`final-factory-self-test.json`。仅为调试 smoke，不是受管发布产物、安装或 release |
| 受管轻量检查 | manifest `--check`、skill metadata、project structure、factory-version、TODO、`git diff --check` 通过；schema4 template/dogfood 字节 hash 一致。Skill 曾因 123 行被拒绝，已精简到 120 行并重新通过，仅重验文本/清单，没有因此重跑 Rust 集合 |
| 实际工厂任务预览 | 本轮调试工厂工具 `develop --preview --packages lifecycle` 返回 `development-analysis-required`、`readiness_scope=task`；有效范围实际展开 checks/core/factory/lifecycle/skeleton/workflow 六包，没有检查 memory 的收口证据。版本、根锁、配置和账本前后 hash 不变；这只是该快照的零写入预览，不是已执行收口 |

上表前五项合计 18 个不同测试方法通过；初轮、重跑和最后正例补验不重复累加。测试过滤均有实际执行数量，没有把 0 项、缺结果或等待状态写为成功。未做固定比例性能提速声明；复用测试实际断言同一候选两次事务中首个检查只执行一次，最终轮收据 `checks.reused=1`，这证明复用行为而非真实业务总耗时。

### 独立审查与修正

同一个 review-auditor 只读审查本轮源码、需求、差异和 Skill，未修改文件或执行测试。发现并修复：依赖 split 端点/调用方闭包遗漏、重复末尾缺值参数、缓存遗漏严格宿主环境和 Git 身份、tracked artifact 与严格 Git guard 的冲突。最后一项采用显式拒绝该缓存组合的保守限制，并补候选零写入负例。审查还指出过 schema4 dogfood 文案滞后，已受管同步并逐字 hash 核对。最终限定代码审查无其他阻断；实际测试和传播证据由主对话提供，审查没有冒充真实运行验证。

15:44 核对：上述最终正例已经返回通过，最后调试工具增量构建 20.52 秒，两项新增能力的实际 self-test 均为 true；当前没有新增源码实现或扩大测试范围。按一小时硬上限进入结果保存和最后轻量核验，不等待或接管其他对话的事务。

### 未完成与接续边界

完整开发仍未完成：本次核心实现及上述定向验证不代替实际受影响子包的受管版本收口。未手改子包版本，根 VERSION、CHANGELOG 和 Cargo.lock 未变；没有提交、推送、正式发布、用户级安装或真实下游写入。TODO-042 的冻结源码快照是前置输入，不是本轮成果，也没有把其尚未完成的收据认证为通过。

原方案的精细局部失效、通用安装/源码证据分层的真实项目适配及真实行为验收未全部验证；当前缓存只承诺上述严格同机完整候选范围。保留 TODO-043 原需求，不把未完成部分改称已验收后续事项。下一次接续必须核对前置稳定基线和本轮有效证据，再完成剩余验证及受管收口；本轮到一小时上限停止，不自动增加预算或启动全仓回归。

## 10. 当前 Forge 本地 dogfood（2026-10-27）

### 后续决定与范围

一小时源码开发停止后，用户要求优先把已实现的核心收尾再 dogfood，不继续以原完整方案的 3–5 小时为本轮前提。核对发现用户级 develop 主入口和 completion-release reference 仍为旧字节；本工作树镜像同步不等于共享安装生效。另一个前置对话已报告完整检查超时及实际失败，不能借本地试用伪称其已通过。

用户明确选择：“先在当前 Forge 本地 dogfood”。本阶段仅使用当前隔离工作树的候选 Skill 和本地受管工具，复用上述验证，保留正式版本收口未完成标记；不更新影响其他项目的用户级共享安装，不发布、不写真实下游、不接管其他对话事务。该续办授权不取消此前一小时开发记录，准备耗时单列，不把等待或编译时间虚构为 token 用量。

### 实际操作与传播四问

1. 仅在本工作树根 AGENTS.md 的项目级专区登记本地 develop 入口，要求本工厂读取 `templates/skills/develop/SKILL.md` 及其 reference，使用受管 `.codex/bin/bridgeforge-factory.exe` 核对新能力。未创建第二份同名 Skill 或手工覆盖用户级受管文件；公共 AGENTS 区逐字文本（规范化换行）与模板一致。
2. 按 INSTALL.md 的现有受管入口执行 `cargo run --locked --offline --release --manifest-path Cargo.toml --bin bridgeforge-factory -- build-assets --factory --project-root .`。复用既有源码和验证，不改可执行源码、依赖或业务版本。该操作是本地运行时准备，不是 Forge release。
3. 传播分类：本地入口为工厂自身配置层，正式记录为元文档；不下沉通用 Template、不改用户级安装和其他项目；不新增版本或 CHANGELOG；受管 manifest 与本地 dogfood 源/产物核对保持有效。

### 本地试用证据

| 项目 | 实际结果 |
|---|---|
| 受管运行时准备 | `status=prepared`、`prepared=3`、`written_files=6`、`self_test_executed=true`；CLI、Hook、factory 及其收据已落到本树 `.codex/bin`。日志 `.runtime/task-closure/local-runtime-prepare.log` 从 16:42:50 至 17:01:47，约 19 分钟，包含首次 release 引导及隔离产物构建；不是重跑业务测试 |
| 严格只读运行时复核 | `.codex/bin/bridgeforge-factory.exe build-assets --factory --project-root . --check` 返回 `current`、`read_only=true`、`reused=3`、`built=0`；记录 `.runtime/task-closure/local-runtime-check.json` |
| 实际本地产物能力 | 本地公共 CLI 与 factory 的实际 self-test 均具有 `task-scoped-development-v1` 和 `resumable-development-checks-v1`；记录 `local-cli-self-test.json`、`local-factory-self-test.json`，不以同一个旧产品版本数字替代源码/能力身份 |
| 真实调用路径 | 执行本地 `.codex/bin/bridgeforge.exe develop --preview --packages lifecycle`，公共 CLI 实际核验并转发给本地 factory，返回 `readiness_scope=task`、所选 lifecycle、`writes=[]`。记录 `.runtime/task-closure/local-dogfood-preview.json` |
| 正式状态没有伪造 | 该预览是 `development-analysis-required`，有效依赖范围仍保守展开六包；没有将它记为 task current 或 development closed，没有执行 apply、升级版本或改写账本 |
| 边界保护 | 用户级 develop 两文件、用户 AGENTS、全局 CLI 和受管账本共五个已存在/缺失状态逐项 hash 不变；本树 VERSION、Cargo.lock、版本配置和收口账本四项 hash 不变。汇总 `.runtime/task-closure/local-dogfood-receipt.json` |

### 本阶段结论

已完成核心的当前工作树本地 dogfood 准备与命令路径试用：新 Skill 有明确项目入口，新受管运行时就绪，真实调用使用任务范围。已有 18 项定向/兼容验证按未改变可执行源码的依据复用，没有再展开全仓回归。本会话依用户决定直接采用候选流程；不宣称其他运行中对话或原主仓库已经切换。

用户级共享副本仍保留原样，正式子包版本收口与完整原需求的未完成标记继续保留；本地试用成功不是正式发布或用户验收。Hook 构建/自检和本地 CLI 路径已验证，自然 Hook 生命周期、其他项目和未来新会话的模型行为没有扩张为已验证结果。

## 11. 正式收口接续（2026-10-27）

用户明确要求在现有工作树接续 TODO-043 已实现核心及必要前置的正式受管开发收口；不开发精细局部失效、通用安装适配等未实现扩展，不 reset、清理或覆盖已有改动。仓库候选 develop 为本轮依据；禁止提交、推送、正式发布、用户级安装或真实下游写入。

原一小时开发预算及本地 dogfood 准备记录保留。曾提出的四小时建议已撤回，不能作为有效预算。用户本次明确选择追加整体预算：90 分钟硬上限、40,000 新增 token（估算）、最多一个只读审查 Agent、最多两轮验证；起点 17:27:17（Asia/Shanghai），截止 18:57:17。token 未实测，既有暂定系数对应周额度约 0.6%，模型与速度匹配未经核实。到限保留现场、不自动扩额，不保证届时一定收口成功。

接续起点：两个旧 Forge 对话均空闲，本工作树和原仓库均无 develop journal；HEAD 为 8fdfd2f57fb1e539633d10bd01bd665eae4aad8b。当前本地运行时只读检查为 current、reused=3、built=0。原仓库的 runtime 漂移不移作本工作树结论。相同任务选择 `--packages lifecycle` 的预览为 development-analysis-required，有效范围 checks/core/factory/lifecycle/skeleton/workflow，未验证范围外包的证据。

执行顺序：选择性整合 TODO-042 末批一致性修正与必要检查输入登记；定位旧跨进程 probe 失败；复用有效测试与审查、仅补受影响部分；冻结源码后受管同步投影与运行时，重新分析并执行同范围 develop；核对 development-closed、最终同范围预览、版本/锁/账本/产物。整包检查尚未启用机器缓存，不能用聊天中的 passed 替代必需检查。未取得终态前仍为开发未完成，用户未验收。

### 本轮实际结果与阻断

**本次开发未完成，正式版本收口未完成。** 必要前置已整合，定向兼容验证及限定独立审查通过；正式 apply 在预先声明的 18:37:17 事务停止点仍未完成，已停止自身事务并经受管恢复。此停止点为最终产物刷新或恢复、核验预留二十分钟，没有延长整体预算，也没有启动下一轮整包检查。

| 项目 | 实际证据与边界 |
|---|---|
| 必要前置 | 对 development.rs 使用当前内容、冻结快照和原仓库修正做三方合并，保留本轮 task scope/cache 代码；补齐缺记录旧包无升版补验、锁 TOML 影响语义及配置初始基线修正。配套 core_release_schema4.rs 与带原子计数器的 core_release_polyglot.rs 与已审查上游字节一致。未复制上游候选版本、锁或账本 |
| 检查输入登记 | 经受管 migration 补 core 的 portable_environment.rs、lifecycle 的 core_package_portability.rs。保留原全部检查、并发及超时；配置迁移收据为 migrated、sync_performed=false、version_changes=false |
| 旧跨进程 probe | 合并前单独复测通过，测试执行 170.99 秒，含 Cargo 启动总计 176.138 秒。没有重现单次子进程 120 秒超时，因此未修改其断言、实现或超时，也未将历史整组失败归因为某个未证实原因。日志 .runtime/closure-resume/probe-before.log |
| 合并后定向兼容集 | 三包 task_development_：CLI 1、factory 1、lifecycle 13，共 15 项通过；lifecycle 671.02 秒，总计 708.909 秒。覆盖选择/闭包/身份、范围外历史问题、兼容入口、缓存成功与失效、失败事务、被跟踪产物保护。日志 task-compatibility.log；本批与历史 18 项有重叠，不相加宣称新增 33 项 |
| 独立审查 | 复用原审查，并由同一只读 review-auditor /root/closure_review 检查本轮整合：未发现阻断。确认 pending 仅遍历已按任务范围生成的 hashes，旧包补验不误升版，历史摘要序列化保持；额外范围判定成本未扩张成本轮优化任务 |
| 投影与运行时 | 锁定 Cargo 生成 manifest/public projection/dogfood 后准备本树 CLI、Hook、factory：prepared=3、written_files=6、self_test_executed=true；合计 1,375,068 ms。正式 apply 前只读复核 current、reused=3、built=0。未更新用户级安装 |
| 新计划 | 相同 --packages lifecycle 返回 development-planned；有效六包。候选为 factory 0.1.3→0.2.0、lifecycle 0.1.4→0.2.0、workflow 0.1.1→0.1.2，其余三包仅重验。缺少稳定验收合同，未进入 1.0.0。此处仅是已中断的历史计划，不能作为已升版成果 |
| 正式 apply | 本树 .codex/bin/bridgeforge-factory.exe 执行，18:01:19 启动，监督用时 2,158,154 ms。事务推进到 factory，checks/core 检查已返回成功；factory 自 18:06:29 运行至停止点，约 30 分 48 秒仍未返回，lifecycle/skeleton/workflow 尚未执行。监督记录 exit_code=124、native_exit_code=-1、budget_interrupted=true；外层 PowerShell 退出 1。这是预算中断，不是测试判定失败，不把缺失结果当通过 |
| 受管恢复 | develop --recover 返回 development-recovered、git_written=false。journal 清除，38 条原账本及 hash 序列完全保留；VERSION、CHANGELOG.md、Cargo.lock、scripts/tests/Cargo.lock、账本文件 hash 与事务前一致。源码和前置修正保留。收据 recovery-state.json、develop-recover.json |
| 候选引导工具 | 在冻结候选期间以独立 release 缓存预编译工厂引导工具，425,405 ms，成功；它对应已撤回的 0.2.0 候选，未安装到 .codex/bin，不把它当作恢复后的最终产物。当前本地产物以 recovered-runtime-check.json 的 current 结论为准 |

### 长检查诊断配置

本次 factory 长检查尚未定位到具体慢用例；进程 CPU 持续增长，一次只读线程描述采样未取得名称，不能据此宣称根因已确定。本轮运行开始时没有逐项落盘日志，整体收据也未形成。

从实际测试程序帮助发现原生选项后进行了真实快速用例核验：--fail-fast 被当前 stable 编译器拒绝（仅 nightly + unstable-options 可用，退出 101），未写入配置；单独 --logfile 执行一项现有参数校验用例通过，写出逐项 ok 记录，但工具有 deprecated 提示。没有切换工具链，没有使用 report-time 或不稳定选项。

恢复后通过受管 migration，仅给 factory/lifecycle 检查追加 --logfile .runtime/bridgeforge-codex/factory-unit-tests.log 与 lifecycle-unit-tests.log。父目录由 develop 的 journal 写入先行创建；不新增包装器，不删改测试选择、并发、超时或失败行为。新日志配置只完成短用例能力核验，尚未整组执行。配置变化使旧分析 fingerprint 不可复用；下一次必须使用最终同范围预览重建分析和计划。

传播分类：前置通用 Rust 修正属于产品层，公共投影/dogfood 已经受管同步；工厂检查输入与日志参数是工厂自身配置，不下沉通用模板；本段和索引是元文档。子包版本收口已恢复，主版本及 CHANGELOG 不变。未提交、推送、发布、更新用户级共享安装或写真实下游；用户未验收。原需求的精细失效、通用安装适配等扩展继续保持未完成。

后续只接续本轮阻断：使用原生日志定位 factory 实际慢用例，保留原覆盖和有效证据，再以当前本树能力重新执行同范围分析/apply；成功后核对追加账本、实际版本与锁、最终产物及 task-development-current。未取得这些证据前，不把已实现核心、prepared、planned 或 development-recovered 改称 development-closed。

### 最终复核

18:44 的最终复核：同一 --packages lifecycle 预览为 development-analysis-required、readiness_scope=task、writes=[]，有效范围仍为六包；主程序 7.0.5，factory 0.1.3、lifecycle 0.1.4、workflow 0.1.1，均未留下候选升版。最终运行时 current、reused=3、built=0；manifest、factory-version、TODO（errors=[]）、project-structure、cargo fmt --check 和 git diff --check 均退出 0。未发现本树残留的 apply/test/Cargo/rustc 进程，没有暂存、提交、推送或发布。

同一个只读审查 Agent 已复核两项日志参数增量，未发现阻断，确认日志父目录先行创建且验收覆盖不变；原生日志有弃用提示，未来更换工具链时须重新核验。新增日志参数尚未经历完整整包检查，不能算作其通过证据。最终预览与实际恢复收据保存在 .runtime/closure-resume/，此前 development-planned 和分析文件的 fingerprint 不可复用。

本轮仅一次正式 apply 尝试，因预定预算截点中断后完成恢复和核验，没有启动第二轮整包回归；使用一个只读审查 Agent并复用其增量复核，token 未实测。保留原一小时开发和本地 dogfood 的累计记录，当前需求状态为 active/awaiting_validation，TODO-043 保持进行中，用户未验收。

## 12. 再次接续（2026-10-27）

用户再次要求继续，原追加九十分钟窗口已于 18:57:17 到期。经一次整体确认，用户选择再追加九十分钟、30,000 新增 token（估算，暂定周额度约 0.5%）；起点 18:59:26，截止 20:29:26（Asia/Shanghai）。保留前轮与原一小时累计记录，不重置修复失败计数或最多两轮验证上限，复用现有只读审查 Agent，不新增 Agent；token 未实测，不把估算当精确已用/余额。本次先定位 factory 实际慢用例再选择最小补验，不重复已通过的定向集，不保证到限一定收口。

初步诊断使用已由锁定 Cargo 构建、源码未变的 0.1.3 factory 测试程序，仅作诊断、不当作正式版本收口证据。首个 180 秒窗口完成前八项；随后仅运行剩余 runtime 组，四项通过、一项设计上的 child helper 忽略，实际 67.59 秒。锁恢复用例没有卡死证据，测试夹具快照也不是读取当前工作树，因此没有据此修改产品代码或超时。下一步对真实 Git 同步测试的夹具准备和同步阶段使用已有 release_metrics 做临时分段计时；临时诊断代码必须在最终候选前恢复并核对原始 hash。

分段计时结果：一个含四种失败场景的 Git 同步测试通过，执行 47.75 秒；每种场景夹具初始化约 9–10 秒，同步约 2–2.5 秒。主要成本为真实 Git 子进程，canonical JSON 哈希仅毫秒级，没有证据支持调整 Rust 优化级别。临时诊断代码已逐字 hash 恢复，profile-restore.json 为 true，未留下产品代码改动。

复用同一只读审查 Agent 核对三类 fixture、真实仓库访问、缓存和子进程：未发现必须串行的共享状态；真实仓库只读，缓存按独立 fixture root 隔离。为 Windows 同时钟 tick 的临时目录防碰撞，在 Factory::new、公共 cache Fixture::new、共享 real_repository 三处增加函数内 AtomicU64 计数器，未改变断言。git_repository.rs 本来已登记，无需补输入。生产路径的时间戳目录未复现碰撞，本轮不扩张修改。

经受管 migration 仅将 factory 原完整 35 项检查的 --test-threads=1 改为 4；debug 编译、断言、全部用例、3600 秒超时不变；lifecycle 保持原四线程/7200 秒。两项原生 logfile 保留。迁移 migrated、version_changes=false、sync_performed=false，独立复核最终 diff 无阻断。

同一锁定构建测试程序、相同四项代表用例（含 strict-inspection 与 bootstrap 两个原 30 秒子进程路径）的单次串并行配对：一线程 98,290 ms、四线程 52,045 ms，两边均真实四项通过，零失败/跳过。只是一组 1→4 顺序配对，包含缓存和环境波动，不能扩大成全 35 项或整个收口的精确提速保证；全 35 项留给正式 apply 验证。没有延长子进程超时或启用 nightly 选项。

候选现已冻结，后续使用同一 --packages lifecycle 重建无分析预览、分析和带分析预览；不得复用上一轮 fingerprint。尽量在本轮整体预算内取得完整受管终态；未完成事务最迟 20:26:26 停止自身进程并受管恢复，整体截止仍为 20:29:26。收口后产物准备依据剩余预算实际执行，证据不足时单列，不用计划或已变版本冒充完成。

19:27 的新正式 apply 在 122,758 ms 后提前失败，budget_interrupted=false。错误为 factory-unit 退出 101、timeout=false：原生测试日志的相对路径不存在，发生于测试启动前；候选写入已自动回滚，journal 不存在。该次错误不是 factory 35 项测试的业务断言失败，后续包未执行。失败收据 .runtime/closure-run/develop-apply.stderr.log 和 develop-apply-timing.json 原样保留。

已用实际 Cargo 入口消除歧义：临时诊断断言并输出 native test cwd 等于 CARGO_MANIFEST_DIR，实际为 source/factory；原 `.runtime/...` 日志参数相对于包目录，先前只从根目录直接执行 test exe 的核验及旧目录审查不足。配置已受管修正为 `../../.runtime/bridgeforge-codex/{id}-unit-tests.log`，factory/lifecycle 两个包均固定两层目录。实际 Cargo 的 factory 快速用例通过；lifecycle 291 项发现清单成功且日志落在根目录（只验证参数及发现，不算测试通过），未改变四线程或任何超时。

临时 cwd 诊断已按 hash 恢复并保留 AtomicU64；Copy 恢复旧 mtime 曾导致 Cargo 复用含诊断的旧测试程序，已在最终字节 hash 不变下重新写入文件、通过锁定 Cargo 重编译，最终清单确认原 35 项且无临时诊断。未清理 Cargo 缓存、reset 或手改证据。旧计划 fingerprint 再次失效，接下来从最终源码和配置重新预览/分析/apply，且不因这次启动错误重复前述 15 项定向集。

### 完整检查发现真实前置缺陷

修正日志路径后，19:39:29 的正式 apply 已实际通过 checks/core/factory，factory 原 35 项为 34 通过、零失败、一个由父测试调用的辅助入口按设计忽略；随后进入 lifecycle。该组 factory 结果对应本节最后源码修复之前，不能当作最后候选的完整检查收据。

19:58 逐项日志出现真实失败 `schema4_both_languages_lifecycle_changes_keep_identity_and_tombstones`。保存 first-test-failure.json/lifecycle-at-failure.log 后核对自身 PID、路径和启动时间，停止本次 apply 并调用受管 recover；不是预算中断。develop-apply-timing.json 记录退出 -1、budget_interrupted=false、1,241,397 ms，diagnostic-recover.json 返回 development-recovered。没有继续等待整组检查，也没有执行第二个并发事务。

单独按真实 Cargo 入口复现：274.31 秒退出 101，Rust split 场景 actual 只有 b=2.0.1、c=0.1.0，expected 还应有存续 a=1.2.4。根因确认：新增无记录旧包的 unchanged_content 判断只看包身份和普通源码变化，将有效生命周期变迁中 payload 未变的存续端点误当成纯证据刷新，从而漏掉 a 升版。没有删改该断言或把它改成期望 level=0。

最小修复：原样提取 plan.rs 中现有、以当前 release boundary 及净身份变化判断的循环为 lifecycle_affected，由原 affected 与开发收口共用；仅在 development.rs 的 None 分支排除该集合。Some(entry)、历史内容 hash、全局 force_bump/mechanical 和普通无记录旧包补验规则不变。独立审查最终 diff 无新增阻断，源码已按受管 manifest 传播到公共投影/dogfood，manifest check changed=false。

修复后直接回归为三个原有用例：原失败的 Rust/Python 十生命周期场景、依赖变化下无记录旧包 level=0 补验、任务生命周期端点与消费者闭包。最终三项均通过，零失败/跳过，测试执行 739.65 秒、含编译共 757,372 ms；证据为 lifecycle-fix-tests.log/lifecycle-fix-timing.json。使用两线程定向验证，不代替原配置的完整四线程检查。

### 当前交接边界

本次开发仍未完成，正式版本收口未完成。源码修复改变了公共运行时输入；只读 runtime check 明确退出 2：`generated asset receipt drifted: codex.hooks`。之前的 current 只代表当时旧候选，不能继续代表最后源码。当前没有重新准备最后运行时，也没有用旧工具启动新的正式收口。

原完整需求扩展继续保留，用户未验收，TODO-043 维持进行中。不写版本或账本，不提交、推送、发布、安装用户级副本或写真实下游；原主仓库不随本工作树修复而自动收口。后续先完成适用的直接回归，再按声明的锁定 Cargo 受管准备入口刷新 CLI/Hook/factory，以最终源码重新预览、分析和完成必需检查/收口，最后核对同任务预览和版本、锁、账本、产物一致性。旧 planned、已恢复事务和修复前通过记录均不能替代 development-closed。

## 13. 本地成果整合与工作树移除（2026-10-27）

用户先要求暂停，随后明确授权把现有成果合并回 `D:/Quant/BridgeForgeCodex` 并删除隔离工作树。当前执行位置回到原始目录；本卡上文的隔离路径保留为历史发生位置，不作为后续执行目录。本次仅做本地文件整合、证据保全和工作树移除，保留未提交状态，不记录用户验收或正式开发收口。

两个目录的共同 HEAD 为 8fdfd2f57fb1e539633d10bd01bd665eae4aad8b。按共同基点做三方文件整合，并以本轮冻结源码及已整合来源 hash 识别 worktree 中已有的 TODO-042 前置。保留原目录独有的版本生命周期设计改动及更新后的 TODO-042 需求全文；索引同时保留 TODO-042 最新入口和 TODO-043。公共镜像与 manifest 通过受管生成器从合并后源码重算，不复制旧运行时作为新成果。

恢复备份位于 `.runtime/worktree-transfer/develop-task-closure-20261027-203345/`：original/ 为合并前原目录变更，worktree/ 为隔离目录变更，两个 binary patch 与 merge-plan.json 记录逐文件来源和决策。原始收据逐字节保存在其 evidence/ 下；上文历史 `.runtime/...` 路径从 evidence/ 拼接同一相对路径查阅。收据中的旧 checkout/fingerprint 不改写，只作为历史证据，不能冒充原始目录当前已认证。

约 5.9 GiB 的编译缓存为可重建产物；删除隔离目录前保全源码、未忽略新文件和全部相关测试/失败/恢复收据。本轮旧本地产物也仅存入备份，不覆盖原目录 `.codex/bin`。桌面工具因旧聊天所有权无法在当前聊天归档；本次使用本地可恢复备份及 Git 工作树移除，旧聊天挂接记录不改写。

原始目录的正式开发收口仍未完成：主程序、子包版本和 38 条账本保持原状态，最后源码已修复并通过三项直接回归，运行时需按当前源码受管准备；完整必需检查和最终 development-closed/同范围 current 仍未取得。开发、用户验收和 Git/发布状态不因文件整合改变。

## 14. 方案 A 与验证循环复盘（2026-10-27）

### 14.1 讨论范围与结论

本次汇总只读查看了四个对话及相关源码、配置和记录：[优化 release](codex://threads/01a17acb-7635-76f2-9d14-5d1059ece457)、[优化收口](codex://threads/01a17be5-9edd-7353-8ccd-1193480c37a5)、[完成 TODO-043 受管开发收口](codex://threads/01a17ca4-1d43-7931-bb9c-b947462ce8f4)、[重构](codex://threads/01a1763c-a0ae-77a1-8d1d-a2ac6e07b482)。Causis 的 Qt 问题单独处理，本讨论不授权修改其他项目。

已确认发生的是流程上的循环返工，不能认定为某段程序必然无限循环：源码验证、版本收口、安装身份和产物状态绑定过紧，后续安装或修正改变了验证输入，又触发重验；失败后成功检查缺少实际可用的接续机制；部分重检查还在最终源码稳定前启动。真实缺陷、资源限制和执行失误叠加，造成长时间没有稳定的最终交付。

关键事实复用原始记录，不算本轮新验证：

1. Causis 曾完成 36 包受管收口，2284 项测试通过、1 项跳过；安装同步后 36 包因 environment 分项变化再次待验证。环境确实发生变化，不能推断所有差异都无影响；问题在于没有充分区分应补验的结论。后续 Qt 原生失败属于真实问题，不能因精简流程直接放行。
2. TODO-042 一次 apply 用时 8,662,932 ms，lifecycle 达到 7200 秒超时，候选版本回滚；这证明失败成本，不能当作正常完成耗时。
3. factory 分段诊断发现真实 Git 夹具初始化是主要成本；并行配置下整包 34 项通过、1 个由父测试调用的辅助入口忽略。后续 lifecycle 发现拆包漏升存续包版本，修复后三项直接回归通过；这些历史结果不代表最终整包已经通过。
4. 现有缓存要求显式环境和 reusable 声明。当前工厂七个包均没有显式 environment，也没有启用 reusable 检查，因此“实现支持复用”没有变成本项目正式检查的实际复用。
5. Agent 新增日志参数时只从根目录直接运行测试程序，遗漏 Cargo 的包目录工作位置，造成一次测试启动前失败；路径后来已修正。审查与构建交错、反复使用“只差最后收口”的表述，也增加了返工和错误预期。

### 14.2 方案 A：已执行的本地止血

用户明确授权的第一步是：只向当前仓库 AGENTS.md 项目级专区写入方案 A，十分钟内本地止血，不编译、不跑全量测试、不改版本和账本，暂不提交或发布。该步骤已经完成，实际规则以 [AGENTS.md 的方案 A](../../../AGENTS.md) 为准；本节仅记录结果，不另维护规则副本。

本次新增该节后核对：仅新增方案 A，公共区和已有内容保持不变；版本、锁、版本配置、账本等受保护文件哈希未变，git diff --check 通过。未运行编译或测试，未刷新运行工具，未终止或接管其他对话事务，未提交或发布。

传播分类：方案 A 是此前明确授权的工厂自身配置；本次 summary 是元文档。均未下沉到公共模板、用户级 Skill 或真实下游，本次不改版本、CHANGELOG 或 dogfood。

### 14.3 日常 develop 的简化分工

在方案 A 下，日常开发按“明确本次范围和完成标准 → 实现修改 → 必要工程验证 → 交付结束”执行。需要实际试用时，必要构建仍属于对应验证；不把正式版本尚未收口作为自动追加修复、整包重验或等待的触发条件。交付分别报告工程结果和正式版本状态，未执行的验证不能记为通过。

普通 git-sync 在已有明确授权下独立保存代码；正式版本收口和 release 由用户明确要求后单独执行，机器门禁仍保留。遇证据缺口或失败时明确停止，不自动转入下一轮修复。本地方案 A 生效不等于通用 Skill、机器证据规则或其他项目已全部修好。

旧流程的验证不能一概认定无效：拆包漏升版、Qt 生命周期等真实缺陷需要适用验证，最终 EXE 是否对应当前源码也要核对。浪费主要来自无关范围扩张、等价输入的重复执行、局部变化引起整批证据失效，以及执行过程中自己制造的返工。应精简这些成本，不能把所有缺失验证自动改写成通过。

### 14.4 F1–F5 最小方案与尚未完成事项

| 编号 | 已有成果 | 最小剩余工作与必要验证 | 边界 |
|---|---|---|---|
| F1 检查范围 | 任务范围、必要依赖展开和纯骨架发布提前退出已有实现与定向证据；拆包漏升版已有修正及三项直接回归 | 不重写；核对合并差异，只补实际受影响的无关包排除、真实依赖纳入、无业务退出及防漂移场景 | 最终工具尚未就绪，不能宣称真实发布全链路通过 |
| F2 证据失效范围 | 显式环境合同、跨路径证据和本地产物分离已有实现 | 明确实际工具/环境依赖，先在工厂一个包试点；验证等价路径可复用、真实输入变化失效、缺产物不等于产物已通过 | 不修改 Causis；本机隔离验证不冒充真实双机或实际安装验收 |
| F3 检查复用 | 失败后复用前项成功记录的代码及回归已有 | 与 F2 共用配置，为 factory 一包试点启用；验证成功项复用、失败不缓存、输入变化和记录/产物损坏拒绝复用 | 当前只支持同机、同一完整候选及工具身份；精细局部失效暂不纳入 |
| F4 检查可见性 | 原生逐项日志和 Cargo 相对路径修正已有，factory 并行检查有历史结果 | 只补检查起止、耗时、缓存命中和日志位置，核对成功/失败/超时及机器输出协议 | 不改造通用进程执行器；不承诺单用例失败立即终止整组 |
| F5 实际生效 | 源码、候选 Skill 及项目入口已整合回原始目录 | 所有修改冻结后受管准备一次 CLI/Hook/factory；核对实际能力、current 收据和任务范围只读预览 | 不自动启动正式 apply、升版或账本写入；用户级安装与发布另行授权 |

讨论中的最小方案估算为 75–120 分钟，含定向验证与一次工具准备，不含正式收口、发布或真实双机验收。分项估算：F1 5–10、F2 10–20、F3 25–35、F4 10–15、F5 15–25 分钟；F2/F3 共用准备，实际总时间不能机械重复累加。这是方案估算，不是用户批准的新增预算。

用户认为该方案过长后，另提出快速备选：不再修改 Rust、环境合同或缓存配置，保留 F1 既有成果、沿用现有日志，仅核对输入并执行一次 F5 工具准备及只读预览，预计 20–30 分钟。依据是先前同类准备约 19 分钟，不能保证本次完全相同；本备选只解决已改代码尚未实际用上，F2/F3 等缺口继续保留。十分钟内只能维持已经完成的本地止血，不能承诺三种工具构建完成。

上述两种后续方案均未获得本轮新增实施授权，未执行。当前明确授权仅覆盖已完成的 AGENTS 止血步骤和本次文档沉淀。方案 A 继续生效，不因讨论或历史实施授权重新自动进入收口。

### 14.5 本次 summary 边界

本轮仅补充既有需求卡及 doc/README.md 导航，保持 lifecycle: active、validation_status: awaiting_validation；不验收、不结算或新增 TODO，不修改代码、配置、Rule、AGENTS、Hook、测试或 Memory，不编译、不测试、不刷新工具、不提交或发布。

已有授权边界、按影响验证及本地方案 A 已覆盖本轮执行约束，不新增 Rule/Hook/AGENTS 候选；无归档候选。正式收口、最终运行工具、真实双机和自然运行效果仍未取得完整最终证据，后续不得从文档落盘推断通过。
