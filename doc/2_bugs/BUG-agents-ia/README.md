---
status: open
severity: high
record_type: bug_topic
scope: bridgeforge-codex 全部下游骨架、用户入口、运行时指令、Skill、Hook 信号、技术收据与文档导航
reported_at: 2026-08-27
source: 用户体验反馈 + 只读静态审计
---

# BUG：骨架的信息架构与人类可理解性存在系统性缺陷

## 结论

当前问题不是单个 `AGENTS.md` 太长，也不是局部中文润色不足，而是骨架没有稳定区分四类受众：

1. Agent 每轮必须执行的运行时指令。
2. 程序读取和验证的机器合同。
3. 产品维护者使用的设计与迁移说明。
4. 用户用来理解、选择和判断结果的界面。

四类内容互相渗透后，形成了常驻指令膨胀、同一规则重复维护、活动文档漂移、机器术语泄漏和用户阅读路径缺失。后续必须自上而下解决；禁止先对单个文件做孤立压缩或文案美容。

## 本次审计边界

已只读检查：

- 根与 Template `AGENTS.md`。
- `skills/**/SKILL.md`、`references/` 和 `templates/skill-routing.json`。
- `README.md`、`INSTALL.md`、`doc/README.md`、Template 文档索引及现行架构说明。
- 项目同步器的计划、执行和失败输出。
- Hook 注册、`clarify` / `focus` 信号、状态注入及主要错误输出。
- 当前相关历史交付文档与静态质量闸。

本次没有修改产品代码，没有运行下游或 Codex Desktop 现场试用。尚未审计全部下游项目的项目专区和嵌套 `AGENTS.md`；因此本文是工厂与公共骨架的总问题基线，不宣称已经覆盖所有下游自定义文本。

## 系统性根因

### R1：按文件分工，不等于按受众分层

当前设计已经区分 `AGENTS.md`、Skill、Hook 和 `doc/`，但每个载体内部仍混合红线、流程、原理、迁移兼容、用户文案和内部收据。物理位置分开了，读者职责没有真正分开。

### R2：安全正确性有硬闸，可理解性没有验收线

现有测试重点证明 ownership、事务、回滚、hash、fixture 和迁移正确；对“用户能否说清当前机制、Agent 是否只读到必要指令、错误结果是否能直接采取行动”没有稳定验收。

### R3：同一语义在多个手写载体中重复

规模预算、Agent 分工、文档路由、升级状态和错误解释同时出现在 AGENTS、多个 Skill、routing JSON、README、INSTALL 和设计文档中。机器镜像可以校验字节一致，跨载体的自然语言语义却会漂移。

### R4：先积累约束，再考虑阅读路径

多数事故通过追加红线、分支和收据修复；单项修复合理，但缺少定期从用户心智和运行时上下文出发的删减、合并与重排，最终形成“每句都有来历，整体不知所云”。

## 问题总表

| ID | 优先级 | 问题 | 当前状态 |
|---|---|---|---|
| IA-01 | P0 | 活动说明包含已退役机制和失效章节引用 | 已传播真实工厂 1.5.13；待真实下游 / runtime |
| IA-02 | P0 | 四类受众缺少明确的信息承载合同 | 已传播真实工厂 1.5.13；待真实下游 / runtime |
| IA-03 | P0 | `skill-routing.json` 的运行时效力不明确 | 路由闭环已传播真实工厂 1.5.13；待真实下游 / runtime |
| IA-04 | P0 | 同一行为契约存在多份手写事实源 | 已传播真实工厂 1.5.13；待真实下游 / runtime |
| IA-05 | P1 | 根 `AGENTS.md` 常驻内容过载 | 已传播真实工厂 1.5.13；待真实下游 / runtime |
| IA-06 | P1 | 高频 Skill 入口承担过多低频细节 | 已传播真实工厂 1.5.13；待真实下游 / runtime |
| IA-07 | P1 | 用户结果依赖 Agent 临场翻译机器收据 | 已传播真实工厂 1.5.13；待真实下游 / runtime |
| IA-08 | P1 | Hook 信号与常驻规则形成重复上下文 | Clarify / Focus / 项目 Memory 已退役，Show State 会话级保留；已传播真实工厂 1.5.13，待真实下游 / runtime |
| IA-09 | P1 | README 和安装说明缺少白话产品心智 | 已传播真实工厂 1.5.13；待新用户试读 |
| IA-10 | P1 | 文档索引不能回答“当前先读什么” | 已传播真实工厂 1.5.14；待真实下游 / 用户试读 |
| IA-11 | P1 | 历史交付状态、当前设计和索引没有可靠收口 | 真实工厂已分类并归档 14 项；待真实下游 / 用户试读 |
| IA-12 | P2 | 用户可见术语、语言和状态命名不统一 | 已传播真实工厂 1.5.16；待真实下游 / runtime |
| IA-13 | P2 | 现有体积闸无法防止不可理解文本 | 已传播真实工厂 1.5.17；待真实下游 / runtime |
| IA-14 | P2 | 缺少真实用户理解度与运行时行为验收 | R2 / R6 已发布并受管安装 1.5.25，runtime 均为 2/2；待用户试用 / 真实下游 |

## 真实工厂传播收据（2026-08-30）

- `refactored-project` 已先合并 IA-08 Memory 退役、自动 Focus 退役与 IA-09 文档改造，再按候选相对可信基线的 119 项差异传播到根 `templates/`、`skills/`、`scripts/`、文档与 `.codex/` dogfood；根 `VERSION` 升至 `1.5.13`。
- 旧 `skill-routing.json`、Clarify / Focus 自动 Hook、项目 Memory Hook / 脚本、`$find-memory` 与 Summary 的项目 Memory 写入链已从正式产品面移除；Show State 只保留 `SessionStart` 路由。
- 工厂既有 `.codex/memory/` 25 个 legacy 文件未修改、未删除，也未被活动 Hook、dispatcher、manifest 或 managed contract 引用；后续清理仍须独立授权。
- `rebuild_shared_skill_manifest.py --check` 返回 `unchanged`；针对性测试 77/77、完整自动测试 333 项通过（1 项既有跳过）、下游 fixture 4/4、项目结构检查退出码 0，`git diff --check` 通过。
- 本收据只证明真实工厂源码、Template、dogfood、fixture 与自动测试；真实业务下游升级、全新 Codex 会话 runtime smoke 和 IA-09 新用户试读尚未执行。下文各阶段中“待传播真实工厂”的句子是当时的历史收据，不再代表当前总状态。

## 逐项证据、影响与关闭条件

### IA-01：活动说明包含已退役机制和失效章节引用

证据：

- `doc/0_architecture/design/codex-native-instruction-architecture.md` 仍称旧无 marker 项目由 `section_layout` 迁移。
- 当前源码中没有 `section_layout` 实现；测试反而要求当前 asset 不含 `managed_blocks` 或 `section_layout`。
- `templates/hooks/focus_reminder.py` 仍引用 `AGENTS.md §9.6`，当前章节实际为 §4.5。

影响：维护者和 Agent 可能按不存在的机制判断问题；长文档一旦失真，用户无法判断哪份说明可信。

关闭条件：所有“当前设计”只描述当前实现；所有章节、文件、命令和字段引用可由静态检查定位到真实目标；历史机制只保留在历史交付、Bug 或 archive 中。

候选解决与收据（2026-08-29）：

- 当前有效面限定为完整候选镜像的根/Template `AGENTS.md`、`templates/`、`.codex/`、`scripts/` 与 `doc/0_architecture/`；`section_layout`、`AGENTS.md §9.6` 和 `§9.6` 均为 0 命中。
- 当前架构文档只描述 marker、项目区逐字保留与 ownership review；历史 Delivery、Bug 和 archive 未改写。
- Template 与 dogfood `focus_reminder.py` 均指向真实标题 `AGENTS.md「任务防漂移」`，两份文件 SHA-256 相同，managed contract 记录的 hash 与实际文件一致。
- `test_instruction_source_check.py` 10/10、manifest `--check`、项目结构门、proposal validator 与 `git diff --check` 均通过。
- 本项不新增“禁止复活”规则、Hook 或负向测试；结论仅关闭候选镜像 IA-01，真实骨架替换仍未执行。

### IA-02：四类受众缺少明确的信息承载合同

证据：

- `templates/AGENTS.md` 同时包含执行红线、骨架原理、Memory 分类、Skill 清单、换机步骤和 Debug 流程。
- `skills/bridgeforge-codex/SKILL.md` 同时包含 bootstrap、运行时 preflight、迁移机制、事务设计、用户话术和内部收据。
- README 的快速开始段落直接解释版本分流、schema、`PreservationManifest` 和公共漂移。

影响：任何读者都被迫阅读不属于自己的细节；文件虽按类型分开，信息仍无法形成清晰入口。

关闭条件：建立“受众 × 信息类型 × 唯一载体”矩阵；每类信息只有一个主事实源，其余载体只保留必要投影或链接。

候选解决与收据（2026-08-29）：

- 候选 Template 与工厂 dogfood 的根 `AGENTS.md` 已将 §2.1 改为“信息放置与指令承载”，按 Agent、程序、维护者和用户四类读者明确红线、流程、机器合同、硬闸、设计资料、产品入口、安装迁移与用户结果的唯一承载位置。
- 同一信息的其他载体只允许保留链接、机械生成投影或执行所需短摘要，禁止手写复制完整规则；下游通过根 `AGENTS.md` 公共区取得相同合同，不新增下游架构文档副本。
- 工厂架构文档只反向引用根 `AGENTS.md` 的同名章节，并明确自身只记录原生加载原理和历史迁移映射，不保存第二份运行时合同。
- 依赖旧章节标题的 Template 与 dogfood `instruction_source_check.py` 已同步；managed contract 与 manifest 已确定性重建，随后 `--check` 返回 `unchanged`。
- `test_instruction_source_check.py` 10/10、`test_current_baseline_project_sync.py` 55/55、下游 fixture 3/3 均通过；项目结构检查退出码为 0，仅输出既存 archive advisory。
- 本项只关闭完整候选镜像中的 IA-02；真实根骨架、真实下游、runtime、版本发布和用户试读仍未验证。

### IA-03：`skill-routing.json` 的运行时效力不明确

证据：

- 文件自称 instruction contract，并要求 root agent 对非 main 阶段显式启动指定 Agent。
- 仓库内只发现它被同步、结构校验和测试读取，未发现运行时 dispatcher 或 Hook 消费者。
- 根 AGENTS 只写“路由明确要求”时启动子 Agent，没有要求 Agent 先读取该 JSON。

影响：它可能只是“可校验但不可执行”的伪契约；实际调度继续依赖 Skill 重复描述或模型猜测。

待验证：Codex 当前运行时是否存在仓库外的原生隐式加载机制；本次没有 runtime 收据。

关闭条件：二选一并形成单一事实源：要么由真实运行时确定性消费并取得 smoke 收据，要么删除伪运行时定位，将必要调度契约放回原生可加载载体。

旧合约候选退役与收据（2026-08-29）：

- 官方 Codex 原生机制使用 `AGENTS.md`、Skill 与 `.codex/agents/*.toml`，没有 `.codex/skill-routing.json` 运行时入口；候选方案选择退役伪合同，不再等待不存在的隐式加载机制。
- 候选 Template 与 dogfood 的两份 `skill-routing.json` 已删除，`codex.skill-routing` 已从 managed contract 移除；现有同步器会安全删除未漂移的旧 whole 资产，漂移目标继续原样保留并阻断。
- Template 与 dogfood `AGENTS.md` 已删除“只有路由明确要求”的悬空依赖；两份 `skill_metadata_check.py` 及三组测试已移除旧 JSON 的存在性、镜像和覆盖率断言。
- fresh init 与旧项目 rebuild fixture 均明确断言下游不再出现 `.codex/skill-routing.json`；下游 fixture 3/3 通过。
- `test_skill_runtime_efficiency.py` 5/5、`test_skill_metadata_budget.py` 8/8、`test_create_worktree_skill.py` 13/13、`test_current_baseline_project_sync.py` 55/55 均通过；manifest `--check` 返回 `unchanged`。
- 完整测试共运行 311 项：298 通过、1 跳过、9 失败、3 错误。未通过项集中在候选镜像没有独立 `.venv`，以及 managed contract 已变化但本轮按候选评审边界没有提前做 VERSION / CHANGELOG 与 release transition；因此不能宣称完整发布门已通过。
- 本阶段没有把旧 JSON 路由迁入 Skill，也没有定义新的统一 Agent 路由机制；因此 IA-03 只完成旧合约退役，后续设计与 runtime smoke 仍待讨论。

新路由候选闭环与收据（2026-08-29）：

- 不新增中央路由表。责任链以原生载体闭环：根 `AGENTS.md` 决定默认由主对话执行及何时允许委派；`SKILL.md` 只为自身阶段点名角色；`.codex/agents/*.toml` 定义角色能力；Codex 原生运行时负责启动、等待、追加指令和汇总。
- Template 与 dogfood 根 `AGENTS.md` 已补齐默认 owner 与显式角色红线：没有用户要求或项目 / Skill 明确委派时必须由主对话执行；委派时禁止只写“独立 agent”“子 agent”等泛称。
- 候选 21 个 Skill 已完成静态复核。`bridgeforge-codex` 的旧骨架审计和受控归一化分别点名 `review-auditor`、`implementation-worker`；`develop` 的事实调研和独立审计分别点名 `light-explorer`、`review-auditor`。其余 Skill 不需要委派或已明确角色。
- Template 与 dogfood `skill_metadata_check.py` 已把工厂 Skill 的含糊角色、未知角色作为硬错误；角色名从 `templates/agents/*.toml` 读取，不维护第二份角色清单。
- 项目同步器会检查下游 `.codex/skills/*/SKILL.md`：含糊或未知角色生成 `project.skill-agent-routing` Gap，禁止自动改写，并在 apply 前零写入阻断；合法的已存在角色通过。
- `test_skill_metadata_budget.py` 与 `test_bridgeforge_codex_root_skill.py` 合计 23/23、`test_current_baseline_project_sync.py` 58/58、下游 fixture 4/4 通过；Skill metadata hook 退出码为 0，manifest `--check` 返回 `unchanged`。
- 本阶段完成 P0 / P1 候选静态闭环；真实工厂传播、至少两个真实下游升级与 Codex runtime smoke 属于 P2，尚未执行，不能宣称 IA-03 最终关闭。

后续待办：

- [x] 骨架分发或升级下游时，检查下游项目级 `.codex/skills/*/SKILL.md` 是否说明角色分配；需要子 agent 的阶段必须指向已存在的 Agent 角色，未说明或含糊项必须作为 gap 报告，禁止替下游猜测补写。
- [ ] P2：传播到真实工厂后，对至少一个低定制和一个高定制下游执行受控升级，并取得角色 Gap、零损失和 no-op 收据。
- [ ] P2：新开 Codex 会话执行一条默认主对话路径和一条 Skill 显式委派路径，核对真实启动角色、等待与最终汇总行为。

### IA-04：同一行为契约存在多份手写事实源

证据：

- `confirm` 与 `develop` 重复维护 S/M/L 规模、时间、token、Agent 和验证预算。
- Agent 职责原来同时出现在根 AGENTS、各 Skill 和 `skill-routing.json`；旧 JSON 已在 IA-03 候选中退役，剩余 Skill 间执行职责仍待逐组审核。
- 文档边界同时出现在 AGENTS、README、Template `doc/README.md` 和 operating guide。

影响：修改一处容易漏掉其他位置；静态镜像一致不等于自然语言语义一致。

关闭条件：逐个契约指定唯一 owner；其他载体只引用、读取或机械生成，不再复制完整规则。

第一组候选清理与收据（2026-08-29）：

- `confirm/SKILL.md` 的“规模与预算硬闸”已明确为 S/M/L、时间 / token / agent / 验证预算及验证轮次口径的唯一 owner，并吸收“等待 agent、重复审计或重复测试仍计入成本”的遗漏规则。
- `develop/SKILL.md` 已删除 20/45 分钟、8k/20k token、token 无法实测和验证轮次定义，只保留 S/M/L 各自的执行路径；开工时必须读取 `confirm` 合同，M/L 从确认卡消费已确认预算。
- 新增单一 owner 回归：具体预算与验证口径只能出现在 `confirm`，同时保证 `develop` 仍保留三条执行路径并显式读取该合同。
- `test_skill_runtime_efficiency.py` 与 `test_skill_metadata_budget.py` 合计 17/17 通过，Skill metadata hook 退出码为 0。通用 `quick_validate.py` 因候选项目 `.venv` 未安装其外部依赖 `PyYAML` 未能运行；本轮未为 Markdown 修改引入新依赖。
- 本次只关闭 `confirm` / `develop` 第一组冗余；`develop` / `collab`、版本戳流程和文档五层职责仍待逐组审核，因此 IA-04 保持打开。

第二组候选清理与收据（2026-08-29）：

- `collab/SKILL.md` 已明确为并行研读、拆分确认、执行分派、串联和独立验证机制的唯一 owner；其他 Skill 只能决定何时进入并消费其收据。
- `develop/SKILL.md` 已区分顺序 L 与并行 L：顺序路径继续自行分派调研、实现和审计角色；并行路径把完整执行交给 `collab`，不再复述文件边界、并行分组、重试和独立 review 规则。
- 已移除 `develop` 跳过 `collab` 用户确认闸的覆盖规则；并行拆分是否成立及拆分方案确认继续由 `collab` 自己负责。并行路径完成后直接复用其 review 收据，禁止二次重复审计。
- 新增并行 owner 回归，保证并行粒度、同组启动和文件不重叠红线只由 `collab` 持有；`develop` 必须完整转交并消费 review 收据。
- `test_skill_runtime_efficiency.py` 与 `test_skill_metadata_budget.py` 合计 18/18 通过，Skill metadata hook 退出码为 0。
- 本次关闭 `develop` / `collab` 第二组冗余；版本戳流程和文档五层职责仍待审核，因此 IA-04 保持打开。

第三组候选清理与收据（2026-08-29）：

- 根与 Template `AGENTS.md` 只保留版本域红线：业务 `VERSION` 与骨架版本戳不能互相代替，骨架戳只能由统一项目同步器修改；已删除验证、ready、最后写和回滚等操作顺序。
- `bridgeforge-codex/SKILL.md` 的“事务边界”已明确为 preflight、事务、回滚和版本戳写入顺序的唯一操作 owner；旧戳迁移段只描述“终态保留当前戳”，不再重复写入顺序。
- `codex-project-operating-guide.md` 只解释双版本属于独立生命周期，并指向根红线和 `$bridgeforge-codex`；已删除 release preflight、ownership classifier、逐文件 G*、回滚和先后顺序的复述。
- 新增版本职责回归，保证 AGENTS 不恢复操作顺序、operating guide 不恢复执行细节，Skill 必须保留最后写戳与事务回滚。
- `test_bridgeforge_codex_root_skill.py` 13/13 通过，Skill metadata hook 退出码为 0。
- 本次关闭版本域与版本戳第三组冗余；文档五层职责仍待审核，因此 IA-04 保持打开。

第四组候选清理与收据（2026-08-29）：

- `templates/doc/README.md` 保持文档布局、各层职责和当前索引的唯一 owner；根与 Template `AGENTS.md` 只保留五层、唯一索引、禁止散落和变更同步等执行红线。
- `codex-project-operating-guide.md` 已删除五层含义和索引同步规则的复述，只指向 `doc/README.md`，并保留调用 `$archive-scan` 的操作入口。
- 清理时发现 Template 从未下发 operating guide，但公共 AGENTS 同时用它承载文档布局和换机步骤，形成候选断链。两份公共 AGENTS 当时分别改为读取 `doc/README.md` 与根 `INSTALL.md`；IA-05 逐项复查又确认 `INSTALL.md` 不下发且未承接项目恢复步骤，现已删除该无效指针，改由项目级“快速命令”与 `$bridgeforge-codex` 分别承接依赖和骨架恢复。
- 新增 owner 与断链回归：公共 AGENTS 必须指向上述两个真实载体，禁止重新引用 factory-only operating guide。
- `test_instruction_source_check.py` 与 `test_project_structure_check.py` 合计 19/19、`test_current_baseline_project_sync.py` 58/58 通过；manifest `--check` 返回 `unchanged`，`git diff --check` 通过。
- 至此规模预算、并行执行、版本戳操作和文档五层职责四组手写双 owner 均已在候选镜像清理；旧 Agent JSON 双 owner 已由 IA-03 退役。IA-04 候选关闭，真实工厂、真实下游与 runtime 仍未传播或验证。

### IA-05：根 `AGENTS.md` 常驻内容过载

证据：

- 原始评审时 Template `AGENTS.md` 为 165 行，工厂根文件为 188 行；完成 IA-01～IA-04 后，两份公共区均为 146 行。
- Template 中“必须 / 禁止 / 不得 / 只能 / 只允许”等硬规则语句约 40 行；工厂根文件约 56 行。
- Memory、文档目录、Skill 入口、换机流程、clarify、focus、Debug 和审计规则全部每轮加载。

影响：Agent 需要从大量制度文字中重新识别当前任务真正相关的约束；人类阅读时缺少从目标到规则的路径。

关闭条件：根文件只保留所有任务确实需要常驻的少量行为和安全红线；低频流程、原理、参数、命令及例外下沉；项目专区只保留对执行真正必要的项目事实。

候选解决与逐项防回退收据（2026-08-29）：

- `[clarify]` / `[focus]` 各压为一条触发与权限边界；完整条件只由受管的 `doc/3_reference/codex-hook-signals.md` 承接，并新增“根文件无完整 SOP、承接文档存在且会下发”的回归。
- Memory 根指令只保留项目内隔离、`MEMORY.md` 索引红线和 `$summary` / `$find-memory` 路由；topic 创建、验收、冷热区和深度检索细节只由两个 Skill 承接。
- 文档五层、唯一索引、交付布局和测试目录红线合并为三条；五层职责仍只由受管 `doc/README.md` 承接。根文件已删除易过期的常用 Skill 硬编码名单。
- 换机规则复查发现 `INSTALL.md` 是 factory-only 产品安装文档，既未下发也未承接下游依赖恢复；候选已删除该无效指针和通用 clone 占位命令，改为项目级“快速命令”恢复依赖、`$bridgeforge-codex` 核验骨架与 Hook。
- Agent 路由只合并主对话与子 agent 边界；默认主对话、显式委派、点名已存在角色及三个主对话专属 Skill 均保留，IA-03 路由合同未回退。
- 沟通、证据和 Debug 只合并同类红线；完整调用链只保留在公共架构红线一处，独立验证、三次失败急停、量化证据、根因置信度、副作用、性能基线和自改审计均有正向语义断言。
- 两份公共区从 146 行降至 112 行并逐字一致；未继续追求预估行数，因为信息承载索引和角色路由属于必须常驻的已解决合同。
- 每组修改后均重建 managed contract 与发布 manifest，并检查事实源重复、下游资产、旧路由、公共区镜像和 diff；最终 manifest `--check` 为 `unchanged`，`git diff --check` 通过。
- 本项直接相关的指令、同步、结构、Skill 与入口测试 126/126 通过；下游 fixture 4/4 通过，覆盖 current init 幂等、旧项目确认重建、当前漂移零写阻断和项目 Skill 角色 Gap 零写阻断。
- 完整测试运行 327 项：314 通过、1 跳过、9 失败、3 错误。未通过项仍集中在候选目录没有独立 `.venv`，以及 managed contract 处于未发布变更、没有 VERSION / release transition；没有 IA-05 新增语义或下游同步失败。候选发布门、真实下游、Codex runtime 与用户试读仍未验证，禁止据此宣称最终发布或 runtime 关闭。

### IA-06：高频 Skill 入口承担过多低频细节

证据：

- `bridgeforge-codex/SKILL.md` 210 行、`summary/SKILL.md` 178 行、`develop/SKILL.md` 113 行、`confirm/SKILL.md` 102 行。
- `bridgeforge-codex` 主文件已存在 `references/`，但大量模式分支、Native Memory 状态和内部收据仍留在入口。
- `summary` 已有 `references/deep-steps.md`，主文件仍保留完整路由、metadata、topic 生命周期和用户级 memory 规则。

影响：每次调用都加载低频分支；主路径被异常分支和治理细节淹没。

关闭条件：入口只保留定位、主路径、选择点、停止条件和用户结果；模式细节、schema、内部收据和低频故障进入按需 references；跨 Skill 的共享契约只有一个 owner。

`bridgeforge-codex` 子项候选解决与收据（2026-08-29）：

- `skills/bridgeforge-codex/SKILL.md` 从 214 行降至 95 行，主入口只保留产品刷新、模式与健康路径判断、按需 Native Memory 路由、只读计划、Apply 入口和默认用户结果。
- 新增 `runtime-preflight.md`、`native-memory.md`、`transaction.md` 与 `technical-receipts.md`；已有 `init/adopt/update` 只承接对应模式。事务顺序、Native Memory 状态矩阵、旧骨架角色分配和字段级收据均只有一个 reference owner，入口只保留读取条件。
- Template 与 dogfood 根 `AGENTS.md` 已增加骨架级渐进披露红线：简单 Skill 保持单文件；复杂 Skill 的入口只保留共同目标、主路径、选择点、停止条件和 reference 路由，禁止入口与 reference 双写。
- Skill metadata Hook 已扩展到扫描入口链接的一层 references；未知 Agent 角色或泛称角色不能借下沉逃过硬闸。
- 四个新增 reference 已显式登记到 `bridgeforge-codex-manifest.json`；新增目录与 manifest 精确对账测试，防止 reference 只存在于工厂而没有安装到用户入口。
- 联合测试 110/110、下游 fixture 4/4 通过；Skill metadata Hook 退出码为 0，manifest `--check` 返回 `unchanged`，`git diff --check` 通过。系统 `skill-creator` 的 `quick_validate.py` 因当前项目虚拟环境未安装 `PyYAML` 未能启动，本轮没有为 Markdown 重构擅自增加依赖。
- 本子项只关闭候选镜像中的 `bridgeforge-codex` 入口过载；真实工厂、真实用户级安装与 runtime 尚未传播或验证。

`summary` 子项候选解决与收据（2026-08-30）：

- `skills/summary/SKILL.md` 从 178 行降至 77 行；入口只保留模式硬闸、证据边界、reference 选择点、共同执行顺序、用户结果和停止条件。
- 普通模式、验收模式、Memory 目标与生命周期、项目 writer 路由分别由 `ordinary-mode.md`、`acceptance-mode.md`、`memory-targets.md`、`writer-routing.md` 唯一承接；已有 `deep-steps.md` 只在旧碎片、确认整理批次、rule 对账或归档候选时读取。
- 主入口直接链接全部五个 reference 并写明读取条件；Skill metadata Hook 新增孤儿 reference 硬闸，`references/*.md` 未由入口路由时提交直接失败。`summary` 目录与发布 manifest 也新增精确对账测试，防止文件只存在于工厂而未安装。
- 核心 Summary、Skill metadata、运行效率与指令 owner 回归 49/49；共享 Skill 分发 26/26、项目同步器 58/58、下游 fixture 4/4 通过。扩大 Hook 回归 85/86，唯一失败仍是候选目录没有独立 `.venv`，与本轮语义和分发无关；本轮没有为 Markdown 重构复制虚拟环境。
- manifest 重建后 `--check` 返回 `unchanged`，Skill metadata Hook 退出码为 0。系统 `skill-creator` 的 `quick_validate.py` 仍因项目虚拟环境未安装 `PyYAML` 无法启动，本轮未为 Markdown 重构新增依赖。IA-06 在候选镜像内完成；真实工厂传播、真实用户级安装、真实下游项目 Skill 重构与 Codex runtime smoke 仍未执行。

`develop` 子项候选解决与收据（2026-08-30）：

- `skills/develop/SKILL.md` 从 107 行降至 83 行；入口只保留 S/M/L 路由、M/L 开工确认、共同执行顺序、预算升级闸、用户结果与停止条件。S 级不再加载任何 M/L 专项流程。
- 新增 `ml-delivery.md` 和 `agent-execution.md`：前者唯一承接 M/L 需求包、文档同步、验证与试用闭环；后者只在 M 级确需 Agent 或 L 级时读取，唯一承接明确角色、顺序 / 并行选择和独立 review 收据复用。
- `confirm` 继续独占规模与预算数值、token 估算和验证轮次口径；`collab` 继续独占并行拆分与协作细则。`develop` 只读取两者，不复制合同。`confirm` 当前 104 行但属于单一连续访谈流程，本轮不为追求行数拆出常驻必读内容。
- 新增 `develop` 入口预算、reference 路由、单一 owner 和 manifest 精确覆盖测试；第一轮合同测试 24/24、Skill 分发与指令联合回归 82/82、项目同步器 58/58、下游 fixture 4/4 通过。Skill metadata Hook 退出码为 0，manifest `--check` 返回 `unchanged`。
- 系统 `skill-creator` 的 `quick_validate.py` 仍因项目虚拟环境未安装 `PyYAML` 无法启动；本轮没有新增依赖。候选镜像内未发现 IA-03 角色含糊、IA-04 双 owner、孤儿 reference 或分发断链复发。

下游传播待办：

- [ ] 传播骨架前，逐个只读审计真实下游的项目级 `.codex/skills/*/SKILL.md`；简单 Skill 保持单文件，只有多模式或包含大量条件性细节的 Skill 才按本规范重构。
- [ ] 每个复杂 Skill 必须由主入口明确写出 reference 读取条件；入口与 reference 不得复制同一规则，所有 reference 必须随项目 Skill 保存在下游并可被实际读取。
- [ ] 每个下游 Skill 修改后单独检查角色名称、孤儿 reference、manifest/ownership、主入口语义和既有业务流程，发现 IA-03、IA-04 或分发断链复发时先修复再继续下一个。
- [ ] 至少选择一个低定制和一个高定制真实下游取得静态对账、升级 no-op 与 Codex runtime 收据；未取得收据前禁止宣称下游重构完成。

### IA-07：用户结果依赖 Agent 临场翻译机器收据

证据：

- 项目同步器默认只打印包含 `safe`、`risk`、`gaps`、`aggregate_fingerprint` 等字段的 JSON。
- `bridgeforge-codex` Skill 用独立章节要求 Agent 不得把这些术语直接展示给用户，并临场改写为白话。
- 失败路径同时输出 JSON 和英文 `BLOCKED` stderr。

影响：同一程序结果可能因 Agent、模型或上下文不同而获得不同解释；用户直接运行脚本时没有稳定的人类界面。

关闭条件：机器 JSON 与人类结果明确分层；程序或确定性 renderer 直接输出“发生了什么、改了什么、为什么停、下一步需要什么”，Agent 只做补充说明，不承担唯一翻译职责。

候选解决与收据（2026-08-30）：

- 项目同步器新增 `--output-format machine|human|combined`。默认 `machine` 逐字段保留旧 JSON 与失败 `BLOCKED` stderr，现有测试、fixture、Hook 和程序调用无需迁移；退出码、plan、确认、Apply、回滚与版本戳语义均未改变。
- `human` 由同步器确定性生成“结论、待处理事项、下一步”，覆盖 no-op、待执行计划、risk / 重建确认、gap、blocker、Apply 成功和失败回滚；不输出 fingerprint、asset ID、内部枚举或 traceback。
- `combined` 同时返回 `machine` 与 `human`。`bridgeforge-codex` Skill 的 plan 与 Apply 均固定使用该模式，按 `machine` 推进流程并原样展示 `human`；Agent 只在用户追问而结果未覆盖背景时补充说明，禁止改写同步器结论。
- `codex-project-sync.md` 和 `technical-receipts.md` 已登记三种模式的唯一职责与兼容边界；共享 Skill manifest 已同步两个变更文件的 hash，`--check` 返回 `unchanged`。
- 项目同步、Skill 分发、metadata、指令 owner 与入口合同联合回归 136/136；下游 fixture 4/4 通过，并新增真实 CLI 的 `machine` 兼容、`human` 无术语、`combined` 成功 / 失败分层与无混合 stderr 断言。
- 完整候选测试运行 338 项：325 通过、1 跳过、9 失败、3 错误。未通过项仍集中在候选目录没有独立 `.venv`、managed contract 处于未发布变更而没有 VERSION / release transition，以及既有 release risk fixture；本项相关的 136 项联合回归与 4 项下游 fixture 全部通过。
- IA-01～IA-06 防回退检查未发现旧 `skill-routing.json`、角色泛称、孤儿 reference、manifest 断链、指令双 owner 或 Skill 入口过载复发。本项在候选镜像内完成；真实工厂传播、真实用户级安装、真实下游与 Codex runtime smoke 仍未执行。

### IA-08：Hook 信号与常驻规则形成重复上下文

证据：

- `clarify` 与 `focus` 的完整响应规则常驻 AGENTS，Hook 又在用户消息后注入信号和指针。
- SessionStart 和 UserPromptSubmit 还会注入 memory、Git 状态、snapshot、archive 等状态块。
- Hook 注释承认重复注入会产生 token 噪声，但当前方案仍依赖较长的常驻章节解释信号。

影响：上下文中同时存在规则正文、信号和状态收据；真实成本与行为收益没有统一量化。

关闭条件：为每类信号指定唯一解释位置；建立注入字符/token、触发频率、误触发率和行为收益基线；删除没有可测收益的重复层。

P0–P1 确定性基线（2026-08-30）：

- 新增只读测量器、32 条独立消息和 3 组七轮 Focus 序列；测试直接加载候选 Template 的真实 Hook，Focus 状态隔离到临时目录，产品 Hook 测试前后 hash 一致。
- Clarify 在 6 条真正模糊大需求上全部触发，但另有 19 次误触发：明确小任务 8/8、明确大任务 6/6，续接消息 5/8；每次固定 66 字符。
- Show State 每轮固定注入 80 字符；32 条样本中只有 5 条标记为直接需要 Git 状态。Memory 在 0 / 1 / 5 个候选时分别注入 45 / 212 / 596 字符，SessionStart Memory 索引上限为 6023 字符。
- 三组 Focus 序列都在第 4、7 轮注入；用户明确换题后仍引用第一轮任务锚。单轮 UserPromptSubmit 不含 Focus 的上下文为最小 126、中位 193、P95 744 字符，20 轮 P95 累计 14880 字符。
- 测量器单元测试 5/5、项目 Memory 回归 3/3、下游 fixture 4/4 通过，连续两次完整结果一致；Hook 路由与运行效率联合回归 33/34，唯一失败仍是候选目录缺少独立 `.venv`。以上字符数不得冒充真实 token；Agent 行为收益和实际 token 仍未验证，因此当前不得删除或修改任何 Hook。
- Clarify P2 使用桌面端现有登录态完成 24 次隔离 A/B：6 个场景、每个重复 2 次，A 组带现行 `[clarify]` 信号，B 组不带；两组各 12/12 判断正确，12 组配对零分歧、零解析失败。信号在本样本中没有产生可测行为收益。
- 结合 P1 的 19 次误触发与每次 66 字符注入，用户已授权退役 Clarify Hook。候选 Template 与 dogfood 的脚本、dispatcher 和受管资产已清除；`AGENTS.md` 与说明文档保留 Agent 原生澄清规则。此时 Show State、Memory、Focus 尚未做行为 A/B，因此未联动处置。
- 退役负向测试与本轮相关 Hook / 结构断言 37/37、项目同步器 59/59、指令单一源 22/22、下游 fixture 4/4、IA-08 测量器 5/5 通过；proposal contract `PASS`，manifest no-op。唯一既有失败仍是候选目录缺少独立 `.venv`。真实工厂发布、真实下游升级与发布后 runtime smoke 尚未执行。
- 完整报告见 [`IA-08-hook-context-test-report.md`](IA-08-hook-context-test-report.md)。CLI 路径因没有可复用登录态而终止；后续只复用桌面端现有登录态，不再要求用户重复登录。
- Show State P2 使用 24 个全新隔离任务完成 A/B：A / B 各 12/12 符合预期、零解析失败。4/12 配对在直接查询 dirty 或 ahead/behind 时产生收益，A 组可直接回答而 B 组需先检查 Git；提交安全与无关任务共 8/12 配对零变化。
- 因 Show State 存在窄但真实的状态查询收益，不整项退役。用户已授权候选处置：Template 与 dogfood 已移除 UserPromptSubmit 的每轮注入，保留 SessionStart 一次初始状态；正式工厂、真实下游与 runtime 尚未传播验证。
- 处置后 Hook / 防复发 / 分发定向回归 32/32、同步器 59/59、指令单一源 22/22、fixture 4/4、proposal 与 manifest 检查通过，四组 Template/dogfood 资产逐字节一致。完整候选回归 343 项中 330 通过、1 跳过、12 项被既有候选 `.venv` / 未发布合同硬闸阻断；详见测试报告，禁止宣称全套通过。
- Memory P2 使用 24 次有效隔离 A/B（另有 4 次系统记忆污染样本排除）：12 组配对中 4 组获得历史事实 / 既有约束召回收益，2 组被词面相似但实体错误的候选误导，2 组对过期候选保持核实，4 组无变化。零候选 45 字符收据没有测出收益。
- 因 Memory 同时存在真实收益与真实误导风险，不整项退役，也不保持原样。下一步先设计“保留召回、取消无效收据、阻断跨实体错误候选”的候选方案；未获授权前不修改实现。
- Focus P2 使用桌面端现有登录态完成 24 次隔离 A/B：6 个场景各重复 2 次，覆盖临时岔开后续接、顺序计划、明确换题、方案替换、正常深入和范围缩小。A / B 各 12/12 判断正确，12 组配对零分歧、零解析失败。
- P1 已证明 Focus 在第 4、7 轮周期注入、每组累计 152–158 字符且明确换题后仍引用旧锚；P2 没有测得行为收益。经用户授权，候选 Template 与 dogfood 已删除自动 `focus_reminder.py`，dispatcher、Hook 注释、managed contract、根指令和活动参考均移除自动信号；手动 `$focus` Skill 保留并改为显式调用时独立管理任务锚。
- Focus 候选退役与手动 Skill 保留测试 5/5、指令单一源 17/17、Skill metadata 14/14、共享 Skill 分发 26/26、同步器 59/59、下游 fixture 4/4 通过；完整候选回归计数未发生新增失败。
- Focus 已进一步传播到真实工厂 `1.5.12`：真实 Template / dogfood Hook、dispatcher、Hook 注释、managed contract、根指令和活动参考均已收口，手动 `$focus` 已解耦。真实工厂 Hook 单一源 22/22、Skill metadata 9/9、指令与结构 22/22、共享 Skill 分发 26/26、同步器 50/50、release transition 27/27、fixture 3/3 通过；完整自动测试 308 项全部通过、1 项跳过。真实业务下游与下游 runtime 尚未传播验证。

### IA-09：README 和安装说明缺少白话产品心智

证据：

- README 快速开始后立即出现产品 home、薄入口、版本阈值、schema、`PreservationManifest`、公共漂移和 ledger 等内部术语。
- INSTALL 同时面向新用户、旧版本迁移、版本戳诊断和维护者，未提供按读者分流的入口。

影响：用户知道命令，却难以回答“这个产品平时替我做什么、哪些内容归我、什么时候会询问、失败后是否改了盘”。

关闭条件：README 首屏用白话建立产品心智和最短使用路径；安装、迁移、维护者协议分流；术语首次出现必须解释或链接词汇表。

候选处置：候选 README 首屏已改为白话说明产品用途、唯一入口、写入范围和安全边界；候选 INSTALL 已按首次安装、旧用户、异常诊断和维护者分流。版本路由、`PreservationManifest` 与 schema 3 均在首次需要处解释，详细同步合同继续由架构文档单一承载。独立复核确认旧技术事实和安全语义无损，候选指令来源回归 22/22 通过，`git diff --check` 通过；真实工厂与下游尚未传播验证。

### IA-10：文档索引不能回答“当前先读什么”

证据：

- `doc/README.md` 用约 50 条高密度 Delivery topic 平铺历史演进。
- 活跃架构、当前交付、待解决 Bug、已完成历史和维护者手册没有形成任务导向入口。
- Template `doc/README.md` 主要是注释占位和目录样例，没有“当前状态 / 从这里开始”区。

影响：索引完成了登记，却没有完成导航；用户需要先理解项目历史才能找到当前真相。

关闭条件：唯一索引顶部先回答当前架构入口、当前交付、开放 Bug、常用操作和历史归档；完整清单可以保留，但不得抢占第一阅读路径。

正式工厂处置（1.5.14）：

- 根 `README.md` 只保留稳定的文档入口，不复制动态状态。
- `doc/README.md` 顶部新增“从这里开始”，直接指向当前架构、当前交付、开放 Bug、项目操作和历史记录；完整清单原样保留在后方。
- Template 同步提供五个受管导航行，下游可追加项目自己的当前主线；同步器只更新公共行，不覆盖项目行。
- 同步器完整回归 61/61、相关结构与发布合同 51/51、完整自动测试 333 项通过（1 项既有跳过）、下游 fixture 4/4；manifest `--check` 为 `unchanged`，项目结构与 `git diff --check` 退出码均为 0。
- 工厂实现与自动测试只证明导航合同已落地；真实下游更新和新用户试读仍未验证。

### IA-11：历史交付状态、当前设计和索引没有可靠收口

证据：

- `codex-agents-structure-reorganization` 需求卡仍为 `status: validating`，正文却已有多轮实施记录。
- legacy `.codex/memory/MEMORY.md` 仍把该事项列为 Active，并声称“真实骨架暂不替换”；项目 Memory 运行链已经退役，该索引不再是当前状态事实源，这条陈旧描述只能作为待迁移历史证据。
- 当前设计文档与后续 current-only 交付发生冲突，但仍同时列为活动架构资料。

影响：无法仅从状态和索引判断事项是否完成、被替代或仍需继续；历史决定继续污染当前说明。

关闭条件：每项交付有可对账的 active/completed/superseded 状态；当前架构只引用当前事实；历史记录进入 topic、Bug 或 archive，并保留明确替代关系。

第一组事实错误处置（1.5.14）：

- `codex-native-instruction-architecture.md` 已改为 Clarify / Focus 自动 Hook 均退役，澄清由 Agent 语义判断，手动任务锚由 `$focus` 承担。
- `design-rationale.md` 已移除 `$git-sync` 重建项目 Memory 索引的退役职责，只保留现行 manifest 与版本文件事务。
- 本节已更正项目 Memory 证据：legacy 索引实际仍有陈旧 Active 条目，但不再具有当前状态权威。
- 本组只修正当前事实，不代表 IA-11 生命周期合同、状态迁移或归档工作已经完成。

第二组生命周期合同处置（1.5.15）：

- `templates/doc/README.md` 成为下游文档生命周期合同的产品事实源，工厂 `doc/README.md` 保持同一投影；需求卡和 Bug 用 `lifecycle` 表示当前性，用 `validation_status` 表示验证进度，旧 `status` 或正文状态只保留为 legacy evidence。缺少 `lifecycle` 时一律视为 `unclassified`，禁止推断为 active 或 completed。
- `$confirm` 只创建 `active / not_started`；`$develop` 只推进验证状态且不提前关闭生命周期；`$summary 同意验收` 才能写 `completed / verified`；`$archive-scan` 只在用户确认移动后写 `archived`，`superseded` 必须保留 `superseded_by`。
- Template 与 dogfood `archive_scan.py` 已优先读取正式生命周期字段，并对旧完成标记保留显式兼容路径；同步器把“文档生命周期”作为受管区块下发，同时保留下游项目自有索引内容。
- 生命周期与 Skill 定向回归 28/28、同步器关键路径 2/2、完整自动测试 337 项通过（1 项既有跳过）、下游 fixture 4/4；项目结构、Skill metadata、manifest `--check` 与 `git diff --check` 均通过。实跑归档扫描器只返回项目顶层候选，嵌套证据镜像不再被误扫。系统 `quick_validate.py` 因项目 `.venv` 未安装其外部依赖 `PyYAML` 未能启动，本轮没有为 Skill 文本调整新增依赖。
- 本组没有批量改写历史需求卡、移动归档或修改 legacy `.codex/memory/`；当前只有生命周期合同自身的需求卡进入新字段。IA-11 仍保持打开，下一组开始逐项分类历史卡。

第三组历史需求卡分类（2026-08-30）：

- 盘点 `doc/1_delivery/**/requirements_*.md` 共 78 张：51 张为 `active`、4 张为 `completed`、23 张为 `superseded`，0 张为 `unclassified` 或 `archived`。
- 只有正文明确记录用户执行 `$summary 同意验收` 或等价验收事实的 4 张卡进入 `completed / verified`；“实现完成”“测试通过”或旧 `status: completed` 但缺少用户验收证据的卡仍保持 active，并按真实缺口写 `awaiting_validation` 或 `awaiting_user_acceptance`。
- 项目 Memory、旧模型 / 套餐路由、旧 switch / Markdown Rule 投影、Stall Warning 和早期 AGENTS 草案等已被现行决策替换的卡写为 `superseded / verified`；23 张卡全部具有可解析且真实存在的 `superseded_by` 文档指针。
- 旧 frontmatter `status` 已从 78 张需求卡清零；新增 `test_document_lifecycle_contract.py`，硬闸要求每张卡同时具有合法 lifecycle / validation 状态，禁止 `archived` 留在 `1_delivery`，并检查 completed 验证状态与 superseded 替代指针。
- 生命周期、扫描器与指令定向回归 22/22；完整自动测试 338 项结束，337 通过、1 项既有跳过；下游 fixture 4/4，项目结构、manifest `--check` 与 `git diff --check` 均通过。实跑 `$archive-scan` 得到 14 个候选，但没有执行移动。
- 本组只完成状态判定，没有移动任何 delivery、没有写 `archived`，也没有修改 legacy `.codex/memory/`。IA-11 保持打开，下一组由 `$archive-scan` 审核并经用户逐项确认后移动归档候选。

第四组历史归档收口（2026-08-30）：

- 用户确认将扫描所得 14 个候选全部归档；13 个 Delivery topic 已用 `git mv` 移至 `doc/4_archive/delivery/`，1 个已解决 Bug 已移至 `doc/4_archive/bugs/`。
- 13 个 topic 内共 16 张需求卡全部写为 `archived / verified`；原有 `superseded_by` 关系继续保留，13 个替代目标均已按归档后的相对位置重新解析并确认存在。归档 Bug 同样写为 `archived / verified`，旧 `status` 只作为历史证据保留。
- `doc/README.md` 已从当前 Delivery 和 Bug 清单移除这 14 项，并新增按日期登记的归档表；当前 Bug、IA 账本与项目 Memory 退役账本中指向旧路径的引用全部改指归档位置，旧 `doc/1_delivery/<已归档 topic>` 与旧 Bug 路径命中清零。
- 生命周期与归档扫描定向测试 5/5、完整自动测试 338 项结束，337 通过、1 项既有跳过；`archive_scan.py --json` 返回空数组，`git diff --check` 通过。
- 工厂文档已具备可对账的 active / completed / superseded / archived 状态，当前索引与历史位置一致；IA-11 在工厂范围关闭。真实下游文档分类与归档仍需在各下游升级时独立执行和验收。

### IA-12：用户可见术语、语言和状态命名不统一

证据：

- 用户文档和错误输出混用 ownership、dogfood、fail-closed、gap、blocker、readiness、fingerprint、schema、current-only 等术语。
- Hook 和脚本输出同时存在中文、英文和中英混排；相似状态使用 `ready`、`completed`、`planned`、`blocked`、`degraded` 等多个维度。

影响：用户需要先理解内部数据模型，才能判断结果；不同入口对同一状态的说法不一致。

关闭条件：定义内部状态模型与用户状态词的稳定映射；用户结果统一使用简体中文和少量固定词；内部英文标识只在技术收据中出现。

真实工厂解决与收据（2026-08-30）：

- 新增 `doc/0_architecture/design/user-facing-result-contract.md`，将输出分为用户结果、技术收据和机器接口三层；用户层只使用“已完成、无需处理、等待确认、未完成、已完成但有待办、提醒”六种结论。
- 项目同步器的 human / combined 输出和 `$git-sync` 已统一为“结论、待处理事项、下一步”；机器 JSON 字段、英文枚举、退出码、事务和回滚语义均未改动。
- 项目结构、跨项目写入、用户配置写入和批量 `git add` 四类普通 Hook 已补齐中文状态与可执行下一步，同时保留稳定技术前缀。
- `config_health_check.py` 与 `encoding_check.py` 保留纯 ASCII 英文诊断：它们需要在 UTF-8 环境或文本本身已经损坏时仍能可靠报告，属于合同明确限定的编码自检例外。
- 同步器与主 Skill 定向测试 76/76、`$git-sync` 与高频 Hook 定向测试 49/49 通过；完整自动测试共 340 项（339 通过、1 项既有跳过），下游 fixture 4/4 通过，manifest 重建后复查为 `unchanged`，Skill metadata 与项目结构检查退出码均为 0，`git diff --check` 通过。
- 本项已完成真实工厂源码、Template 与 dogfood 的实现；真实业务下游升级和 Codex runtime smoke 尚未执行，不能宣称对应场景已关闭。

### IA-13：现有体积闸无法防止不可理解文本

证据：

- `skill_metadata_check.py` 只要求单个 `SKILL.md` 不超过 500 行。
- 根 `AGENTS.md` 没有体积、职责数量、交叉引用有效性或重复契约检查。
- 当前所有高频 Skill 都远低于 500 行，因此现有闸不会对本次问题报警。

影响：闸只能阻止极端膨胀，不能阻止一个文件承担过多职责、规则重复或引用失效。

关闭条件：新增职责与入口预算、失效引用检查、退役术语检查和重复事实源审计；体积只作预警信号，不能替代语义审计。

真实工厂解决与收据（2026-08-30）：

- 没有新增 Hook。现有 `skill_metadata_check.py`、`instruction_source_check.py` 与 `project_structure_check.py` 分别继续拥有 Skill、原生指令和文档结构边界，避免增加第四个信息架构运行部件。
- 对19个当前 Skill 入口实测后，将入口硬预算从宽泛的500行收紧为120行，并增加最多8个 H2 章节的职责代理指标；当前基线为41～104行、最多8个 H2。入口与一层 reference 之间完全相同且不少于80字符的长段落会被阻断。
- Skill 入口、已链接 reference、根与嵌套 `AGENTS.md` 均禁止重新引用已退役的 `skill-routing.json`、Clarify / Focus 自动 Hook 和项目 Memory writer / index / lint 运行时资产；`AGENTS.md` 指向的受管索引与 Hook 信号文档必须真实存在。
- `project_structure_check.py` 现在验证活动文档的本地文件链接；`doc/4_archive/**` 和冻结的 `refactored-project` 证据镜像不追溯改写，目录型布局示例不冒充必须存在的文件。首次实跑发现并修复 `memory-scoring-design.md` 指向已移动辩论记录的真实断链。
- `doc/README.md` 不再复制 IA 子项进度，只链接 IA 总账；该单一事实源边界已有专项回归。跨文件语义改写后的近义重复仍不能由确定性 Hook 可靠判断，继续由专项 owner 测试和 IA-14 用户审阅承担。
- 新硬闸定向回归 46/46；完整自动测试共347项（346通过、1项既有跳过），下游 fixture 4/4；三项 dogfood Hook、manifest `--check` 与 `git diff --check` 均通过。项目结构只保留既有 archive advisory，没有活动文档错误。
- 本项已完成真实工厂源码、Template 与 dogfood 的确定性防复发闭环；真实业务下游升级和 Codex runtime smoke 尚未执行，不能宣称对应场景已关闭。

### IA-14：缺少真实用户理解度与运行时行为验收

证据：

- 既有 AGENTS 重组验收重点是标题迁移、ownership、fixture、回滚和高定制项目安全。
- 当前没有“用户能否在短时间内说清机制”“Agent 是否稳定遵循精简指令”“错误结果是否直接可行动”的验收记录。

影响：机器验证可以全部通过，产品仍可能难懂、啰嗦或依赖专家解释。

关闭条件：增加新用户阅读试验、典型任务运行时 smoke、错误结果可行动性检查和精简前后上下文/行为对比；必须区分自动测试、runtime smoke 和用户试用收据。

当前验收口径、测试样本和逐项收据见 [`IA-14-user-runtime-acceptance.md`](IA-14-user-runtime-acceptance.md)。P0 首次运行发现并修正测量器把 SessionStart Show State 混算为每轮 prompt 上下文的问题；当前确定性基线为每轮 0 字符、SessionStart 固定夹具 80 字符。盲读代理与用户结果可行动性均为 5/5；首轮 12 组 runtime 严格统计为 7 组通过、2 组安全替代通过、3 组失败。1.5.23 安装复测确认 R6 2/2、R2 0/2，随后把自然语言范围硬闸上移到公共 AGENTS。1.5.25 已发布并受管安装，R2 / R6 最终 runtime 均为 2/2；用户最终试用和真实下游仍未执行。

## 自上而下的解决顺序

后续任何实施必须按以下阶段推进；上一层事实源未稳定前，禁止先优化下一层文案。

### 阶段 0：定义目标信息架构

- 产出四类受众的信息承载矩阵。
- 为每类契约指定唯一事实源、允许的投影和禁止的重复。
- 定义“当前设计、历史记录、运行时指令、用户结果”的边界。

### 阶段 1：恢复当前事实与有效路由

- 处理 IA-01、IA-03、IA-04、IA-11。
- 先消除失效引用、伪契约和重复事实源，再改写正文。
- 对 `skill-routing.json` 作保留并接入或退役的明确架构决定。

### 阶段 2：收缩常驻运行时层

- 处理 IA-02、IA-05、IA-08。
- 重构 Template `AGENTS.md`，同步工厂 dogfood。
- 量化 SessionStart/UserPromptSubmit 注入，去除重复解释层。

### 阶段 3：重构按需流程层

- 处理 IA-06。
- 先统一跨 Skill 契约，再对高频 Skill 做渐进加载。
- 每个 Skill 先保证一条清晰主路径，再补异常分支。

### 阶段 4：建立稳定用户界面

- 处理 IA-07、IA-09、IA-12。
- 程序结果、人类结果和内部技术收据分层。
- 重写 README / INSTALL 的读者路径和术语入口。

### 阶段 5：重建文档导航与生命周期

- 处理 IA-10、IA-11。
- `doc/README.md` 先导航当前工作，再承载完整索引。
- 对账 validating、completed、superseded 和 archive。

### 阶段 6：建立防复发与真实验收

- 处理 IA-13、IA-14。
- 增加静态闸、上下文成本基线、runtime smoke 和用户试用。
- 完成后再决定合理的行数或字符预算，禁止先拍脑袋设数字。

## 全局非目标

- 不以删字数为目标削弱安全、回滚、ownership 或证据语义。
- 不因为机器文件很长就重写 `managed-skeleton.json` 等生成合同。
- 不在总架构未确认前逐文件润色。
- 不把历史记录改写成从未发生；只做状态、替代关系和阅读路径治理。
- 不自动 commit 或 push。

## 待验证项

1. 新路由责任链在 Codex 真实会话中是否按默认主对话与 Skill 显式角色委派运行。
2. 根 AGENTS、Skill 和各 Hook 注入在真实会话中的 token / 字符成本。
3. 精简或移动规则后，对 Agent 行为的实际影响。
4. 用户直接运行同步器 CLI 的真实频率和期望输出。
5. 典型下游项目的项目专区与嵌套 AGENTS 是否存在同类问题。
6. 旧交付文档中还有多少状态、章节和已退役机制漂移。

## 总 Bug 关闭证据

本 Bug 关闭时必须分别提供：

1. **源码**：运行时载体、renderer、检查器和测试的实际改动。
2. **产品传播**：Template、shared skills、manifest、VERSION 与 CHANGELOG 对账。
3. **dogfood**：工厂根 AGENTS、`.codex/**` 与 Template 一致性收据。
4. **fixture**：完整下游 fixture 与针对性回归。
5. **真实下游**：至少一个低定制和一个高定制项目的零损失更新证据。
6. **runtime / 用户试用**：新会话指令行为、用户结果和阅读理解度现场收据。

任一类别缺失必须明确标为未验证，禁止用自动测试代替 runtime 或用户试用。

## 当前方案草案

- [`proposal/README.md`](proposal/README.md)：AGENTS 信息架构第十一版历史实施基线；已由 `refactored-project` 与真实工厂 `1.5.13` 取代，不参与运行时，也不再作为当前发布门。
- [`project-memory-retirement-ledger.md`](project-memory-retirement-ledger.md)：项目 `.codex/memory/` 24 个资产的 P0 逐文件迁移账本；24/24 已审核，P1 迁移已授权，删除仍未授权。
- [`../../1_delivery/project-memory-retirement/requirements_2026-08-30_project-memory-retirement.md`](../../1_delivery/project-memory-retirement/requirements_2026-08-30_project-memory-retirement.md)：下游通用的程序扫描、Agent 语义审核、用户逐项确认、受控迁移与独立清理授权合同。

### 2026-08-27 迭代中断进度

- 共落盘 11 个 debate 版本；V1 至 V10 已形成结论，V11 在独立复评期间中断。
- V11 最后发现快速命令子串检查可被尾缀污染；候选修补已写入，但完整验证和双评审均未完成。
- 用户明确要求停止讨论；不建立 V12，不把当前 proposal 冒充已验收或已安装骨架。
- 后续若重启，必须先把验收从“找不到任何未来反例”改成固定语义清单、固定反例集、固定独立评审轮次和明确剩余风险。

### 2026-08-28 V11 定向修订

- 用户暂定 V11 为收口基线，并确认采用根级目录读取路由、三个工厂嵌套 `AGENTS.md` 与 Hook/测试硬闸。
- `scripts/AGENTS.md`、`skills/AGENTS.md` 和 `doc/2_bugs/AGENTS.md` 保留工厂目录专属红线；从项目根启动时由根路由要求主动读取，不冒充 Codex 原生动态加载。
- `.codex/rules/*.rules` 只用于命令权限，不承载目录语义；当前工厂嵌套内容不原样下发普通下游。
- 新增确认的产品缺口：StratusAgent 与 CausisRiskSuite 仍保留 Claude 式 `.codex/rules/*.md + paths:`，当前只靠根索引软路由；同步器必须提供逐文件无损迁移，`$summary` 也必须停止生成该旧机制。BridgePersonalAssist 当前没有旧 Rule 目录。
- 当前修订不代表 V11 已通过独立评审，也不代表真实骨架、下游或 runtime 已安装验证。

## 已确认设计结论

### D-01：项目操作指南并入根 README

- 原方案中的“文档 4：Codex 项目操作指南”不再作为独立文档长期存在。
- BridgeForgeCodex 根 `README.md` 保留工厂自身的产品介绍、安装方式和仓库说明，同时承载一段由 BridgeForge 管理的公共协作说明。
- 下游项目根 `README.md` 在项目自有内容之后承载同一段公共协作说明，通常位于文档后半部分。
- 公共协作说明使用明确的 `BRIDGEFORGE:README:BEGIN/END` 标记；工厂与下游公共区必须逐字一致，更新器只能修改标记内部。
- 标记外内容归各项目所有，更新器不得覆盖；`doc/README.md` 继续只负责项目文档索引，不替代根 README。
- proposal 中独立的 `shared-docs/codex-project-operating-guide.md` 已移除，内容已经并入 `readme/bridgeforge-public-section.md` 草案。

## 重构项目完整镜像

- [`refactored-project/`](refactored-project/) 是本 Bug 的完整候选项目，基线为提交 `53722fc845f6d6934f4c7d88fec36257171392b5`。
- 后续重构先在该镜像内完成，用户手动查看并确认前，不替换仓库根目录的真实运行资产。
- 镜像包含基线提交中的全部 Git 跟踪文件，不包含 `.git`、`.venv`、缓存或运行时临时文件，也不递归复制自身。

### 2026-08-28：卡点 #1 候选镜像已解决

- Hook 信号说明已作为 `codex.doc.hook-signals` 公共 whole 资产进入 Template、dogfood 镜像、文档索引和 managed contract；不放入 `.codex/hooks/AGENTS.md` 冒充 Hook 触发时动态加载的指令。
- 新 whole 资产遇到旧合同没有整文件所有权的不同内容同名文件时，计划必须列为 `risk replace`；只有正确 plan fingerprint 与 `--confirmed-risk` 同时存在才允许事务替换。
- 同内容目标安全接管且不重写；旧合同已拥有但现场漂移继续走原 drift blocker；merge、region、seed 各自语义未改变，转 whole 的跨策略边界已受保护。
- 候选同步测试 55 项、下游 fixture 3 项、结构检查、确定性重建检查、proposal contract 与 `git diff --check` 均通过。
- 本结论只关闭候选镜像中的卡点 #1；真实根骨架、真实下游、runtime、版本发布和用户试用仍未执行。
- 需求与收据见 [`requirements_2026-08-28_hook-signal-doc-packaging.md`](../../1_delivery/agents-md-simplification-review/requirements_2026-08-28_hook-signal-doc-packaging.md)。

### 2026-08-29：卡点 #2 被 #1 吸收并关闭

- #1 的确定性重建已同步 Template 与 dogfood managed contract，修正 `focus_reminder.py` 旧 hash，并登记 `codex.doc.hook-signals`。
- 重建器 `--check` 为 `unchanged`，proposal contract 通过；#2 不再需要独立实现或重复验证。

### 2026-08-29：卡点 #3 独立收口通过

- 新独立 `review-auditor` 首轮发现 1 个 P1：旧 `merge/region/seed` 路径转 `whole` 时只按 target 判断，会误列 safe 并删除项目自有内容。
- 同步器已改为核对旧资产是否真正 whole-owned；三种跨策略转换必须列为 risk，未确认时零写，确认后事务替换并二次 no-op。
- 补齐真实 CLI 的 plan、未确认拒绝、confirmed apply 与 no-op 回归；候选同步测试增至 55 项。
- 同一独立审计者有限复审结论为 `PASS`：无 P0/P1，允许进入 IA-01～IA-14；真实根、真实下游、runtime、release 和用户试读仍未验证。

## 关联记录

- `doc/1_delivery/codex-agents-structure-reorganization/requirements_2026-08-16_codex-agents-structure-reorganization.md`
- `doc/4_archive/delivery/codex-skill-routing-dispatch/requirements_2026-07-15_codex-skill-routing-dispatch.md`
- `doc/1_delivery/skill-runtime-efficiency/requirements_2026-08-15_skill-runtime-efficiency.md`
- `doc/4_archive/delivery/bridgeforge-command-clarity/requirements_2026-07-08_bridgeforge-command-clarity.md`
- `doc/0_architecture/design/codex-native-instruction-architecture.md`
- `doc/0_architecture/design/design-rationale.md`
- `doc/0_architecture/design/codex-project-sync.md`
