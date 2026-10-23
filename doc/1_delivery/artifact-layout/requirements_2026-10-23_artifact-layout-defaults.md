---
lifecycle: completed
validation_status: verified
---

# artifact-layout 公共默认规则增量

关联历史主题：TODO-020。上一批合并与验收见 [原需求卡](requirements_2026-10-23_artifact-layout-merge.md)；本卡只记录本轮增量，不重新结算旧 TODO。

## 本轮计划与执行

### 15.1 本轮决定与边界

用户确认完整计划后明确要求：“按这个计划开始实施，先完成 Forge 的修改和验证，四个下游暂时不写入，不提交或推送。”因此只修改 Forge 产品源、dogfood、测试及文档，完成工程验证、离线性能与独立审计；不发布、不升级 VERSION、不写真实下游。工程准备是隔离验证，不等于 release。

目标是无配置即使用公共测试布局规则，项目只声明差异。公共 AGENTS 在第 5 节保留两条简短约束；完整机器语义以 [布局合同](../../0_architecture/design/artifact-layout.md) 为准。测试、基准及辅助文件归 scripts/tests，工具归 scripts，内联单元测试保留。项目源码根、缓存允许目录和 Cargo 范围不自动猜测。schema 1 字段保持兼容，删除配置回到默认值；无配置、最小配置、完整默认值等价，非法配置明确失败。

按 M 级执行：已知模块内的默认行为调整，三入口和暂存影响分析需保持一致；默认预算 45 分钟、约 20k 新增 token、1 名 review-auditor、最多两轮完整验证闭环。token 未实测；本机周额度记录属于另一模型配置，不据此伪造本轮百分比。全部累计包含权限重试、等待、独立审计及修复；预计不足时停止新增超额工作并说明具体缺口。

受管产物写入权限失败及构建临时路径消失增加环境恢复成本后，用户明确选择“延长到 90 分钟，完成全部验证”。仅时长上限调整为 90 分钟，其余范围和授权边界不变，不重置累计消耗。

前两个性能场景完成后的实测速度表明，保留全部 100 样本及后续七项工程检查将超过 90 分钟；用户再次明确“延长到 150 分钟，完成全部验证”。总时长上限现为 150 分钟，累计不重置，样本和检查不缩减；下游与 Git 禁止写入边界不变。

传播四问：产品层通用能力，进入 templates 并同步 dogfood；文档记录本批决定和证据；正式发布需要产品版本与 CHANGELOG，但本轮没有 Git 或发布授权。复用 TODO-020 主题关联，保留原已验收记录，不把本批写成已验收。

### 15.2 实施与验收清单

| 编号 | 工作 | 验收 |
|---|---|---|
| D1 | 公共条款及镜像 | 两条约束一致，根项目专区不变 |
| D2 | 默认配置及差异解析 | 无文件、最小配置、完整默认值一致；非法与历史未知分别处理 |
| D3 | worktree / staged / edited | 使用同一默认规则；暂存 blob 为准，A 模式不扩大历史阻断 |
| D4 | 受管身份及审计剪枝 | 无配置仍识别精确受管资产；派生产物树只检查目录后报告剪枝，暂存路径不豁免 |
| D5 | 工程验证 | 定向回归、本地 Git runtime、七项工程检查及独立审计 |
| D6 | 性能 | 以本批修改前 CLI 为基线，既有八场景和最小/删除配置场景，每侧五样本；逐场景判定 |
| D7 | 交付 | 源码、产品、dogfood、fixture、runtime 分别列证据；真实下游与自然 Codex 触发保持未验证 |

独立测试目录检查限于确定的路径及显式注册；不新增文件名黑名单、任意豁免配置或完整依赖分析。Forge 全量工程检查沿用 development-checks.json，不降低检查标准。性能夹具只连接本地临时 bare remote，不操作当前仓库 Git 历史或真实远端。

### 15.3 下游只读结论

四个项目的公共区均有原 scripts/tests 单句；仅 Stratus 有较细的测试集和生产依赖条款，Causis 有运行产物及根目录白名单约束。未发现 HoldemPoker，本机实际目录为 HoldemTrainer；后续写入前确认项目身份。各项目均未发现 artifact-layout.json，本轮不补写。

Stratus 保留构建组、测试集版本、scripts/target、.runtime 和 pytest 入口；旧布局 Hook 的文件名判定、目录整体豁免及历史专名需在后续获准迁移时逐项核对。Causis 保留根目录治理及外部副作用约束。Holdem 保留测试命令。Assist 的 scripts/tests/run_project_vault_tests.ps1 静态指向 scripts/tests/project_vault_tests.rs，构建、结果和日志位于 .runtime/vault-hook-debug；无需因入口位置单独配置，脚本未运行，vault 正文未读取。

### 15.4 本轮证据

实施前工作区干净，业务版本 3.1.0。性能基线 CLI 已保存于本机忽略目录 .runtime/artifact-layout-defaults/baseline/bridgeforge.exe，SHA-256 为 10d80d24bc497efa28894ee6a9810a4e9f66b821fb56d1d2b52661763595ceb1；不覆盖上一批性能收据。

以下保留初始实施阶段的恢复过程，最终验证与验收以末节记录为准。首次 dogfood 写入和 Cargo 测试受 .codex 写入权限限制；串行命令未遇错即停导致模板实现被旧镜像覆盖，已恢复修改并改用遇错即停的窄范围权限调用。这是执行恢复，不是功能验证失败，不改变下游与 Git 授权边界。

定向命令 `cargo test --locked --config scripts/tests/factory-cargo.toml --manifest-path .codex/hooks/Cargo.toml --workspace artifact_layout -- --test-threads=4` 已通过：core 26 项、Hook 1 项，覆盖新增默认值、配置生命周期、暂存语义、受管身份、派生树及项目升级不生成配置。独立 review-auditor `/root/layout_defaults_review` 已审查实际 diff、源码、dogfood、测试与合同，未发现新增阻断问题；完整工程、性能及 runtime 执行待补，不由静态审计代替。

第一次 build-assets 完成编译但产物事务写入返回 os error 5 并回滚；调用程序自身位于被替换目标 .codex/bin/bridgeforge.exe，恢复时使用 SHA-256 相同的隔离副本继续调用同一受管入口。第二次编译报临时 target 的 invoked.timestamp 路径不存在（os error 3），根因未确认；未回退编译器或改写收据。第三次将当前进程 TEMP/TMP 固定在 .runtime/artifact-layout-defaults/tmp 后重试，后续验证沿用该环境并重新生成指纹。

审计补查发现 P2：tests_root 细化为 scripts/tests/unit 时，worktree 对祖先目录 scripts/tests 误报，与 edited/staged 不一致。原条件已存在，但属于本轮差异配置与三入口合同，应补正。首轮静态无阻断结论已撤回；仅放行“目录本身是测试根祖先”，不豁免祖先下的文件或兄弟目录，补齐三入口及负向回归后再复审。

窄测试根回归修复前实际执行 1 项失败，复现 worktree 误报；修复后同一定向集 core 28 项、Hook 1 项全部通过。真实空目录只抑制 misplaced-test-directory，隐藏文件、根禁令及项目明确禁止规则仍检查；未删除两个历史空目录。新增 Windows 空 junction 回归由最终完整工程集验证。独立审计补正复核已通过，读取失败的空目录分支未单独注入故障测试，不将其静态错误传播检查冒充现场故障实测。

旧需求卡恢复 completed/verified，本批增量独立落在同一 topic，以免破坏 TODO-020 的历史验收证据。该文档拆分只解决生命周期事实源冲突，不重开或重结算旧 TODO。

最终边界补正后，受管 build-assets 成功，源码树 SHA-256 为 88eec5e10fbcdc4cba1728eb0131d80ac7a81dc63c2e1fa8ffb47cc9e090e620，CLI 为 09b2ca19f02efa2c12e2387229242e574bea129990850ec18be791eccef13e24，Hook 为 34c4949e02ae8f25a64613eec35666a57c313dd7d95c57acee42a3b25fea3660。随后 check project-structure 返回 errors=[]；既有归档建议未处理。最终静态独立复核通过，包括新增空 junction 测试的断言范围。

真实 CLI/Git 隔离夹具命令 `cargo test --locked --manifest-path scripts/tests/Cargo.toml artifact_layout_runtime -- --test-threads=1 --nocapture`：2 passed、0 failed；性能项在此命令中 ignored，另行显式执行。覆盖无配置嵌套错放测试阻断、合法 scripts/tests 提交、本地 HEAD/测试远端不因失败前进、普通同步不升版及不涉及的历史缓存不阻断；不等于真实远端或自然 Codex 事件验收。

### 15.5 性能发现与限定修复

首轮十场景、每侧五样本共 100 次测量执行完毕（2136.36 秒），原始数据保存在 .runtime/artifact-layout-defaults/performance-comparison.json，汇总为 performance-summary.json。测试程序退出 0 仅证明功能断言完成，不等于性能达标。documents / unconfigured / deleted-configuration 的 commit_and_hooks 配对中位增量分别为 97 / 102 / 107 ms，与无配置时额外启动一次 Git 读取受管身份一致；该字段包含提交及 Hook，不冒充独立 Hook 计时。configuration 的全程配对中位增量 649 ms、对应阶段增量 2 ms，仍需复测排除全程退化；clean 的全程配对中位增量 24 ms，亦纳入复测。

限定修复只改变无配置分支的身份按需读取：配置存在仍走原路径；受管清单变化或 selected 中存在可能享受身份例外的路径时仍读取真实旧新身份。selected 包含政策变化影响的历史路径。新增进程次数断言及无配置身份撤销回归，不以读取失败回退空集合。独立审计确认此设计覆盖必要数据边界。

最终候选复测 documents、unconfigured、deleted-configuration、clean、configuration 五场景，每侧五样本。其余五场景复用首轮样本，依据为本次优化未改变有配置分支和无布局调用路径；明确保留首轮与最终候选的不同 hash，不宣称最终候选十场景全部同批实测。若复测仍有可重复可归因退化则不标性能通过。

新增空 junction 测试首次失败在夹具 mklink 阶段，捕获 stderr 为 `Invalid switch - "tests".`；根因是传给 Windows 内建命令的 PathBuf 含 `src/tests` 正斜杠，改为逐组件 join 生成原生分隔符。未更改产品判断或放宽断言。该项将在定向与完整工程集重验。

该修正后定向回归实际为 core 30 passed、Hook 1 passed、0 failed，已覆盖空 junction 和无配置身份撤销；测试执行环境沿用专用 TEMP/TMP。独立审计已核对最终按需读取实现及最后的夹具路径修正，没有新增发现。runtime/性能夹具最终源码已预编译，待最终受管产物完成后复测。

按需读取优化后的最终受管构建成功：源码树 SHA-256 为 8d2727fa86fe19da3889295aeda3e059513ddf9d77d71ce2424d30b8663e726f，CLI 为 bc9ea1c59aee27c03aa6cf135c16033effb6606fe1fc40b2184dc49890d9bcd6，Hook 为 d655e2e03cda32ec96a652549c11e75f7072c60d40dd5155bffaefdf2c9ed28a。development-status 返回 not-prepared，开发输入指纹 sha256:a5732c0c8bd5e2bdcd746d86f2428d51899bb720c27d4b15f6bbd958b303dc6a；独立审计复核最终文档差异后绑定该指纹。后续准备必须核对一致，不能将旧审计记录重绑定到变化后的输入。

### 15.6 用户调整收口范围与最终交付

用户随后明确要求：“我不能忍受无限追加测试时间，我需要你精简流程，尽快完成开发……开发时间已经远超我预期了。”据此停止本轮长验证链，不再将完整回归和隔离发布准备作为本轮交付前置。此决定覆盖本次任务的原验证安排，不修改全项目 AGENTS、开发 Skill 或 development-checks.json 的长期要求；不代表用户已验收功能。

11:28 精确核对本仓库 --prepare-release 进程及后代后终止，复核剩余进程数为 0。workspace 是主动中止，不能记为通过或测试失败；工厂 fixture 与隔离目标构建未开始。未形成有效 prepared 收据，不能把缓存中前五项成功或 current.json 的 preparing 中间态当成完成。

| 验证项 | 本轮实际证据与边界 |
|---|---|
| 源码与 dogfood | 默认规则、差异配置、暂存检查、精确身份与空目录边界已实现，最终受管 CLI/Hook 已构建；两镜像一致 |
| 定向回归 | 最终源码 core 30 + Hook 1 项通过，0 失败，含 Windows junction、配置删除及身份撤销 |
| CLI/Git runtime | 前轮候选 2 项通过；最终按需读取仅改变无配置下的必要读取，阻断逻辑由最终定向回归覆盖。没有冒充最终候选再次运行了这两项 |
| 性能 | 首轮 100 样本发现额外身份读取增量；最终候选五场景 50 样本复测完成，另五场景按未变分支复用首轮。不是最终候选十场景全部同批实测 |
| 工程轻量检查 | baseline、metadata、manifest、structure、mirror-drift 均退出 0；耗时分别 579、197、237、363、700 ms |
| 独立审计 | 最终源码、测试、合同及原始性能配对数据已独立复核，无未解决阻断发现 |
| 完整工程与发布准备 | workspace 主动中止；factory fixture 与隔离目标构建未执行；未取得 prepared |
| 下游与自然触发 | 四个真实下游未写入；自然 Codex Hook 事件未验证；没有提交、推送或正式发布 |

最终性能原始收据为 .runtime/artifact-layout-defaults/performance-focused.json，汇总为 performance-summary-focused.json，执行 971.69 秒。配对中位数如下，正数为候选较慢，单位 ms：

| 场景 | 总耗时配对中位差 | commit_and_hooks 配对中位差 |
|---|---:|---:|
| clean | 4 | 0 |
| configuration | -506 | -26 |
| deleted-configuration | -472 | 4 |
| documents | 103 | 35 |
| unconfigured | 426 | -11 |

独立审计核对原始阶段数据后确认：首轮额外读取的主要增量已消退，最终五场景样本未显示可重复、可归因于本次修改的性能退化。documents 的阶段差值正负混合；unconfigured 较慢配对主要来自 local_git，提交阶段未同向增加。此结论不证明所有环境绝无退化，也不扩大为真实网络或对话端到端验收。

本轮流程问题：五次冷构建调用中两次受环境问题影响，另有候选修正；150 次性能采样共约 51.8 分钟，其中被测命令累计约 22.8 分钟，夹具准备、清理等外围成本约 29 分钟。不能把这些成本都称为必要业务验证。后续同类任务应先稳定代码和定向测试，再集中构建；按影响范围验证和复用证据，不把发布候选准备机械叠加到每次开发收口。此处记录经验，不自动修改长期流程约束。

## 用户验收2026-10-23

用户明确调用 `$summary accept` 和 `$git-sync release`，验收 TODO-020 的公共默认规则增量，并授权本次受管正式发布。验收覆盖无配置默认检查、项目差异配置、受管身份按需读取、路径与空目录边界、公共条款和 dogfood；采用本卡 15.6 节经用户调整后的收口范围，不恢复已取消的全量验证前置。

依据为已有 31 项定向回归、五项轻量检查、独立代码及性能审计，以及前候选 CLI/Git 2 项和最终五场景 50 样本的明确复用边界。全量 workspace 中止、工厂 fixture 与隔离候选构建未执行、无有效 prepared、真实下游与自然事件未验证，均原样保留。accept 不重新运行这些检查，也不修改运行时代码。

TODO-020 保持已完成并增加本次验收链接，原合并验收不被替换。默认规则会收紧无配置项目的提交行为，用户此前已明确批准该计划并在本次接受交付；本次发布分析据此按 major 处理，正式目标和提交/推送结果以受管 release 收据为准，不预写 VERSION 或宣称已发布。
