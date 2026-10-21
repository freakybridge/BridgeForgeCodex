---
name: git-sync
description: 分析当前 Git 变更，生成简体中文提交消息与代码变动效果摘要，并安全完成 fetch、必要的快进更新、commit、push 和最终同步核验；用户明确调用 /git-sync 或 $git-sync 时使用。
user_invocable: true
argument: 可选：release
---

# git-sync — 提交并推送

## 定位与边界

用户显式调用即授权本轮执行 Git 同步闭环。优先使用项目提供的确定性脚本；自动化只覆盖安全机械步骤，任何分叉、冲突、缺 upstream 或破坏性恢复都必须停止并交回主对话。

## 同步与发布

普通同步始终不自动升版，只有用户显式调用 `release` 才可生成版本、原生 manifest/lock 和 CHANGELOG 更新。项目 `.codex/bridgeforge-version.json` 缺失、未填写 `release_policy` 或保留旧值 `per_commit` 时也遵守同一规则；旧配置仍可读取，不自动改写项目配置。未知值、非法 schema 或重复字段仍须停止并报告。

任何同步前，先运行受管二进制 `self-test --json`，确认 `capabilities` 包含 `git-sync-release-only-v1`；显式发布还需 `git-sync-explicit-release-v1` 和 `git-sync-direct-release-v1`。旧二进制可能自动升版或要求开发收据，不能试跑未知参数探测能力；缺能力时明确提示升级工具，不归咎于项目缺开发 JSON。

普通同步不要求先选择升版策略或迁移配置。显式 release 须确认 `git-sync-schema4-v1`，只执行 schema 4；配置缺失、旧格式或用户主动换策略时读取[首次配置引导](references/first-use.md)。策略只有 synchronized（统一版本）、independent（项目与内部包独立版本），两者均可使用 Rust/Python。有效决定直接复用；只有新策略或新影响才补问。配置和迁移仅经受管 plan/apply，不手改基线，不修改已发布记录。

普通同步和显式发布均不依赖 develop、development-checks.json、prepared 或审计收据。无配置、非法配置、过期或失败开发记录都不是使用阻断。不得在本 Skill 运行 --development-status/--prepare-release，或要求先完成开发验收。显式 release 是用户选定的升版功能，不因缺开发记录静默退回普通同步。

--release-status 只读检查 VERSION、原生版本数据和发布历史，不构建或运行测试。版本非法或无法可靠计算时说明实际数据问题，保留现场，不猜 patch、不手改或降回已发布 VERSION。--development-status 仅供独立开发验证查看收据，与同步和升版使用无关。

| 用户用法 | 执行行为 |
|---|---|
| `$git-sync` | 提交并同步；不自动修改版本、原生 manifest/lock 或 CHANGELOG |
| `$git-sync release` | 检查最终差异、提交逐对象分析，预览 schema 4 发布计划，再使用 `--release` 升版并同步 |

显式发布比较最后正式发布与当前最终状态，包括已提交、暂存、未暂存和未忽略新文件；完整撤销不升包。不再累计提交标题判级。Agent 按实际公开合同与改动为程序/每个受影响包提供 patch/minor/major 及理由，程序级别覆盖最高实际影响；不把一个 feat 涉及的所有包都判为 minor。Rust 核验覆盖、版本、证据、归属和漂移，不能自动证明任意代码兼容性。兼容性未知或尚未接受的不兼容停止；major 本身不代表用户已接受。

工作区干净但有最终未发布变化仍可发布。没有发布相关变化且没有显式目标时 no-op，不创建空提交；显式纯升版须有原因。支持程序及多个包独立目标，目标低于建议或复用旧版本直接拒绝。依赖目标升版不自动带升调用方；实际依赖要求变化另行分析。所有 Cargo.lock、声明的 Python lock、明确全局构建输入和自动记录不直接升包，业务生成源码/资源不属于自动记录。完整格式见[发布分析合同](references/schema4.md)。

## 核心流程

### 0. 最短执行路径与计时

按下列决策直接执行，不为成功路径增加仓库审计或重新访谈：

1. **一次前置读取。** 在同一工具编排中先运行 `self-test --json` 并检查上述能力；若包含 `git-sync-preflight-v1`，接续运行 `.codex/bin/bridgeforge git-sync --preflight`。它聚合配置确认、分支/HEAD、upstream/实际 push 目标、状态与差异摘要。旧工具没有该能力时不试跑未知参数，改为同一批中的 `--setup-status` 和只读 Git 状态/差异/目标查询。不要把自检、配置、状态拆成多轮对话。
2. **只补必要决策。** 普通同步不因 configuration-required 停止；release 才按 first-use 处理策略与迁移。预检是本地建议快照，`remote_state=not_fetched` 不代表远端一致。clean 不展开无关业务文件；dirty 按清单审阅真实 diff，不能拿 diff_stat 代替内容。聚合预检遇活动 clean/process filter 或子模块时保守阻断，不运行未隔离的项目 helper；仅全局注册而未使用的 filter 不阻断。交回主对话核对项目专用 Git 流程，不自动绕过或禁用全局 filter。已有 Skill/约束和未变化的证据直接复用，不重读 Memory、历史、全量 TODO 或无关源码。
3. **确定消息后同步。** 普通同步跳过发布预览；release 先取得 analysis-required 输入清单，再按 schema4 reference 写入本机分析文件，执行一次带分析的发布预览。仅展示必要目标、逐包原因和阻断项，保留完整返回值。能连续编排时，预览通过后执行受管同步，成功后接续一次只读终态核验。进程未结束只等待同一进程，回读约 20–30 秒、单次不超过 60 秒；失败不能执行成功分支。
4. **核验后立即答复。** 独立确认工作区、HEAD、实际 push 目标与 ahead/behind；与收据矛盾才追查。保留版本、commit、目标、干净状态/0/0、产物复用/重建及已固化效果。工具内部完成的自检和轻量保护不在外层重跑，不再读取完整 CHANGELOG 或追加待办结算。

计时区分用户指令到最终回复的端到端时间与收据 `sync_elapsed_ms`：后者只覆盖受管 sync 函数内部（含锁/检查/Git/必要构建），不含进程启动、参数发现、输出发送及 Agent 前后处理。旧版本不提供该字段时注明未提供，不能猜值或当作 0。计时不新增日志文件、轮询服务或依赖，不减弱 Git 保护。下次真实对话使用同一时间戳口径复测；步骤减少和本地夹具不能宣称真实端到端已达标。

### 1. 只读判断、效果摘要与提交消息

1. 检查状态、diff 和当前分支，概括本轮实际变更。
2. 在运行同步脚本前，根据真实 diff 固化 1-3 条“代码变动效果”：说明增加的功能、修复的问题或改变的系统行为，以及它们带来的实际结果；禁止只罗列文件名、提交类型或“更新了代码”等空话。无法可靠判断时明确标记“未能从 diff 可靠判定”，禁止猜测。
3. 生成简洁的简体中文消息：`<类型>: <描述>`；类型限 `feat`、`fix`、`refactor`、`perf`、`docs`、`chore`。

### 2. 当前宿主确定性 Rust 入口

Codex 项目使用 `.codex/bin/bridgeforge[.exe] git-sync`。二进制存在时，主 agent 完成只读范围审查和提交消息决策后，必须直接且只运行该入口：

```text
.codex/bin/bridgeforge git-sync --message "<类型>: <描述>"
```

用户调用 `$git-sync release` 时，先执行零写入预览并展示目标版本、累计改动及升级依据。`release` 是 Skill 参数，底层 CLI 仍使用 `--release`：

```text
.codex/bin/bridgeforge git-sync --release-preview --message "<类型>: <描述>"
.codex/bin/bridgeforge git-sync --release-preview --release-analysis-file .runtime/release-analysis.json --message "<类型>: <描述>"
.codex/bin/bridgeforge git-sync --release --release-analysis-file .runtime/release-analysis.json --message "<类型>: <描述>"
```

预览无需再次索取已有发布授权；只在结果暴露新范围或兼容性疑问时澄清。analysis-required 是分析输入阶段，不是功能通过；带分析预览须为 release-planned 才执行。nothing-to-release 时说明不升版，仍可按原授权完成同步。fetch/快进造成 fingerprint 失效时重新核对最终差异和证据，不重用旧分析、不降级普通同步。多行消息优先使用 --message-file，预览与执行使用同一消息和分析文件。

release 更新版本、CHANGELOG 和原生 manifest/lock；受管 CLI/Hook 需要刷新时，同一入口自动构建并生成真实收据，优先复用有效缓存。必要构建不是业务测试。工具链、锁文件或构建实际失败必须说明原因，不回退解释器或伪造产物。时间报告以用户指令到最终回复为准，CLI 耗时单独标注。

保留项目轻量 pre-commit（编码、文档索引、版本和产物一致性），不以 --no-verify 绕过。检查失败指出具体文件/阶段和原因；不主动组织业务测试、全量回归、独立审计、用户验收或启动业务程序。源码和文档变化都可直接同步，不要求更新开发收据；构建需求只由受管产物真实输入决定。

需要审批时只为该项目脚本申请合理前缀，不分别为 fetch、add、commit 和 push 申请持久规则。脚本执行 fetch、ahead / behind 判断、安全 stash、`pull --ff-only`、版本事务、必要构建、add、commit、push 和最终检查。版本、manifest、产物和真实构建收据纳入同一回滚事务；pre-commit 只读验证。普通同步不自动升版，纯骨架更新不升级项目版本。

**仅明确本地文件权限错误可提升权限重试。** 首次受管命令必须有当前项目具体文件/目录不可写的证据，例如 `.git/FETCH_HEAD` 写入被拒绝，才以**完全相同的 repo-local 命令**、`require_escalated` 重试一次。裸 `Permission denied` 或发生于 `fetch` 阶段本身均不够；`Permission denied (publickey)`、password、keyboard-interactive 等远端鉴权失败不属于本地权限恢复。审批仅涵盖当前项目 Git 元数据；不得改 ACL、扩大目录或换成手工 Git。

| 错误证据 | 本轮动作 |
|---|---|
| 明确文件/元数据写权限错误 | 按上述条件原命令提升权限重试一次；仍失败则停止 |
| TLS/SSL、超时、连接失败 | 报告网络阶段与原始错误，停止本轮自动同步；不套用权限重试、不改 TLS/Git 配置 |
| 凭据/鉴权、分叉/冲突、Hook/版本/产物失败 | 保留现场并返回主对话；不因错误发生于 fetch/commit 而重试完整同步 |

用户明确再次要求重试时复用有效授权和未变化的审查证据，核对当前状态后重试；不能把本轮自动重试失败伪报为恢复成功。CLI 已有的 push 失败后远端复核属于原事务保护，不在 Skill 外额外组织一次完整同步。

受管二进制不存在、自检失败或当前项目 baseline 不健康时必须停止并报告；禁止回退 Python、PATH 脚本或手工 Git。即使用户要求逐条执行，也不得退化为手工 fetch、add、commit 或 push。

## 输出与收据

- 当前分支、upstream、同步前后的 ahead / behind。
- 实际提交消息、commit id 和 push 目标。
- 项目策略、是否要求发布、是否实际升版，以及升版前后版本；普通同步完成不能表述为版本发布完成。
- 收据提供 `generated_assets_reused` / `generated_assets_built` 时，报告工厂产物复用和重建数量，便于区分无构建同步与实际重建。
- 同步成功后输出此前固化的“代码变动效果”，最多 3 条；显式发布概括累计改动。若既无本地变更也无累计发布，则说明本轮没有新提交，并按收据说明远端同步结果。
- 工作区最终状态；只有状态干净且实际 push 目标的 ahead / behind 为 `0 0` 才报告同步完成。upstream 与 push 目标不同时，禁止以上游一致代替推送完成。
- 收据提供 sync_elapsed_ms 时可报内部耗时，但不把它标为端到端耗时。缺少用户起止时间戳时如实说明端到端未计量，不为普通同步新增一次会话日志扫描。
- 新工具的 timings_ms 可显示异常或主要阶段；commit_and_hooks 包含 Git commit 与其 Hook，release_analysis/artifact_preparation 包含其中子过程，父子时间不能相加。失败计时不代表同步成功，不把缺失的远端状态解释为 clean 或 0/0。
- 失败时给出原始错误阶段和保留的现场状态。

## 停止条件

- diverged、缺 upstream 或无法可靠判定远端状态时，停止并由主对话决定。
- `pull --ff-only` 失败时停止，不改变历史。
- `stash pop` 冲突时保留 stash 和冲突现场，交给用户处理。
- push 失败时重新 fetch 并判定一次；若出现竞态或分叉，停止，不强推。
- pre-commit 或衍生产物刷新失败时停止，不绕过检查。
- 提交 tree 被 Hook 改写时，保留本地提交和 guard，停止推送与自动重试；先核对差异，不自动 reset 或清除 guard。这是提交内容保护，不是开发验收门槛。

## 禁止事项

- 禁止自动 rebase、merge、`reset --hard`、force push 或丢弃 stash。
- 禁止在 `0/0` 且无变更时创建空提交。
- repo-local 确定性脚本存在时，禁止主 agent 把 fetch、add、commit、push 拆成手工 Git 命令，或把脚本执行再次委派给其他 agent。
- 禁止脚本或其他 agent 处理分叉、冲突或失败后的决策；这些决策始终留在主对话。
- 禁止用文件清单代替代码变动效果，或因提交后 diff 已清空而省略成功收据中的效果摘要。
- 禁止只说“已同步”而不提供最终干净状态和 `0 0` 收据。

$ARGUMENTS
