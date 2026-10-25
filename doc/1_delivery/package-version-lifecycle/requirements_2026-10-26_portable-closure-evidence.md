---
lifecycle: completed
validation_status: verified
---

# 跨对话开发证据与发布预览

关联 TODO-038。2026-10-26 用户调用 autopilot 修复，随后明确批准整体预算 10,000,000 token，并要求对 Stratus 执行 release dry-run，避免仅工厂自测通过而实际仍不能发布。本卡是本任务唯一需求、执行和交接记录；当前对话已建立 Goal，目标为实施、验证和交付，不代替用户验收。

当前工程交付已于 2026-10-26 获得用户验收，并另获 Forge release 授权。Forge 修复、受管子包收口和独立复核通过；真实 Stratus 完成授权八包补验，两种进程环境均得到 `release-planned`，没有执行 Stratus 实际 release。

## 授权、预算与边界

- 修改范围：D:\Quant\BridgeForge；允许 Rust 源码、针对性测试、必要公共投影/dogfood、受管子包收口及相关文档。真实 D:\Quant\StratusAgent 初始只读；后续明确授权扩展为下文指定八包的现有检查、对应构建产物与受管账本补验，内容不变不升版。release dry-run 阶段保持零写入；不改下游业务源码和检查合同、不安装工具、不执行正式 release，不修改用户级安装。
- 不提交、不推送、不发布；主程序 VERSION 和正式 CHANGELOG 留待独立发布授权。不伪造检查、重写旧认证摘要、放宽真实验证或以无关全量回归替代定位。
- L 级顺序任务包：主对话实现，最多一个 review-auditor 独立审查。预计 3–5 小时，不设时间硬上限；整体 10,000,000 token 为用户指定停止阈值，非预期消耗，包含主/子 Agent、验证、返工和收尾。原生 Goal 统计对子 Agent 的归集覆盖未验证；无适用周额度系数，不虚报百分比。预算保持累计，预留收尾，平台耗尽即保存进度；连续两次实质修复失败进入现有升级流程。
- 复用已核实来源，不向原对话发送任务。机器、客户端和项目须保持可访问；不创建定时任务或守护进程。

## 来源与现状

- [Stratus 发布对话](codex://threads/01a1358e-be05-7d33-9794-8c6d96e38835)：7.0.2 发布预览拒绝十个包，包括刚认证的八包。
- [补全开发收口对话](codex://threads/01a173d5-e905-7fc2-a50d-f5935b1cf0fc)：apply 退出 0、返回 development-closed；八包 52 项配置检查记录通过，Backtester 1.56.0→1.56.1，其他七包不升版。同一对话后续发布预览只拒绝两个 Gateway 包。
- 本轮只读核实三个下游 Cargo.lock 的变化均仅 Backtester 版本。environment 收集除 CODEX_TOOL_CALL_ID 外的全部环境变量，证据包含 CODEX_THREAD_ID 等会话身份；published_unchanged 比较整份共享锁文件。来源差异与两条程序路径吻合，但旧记录没有环境分项，不能声称现场只有一个环境变量变化。
- Forge 初始 main/HEAD 6e0122bccfe82d15a3cd957e232124727419f633，工作区干净；CLI 7.0.3 自检正常，受管 assets --check 为 current/reused=3/built=0。Rust/Cargo 1.94.1 可执行。Goal 无冲突；Hook 注册已静态读取，未手动重放自然事件。构建与必要 .codex 写入遵守平台授权，不承诺平台一定放行。
- TODO-037 已验收的是 Skill 执行和完成状态，本次独立修复程序证据语义；不将其历史验收当成本次通过。

## 任务包与验收

| 编号 | 目标与文件边界 | 验收证据 | 当前状态 |
|---|---|---|---|
| E1 | source/lifecycle/src/release/schema4 的执行环境与长期开发证据分离 | 两个实际进程的会话元数据不同仍复用新证据，真实编译环境变化仍阻断 | 已验证；真实下游两个受控进程的严格指纹不同，均可形成发布计划 |
| E2 | 新旧记录共享按包依赖范围的 Rust build/lock 语义，不忽略显式全局输入 | 无关包升版可复用，相关依赖、全局 profile、显式检查输入变化阻断 | 已验证；首次真实预览已消除两个 Gateway 的误阻断 |
| E3 | 旧记录安全兼容和失效原因分项，不输出敏感环境值 | 旧哈希链不改写；未知证据 fail-closed；失败/中断不生成认证 | 已验证；旧八包需真实补验，不重绑定旧摘要 |
| E4 | scripts/tests 下回归、CLI 集成、独立审查、传播及子包收口 | 必需检查、真实收据、最终 nothing-to-close 和审查发现闭环 | 已完成；两轮受管收口、独立收据复核及最终候选核验通过 |
| E5 | 候选工具在真实 Stratus 执行授权补验，再执行零写入 release preview 与带分析计划 | 八包不升版、业务与工具安装不变；两种进程环境均形成计划，前后关键状态一致 | 已完成；52 项配置检查、8 条追加记录、两轮 release-planned 和零写入比对通过 |

传播四问：通用产品层 Rust 能力，源码真源 source/lifecycle，公共八包投影与自身 dogfood 按受管入口同步。lifecycle 及实际受影响依赖包按受管 develop 判定版本；无内容变化只重验，不批量升版。主版本/CHANGELOG 本轮不写。文档只描述实际实现和证据，不修改 AGENTS 或添加重复 Skill 规则。

## 验证安排和停止条件

先复现并实现，再执行定向正反场景、独立审查及修正、必要受管构建/相关包必需验证和开发收口。预留一轮修复重测，遵守失败升级规则；实现未变的环境重试不伪算新轮次。不跑无关全 workspace 回归。

证据诊断区分内容、依赖/构建、环境、配置和检查/产物，保留摘要而非环境原值。旧记录只有总摘要时，先检验可证明的历史路径，不手工合成有效记录；无法安全证明时明确包名、原因和继续条件，不承诺所有旧记录无需补验。

真实 dry-run 使用 Forge 构建的候选二进制，不执行下游同步、安装或业务构建，分析和收据保存在 Forge .runtime，保存前后证据。其前置补验仅按后续明确授权执行指定八包的既有检查和账本追加。若出现额外业务变化或超出授权八包的写入需求，保留现场、报告具体差异；不能把带 blocker 的 dry-run 标为发布通过。

## 执行与交接

### 下游补验的后续明确授权

首次真实候选预览已消除两个 Gateway 的阻断，仍拒绝八条未发布 schema 2 记录；3,040 个文件及关键状态前后完全一致，原始收据位于 `.runtime/portable-closure-evidence/stratus-verification.json`，退出码 2，预览耗时 124.53 秒。旧记录没有环境/依赖分项，独立审查确认不能安全重绑定。

用户随后明确选择“授权受管补验并继续 dry-run”。这条授权覆盖并更新前文的真实 Stratus 只读边界：Forge 修复验证完成后，主对话可以使用候选工具对 stratus-app-backtester、stratus-app-data-manager、stratus-common-database、stratus-launcher、stratus-replayer、stratus-strategy-sdk、stratus-test-suite、stratus-trader 执行现有配置检查，更新其对应构建产物与受管收口记录。内容不变不升版，不安装用户级或下游工具、不提交、不推送、不正式发布，不更改业务源码或检查合同。完成后执行跨进程/会话身份 release dry-run。原生 Goal 不重建、不清零；其启动文字中的只读限制由本条后续用户授权更新，整体预算不变。

验证边界：真实用户工作区受管补验属于明确授权的特定路径，继续保护已有源码、版本、检查配置、HEAD 和索引。若出现授权八包之外的新影响或业务输入漂移，先说明实际差异，不自动纳入全包补验。

2026-10-26：启动确认完成，原生 Goal active；调查证据已复用，尚未实现或通过本轮测试。当前从 E1/E2 调用链与 Rust lock closure 实现开始。用户级 confirm 引用的 todo contract 文件缺失，已定位并读取工厂 templates/skills/todo/references/contract.md，不修改用户级安装。

初稿已实现 schema 3 分项证据、会话身份排除、历史 Rust 锁依赖闭包比较、pending_reasons，以及只限 release preview 的外部分析读取。公共 CLI 和 factory 声明能力；实际 release/build/develop 的分析路径限制保留。新增跨进程检查和外部分析正反场景；独立审查 Agent 已读取真实旧记录，确认未发布 schema 2 总摘要不可无证明重绑定。

第一组五项测试在 sandbox 因临时 Git 配置写入 Permission denied，只有纯环境摘要测试执行通过；保留原命令提升权限重试，不修改 ACL 或测试条件。提升权限后三项通过，两处新增测试夹具错误（子进程精确用例名称带 crate 前缀、旧摘要 JSON 键排序）已修正，十三项针对性集正在运行。不是业务逻辑放行修复，也未把失败报告为通过。候选 debug CLI 通过锁定 Cargo 构建，正在对真实 Stratus 运行零写入预览及前后状态比较；尚未取得通过结果。

后续结果：十三项定向测试十二项通过，剩余旧 fixture 暴露声明的 ignored 全局构建输入未进入原有摘要的实现缺口。独立审查确认后增加 development_inputs 模块，主动捕获文件/缺失状态及目录成员，保留旧算法并拒绝覆盖不足的旧认证；验证期间仅允许已知候选写入和声明产物。新增三项加修正后的旧 published fixture 共四项实际通过（192.72 秒，退出 0），覆盖 ignored 文件增删改、空目录、成员漂移、合法产物更新。前述真实 Stratus 首次预览已经完成，结果见本节授权记录；尚未进行补验或取得 release-planned。

独立审查进一步检查真实消费者，发现目录快照须从 FileReleasePlan 携带到直接 release apply、git-sync 缓存复用和最终写前、可选 prepare-release 的 progress/prepared 收据前。上述入口已补齐同一个 BuildDirectoryGuard。直接 apply 的正反回归通过（46.80 秒）；git-sync 首轮晚期注入点实际落在较早的复用检查，工具已阻断但未覆盖目标边界，按调用链修正测试注入位置后重测。CLI 能力/互斥参数测试通过。独立静态复核确认两个发现已闭合，仍待实际后续测试与受管收口。实现过程中已启动的 runtime prepare 因源码变更正确报 source_tree_sha256 mismatch 并退出，未复用该候选；源码投影重新生成后按官方入口再次准备。

修正注入点后的 workflow 两项回归全部通过（199.01 秒，退出 0），覆盖 fetch 后计划复用、产物准备后的最终写前阻断，以及准备检查新增 ignored 空目录不得认证。独立审查最终未发现本轮仍遗漏的消费者。进入受管收口的源码范围为 lifecycle、workflow、factory；workflow 因实际新增事务门禁需兼容修复升版，不再只是依赖补验。CLI 为产品载体，主程序版本仍不改。

### Forge 已完成的受管收口与候选

两轮受管 apply 均返回 `development-closed`，随后预览均为 `nothing-to-close`。首轮实际执行 core-unit、factory-unit、lifecycle-unit、skeleton-unit、workflow-unit 五项配置检查；包括前述回归在内的配置测试集退出成功。独立审查确认新增模块需要纳入 core 的公共源码投影检查输入，因此在首轮事务结束后通过配置迁移，仅向 `core-unit.inputs` 增加 `source/lifecycle/src/release/schema4/development_inputs.rs`，再次实际执行 core-unit，未扩大其他包范围。

| 子包 | 开始版本 | 最终版本 | 收口依据 |
|---|---|---|---|
| lifecycle | 0.1.2 | 0.1.3 | 可移植证据、依赖锁范围、完整声明输入与目录保护 |
| workflow | 0.1.0 | 0.1.1 | 发布消费者保留目录快照并在实际写入前核验 |
| factory | 0.1.1 | 0.1.2 | 工厂能力声明及相关入口 |
| core | 7.0.1 | 7.0.1 | 检查输入变化，两次重验，内容不变不升版 |
| skeleton | 0.1.0 | 0.1.0 | 公共 manifest 输入变化，重验不升版 |

主 VERSION 保持 7.0.3。独立收据复核确认账本从 25 条增至 31 条，历史 25 条未改，新增六条均有 `passed` 检查、输出摘要及 schema 3 分项证据；无未完成事务。配置 diff 只有上述一个输入登记。

官方 `cargo run --locked --release --manifest-path Cargo.toml --bin bridgeforge-factory -- build-assets --factory --project-root .` 完成三个产物准备，返回 prepared=3、written_files=6。最终 manifest `--check` 返回 changed=false，factory-version 为 healthy，assets `--check` 返回 current/reused=3/built=0，最终候选执行 develop preview 仍为 nothing-to-close。CLI SHA-256：`CF3E215FEB0D7CEDEE2B9986EA4E3A889A057C85C47FB0B9CBB919377AE9AC40`。

本机原始收据位于 `.runtime/portable-closure-evidence/`：`forge-initial-closure.txt`、`forge-develop-apply.txt`、`forge-core-input-plan.txt`、`forge-core-input-apply.txt`、`forge-final-verification.json` 及 `forge-final-*.txt`。它们是忽略产物，不随 Git 携带；文档不代替这些缓存或真实检查。

### 真实 Stratus 补验与发布预览结果

候选 CLI 使用 Forge 的绝对路径，进程工作目录设为 Stratus。受管 apply 返回 `development-closed`，最终 develop preview 返回 `nothing-to-close`。52 项既有配置检查全部通过，账本仅追加八条 schema 3、level=0 的记录，历史记录不变；八包版本保持原值，主版本保持 1.56.1，27 个不同产物路径进入认证。成功 apply 及保护比对耗时 303.93 秒。

3,015 个受保护项前后完全一致，覆盖业务源码、配置、版本、已安装工具、HEAD 和索引。独立审查直接复核账本及收据，并核对当前索引、Git 配置和三个已安装工具的哈希，结果一致。Stratus HEAD 保持 `3c2bb9a91a0cbd09b85ba9492bcb502ffa2ba807`。

| release dry-run | 无分析预览 | 带分析预览 | 保护比对 | 耗时 |
|---|---|---|---|---|
| 正常宿主独立进程 | analysis-required，仅 Backtester 内容变化，无额外 required_checks | release-planned，主版本计划 1.56.1→1.56.2 | 3,041 项不变 | 82.35 秒 |
| 改变四项会话元数据的另一个进程 | analysis-required，同一业务范围 | release-planned，相同主版本计划 | 3,041 项不变 | 81.76 秒 |

两个严格计划指纹分别为 `sha256:cf1b76f9effd2a31439182211c24d8863fe6824baba21f263595e4195de47762` 和 `sha256:fe9238b8d1b837db635d88edfe1cc9fd3bd3d6ad086856de2a40a19161ca940f`。比较程序确认指纹不同、候选哈希相同、HEAD 相同，且第一轮后快照与第二轮前快照相同。长期开发证据可复用，严格事务计划仍区分执行环境。

两轮计划的写入列表均仅为 `VERSION` 和 `CHANGELOG.md`，没有执行这些写入。`package_changes` 中 Backtester 的 1.56.0→1.56.1 是自上次正式发布以来已有的开发变化，不是此次补验再次升版。外部分析文件与 dry-run 收据全部保存在 Forge `.runtime/portable-closure-evidence/`；关键文件为 `stratus-develop-verification.json`、`stratus-develop-final.txt`、`same-host-release-verification.json`、`new-session-release-verification.json`、`stratus-cross-session-verification.json` 及对应 `*-release-plan.txt`。

执行中保留了两次真实失败：第一次辅助脚本传入普通 Windows 绝对分析路径，被既有项目路径校验拒绝，改为项目内 `.runtime` 相对路径后通过；第二次从 Forge 工作目录启动时，配置中的 `.venv/Scripts/python.exe` 被 Windows 解析为 Forge 的解释器，导致缺少 pytest。只读核实 Stratus 自己的虚拟环境已有 pytest 后，将验证进程工作目录改为 Stratus 并重新执行全部必要检查，没有安装依赖、修改检查合同或跳过失败检查。两次失败的 3,015 项保护比对均一致；第二次受管流程明确返回回滚结果。失败收据保留为 `stratus-path-rejection*` 和 `stratus-python-cwd-rejection*`。

### 六类证据与剩余限制

| 证据类别 | 已验证内容 | 限制 |
|---|---|---|
| 源码 | 分项证据、会话元数据隔离、相关依赖锁与完整声明输入、发布消费者目录保护；独立审查发现已闭合 | 不等于已发布版本 |
| 产品传播 | manifest 生成及校验、公开八包投影、候选 CLI/Hook/factory 构建与能力声明 | 没有用户级或下游安装 |
| dogfood | 当前源码镜像一致，三个运行时产物 current，最终 develop preview 无待收口包 | 未手工重放自然 Codex Hook |
| fixture | 定向正反回归及实际受影响包的配置测试集通过；环境、旧记录、锁文件、目录变化和写入前保护均有覆盖 | 无关 workspace 全量测试未运行 |
| 真实下游 | Stratus 授权八包 52 项检查、无升版收口、两个实际进程 release dry-run、前后状态比对通过 | 使用修复候选工具，现有下游工具安装未替换 |
| runtime | 候选实际执行、已配置的 Stratus 构建及 DLL/运行测试、实际进程的元数据变化验证 | 自然新 Codex 对话、另一台机器、正式 release/Hook/提交/推送链路未验证 |

本轮只排除四项已确认会话元数据：CODEX_TOOL_CALL_ID、CODEX_THREAD_ID、CODEX_SESSION_ID、CODEX_APP_TOOLS_PIPE_PATH。真实编译环境和输入变化仍会失效；不承诺任意环境变化都可复用。只有旧总摘要且无法证明一致的记录仍须真实补验，不能重写旧认证。

从其他目录传入 `--root` 时，相对检查程序在 Windows 上的解析问题未纳入本次生产源码修复；本轮验证均在 Stratus 项目目录调用。进程异常路径会尝试保存失败后快照，强杀等情况不能保证 finally 执行，须另行核验现场。当前安装的旧下游工具没有获得此候选修复，后续正式发布应使用已验证候选或完成修复版本的受管分发，不能把本轮预览成功解释为旧安装已自动更新。

### 交接与预算

工程交付时 Forge 为 main，基准 HEAD 为 `6e0122bccfe82d15a3cd957e232124727419f633`；当时源码、配置、投影、账本和文档未提交，没有提交、推送、发布或用户级安装操作。后续用户验收及 Forge 发布授权见下节，实际 Git 发布结果以受管收据为准，不自动归档。

同一 Goal 未重建、预算未清零，最终完成回执为 839,396 / 10,000,000 token、11,623 秒，状态 complete。该统计覆盖工程任务，不包含后续独立的验收与发布调用。工程进程均已退出；独立审查 Agent 只读核验，不持有待执行写入。

接续所需忽略产物是 `.runtime/portable-closure-evidence/` 收据、受管构建缓存和 `.codex/bin` 候选二进制；另一台机器不能假设这些内容、原进程或预算状态自动恢复。可复制接续提示：读取 `doc/1_delivery/package-version-lifecycle/requirements_2026-10-26_portable-closure-evidence.md`，先核对当前分支、代码与文档差异及发布收据，再在已有授权范围内继续；缺失证据如实标明。

### 用户验收与发布授权2026-10-26

用户明确调用 `$summary accept` 和 `$git-sync release`。据此验收 TODO-038：本次 Forge 程序修复、实际受影响包收口、独立审查、真实 Stratus 八包 52 项补验和两个进程的 release dry-run 达到本任务条件，标记已完成。复用本卡现有收据，不为验收重跑构建、业务测试或审计。Forge 提交、推送和主程序发布由随后独立的受管 git-sync 流程执行；此授权不包含 Stratus 正式发布、用户级安装或下游工具升级。

验收保留六类证据表中的限制：自然新 Codex 对话、另一台机器和 Stratus 正式提交/Hook/推送链路仍未验证。严格事务指纹与长期开发证据必须分开；旧总摘要缺少分项证明时应真实补验，不能重绑定；真实 dry-run 必须使用明确候选并保存前后保护快照。这些经验已由实现、合同或现有 AGENTS 覆盖，不新增重复 Rule/Hook/AGENTS 建议，不修改原生 Memory。当前 topic 可列归档候选，需另行 `$archive-scan`，本轮不移动文档。

### TODO-039 Windows 跨目录相对检查程序解析

真实 Stratus 补验从 Forge 目录启动、指定 `--root` 为 Stratus 时，配置中的 `.venv/Scripts/python.exe` 解析到 Forge 解释器并缺少 pytest；绝对入口核验确认 Stratus 自己的环境具备 pytest。`source/core/src/process.rs` 将相对 program 直接交给 Command，同时设置 child cwd，Windows 实际解析受父进程目录影响。本轮改为在 Stratus 目录调用后，原检查全部通过。该独立问题尚未修改生产实现，登记 TODO-039 为待确认、后续事项；后续需覆盖跨目录调用及保留 PATH 命令语义的回归。

### TODO-040 Windows 项目内绝对分析路径归一化

真实补验使用 `D:\Quant\StratusAgent\.runtime\forge-portable-develop-analysis.json` 时返回 `release analysis must belong to this project`，改为项目相对路径后通过。ProjectContext 会 canonicalize 根路径，但 `analysis::read` 对绝对参数直接 strip_prefix，缺少一致的 Windows 路径表示处理。该独立问题尚未修改生产实现，登记 TODO-040 为待确认、后续事项；后续需在保留项目归属、ignored runtime、link/reparse 防护的前提下验证普通绝对路径、扩展路径及相对路径。
