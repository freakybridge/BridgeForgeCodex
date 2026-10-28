---
lifecycle: active
validation_status: awaiting_user_acceptance
---

# develop 共用工作流与轻量收口实施计划

日期：2026-10-28。关联需求：TODO-046。

## 1. 决定与授权

用户要求按此前精简建议生成 plan，说明 S/M/L 的工作流、共用部分及 F01–F12 的统一取舍，并授权由 Agent 把握取舍、将可复用部分移入共用流程。随后明确要求“把这份 plan 落盘，然后开始开发吧”。本文件是该任务唯一实施记录；前期动机见[收口要求记录](notes_2026-10-28_develop-closure.md)。

已授权：计划落盘、当前仓库中的 Skill/工具实现、必要验证、投影同步和适用受管子包收口。未授权：提交、推送、主程序 release、用户级安装或真实下游写入。不修改其他任务的既有成果，不把既有改动计为本轮实现。

任务按 L 级处理：涉及多个 Skill 和版本事务。用户已回复“按建议预算推进”，确认整体预算 120 分钟、约 60k 新增 token（估算，含子 Agent）、最多 2 个子 Agent、3 轮验证；token 未实测，周额度换算暂无当前配置可靠依据。沿用整体预算，不按阶段重复审批。

传播四问：通用 Skill 和 Rust 行为属于产品层；工厂专属约束留在自身配置及工厂设计文档；实现后由受管流程处理实际子包版本，主程序 VERSION 与正式 CHANGELOG 等待独立 release；同步实际受影响的 dogfood/公共投影，不更新用户级安装。

## 2. 共用最短路径

目标：D1 明确任务 → D2 调查与实现 → D3 验证与修正 → D4 适用的版本收口 → D5 交付。阶段不是审批点，不要求新增五份文档。

| 阶段 | 共用动作 | 精简方法 |
|---|---|---|
| D1 明确任务 | 明确结果、范围、授权和完成标准，判断规模、风险、既有改动 | 复用对话和唯一记录，只补关键缺口；一次判断文档、预算、协作及独立验证需求 |
| D2 调查与实现 | 查看直接调用、配置和失败路径，实施并完成必要传播 | 按影响调查，复用已有证据；工厂传播判断并入本阶段，不扫描无关全仓 |
| D3 验证与修正 | 执行按风险选定的必要检查，处理本次问题及适用审计 | 一份验证安排、一组有效结果；修正后只重验受影响部分 |
| D4 适用收口 | 更新实际受影响子包版本、依赖、记录并核对最终一致性 | 引用 D3 有效证据，新增影响才补验；机械编排由受管工具承接，无变化跳过 |
| D5 交付 | 返回实际改动、验证/版本结果、限制和试用路径 | 一次更新唯一记录、一次交付，不自动串联 summary、验收、Git 或 release |

共用的是方法和触发条件，不是把各分支要求取并集。条件不满足的动作直接不执行，不新增“申请省略”的步骤。

## 3. S/M/L 只保留必要差异

| 项目 | S | M | L |
|---|---|---|---|
| 规模 | 局部明确、低风险，包括小新增 | 跨模块或少量分叉，边界可确定 | 架构、迁移、高风险或多个关键未知项 |
| 路径 | D1–D5 | 同一主路径 | 同一主路径，D2/D3 按依赖分段 |
| 记录 | 默认对话 | 短任务可用对话，需要持续跟踪/交接才落盘 | 唯一任务记录保存阶段、依赖及完成条件 |
| 执行 | 主对话 | 默认主对话 | 按依赖推进，确有收益才协作 |
| 独立验证 | 按风险及明确要求 | 同左 | 同左；架构、迁移通常触发，不仅按 L 标签 |
| 预算 | 默认额度，不增加访谈 | 默认额度，明显超出时说明 | 一次约定整体预算，阶段不重置 |
| 交付 | 简短结果 | 按需附唯一记录 | 附阶段及实际独立验证结论 |

新增功能不自动归 L；规模决定组织方式，风险决定验证强度。保留公共 AGENTS 对重写、迁移和审计自己改动的独立验证要求。

## 4. F01–F12 统一取舍

| 编号 | 决定 | 共用位置与条件 |
|---|---|---|
| F01 分级与预算 | 简化普通任务预算访谈和周额度计算 | D1 一次判断；需要报价、调整预算或用户要求才计算；用户显式预算及超额停止保留 |
| F02 需求卡、TODO、索引与状态 | 按需持久记录，避免重复建档 | D1/D5 决定唯一载体并增量维护；M 不固定要求完整卡，L 保留记录；不因进入 develop 自动建 TODO |
| F03 改动归属 | 保留按混杂程度核对 | D1 建必要基准，D4 只核对实际变化；同包不明来源仍须查清 |
| F04 Agent 与审计 | 取消按规模机械启动 | D2/D3 按任务边界、收益、风险和明确要求触发；复用真实结果，协作记录可放原卡 |
| F05 传播四问 | 合并到影响判断 | D1/D2 一次处理，范围未变不重复；工厂细节不下沉 |
| F06 manifest、dogfood、投影 | 保留按影响触发 | D2 必要传播，D3 统一验证；D4 不再重复组织检查 |
| F07 能力与工具准备 | 合并检查，缺口才处理 | 实际入口检查能力，不反复独立 self-test；缺失/不匹配明确报告，不隐式安装升级 |
| F08 包与关联范围 | 工具负责主要判定 | D1/D4 共用范围；区分直接修改、必要连带和仅依赖关系，不遗漏真实影响 |
| F09 收口编排 | 合并机械步骤 | D4 语义分析归 Agent，工具校验并执行；只读预览按需，不强制两次外部预览 |
| F10 验证与构建 | 默认复用有效开发证据 | D3/D4 共用证据；升版新增影响或证据缺口才补验，不接受聊天 passed 冒充证据 |
| F11 版本与账本 | 保留 | D4 一次完成，不重复升版；仍是适用任务的完成条件，不移给 release 补做 |
| F12 证据与状态汇报 | 保留含义、精简表达 | D3 留证据，D5 汇总实际结果；工厂六类证据可一张表，未执行项标未验证 |

共用方法只在 develop 主流程维护；规模分支只写差异，collab 只写协作专项，版本合同只写机器语义，工厂文档只写工厂增量。

## 5. 当前实现依据与技术方向

当前 `development.rs::prepare` 要求各 affected 包选择已登记检查；`execute` 写入候选版本后调用 `run_candidate_check`。现有缓存主要针对同机完整候选，不能直接把升版前日志重新绑定为最终候选证据。`apply_scoped` 已会内部 prepare，外部重复预览有合并空间。factory/公共 CLI 分别在 commands.rs 和 lib.rs 路由开发操作。

优先扩展现有检查记录和输入模型，明确源输入、命令、环境、产物与候选之间的有效关系，不新建通用认证平台。不能证明已有证据覆盖的部分补验。必要构建与功能验证分开判断；自动 1.0.0 的稳定合同及失败回退保持正确。具体接口在核对现有模型后确定，不用虚构参数占位。

## 6. 实施任务

| 任务 | 优先级 | 修改范围 | 验证重点 |
|---|---|---|---|
| P1 共用流程 | P0 | templates/skills/develop/SKILL.md、ml-delivery.md、agent-execution.md | S/M/L 最短路径与条件触发，无重复流程 |
| P2 确认与协作 | P0 | templates/skills/confirm/SKILL.md、collab/SKILL.md 及必要引用 | 不重复授权、不强制重复建档，显式调用职责完整 |
| P3 验证与版本事务 | P0 | source/lifecycle/src/release/schema4/development.rs、analysis.rs 及必要输入模型，completion-release.md、schema4.md | 有效证据复用、真实漂移拒绝、稳定条件、回滚与账本 |
| P4 入口与范围 | P0 | source/factory/src/commands.rs、source/cli/src/lib.rs、prepare/apply_scoped | 减少外部往返、只读零写入、关联范围准确 |
| P5 传播与交付 | P1 | 相应 dogfood/公共源码投影、scripts/tests/**、工厂开发收口设计 | 源与分发一致、针对性回归、实际调用数及耗时对比 |

顺序：P1/P2 明确合同，P3/P4 实现合同，P5 验证传播；不得只改 Skill 宣称新工具能力生效。独立审计在源码稳定后执行并复用已有验证。

## 7. 验收场景

1. 小新增不机械归 L，已有授权直接执行。
2. M 短任务不强制完整卡，L 使用一个记录分段推进。
3. 协作/审计条件触发，已有检查不因交接重跑。
4. 已验证内容仅改版本标识时复用结果。
5. 版本进入二进制或依赖实际改变时，分别补构建或相关验证。
6. 缺证、漂移、失败、中断不能产生成功收口记录；恢复保留外部编辑。
7. 无变化、重复收口和无关历史变化不重复升版、不要求全仓清零。
8. 自动 1.0.0、稳定失败回退、主程序 release 保持必要语义。

先运行覆盖上述风险的定向测试；只有具体跨模块风险才扩大。性能比较使用相同场景和方法，分别记录命令往返、检查执行/复用次数及耗时，不承诺未测量的提速幅度。真实下游、用户级安装和自然 runtime 未执行不得宣称通过。

## 8. 实施状态与风险

计划已落盘，实施授权及整体预算已确认，开始实施。协作记录复用本节：workflow_skills implementation-worker 负责 develop 主入口、ml-delivery、agent-execution、confirm 和 collab 五份 Skill；主 Agent 负责 Rust、版本合同、收口 reference 及整合传播；源码稳定后使用第二个独立 Agent 审查。文件边界不重叠，主 Agent 不重复子 Agent 已完成的有效文本检查。

开工前实际核对：

- `.codex/bin/bridgeforge-factory.exe self-test --json` 返回 status=ok，factory=0.3.0、product=8.0.1，具有任务范围和确定性检查复用能力。
- `.codex/bin/bridgeforge-factory.exe develop --preview --packages lifecycle,factory` 返回 `task-development-current`、affected_packages=[]；展开范围为 checks/core/factory/lifecycle/skeleton/workflow。本次开始时这些范围没有待收口项；lifecycle=0.4.0、factory=0.3.0。此结果不是全仓就绪或本轮验证证明。
- CLI 是产品版本载体，不是可传入 --packages 的独立包；首次包含 cli 的预览被工具拒绝，没有写入，随后使用上述正确选择。
- TODO 使用公共 CLI：`.codex/bin/bridgeforge.exe check todo --root .` 返回 errors=[]、退出码 0。factory 的 check 仅接受 factory-version，首次误用 factory 检查 TODO 已纠正。
- 文档 diff 空白检查通过；未运行工程测试、构建或收口 apply。

工作区已有核心清理等任务的约 96 项改动，包含 lifecycle、factory、CLI、schema4 合同、测试及已完成子包收口。必须保留并逐项核对重叠文件；不复写已有结果，不把其他任务的验证当本轮通过。版本事务复用机制与关联范围是主要风险；无法建立有效证据关系时限定补验，不虚构成功。出现实质范围变化或预算不足时只暂停相关动作。

## 9. 实施记录

- P1/P2：五份 Skill 已统一 D1–D5；分级、预算、记录和 Agent 规则按条件触发，净减 133 行。worker 完成相对链接和 diff 空白检查；主 Agent 未重复该调查。
- P3/P4：增加 `staged-development-validation-v1` / `direct-development-apply-v1`；预览收据携带能力和包角色，apply 内部核对分析，可省略外部带分析预览和 fingerprint 参数。需要分开时使用 validate 对最终候选检查并恢复版本，不写账本；apply 引用有效结果。
- 复用在现有检查记录结构上实现，完整显式 environment 为准入条件。未声明环境或 always_run 检查实际执行；最短路径一次 apply 完成 D3/D4，不因拆阶段重复跑同一检查。staged 按包/依赖与声明输入绑定，长期失败重试缓存保留原合同。没有接受任意外部日志或聊天声明的入口。
- F08 保留必要依赖和调用方，收据区分 requested / affected-dependency-or-consumer / context-only；最终范围展开后重算分类，不把 context-only 包变成必验/必升包。
- P5：收口 reference、版本合同、工厂开发收口设计、README/INSTALL 和工厂 AGENTS 入口已同步；受管 manifest 生成了实际公共源码投影和 git-sync 镜像。develop 等共享 Skill 在本工厂按既有约定直接读取 templates 候选，不新增第二份镜像或用户级安装。

独立 review-auditor 检查真实差异，发现并修正四项：staged 复用后的终态 Git guard、未声明环境误复用、独立包修正使无关包失效、最终 affected 扩展后角色分类未刷新。最终静态复核无新增确定阻断；不代替工程和真实运行验证。

### 当前验证进度

- 首次 sandbox fixture 在临时 .git/config 写入被拒，未进入业务断言；相同 Cargo 命令窄提权重试。
- 第一组新 fixture 因 Windows 工具名缺 .exe 失败，修正夹具后 6 项通过（405.34 秒）；此为审查最终修改前的候选证据，不代替最终验证。
- 最终 `cargo test --locked --offline --config scripts/tests/factory-cargo.toml --manifest-path Cargo.toml -p bridgeforge-lifecycle --lib lightweight_tests --target-dir .runtime/source-workspace-refactor/cargo -- --test-threads=4 --nocapture` 14 项通过（718.90 秒），含新增 staged 和既有轻量收口场景。新增测试并入已登记的 core_package_lightweight.rs，不额外扩大检查输入配置。
- 同一受管 Cargo 命令以 `task_development_cache_` 过滤执行既有缓存回归，8 项通过（445.17 秒），覆盖失败复用、源码/产物/宿主/Git 身份变化、篡改及显式环境边界。与 CLI 1 项合计最终工程验证 23 项通过；此前候选的 6 项不重复计数。
- CLI `development_capabilities_and_mutation_flags_are_explicit` 1 项通过；skill-metadata issues=[]/warnings=[]；project-structure errors=[]；manifest --check changed=false；diff 空白检查通过。无关归档提示不纳入本轮。
- 首次受管当前源码工具准备成功：status=prepared，prepared=3，written_files=6，自检执行成功。
- 实际 `develop --preview --packages lifecycle,factory` 确认 affected=factory/lifecycle/workflow；checks/core/skeleton 为 context-only。选定范围内没有来源未明的既有待收口改动，workflow 是 lifecycle 依赖覆盖连带项。
- 同宿主生成 `.runtime/develop-common-path/analysis.json` 后直接 `develop --apply --packages lifecycle,factory --analysis-file .runtime/develop-common-path/analysis.json`，未执行外部带分析预览、未传重复 fingerprint；退出码 0、status=development-closed、checks executed=3/reused=0、git_written=false。factory 0.3.0→0.4.0，lifecycle 0.4.0→0.5.0；workflow 0.2.0 保持，只刷新依赖覆盖。主程序 8.0.1 不变；未配置 stability 合同，子包保持 0.x，不伪称已达到稳定条件。
- 事务内三项为：factory 的参数入口保护、lifecycle 的最终版本候选验证/复用、workflow 的未完成事务阻断同步。前述 23 项覆盖本次源代码风险；版本写入使工具自身构建身份变化，事务只补选必要检查，未重跑 23 项验收集。
- 实际新 factory 对 `develop --validate --dry-run` 返回阻断，未执行写入；预览能力收据已包含两项新能力。
- 最终受管准备成功：prepared=3、written_files=6，实际执行自检。之后零写入 `build-assets --factory --project-root . --check` 返回 status=current、read_only=true、reused=3、built=0；manifest --check changed=false，factory-version healthy=true，journal 不存在，diff 空白检查通过。真实用户级安装、下游和自然 Agent 调用效果仍未验证。

端到端提速未实测；前期 fixture 的检查调用数证明同一最终候选的 validate/apply 可避免重复检查，不把并行负载下的测试耗时作为真实开发提速幅度。

## 10. 交付状态与试用

本次实现、必要工程验证、独立静态审查、受管子包收口、产品传播/dogfood 及最终工厂产物核对已完成，等待用户试用验收。未执行提交、推送、主程序发布、用户级安装或真实下游写入；不将工厂本地结果扩大为用户级 Skill 已生效或自然运行已验收。

当前工厂依根 AGENTS 直接使用 templates 的 develop 候选。最短路径复用已有需求，按 D1–D5 执行；S 直接、M 按需记录、L 使用唯一任务记录。默认一次受管 apply 完成 D3/D4；仅有分离需要且环境合同完整时使用 validate/apply。具体命令以收口 reference 为准，不把本文作为第二份机器合同。

版本结果：factory 0.3.0→0.4.0（兼容新增入口），lifecycle 0.4.0→0.5.0（兼容新增任务验证复用及直接收口），workflow 0.2.0 不变（依赖覆盖刷新）；主程序 VERSION 8.0.1 未变化。对应本轮账本末三条 level 为 2/2/0，三项最终候选检查均实际执行通过。

预算执行：使用 2 个子 Agent、3 轮验证；沙箱权限重试和纯等待不另计验证轮次。token 未实测，不声称整体精确 token 用量或订阅额度消耗已核实。不存在遗留运行中的本轮工具事务或必需工程验证阻断。
