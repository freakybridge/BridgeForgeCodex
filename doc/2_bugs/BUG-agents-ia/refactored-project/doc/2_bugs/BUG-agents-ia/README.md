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
| IA-01 | P0 | 活动说明包含已退役机制和失效章节引用 | 已核实 |
| IA-02 | P0 | 四类受众缺少明确的信息承载合同 | 已核实 |
| IA-03 | P0 | `skill-routing.json` 的运行时效力不明确 | 部分核实，需 runtime 验证 |
| IA-04 | P0 | 同一行为契约存在多份手写事实源 | 已核实 |
| IA-05 | P1 | 根 `AGENTS.md` 常驻内容过载 | 已核实 |
| IA-06 | P1 | 高频 Skill 入口承担过多低频细节 | 已核实 |
| IA-07 | P1 | 用户结果依赖 Agent 临场翻译机器收据 | 已核实 |
| IA-08 | P1 | Hook 信号与常驻规则形成重复上下文 | 已核实，成本待量化 |
| IA-09 | P1 | README 和安装说明缺少白话产品心智 | 已核实 |
| IA-10 | P1 | 文档索引不能回答“当前先读什么” | 已核实 |
| IA-11 | P1 | 历史交付状态、当前设计和索引没有可靠收口 | 已核实 |
| IA-12 | P2 | 用户可见术语、语言和状态命名不统一 | 已核实 |
| IA-13 | P2 | 现有体积闸无法防止不可理解文本 | 已核实 |
| IA-14 | P2 | 缺少真实用户理解度与运行时行为验收 | 已核实 |

## 逐项证据、影响与关闭条件

### IA-01：活动说明包含已退役机制和失效章节引用

证据：

- `doc/0_architecture/design/codex-native-instruction-architecture.md` 仍称旧无 marker 项目由 `section_layout` 迁移。
- 当前源码中没有 `section_layout` 实现；测试反而要求当前 asset 不含 `managed_blocks` 或 `section_layout`。
- `templates/hooks/focus_reminder.py` 仍引用 `AGENTS.md §9.6`，当前章节实际为 §4.5。

影响：维护者和 Agent 可能按不存在的机制判断问题；长文档一旦失真，用户无法判断哪份说明可信。

关闭条件：所有“当前设计”只描述当前实现；所有章节、文件、命令和字段引用可由静态检查定位到真实目标；历史机制只保留在历史交付、Bug 或 archive 中。

### IA-02：四类受众缺少明确的信息承载合同

证据：

- `templates/AGENTS.md` 同时包含执行红线、骨架原理、Memory 分类、Skill 清单、换机步骤和 Debug 流程。
- `skills/bridgeforge-codex/SKILL.md` 同时包含 bootstrap、运行时 preflight、迁移机制、事务设计、用户话术和内部收据。
- README 的快速开始段落直接解释版本分流、schema、`PreservationManifest` 和公共漂移。

影响：任何读者都被迫阅读不属于自己的细节；文件虽按类型分开，信息仍无法形成清晰入口。

关闭条件：建立“受众 × 信息类型 × 唯一载体”矩阵；每类信息只有一个主事实源，其余载体只保留必要投影或链接。

### IA-03：`skill-routing.json` 的运行时效力不明确

证据：

- 文件自称 instruction contract，并要求 root agent 对非 main 阶段显式启动指定 Agent。
- 仓库内只发现它被同步、结构校验和测试读取，未发现运行时 dispatcher 或 Hook 消费者。
- 根 AGENTS 只写“路由明确要求”时启动子 Agent，没有要求 Agent 先读取该 JSON。

影响：它可能只是“可校验但不可执行”的伪契约；实际调度继续依赖 Skill 重复描述或模型猜测。

待验证：Codex 当前运行时是否存在仓库外的原生隐式加载机制；本次没有 runtime 收据。

关闭条件：二选一并形成单一事实源：要么由真实运行时确定性消费并取得 smoke 收据，要么删除伪运行时定位，将必要调度契约放回原生可加载载体。

### IA-04：同一行为契约存在多份手写事实源

证据：

- `confirm` 与 `develop` 重复维护 S/M/L 规模、时间、token、Agent 和验证预算。
- Agent 职责同时出现在根 AGENTS、各 Skill 和 `skill-routing.json`。
- 文档边界同时出现在 AGENTS、README、Template `doc/README.md` 和 operating guide。

影响：修改一处容易漏掉其他位置；静态镜像一致不等于自然语言语义一致。

关闭条件：逐个契约指定唯一 owner；其他载体只引用、读取或机械生成，不再复制完整规则。

### IA-05：根 `AGENTS.md` 常驻内容过载

证据：

- Template `AGENTS.md` 为 165 行，工厂根文件为 188 行。
- Template 中“必须 / 禁止 / 不得 / 只能 / 只允许”等硬规则语句约 40 行；工厂根文件约 56 行。
- Memory、文档目录、Skill 入口、换机流程、clarify、focus、Debug 和审计规则全部每轮加载。

影响：Agent 需要从大量制度文字中重新识别当前任务真正相关的约束；人类阅读时缺少从目标到规则的路径。

关闭条件：根文件只保留所有任务确实需要常驻的少量行为和安全红线；低频流程、原理、参数、命令及例外下沉；项目专区只保留对执行真正必要的项目事实。

### IA-06：高频 Skill 入口承担过多低频细节

证据：

- `bridgeforge-codex/SKILL.md` 210 行、`summary/SKILL.md` 178 行、`develop/SKILL.md` 113 行、`confirm/SKILL.md` 102 行。
- `bridgeforge-codex` 主文件已存在 `references/`，但大量模式分支、Native Memory 状态和内部收据仍留在入口。
- `summary` 已有 `references/deep-steps.md`，主文件仍保留完整路由、metadata、topic 生命周期和用户级 memory 规则。

影响：每次调用都加载低频分支；主路径被异常分支和治理细节淹没。

关闭条件：入口只保留定位、主路径、选择点、停止条件和用户结果；模式细节、schema、内部收据和低频故障进入按需 references；跨 Skill 的共享契约只有一个 owner。

### IA-07：用户结果依赖 Agent 临场翻译机器收据

证据：

- 项目同步器默认只打印包含 `safe`、`risk`、`gaps`、`aggregate_fingerprint` 等字段的 JSON。
- `bridgeforge-codex` Skill 用独立章节要求 Agent 不得把这些术语直接展示给用户，并临场改写为白话。
- 失败路径同时输出 JSON 和英文 `BLOCKED` stderr。

影响：同一程序结果可能因 Agent、模型或上下文不同而获得不同解释；用户直接运行脚本时没有稳定的人类界面。

关闭条件：机器 JSON 与人类结果明确分层；程序或确定性 renderer 直接输出“发生了什么、改了什么、为什么停、下一步需要什么”，Agent 只做补充说明，不承担唯一翻译职责。

### IA-08：Hook 信号与常驻规则形成重复上下文

证据：

- `clarify` 与 `focus` 的完整响应规则常驻 AGENTS，Hook 又在用户消息后注入信号和指针。
- SessionStart 和 UserPromptSubmit 还会注入 memory、Git 状态、snapshot、archive 等状态块。
- Hook 注释承认重复注入会产生 token 噪声，但当前方案仍依赖较长的常驻章节解释信号。

影响：上下文中同时存在规则正文、信号和状态收据；真实成本与行为收益没有统一量化。

关闭条件：为每类信号指定唯一解释位置；建立注入字符/token、触发频率、误触发率和行为收益基线；删除没有可测收益的重复层。

### IA-09：README 和安装说明缺少白话产品心智

证据：

- README 快速开始后立即出现产品 home、薄入口、版本阈值、schema、`PreservationManifest`、公共漂移和 ledger 等内部术语。
- INSTALL 同时面向新用户、旧版本迁移、版本戳诊断和维护者，未提供按读者分流的入口。

影响：用户知道命令，却难以回答“这个产品平时替我做什么、哪些内容归我、什么时候会询问、失败后是否改了盘”。

关闭条件：README 首屏用白话建立产品心智和最短使用路径；安装、迁移、维护者协议分流；术语首次出现必须解释或链接词汇表。

### IA-10：文档索引不能回答“当前先读什么”

证据：

- `doc/README.md` 用约 50 条高密度 Delivery topic 平铺历史演进。
- 活跃架构、当前交付、待解决 Bug、已完成历史和维护者手册没有形成任务导向入口。
- Template `doc/README.md` 主要是注释占位和目录样例，没有“当前状态 / 从这里开始”区。

影响：索引完成了登记，却没有完成导航；用户需要先理解项目历史才能找到当前真相。

关闭条件：唯一索引顶部先回答当前架构入口、当前交付、开放 Bug、常用操作和历史归档；完整清单可以保留，但不得抢占第一阅读路径。

### IA-11：历史交付状态、当前设计和索引没有可靠收口

证据：

- `codex-agents-structure-reorganization` 需求卡仍为 `status: validating`，正文却已有多轮实施记录。
- 该事项未出现在当前项目 memory 冷热索引中。
- 当前设计文档与后续 current-only 交付发生冲突，但仍同时列为活动架构资料。

影响：无法仅从状态和索引判断事项是否完成、被替代或仍需继续；历史决定继续污染当前说明。

关闭条件：每项交付有可对账的 active/completed/superseded 状态；当前架构只引用当前事实；历史记录进入 topic、Bug 或 archive，并保留明确替代关系。

### IA-12：用户可见术语、语言和状态命名不统一

证据：

- 用户文档和错误输出混用 ownership、dogfood、fail-closed、gap、blocker、readiness、fingerprint、schema、current-only 等术语。
- Hook 和脚本输出同时存在中文、英文和中英混排；相似状态使用 `ready`、`completed`、`planned`、`blocked`、`degraded` 等多个维度。

影响：用户需要先理解内部数据模型，才能判断结果；不同入口对同一状态的说法不一致。

关闭条件：定义内部状态模型与用户状态词的稳定映射；用户结果统一使用简体中文和少量固定词；内部英文标识只在技术收据中出现。

### IA-13：现有体积闸无法防止不可理解文本

证据：

- `skill_metadata_check.py` 只要求单个 `SKILL.md` 不超过 500 行。
- 根 `AGENTS.md` 没有体积、职责数量、交叉引用有效性或重复契约检查。
- 当前所有高频 Skill 都远低于 500 行，因此现有闸不会对本次问题报警。

影响：闸只能阻止极端膨胀，不能阻止一个文件承担过多职责、规则重复或引用失效。

关闭条件：新增职责与入口预算、失效引用检查、退役术语检查和重复事实源审计；体积只作预警信号，不能替代语义审计。

### IA-14：缺少真实用户理解度与运行时行为验收

证据：

- 既有 AGENTS 重组验收重点是标题迁移、ownership、fixture、回滚和高定制项目安全。
- 当前没有“用户能否在短时间内说清机制”“Agent 是否稳定遵循精简指令”“错误结果是否直接可行动”的验收记录。

影响：机器验证可以全部通过，产品仍可能难懂、啰嗦或依赖专家解释。

关闭条件：增加新用户阅读试验、典型任务运行时 smoke、错误结果可行动性检查和精简前后上下文/行为对比；必须区分自动测试、runtime smoke 和用户试用收据。

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

1. Codex 是否以仓库外机制隐式加载 `.codex/skill-routing.json`。
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

- [`proposal/README.md`](proposal/README.md)：AGENTS 信息架构、工厂与 Template 第十一版候选、根级目录路由、三份工厂嵌套指令、README 公共区、逐条语义合同和可执行机器合同。当前不参与运行时，也不是最终通过版。
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

## 关联记录

- `doc/1_delivery/codex-agents-structure-reorganization/requirements_2026-08-16_codex-agents-structure-reorganization.md`
- `doc/1_delivery/codex-skill-routing-dispatch/requirements_2026-07-15_codex-skill-routing-dispatch.md`
- `doc/1_delivery/skill-runtime-efficiency/requirements_2026-08-15_skill-runtime-efficiency.md`
- `doc/1_delivery/bridgeforge-command-clarity/requirements_2026-07-08_bridgeforge-command-clarity.md`
- `doc/0_architecture/design/codex-native-instruction-architecture.md`
- `doc/0_architecture/design/design-rationale.md`
- `doc/0_architecture/design/codex-project-sync.md`
