<!-- BRIDGEFORGE:PUBLIC:BEGIN -->
## BridgeForge 公共区

> 本区由 bridgeforge-codex 管理，项目不得直接修改；项目规则必须写在文末“项目级专区”或对应目录的嵌套 `AGENTS.md`。
> 用户提到“换电脑 / 新机 clone / 重装”时，必须先读根 README 的“第一次 clone、换机或重建环境”并按该入口执行。

## 1 先找对位置

| 内容 | 唯一承载位置 |
|---|---|
| 全项目常驻红线 | 根 `AGENTS.md` |
| 目录专属红线 | 对应目录的嵌套 `AGENTS.md` |
| 用户主动调用的流程 | 对应 Skill |
| 可由程序判断的约束 | Hook、pre-commit 或测试 |
| 操作说明 | 根 README 的 BridgeForge 公共区 |
| 原理、案例和历史 | `doc/README.md` 指向的专题 |

项目 Skill 位于 `.codex/skills/<name>/SKILL.md`；用户级 Skill 以当前会话提供的可用列表为准。

`.codex/rules/*.rules` 只用于命令执行策略。Markdown 中的 `paths:` 不会让 Codex 自动加载规则；修改“目录级 AGENTS 索引”登记目录或其子目录前，必须读取从项目根到目标路径上的目录指令。

## 2 交付与证据

- 必须先给结论和证据；禁止用空泛安抚或连续的“可能 / 建议”稀释判断。不确定时标明“未验证 / 缺证据 / 只是推断”，并说明如何验证。
- 必须保留用户已暂存、未暂存和未跟踪的改动；禁止覆盖、回退或整理无关内容。
- 执行类任务必须先读上下文、判断风险，再推进到修改或执行、验证和结果汇报；只有遇到授权边界、关键选择或真实阻断时停止。
- 代码审查必须按严重度列问题，并给出文件、行号和行为风险。
- 排障必须先给最可能根因、证据和验证动作；架构判断必须给推荐结论、主要取舍、主要风险和适用条件。
- 交付必须区分已完成、已验证、未验证和剩余风险；交付处或危险处拿不到真实证据时必须直说“未验证 / 不知道”，禁止用“你可以……”代替结果汇报。
- 使用文件、路径、字段、接口、配置或外部状态前必须当次核验；禁止编造资源、静默换来源或把未发生的操作归咎于用户、工具或环境。
- 找文件和查内容必须使用受控的 Glob、Grep、Read；shell 只用于构建、测试、Git 和进程等执行动作，禁止用 `find`、`Get-ChildItem`、`Select-String` 做大检索。
- 重写、移植或替换必须保持功能、配置、数据链和错误语义等价；禁止用 TODO、临时绕过或上层内联代替缺失能力。
- 修改、迁移或排障前必须追踪完整调用链、调用方、配置入口、数据链和失败路径；禁止只凭文件名或单条报错猜根因。
- 工具结果出现重影、异常零命中、未知文件或解析失败时必须二次验真。
- 发现自身结论或操作错误时必须立即更正并重新验证。
- 写“验证通过”必须同时给出实际命令或收据、验证断言和覆盖场景。

## 3 环境与安全

- 每个项目必须自建 CPython 3.11+ `.venv`；Python 依赖必须安装到其中，禁止全局安装。
- 禁止隐式写入用户级配置；项目运行必需的关键配置禁止只放在用户目录；依赖清单禁止写本机绝对路径。
- 骨架脚本和 Hook 只能使用当前项目的 `.venv`。
- 只有 init/adopt 且 `.venv` 完全缺失时，才允许用经验证的 PATH Python 创建环境；创建后必须立即切换到项目 `.venv`。
- 新依赖必须写入可复现的项目依赖清单。
- 非 ASCII 正文禁止经 shell 字符串中转写入或动态执行；配置、Hook 和入口脚本的编码与注册必须通过项目硬闸。
- GUI、Hook 和后台任务启动非交互命令时，必须使用可验证的无可见控制台入口，并保持 stdin、stdout、stderr、退出码和超时语义；只有用户明确要求可见交互窗口时例外。
- 破坏性写入、发布、跨项目修改和敏感数据外发必须先取得与目标、范围和影响匹配的明确授权。

## 4 任务控制与排障

- 琐碎、续接或细节完整的任务直接执行；新的、大而模糊且关键取舍会改变实现或验收时，按 `doc/3_reference/codex-hook-signals.md` 每轮只确认一个关键问题。
- 收到 `[focus]` 时必须读取 `doc/3_reference/codex-hook-signals.md` 的对应章节再判断；信号本身不构成阻断。主动澄清由 Agent 按上一条语义直接判断，禁止恢复 Clarify Hook。
- 主观“慢、难用、不清楚”且没有稳定复现时禁止猜修；必须一次性收集可观察证据、触发步骤、发生频率和是否能保存现场，再用 timer、counter 或 log 建立基线。
- 任务跨两个以上陌生模块时必须先查清模块边界和调用链。
- 修 Bug 必须区分事实与假说；根因未确认时标明置信度，禁止先加兜底掩盖问题。
- 修复前必须确认数据源、用户路径、边界条件和外部副作用；禁止因焦虑跨层连带修改。
- 同一 Bug 每次失败后、再次修改前必须取得一项新量化证据；前三次修改仍失败时，第四次禁止继续试写，必须列出已试方案和未验证假说，并进入 `$escalate` 或 `$debate`。
- 性能优化必须先用 timer、counter 或 log 建立基线，修改后按同一方法复测，并移除临时诊断工具。
- 等价性验收、重写或移植完成后必须独立验证；实现者不能独自证明自己的改动正确。
- 审计包含本轮 agent 自己的改动，且用户要求审计、复核需求或找遗漏时，必须启动独立 agent。普通解释、轻量自查或用户未要求审计时不强制。

## 5 协作与项目资料

- 模型和思考强度由用户选择；项目模板不得锁定，Skill 和子 agent 默认沿用当前会话设置。
- 主对话负责沟通、确认、授权和最终汇总。
- 只有 Skill 或项目规则明确要求时才启动子 agent；子 agent 只做被分配的阶段，不代替用户决策，也不重复已完成工作。
- `$bridgeforge-codex`、`$create-worktree` 和 `$git-sync` 必须由主对话执行；`$git-sync` 只能运行当前项目自带的同步脚本。
- 骨架 init、adopt 和 update 必须由主对话经 `$bridgeforge-codex` 执行；禁止手工复制、合并或直接运行同步器旁路受管流程。
- 项目 Memory 位于 `.codex/memory/`，Codex 原生 memories 位于 `~/.codex/memories/`；两者禁止合并、目录联接（junction）、混写或互相代替。
- `.codex/memory/MEMORY.md`、`MEMORY_COLD.md` 和 `_stats.json` 是派生索引，禁止手工修改。
- 检索项目 Memory 必须先读 `MEMORY.md`；任务锚或确认卡能唯一定位 topic 时再读该 topic，主索引未命中时才使用 `$find-memory`。
- 修改文档布局前必须读取根 README 公共区；`doc/README.md` 的 `delivery_layout` 是交付布局的单一事实源，禁止靠目录猜布局。
- 文档只能进入公共区定义的五层目录；禁止删层、跳层、改名、合并、新增同级目录或散落到项目根与源码目录。
- 文档路径变化必须同步 `doc/README.md`；测试只能进入 `scripts/tests/**`。

## 6 版本与升级

- 项目业务版本以根 `VERSION` 为唯一事实源；语言原生清单（manifest）由项目发布流程同步，禁止用骨架版本戳代替业务版本。
- 骨架版本戳只能由统一同步器修改，其他操作禁止修改。
- 因性能、UI 行为、字体等可感知体验而跨大版本升级依赖时，必须按根 README 在主项目外用新版依赖和最小代码做 2–4 小时实验，并与现状对比；只有用户确认体验确实改善后才能进入全项目升级。
- 禁止先改主项目或 lockfile；禁止只凭 CHANGELOG 或“升级完成”宣称诉求已经改善。
<!-- BRIDGEFORGE:PUBLIC:END -->

<!-- BRIDGEFORGE:PROJECT:BEGIN -->
## 项目级专区

> 本区由 bridgeforge-codex 工厂所有。骨架更新必须逐字保留，不得覆盖、删除、吸收或重新格式化；产品更新不得把工厂事实写入 Template。

### 项目架构红线

- bridgeforge-codex 同时是骨架产品工厂和工厂自验证镜像（dogfood）；`templates/**`、`skills/**` 是产品源，`.codex/**` 是工厂镜像。
- 任何改动落地前必须回答传播四问。前两问：改的是产品、自身配置还是元文档；通用还是工厂专属。
- 后两问：是否需要 VERSION/CHANGELOG；是否需要同步工厂镜像。
- 通用改动必须进入对应产品源并同步工厂镜像；项目所有区必须逐字保留。工厂专属事实禁止进入 Template。
- 上游下沉与下游反哺分别以 `doc/0_architecture/design/sync-from-upstream-playbook.md` 和 `doc/0_architecture/design/reverse-sync-playbook.md` 为准。
- 禁止对本仓库执行下游 `project_sync adopt/apply` 或写入 `.codex/.bridgeforge_codex_version`；工厂合规只能由 dogfood 一致性硬闸证明。
- 本项目模块职责、依赖方向、数据流和外部副作用必须与“项目目录地图”一致。
- 只有产品层改动必须更新根 `VERSION`；产品、仓库配置和元文档分别用 `[product]`、`[repo]`、`[meta]` 记录 CHANGELOG。
- 受管资产必须使用显式目标（target）、稳定资产编号（asset id）、可验证的历史哈希（hash）和单一所有权策略（ownership strategy）；禁止用 glob 声明资产所有权。
- 骨架事务必须在 apply 前重算 safe/risk/gap 计划与聚合指纹；漂移时禁止写入。
- 失败必须回滚本事务写入；degraded 禁止写入新版本戳。
- 完整操作合同见本仓库 `skills/bridgeforge-codex/SKILL.md`，根文件不复制操作算法。
- 发布前必须通过 factory dogfood、Skill metadata、分发清单（manifest）、project structure、镜像漂移（mirror drift）、完整下游样例回归（fixture）、完整自动测试和独立审计。
- 未执行真实下游或运行时冒烟（runtime smoke）时，禁止宣称对应验证通过。

### 项目业务与安全红线

- 工厂不得吸收下游业务约束污染公共 Template。
- 真实下游写入只允许发生在用户明确授权的测试 worktree。

### 项目目录地图

| 目录 | 职责 |
|---|---|
| `templates/**` | 下游公共骨架产品源 |
| `skills/**` | 用户级通用 Skill 产品源 |
| `scripts/**` | 同步器、迁移器、重建器和 `scripts/tests/**` 自动测试 |
| `.codex/**` | 当前模板的工厂 dogfood 镜像、项目 Memory 和本仓库配置 |
| `doc/**`、`README.md`、`CHANGELOG.md` | 架构、交付、Bug、参考、归档和发布说明 |

### 项目快速命令

```powershell
.venv\Scripts\python.exe -B -m unittest discover -s scripts/tests -p "test_*.py"
.venv\Scripts\python.exe -B scripts/tests/run_downstream_fixture.py
.venv\Scripts\python.exe -B scripts/rebuild_shared_skill_manifest.py --check
```

### 目录级 AGENTS 索引

- `scripts/AGENTS.md`：修改同步器实现时必须保持的事务不变量和测试。
- `skills/AGENTS.md`：修改通用 Skill 产品源时的分发与镜像规则。
- `doc/2_bugs/AGENTS.md`：关闭工厂 Bug 时的六类证据。
- 新增嵌套 `AGENTS.md` 时必须在此登记其适用目录与职责。
<!-- BRIDGEFORGE:PROJECT:END -->
