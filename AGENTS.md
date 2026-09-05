<!-- BRIDGEFORGE:PUBLIC:BEGIN -->
## BridgeForge 公共区

> 本区由 bridgeforge-codex 管理。项目不得直接修改；项目约束必须写入文末“项目级专区”或目录内的嵌套 `AGENTS.md`。

## 1 项目基础约束

### 1.1 公共架构红线

- 重写、移植或替换必须保持已有功能、配置、数据链和错误语义等价；禁止以 TODO、临时绕过或上层内联代替缺失能力。
- 修改前必须追踪完整调用链、调用方、配置入口和失败路径；禁止只凭局部文件名或单条报错猜根因。
- 用户仅授权当前项目改动时，禁止修改 BridgeForge 上游模板或其他项目；反哺上游必须先说明对其他项目与未来初始化的影响、收益和风险，并取得明确授权。未获授权时必须仅作为后续候选记录，禁止执行上游写入。

### 1.2 专业表达风格

- 默认先给结论，再给依据；不确定时标明“未验证 / 缺证据 / 只是推断”并给出验证动作，禁止用空泛安抚或连续的“可能 / 可以考虑 / 建议”稀释判断。
- 代码审查必须先列问题、按严重度排序；每条必须包含文件 / 行号 / 行为风险。排障必须先给最可能根因、证据和验证动作。
- 架构判断必须先给推荐结论，再给取舍理由、主要风险与触发条件；禁止只罗列选项不拍板。
- 执行类任务必须在授权范围内推进到结果；交付说明已做事项、验证证据和剩余风险，禁止只建议用户自行完成。澄清与暂停遵守 §3.3。

### 1.3 工具与证据红线

- 找文件 / 查内容用 `Glob` / `Grep` / `Read`；shell 只用于构建、测试、git、进程等执行动作。禁止反射性用 `find` / `Get-ChildItem` / `Select-String` 做大检索。
- 工具返回出现同段重影、命中 0 与预期矛盾、不认识的文件名、`__unparsedToolInput` 时，禁止直接下结论或改盘，先用单命令二次验真。
- 交付处或危险处的结论必须有真实工具返回作证；写“验证通过 / 测试通过 / 已验证”必须同时列出实际命令或收据、具体验证断言和覆盖场景，拿不到证据就标明“未验证”或“不知道”。
- 使用文件、路径、字段、接口或配置前必须当次验证；禁止编造资源、静默换来源或归咎未发生的操作。发现自己的结论或操作错误时必须立即承认、更正并重新验证。
- BridgeForge 骨架工具与 Hook 只能使用受管 Rust workspace 和锁定的 `Cargo.lock`；Cargo 缺失、版本不足、锁文件漂移或构建失败必须明确阻断，禁止回退 Python、脚本包装器或隐式写用户级配置。下游项目自身的业务语言依赖不受此条改变。
- 非 ASCII 正文禁止经 shell 字符串中转写入或动态执行；配置、hook 与入口脚本的编码和注册必须通过项目硬闸。
- Windows 上由 GUI、Codex Hook 或后台任务启动非交互、无人值守命令时，必须使用可验证的无可见控制台窗口入口，并保持 stdin、stdout、stderr、退出码与 timeout 语义；除非用户明确要求可见交互窗口，禁止让 shell 或 Rust 子进程控制台弹到用户桌面。

## 2 bridgeforge-codex 协作骨架

### 2.1 信息放置与指令承载

新增或修改信息前，必须先按主要读者选择唯一事实源。其他载体只能保留链接、机械生成的投影或执行所需的短摘要；禁止手写复制完整规则。

| 信息类型 | 唯一承载位置 | 主要读者与生效方式 |
|---|---|---|
| 全项目始终遵守的红线 | 根 `AGENTS.md` | Agent 每轮原生加载 |
| 目录专属红线 | 相应目录的嵌套 `AGENTS.md` | Agent 处理该目录时加载 |
| 用户调用后的操作流程 | 对应 Skill | Agent 调用时按需读取 |
| 机器合同 | 对应 manifest、schema 或结构化配置 | 程序直接读取 |
| 可自动判定的硬闸 | Hook、pre-commit；测试负责验证 | 程序自动执行 |
| 设计原因、迁移说明、案例与长 SOP | `doc/0_architecture/` 或 `doc/3_reference/` | 维护者按需查阅 |
| 项目自有知识话题与资料 | `doc/5_project_knowledgebase/` | 用户与 Agent 按需查阅，不作为指令源 |
| 产品用途与最短使用路径 | 根 `README.md` | 用户阅读 |
| 安装与迁移步骤 | `INSTALL.md` | 用户需要安装或迁移时阅读 |
| 本次执行的选择、结果与下一步 | 确定性的用户结果输出 | 用户直接判断，不依赖 Agent 临场翻译 |

`.codex/rules/*.rules` 只用于 Codex 命令执行策略。Markdown `paths:` 不会被 Codex 自动加载，禁止把它声明为指令加载机制。

- 下游业务版本必须以项目根 `VERSION` 为唯一事实源；原生 manifest 中的业务版本字段必须由项目发布流程同步，禁止让骨架版本戳代替业务版本。
- bridgeforge-codex 骨架版本只记录在 `.codex/.bridgeforge_codex_version`，仅允许统一项目同步器修改；业务提交和项目本地定制禁止修改该戳。

### 2.2 Codex 原生 Memory 与 legacy 项目资产迁移

- Codex 原生 `~/.codex/memories/` 必须保留官方生成和注入机制；BridgeForge 对内容的检索、阅读与分析必须只读，禁止创建、改写或删除正文。跨电脑同步必须只把该目录视为不透明整树快照，禁止依赖内部语义。
- legacy `.codex/rules/*.md` 不是 Codex 指令源；必须逐源文件确认迁移包，将红线、命令策略、说明和废弃内容分别落到正确资产，禁止改扩展名后冒充 `.rules`。
- 禁止新建或继续使用项目 `.codex/memory/`，也禁止注入、检索、索引、写入、lint、duplicate、usage 或 `$find-memory` 运行链。
- 下游既有 `.codex/memory/` 必须逐源文件确认迁移包；`MEMORY.md`、`MEMORY_COLD.md` 与 `_stats.json` 固定退役，不做语义转换。
- 全部 legacy Rule / Memory 确认前必须零写入；确认后，新资产、最新基线和已确认源文件删除必须在同一可回滚事务完成。逐文件确认同时构成对应删除授权，禁止再索取独立清理授权或删除未确认资产。
- `$summary` 必须按 Skill 流程检索并阅读相关原生 Memory 正文、结合当前上下文提出建议，同时完成阶段总结或“同意验收”收口；Rule / Hook / AGENTS.md 候选必须等待用户采纳，禁止写项目 Memory、直接写原生 Memory、自动写 Rule / AGENTS.md 或实现 Hook。

### 2.3 文档管理

- 文档必须使用 `doc/0_architecture`、`1_delivery`、`2_bugs`、`3_reference`、`4_archive`、`5_project_knowledgebase` 六层结构；禁止散落根目录或源码目录，也禁止删层、跳层、改名、合并或新增其他同级目录。
- `5_project_knowledgebase/` 中的话题与资料必须保持项目所有权；骨架升级禁止覆盖或删除正文，禁止因日期或完成状态自动归档。
- `doc/README.md` 是唯一索引，`delivery_layout` 是交付路径单一事实源；任何 `doc/**.md` 新增、删除、移动或重命名必须同步，禁止靠目录猜测布局。
- 所有测试代码必须放在 `scripts/tests/**`；禁止重建根 `tests/` 或在产品目录散落测试。

各层职责与当前布局以 `doc/README.md` 为准；完成事项需要归档时调用 `$archive-scan`。

### 2.4 Skills

项目 Skill 位于 `.codex/skills/<name>/SKILL.md`，用户级通用 Skill 位于 `~/.codex/skills/<name>/SKILL.md`。
可用 Skill 以当前会话原生发现结果为准；禁止在根 `AGENTS.md` 维护易过期的 Skill 名单。
- 简单 Skill 必须保持单文件；多模式或包含大量条件性细节的 Skill，入口只保留共同目标、主路径、选择点、停止条件和明确的 reference 读取条件。入口与 reference 禁止手写复制同一规则。

## 3 开发

### 3.1 换机首次启动 Checklist

用户提到“换电脑 / 新机 clone / 重装”时，必须先按项目级“快速命令”恢复主语言依赖和 Rust/Cargo，再调用 `$bridgeforge-codex` 核验骨架与 Hook；禁止照抄通用 clone 占位命令猜测仓库地址或项目名。

### 3.2 模型与 Skill 执行分工

所有 Skill 和子 Agent 默认继承当前会话的 model / effort；骨架禁止擅自锁定。

- 主对话负责沟通、授权和汇总；子 Agent 仅做分配任务，禁止代替用户决定或重复工作。
- 用户明确指令与有效授权优先于 Skill 默认流程；确认需求不等于开工，开工不含未授权目标、发布或破坏性操作，平台权限与安全边界始终有效。
- 已答事项、授权与预算必须跨 Skill 继承；缺卡不得替代授权判断，记录只能整理真实决定。范围或现场变化只重核受影响部分，禁止重复访谈或重置预算。
- 没有用户或适用指令的显式委派时，必须由主对话执行；委派必须点名已存在的 Agent 角色，禁止泛称。
- `$bridgeforge-codex`、`$create-worktree` 和 `$git-sync` 始终由主对话执行；`$git-sync` 只能运行当前项目自带的同步脚本。

### 3.3 较大需求主动澄清

取证必须必要、有限且已获授权，禁止把可查事实交回用户。未决信息会实质改变范围、行为、验收或风险时，问一个关键问题并暂停依赖它的动作；其余已授权工作继续。禁止因调查或需求确认擅自实施、运行完整回归或产生外部副作用；明确只读时禁止写盘。流程见 `doc/3_reference/codex-hook-signals.md`。

暂停必须说明原因、受影响动作和恢复条件；Skill 导致暂停或改变路径时，须链接已读 `SKILL.md`、引用触发条款，区分硬性要求与解释。条件满足后从卡点恢复，禁止再次开工确认。

## 4 Debug 与验证

### 4.1 主观体验报告主动问范式

用户报告“感觉慢、难用、不清楚”等主观体验且缺少稳定复现时，禁止猜修。一次性收集可观察证据、触发步骤、发生频率和是否能保存现场；拿到信息后先用 timer、counter 或 log 量化。

### 4.2 鬼打墙觉察与渐进升级

- 等价性验收、重写或移植完成后必须独立验证，禁止只靠自测；任务跨两个以上陌生模块时必须先调研再动手。
- 同一问题连续两次实质修复失败必须停止盲修，整理证据并进入 `$escalate` 或 `$debate`；权限重试、输出故障和未改代码的重测不计次数。用户明确求外援时禁止等待失败次数达标。
- 修 Bug 必须区分事实与假说，并确认数据源、用户路径、边界条件和外部副作用；根因未确认时必须标明置信度，禁止先加兜底或基于焦虑跨层连带修改。
- 性能调优必须先用 timer、counter 或 log 建立基线，优化后用同一方法复测，并清理临时诊断工具。

### 4.3 自改审计独立性

审计对象包含本轮 agent 自己的改动，且用户要求审计、复核需求或找遗漏时，必须启动独立 agent 二次审计。普通解释、轻量自查或用户未要求审计时不强制。

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
