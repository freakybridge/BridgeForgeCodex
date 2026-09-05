<!-- BRIDGEFORGE:PUBLIC:BEGIN -->
## BridgeForge 公共区

> 本区由 bridgeforge-codex 管理。项目定制写入文末“项目级专区”或项目自有的嵌套 AGENTS.md，不直接修改公共区。

## 1. 授权与协作

- 在用户授权范围内完成任务。需求确认、实施、提交、推送和发布是不同授权，不相互推定。
- 已确认的目标、决定、授权和预算跨步骤、Skill 与 Agent 继承。缺少记录时整理已有决定，不重新访谈，也不因缺卡重复索取授权。
- 只在未决事项会实质改变范围、行为、验收或风险时，问一个关键问题；暂停依赖该答案的动作，其余已授权工作继续。
- 只读调查不得写文件、安装、修复、运行完整回归或产生其他未经授权的副作用。可查事实由 Agent 做必要、有限的核验。
- 暂停时说明原因、受影响动作和恢复条件。若 Skill 导致暂停或改变路径，链接实际读取的 SKILL.md，引用触发条款并说明适用关系。条件满足后从卡点恢复。
- 用户仅授权当前项目时，不修改上游模板或其他项目。反哺上游前说明传播影响、收益和风险，并取得相应授权。
- 用户明确指令和有效授权优先于 Skill 默认流程；平台权限与安全边界始终有效。

## 2. 修改与证据

- 修改前核实与本次行为相关的调用方、配置入口、数据流和失败路径，调查深度与影响范围相称；不得只凭文件名或单条报错猜根因。
- 重写、移植或替换应保持未经用户授权改变的功能、配置、数据链和错误语义。不得用 TODO、临时绕过或上层内联掩盖缺失能力。
- 使用当前实际可用的工具查找和读取资料，限定路径与输出范围，优先使用只读能力，避免无差别扫描。
- 不编造文件、接口、字段、配置或执行结果。当前任务中已核实且仍有效的证据可以复用；现场变化、证据矛盾或工具输出异常时，做针对性复核。
- 发现自己的结论或操作错误时，明确更正并核验受影响部分。
- 交付和高风险结论必须有实际证据。声称验证通过时，说明命令或收据、验证断言及覆盖场景；未取得证据的部分明确标为未验证。
- 审查先列问题，按严重度排序，给出文件位置、行为风险和依据。架构判断给出推荐、主要取舍及适用条件。
- 交付说明实际完成的内容、验证结果和剩余风险，不将已有改动归为本轮成果。

## 3. 工具与运行环境

- BridgeForge 工具与 Hook 使用受管 Rust workspace 和锁定的 Cargo.lock。工具链缺失、版本不足、锁文件漂移或构建失败时停止相关动作，不回退到 Python、脚本包装器或其他未受管入口。
- 下游业务代码遵守项目自己的语言、环境和依赖规范。
- 不通过 shell 字符串中转写入或动态执行非 ASCII 正文；配置、Hook 和入口脚本的编码与注册遵守项目硬闸。
- Windows GUI、Hook 和后台任务启动的非交互命令应使用可验证的无可见控制台入口，并保持输入输出、退出码和超时语义。只有用户明确要求时才打开可见交互窗口。
- 换机、重装或新机 clone 时，按项目快速命令恢复依赖和 Rust/Cargo，再调用 bridgeforge-codex 核验骨架与 Hook；不猜测仓库地址或照抄占位命令。

## 4. 信息放置

只保留必要的常驻约束和入口。新增或修改信息时，选择一个主要事实源；其他位置只保留链接、机械生成内容或执行所需的短摘要。

| 内容 | 承载位置 |
|---|---|
| 全项目约束与必要入口 | 根 AGENTS.md |
| 目录专属约束 | 相应目录的嵌套 AGENTS.md |
| 可重复的操作流程 | 对应 Skill |
| 机器合同 | manifest、schema 或结构化配置 |
| 可自动判定的检查 | Hook、pre-commit 和测试 |
| 设计原因、迁移说明、案例与长流程 | doc/0_architecture/ 或 doc/3_reference/ |
| 项目知识与资料 | doc/5_project_knowledgebase/ |
| 产品用途和最短使用路径 | 根 README.md |
| 安装与迁移步骤 | INSTALL.md |
| 本次选择、结果和下一步 | 用户可直接判断的交付输出 |

- AGENTS 的原生发现依赖启动目录及目录层级。目录索引用于发现，不代表处理某个文件时一定会动态加载对应指令。
- .codex/rules/*.rules 用于 Codex 命令执行策略；Markdown 的 paths 声明不是 Codex 原生指令加载机制。
- 可用 Skill 以当前会话实际发现结果为准，不在根 AGENTS.md 维护 Skill 名单。
- 简单 Skill 保持单文件。复杂 Skill 的入口保留主路径、选择点、停止条件和明确的 reference 读取条件，不与 reference 重复维护完整规则。

## 5. 文档与资产边界

- 项目文档使用 doc/0_architecture、1_delivery、2_bugs、3_reference、4_archive、5_project_knowledgebase 六层结构，不擅自新增、删除、改名或合并同级层。
- doc/README.md 是文档唯一索引，delivery_layout 决定交付路径。新增、删除、移动或重命名文档时同步索引。
- 项目知识资料归项目所有；骨架升级不覆盖或删除正文，不因日期或完成状态自动归档。
- 测试代码放在 scripts/tests/**。
- 下游业务版本以根 VERSION 为事实源，相关原生 manifest 由项目发布流程同步。
- 骨架版本只记录在 .codex/.bridgeforge_codex_version，由统一项目同步器维护，不作为业务版本。
- Codex 原生 Memory 保留官方生成和注入机制。BridgeForge 对其内容只读；跨电脑同步将其视为不透明整树快照，不依赖内部语义。
- 不新建或继续使用项目 .codex/memory/，不恢复其读取、索引、写入或统计链。
- 既有 legacy Rule / Memory 只通过 bridgeforge-codex 的受控迁移流程处理。保留逐源确认、未确认零写入和事务回滚要求；具体分类、固定退役项与执行步骤由该 Skill 及迁移手册维护。
- 阶段总结或验收收口遵循 summary Skill。总结、验收与采纳规则建议分别判断，不因总结或验收自动修改 AGENTS、Rule、Hook 或 Memory。
- 文档布局以 doc/README.md 为准；归档已完成事项时使用 archive-scan。

## 6. Agent 分工

- Skill 和子 Agent 默认继承当前会话的 model / effort，不擅自锁定。
- 主对话负责沟通、授权、整合和交付；子 Agent 只完成分配任务，不代替用户决定，也不重复已有工作。
- 没有用户或适用指令的显式委派时，由主对话执行。需要委派时，点名已存在的 Agent 角色。
- bridgeforge-codex、create-worktree 和 git-sync 始终由主对话执行；git-sync 使用当前项目提供的受管同步入口。

## 7. 排障与验证

- 修 Bug 时区分事实和假说，核实数据源、用户路径、边界条件及外部副作用。根因未确认时说明置信度，不先加兜底或跨层连带修改。
- 用户报告“慢、难用、不清楚”等体验且没有稳定复现时，先取得可观察证据和触发条件，再决定验证方法，不猜修。
- 性能优化先建立基线，再用同一方法复测；完成后清理临时诊断工具。
- 跨两个以上陌生模块的任务先调研。等价性验收、重写或移植完成后进行独立验证，不只依赖实现者自测。
- 同一问题连续两次实质修复失败后停止盲修，整理证据并进入 escalate 或 debate。权限重试、输出故障和未改代码的重测不计次数；用户明确要求外援时立即响应。
- 审计包含本轮 Agent 自己的改动，且用户要求审计、复核需求或找遗漏时，启动独立 Agent 二次审计。普通解释和轻量自查不强制。

<!-- BRIDGEFORGE:PUBLIC:END -->

<!-- BRIDGEFORGE:PROJECT:BEGIN -->
## 项目级专区

> 本区由项目完全所有。bridgeforge-codex 更新时必须逐字保留，不得覆盖、删除、吸收或重新格式化。

### 项目架构红线

- bridgeforge-codex 同时是 Codex 协作骨架工厂与 dogfood 样板；公共 AGENTS 以 `templates/AGENTS.md` 为单一事实源，工厂专属约束只留在本节。
- 任何改动落地前必须明确回答传播四问：属于产品层、自身配置层还是元文档；是通用能力还是工厂专属；是否需要版本与 CHANGELOG；是否需要同步自身 dogfood。
- 通用改进必须进入 `templates/` 或共享 `skills/` 并同步自身镜像；工厂专属事实禁止下沉污染 Template。
- 上游到下游与下游反哺上游的边界分别以 `doc/0_architecture/design/sync-from-upstream-playbook.md` 和 `doc/0_architecture/design/reverse-sync-playbook.md` 为准。
- 禁止对本仓库执行下游 `project_sync adopt/apply`，禁止写入 `.bridgeforge_codex_version`；工厂合规只能由 dogfood 一致性硬闸证明。
- 本项目的模块职责、依赖方向、数据流和外部副作用边界必须与本文件“项目目录地图”一致。
- 产品层改动必须 bump 根 `VERSION` 并在 `CHANGELOG.md` 标记 `[product]`；自身配置与元文档分别标记 `[repo]` / `[meta]`。
- 受管资产必须使用显式 target、稳定 asset id、可验证历史 hash 和单一 ownership strategy；禁止 glob ownership。
- safe/risk/gap 计划必须在 apply 前重算 aggregate fingerprint；漂移时禁止写入。gap 必须原样保留并降级收据。
- Skill 或 Agent 增删改名必须同步分发登记、dogfood 与引用校验；禁止维护第二份角色路由表。
- `--check` / `--dry-run` 必须零写入；`init/adopt/update` 必须只经受管 `bridgeforge project-sync` apply。
- Bug 关闭必须分别记录源码、产品传播、dogfood、fixture、真实下游与 runtime 六类证据；缺失项必须标为未验证。
- 发布前必须通过 factory dogfood、skill metadata、manifest `--check`、project structure、mirror drift、完整 fixture、完整自动测试与独立审计。
- 未执行真实下游或 runtime smoke 时，禁止宣称对应验证已通过。

### 项目业务与安全红线

- 工厂不得把下游业务约束吸收到公共 Template；真实下游写入只允许发生在用户明确授权的测试 worktree。

### 项目目录地图

- `templates/**`：下沉到项目的 Codex 公共骨架与公共 `AGENTS.md` 源。
- `skills/**`：由用户级分发器安装的通用 Skill 与 bridgeforge-codex 入口。
- `scripts/**`：工厂同步、迁移、manifest 重建器及 `scripts/tests/**` 自动测试。
- `.codex/**`：本仓库 dogfood 镜像与本仓库配置；legacy 项目 Memory 仅在迁移完成前保留，不再作为运行时设施。
- `doc/**`、`README.md`、`CHANGELOG.md`：架构、交付、Bug、参考资料与发布说明。

原生 Memory 跨电脑同步的工厂架构事实源为 `doc/0_architecture/design/codex-native-memory-sync.md`。

### 项目快速命令

```powershell
cargo build --locked --release --manifest-path .codex/hooks/Cargo.toml
cargo test --locked --config scripts/tests/factory-cargo.toml --manifest-path .codex/hooks/Cargo.toml --workspace
cargo test --locked --manifest-path scripts/tests/Cargo.toml
.codex\bin\bridgeforge.exe manifest --root . --check
.codex\bin\bridgeforge.exe check factory-version --root .
```

### 目录级 AGENTS 索引

- 当前没有项目自有的嵌套 `AGENTS.md`；新增时必须登记其适用目录与职责。
<!-- BRIDGEFORGE:PROJECT:END -->
