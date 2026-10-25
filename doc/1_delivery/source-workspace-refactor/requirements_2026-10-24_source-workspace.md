---
lifecycle: completed
validation_status: verified
---

# TODO-032：源码子包、Skills 与分发结构重构

## 目标与授权

用户已确认源码根目录为 `source/`，下设无前缀目录 `core`、`skeleton`、`lifecycle`、`workflow`、`memory`、`checks`、`factory`、`cli`、`hook`；Cargo 原生名称保留 `bridgeforge-*`。通用 Skills 迁入 `templates/skills/`。2026-10-24 用户在完整计划及两项风险核实后明确“开始吧”，授权按本需求实施。

不包含提交、推送、正式发布、用户级正式安装或真实下游写入。保留开工前 33 项工作区变化及其当前内容，尤其不恢复已退役的历史资产语义迁移。

## 规模与预算

- 最新授权：用户明确按**剩余 8–12 小时、100,000–160,000 token** 完成重构、验证和子包收口；不提交、不推送、不发布、不修改真实下游。此为当前剩余工作预算，覆盖修复、验证、独立审计与交付；不把历史估算当作已实测消耗。

- L 级，跨包依赖、版本历史、构建分发和事务变更。
- 此前整项预算估算为 24–36 小时、300,000–500,000 token；当前继续执行以上述最新剩余预算为准，不重置或追加额度。Token 未实测，周额度缺可靠系数，不编造百分比。
- 主对话实施和整合；版本合同可委派一个独立实现任务；最终由独立 review-auditor 审计。其他委派须有明确无重叠边界并计入同一预算。
- 各阶段先定向验证；最终做一次覆盖源码、版本、构建、分发和事务风险的集成验收。实质失败按公共 AGENTS 升级，不重复冷构建消耗预算。
- 接近预算上限而必需工作未完成时停止新增工作，保留进度及证据，不自动扩额。

## 已核实事实

开工基准 `ecbffed0c053f6f61e70fc0d96e5f34970a88415`，主程序 `6.0.0`。现有 Rust workspace 位于 `templates/hooks/`，自身投影位于 `.codex/hooks/`，独立版本登记为空。当前 CLI 实际 self-test 为 6.0.0，具备开发收口、自动稳定和发布门槛能力。

风险核实确认：混合主程序载体与独立包的同 workspace 被旧产品映射合同阻断；开发收口未包含源码投影生成；构建器递归收集整个源码目录且限制两个公共二进制；新布局需要显式公共投影和旧资产迁移。Skills 安装器按清单读取来源路径，可保持安装目标。

## 目标合同

1. `source/**` 是 Rust 唯一维护源；根 Cargo workspace 显式登记九个包。`core` 不拥有业务决策，公共包不依赖工厂包，依赖图无环。
2. CLI、Hook 是主程序原生版本载体；七个库/工厂包登记稳定身份与独立版本。既有 core 版本连续，新包建立新身份；不伪造历史 split，不修改不可变发布基线。
3. 开发收口先形成原生版本/锁候选，再生成公共投影和 dogfood，验证最终状态后写收口记录。全过程保护原文件、Git index、外部改动及恢复日志。
4. 公共投影保留 `.codex/hooks/` 安装根，内部采用 `source/<package>/`。生成公共 workspace/lock，排除 factory；原生依赖解析保持一致。
5. `templates/skills/` 是 Skill 唯一维护源，用户安装目标保持不变。自身 `.codex/skills/` 为受管镜像。
6. 显式 source/target、稳定 asset id、可验证历史 hash、单一 ownership；未知旧资产保留并报告，项目 `project_*` Hook 不受清理影响。
7. 工厂从当前源码经锁定 Cargo 引导，准备 CLI/Hook/工厂工具后才检查能力。`--check`/`--dry-run` 零写入。工厂不执行下游 adopt/apply，不写下游版本戳。

## 阶段与验收

| 阶段 | 内容 | 必需证据 |
|---|---|---|
| T01 | 基线保护、身份/资产/命令映射 | 基线清单与原有改动可恢复 |
| T02 | 根 workspace、公共投影、版本混合角色合同 | 定向测试、公共锁定构建、工厂包隔离 |
| T03 | core、memory、checks 拆分 | 对应单元和入口场景，依赖无反向引用 |
| T04 | skeleton、lifecycle、workflow、factory 拆分 | 锁、漂移、回滚、恢复、历史和命令等价性 |
| T05 | Skills 迁移和安装引用 | 清单、metadata、相对引用、隔离安装恢复 |
| T06 | 分发、dogfood、旧路径迁移和收据 | 新装/升级/重复执行/漂移/故障 fixture |
| T07 | 独立包登记和受管版本收口 | 首次登记、影响范围、重复收口、release gate |
| T08 | 当前文档及工程入口更新 | 索引、事实源、路径与命令一致 |
| T09 | 集成验证、独立审计、交付 | 最终源码/投影/版本/产物一致；缺口逐项说明 |

真实下游和自然生命周期未经本次授权，不用离线 fixture 冒充。主程序 VERSION 与正式 CHANGELOG 不在开发阶段升版。

## 传播四问

公共库、Skills 和分发逻辑属于产品层；工厂专属包及自身接入包含工厂配置；记录与设计属于元文档。公共改动需要同步自身投影。正式发布需要版本和 CHANGELOG，但本次只执行开发子包收口，不执行 release。

## 实施记录

- 2026-10-24：继承已有决定、授权和预算，登记 TODO-032；开始 T01。完整方案和风险核实来自本对话，不重新访谈。
- T01：31 个原有 tracked 变更路径的内容及 Git 差异保存在忽略的 `.runtime/source-workspace-refactor/baseline/`；未撤销原有修改。dogfood 写入改用精确文件清单、前后 hash 和旧内容快照；最初的批量复制被自动审批拒绝，补齐这些证据后已成功执行精确更新。
- T02：新增显式 `product_packages` 角色；旧 `product_manifests` 保持整 workspace 语义。6 项 `schema4_explicit_product_` 实际通过（37.58 秒）。
- T02：新增有界公共源码投影及锁依赖闭包。4 项 `source_projection_` 实际通过（0.04 秒）；首轮暴露缺失可选 TOML 表的访问问题，修正后重测通过。
- T02：开发收口支持由应用层传入零写入的 candidate materializer，派生结果进入同一 journal；4 项 `package_development_projection_` 实际通过（85.23 秒），覆盖预览、正常收口、失败回滚、保护主程序版本和中断恢复。
- 上述命令统一使用 `cargo test --locked --config scripts/tests/factory-cargo.toml --manifest-path .codex/hooks/Cargo.toml -p bridgeforge-core --target-dir .codex/hooks/target <filter> -- --test-threads=1`，均为 exit 0；尚未代表新 workspace 的实际 Cargo 构建、最终分发或真实生命周期通过。
- T03/T04：根 workspace 九包运行时及测试代码均已通过 `cargo check --locked --offline --config scripts/tests/factory-cargo.toml --manifest-path Cargo.toml --workspace --tests --target-dir .runtime/source-workspace-refactor/cargo`。原 Git 同步 62 项测试分配为 workflow 48 项、factory 14 项，数量和断言保留；尚未执行最终整体验证。
- T02/T06：实际八包公共投影在没有 factory 与工厂测试源码的目录中锁定编译通过；第三方依赖 105 条版本和校验和保持一致。单文件项目 Hook 已适应虚拟 workspace；实际 Cargo 构建测试通过，原 manifest、lock 和安装目标保持原状。首轮因测试临时目录 os error 5 失败，原命令提升重试通过，未修改 ACL 或添加绕过。
- T03：指令来源与编码判定提取至 checks，Hook 保留事件与诊断适配；5 项相关定向测试通过。CLI 指令检查改为直接调用 checks，尚待最终入口验证。
- T05：Skills 已迁入 `templates/skills/`，44 条分发来源更新，安装目标不变；命令源入口、行为 fixture 和当前文档链接同步调整。
- T06：历史源码合同第一版误含此前已退役的 Python 资产；两轮校验暴露范围问题后按 escalate 引入独立 review-auditor。复核确认以旧 Rust workspace 为界，排除 29 项 Python 资产，保留 92 项、454 条经 Git blob 验证的历史 hash，不放宽项目 Hook 保护。4 项 manifest/候选锁定向测试通过，涵盖稳定 ID、公共边界、零写入渲染、项目 Hook 大小写负例及消费锁版本候选；机械删除计划另 1 项测试通过。
- T06：182 个旧维护副本文件已逐文件核验 hash、备份后精确移除，空目录已清理；保留 root Cargo 文件、缓存和独立项目 Hook。生成器已产生 `.codex/hooks/source/` 八包投影，`manifest --check` 实际 `changed=false`；该投影实际锁定编译通过。运行时忽略模板最终位于 `templates/hooks/.gitignore`（2026-10-25 按用户要求调整回此路径），保留既有资产 id；根目录新增 Cargo target 忽略规则。
- T07：真实仓库已通过受管配置提案登记七个独立包与 CLI/Hook 两个产品载体；受管收据 `migrated`、`sync_performed=false`、`version_changes=false`。脚本在同一执行环境重算计划，核验唯一写入为 `.codex/bridgeforge-version.json`；原 baseline、VERSION、CHANGELOG 保持原状。开发预览实际识别恰好七个受影响包；真实版本尚未收口。
- 构建依赖按各包实际使用收窄，105 条第三方 registry 的名称、版本、来源与 checksum 与开工 Git revision 完全相同。
- 独立审计已检查九包依赖、公共投影、版本身份、派生事务、安装与刷新入口。发现的工厂合法自刷新、Windows 活跃父 CLI 替换、安装 Skill 旧路径、普通项目误判工厂、release 消费锁遗漏、严格 check 写临时目录及回滚读取失败等问题均已修正；对应定向测试实际通过。上述局部证据不替代最终回归、真实七包收口与三产物准备。
- 工厂专属四个 git-sync Skill 镜像由 `source/factory/src/dogfood.json` 显式登记，不进入公共资产合同；manifest 与开发 materializer 共用候选输出。独立复核确认事务接线，指出零写入测试基线顺序错误，已改为调用前取证。首次定向 1 项通过，最终回归将包含修正后的断言。
- `.codex/development-checks.json` 保留既有检查 id 与检查范围，工厂单元测试入口由八包公共投影改为根九包 workspace；公共投影只验证锁定构建，不依赖未分发的工厂测试。
- 同一可选准备合同的 mirror-drift filter 已同步为实际存在的 `rust_source_is_identical_in_public_projection_and_dogfood`，避免旧名称筛选到零项；最终候选将验证实际执行数量。
- 当前源码三产物首次实际准备成功：`prepared=3`、`written_files=6`，已安装 CLI 实际声明混合 workspace、包纳管与投影能力。热复用实际 `reused=3`、`prepared=0`、`written_files=0`；严格 `--check` 为 `read_only=true`、`built=0`，安装文件哈希与时间戳不变。最终 core 版本变化后仍需按新输入核验产物。
- 实际 baseline、Skill metadata、manifest、factory-version 与 project-structure 检查通过（结构检查只有既有归档候选/忽略缓存提示，无错误）。baseline 首轮被沙箱临时 Git 配置权限阻断，原命令提升后通过，未修改 ACL。
- 独立复核未发现 31 个原有 tracked 脏路径的语义改动丢失；此前已授权的语义迁移退役仍保留。两个原有 untracked 文档当前存在，但缺少开工前正文快照，无法证明逐字未变，此项不作超出证据的保证。
- 完整 fixture 首轮发现隔离工厂配置 helper 仍只识别旧清单字段。已改为仅在 TEMP 内、尚无 Git 历史的 fixture 建立保留混合角色的 schema 3 种子，交受管迁移建立自己的基线；不复制真实 baseline/adoption/closure。独立审计未发现修复阻断，后续重跑结果如下。
- 完整 fixture 首轮结果为 91 通过、1 失败、9 条件跳过（1976.84 秒）；失败确认为旧清单字段 unwrap。修复后的同一真实工厂场景定向 1 项通过（404.73 秒），覆盖活动父 CLI/子 factory 自刷新、真实 pre-commit、主版本隔离测试、干净运行时修复、零替换热路径、索引与本地远端一致。发布/提交/推送只发生在临时 fixture，不是本仓库或真实下游。
- 共享安装器新增源码树逐层 reparse 检查：新 junction 场景修改前实际失败，修复后在 Cargo 前拒绝且计数不增加。三项相关共享 updater 回归通过（6.68 秒），全部使用隔离 profile；未运行正式用户级安装。
- 长路径缓存单元改用隔离 Rust PE probe，保留真实进程、自检、长路径和字节保护，解除“候选 manifest 配旧安装 receipt”的开发收口循环依赖。probe 首轮格式字符串编译失败，修正后定向 1 项通过（1.18 秒）；不将旧安装收据改标为候选版本。
- 独立审计补齐必要外部验证数据，受管配置更新实际 `version_changes=false`、`sync_performed=false`，不可变 baseline 和主版本文件保持原状。检查输入数：core 92、factory 113、lifecycle 13、memory 11、skeleton 10、workflow 10、checks 5；只列合同展开的明确文件，不登记整个仓库、运行时 bin 或 ledger 自身。原生依赖仍由实际 Cargo 闭包绑定；显式原始模板/Skill 输入变化只要求相关包重新验证，不伪造业务升版。
- 首次完整 lifecycle 回归已实测超过 46 分钟且持续推进，受管配置将同一必需检查的超时从 3600 调为 7200 秒保留余量，命令、检查范围和通过标准不变；本轮剩余预算不追加。
- lifecycle 完整组实际 231 通过、4 条件跳过，耗时 5676.67 秒（约94.6分钟）；可选准备合同中完整 workspace 的超时因此由5400调整为10800秒，检查范围、命令和通过标准保持不变。其余包仍继续验证；未触发可选发布候选准备。
- 九包完整单元集首轮汇总：585 通过、1 失败、16 条件跳过；唯一失败为 `unfinished_development_transaction_blocks_sync_before_network` 的替身拒绝两条必要本地 Git 查询。生产顺序未改；替身精确允许 git-dir/common-dir 读取、拒绝其余所有进程，保留调用数量2与恢复提示，新增 journal 字节不变；修复后定向1项通过（0.01秒），独立审计确认未放宽保护。workspace 日志与定向日志分别保存在忽略的 `workspace-tests.log`、`workflow-recovery-retest.log`。
- 单元组实际结果：checks 41；CLI 21；core 50（5忽略）；factory 33（1忽略）；Hook 32（2忽略）；lifecycle 231（4忽略）；memory 43（3忽略）；skeleton 38（1忽略）；workflow 首轮96通过/1失败，修复后该项通过。core长路径替身变更已独立定向通过，七包收口将再执行最终版本候选的完整必需检查；CLI/Hook、doc-test及公共测试编译在最终版本后补验。
- 2026-10-25：七包实际受管收口成功，收据 `development-closed`、`git_written=false`。core 从原生 6.0.0 升为 7.0.0，原因是原公共接口和导入路径的职责拆分不兼容；checks、factory、lifecycle、memory、skeleton、workflow 首次独立登记为 0.1.0。六个新包缺少上一有效独立开发/发布基线用于稳定性比较，未自动宣称达到 1.0.0。主程序、CLI/Hook 仍为 6.0.0。
- 收口使用最终候选重新运行七包完整必需检查，七项均 `passed`；版本、原生锁、公共投影及其锁、manifest 与消费锁通过同一事务落地，ledger 共七条，journal 已清理。主程序 VERSION、CHANGELOG、配置不可变 baseline、Git index 与 HEAD 保持原状。分析、计划和结果分别为忽略目录中的 `final-development-analysis.json`、`final-development-plan.json`、`final-development-result.json`。
- 最终 core 7.0.0 输入重新准备三产物实际 `prepared=3`、`written_files=6`、`self_test_executed=true`；安装 CLI/factory self-test 与工厂版本检查通过。严格 `--check` 实际 `read_only=true`、`reused=3`、`built=0`，安装文件 hash 和修改时间逐项不变。manifest 检查 `changed=false`，baseline 为 `clean`。
- 最终版本 CLI/Hook 单元再次通过：21+32 项、2 个由父测试调用的条件辅助入口忽略。九包 doc-test 入口全部 exit 0，当前均无文档测试用例。公共八包在物理不存在 factory/测试源码的目录中 `--workspace --all-features --no-run --locked --offline` 编译通过；独立测试消费者 `cargo check --locked --offline` 通过。配置中的镜像 filter 实际选中且通过 1 项，而非零项空跑。
- 最终重复开发预览返回 `nothing-to-close`、`affected_packages=[]`、`writes=[]`。普通准备入口热复用 `reused=3`、`prepared=0`、`written_files=0`。只读 release-preview 已通过开发收口门槛，返回正常待分析状态 `analysis-required`、七个累计子包及 `writes=[]`；这不代表已准备或执行发布。该预览前后 VERSION、CHANGELOG、版本配置、ledger 和 index 的 hash 逐项相同。
- 首轮 ledger SHA256：`7B7051B9EB43CAFAF9DC0AF988112E9749B9084C6F2FD6A422322AB03C85D206`。最终不可变 baseline 与原配置一致；HEAD 仍为开工 revision，暂存区为空，VERSION/CHANGELOG 无差异，未出现下游版本戳。`cargo fmt --all -- --check`、`git diff --check`、Skill metadata 与项目结构检查通过；结构提示仅为既有归档候选和忽略缓存，未自动处理。后续路径调整的追加记录见文末。
- 重构首轮最终独立 review-auditor 复核未发现阻断：七条 ledger 的检查 id、命令、参数、超时及输入与当前配置逐项一致，最终公开产物 receipt 与二进制和合同输入匹配，factory 二进制 hash 与 receipt 匹配；核对公开八包物理隔离、版本和 Git 保护、最终日志及重复/发布预览。严格 check 的前后文件时间保护复用主对话实际执行证据，不冒称审计者重复执行。工程实现与必要验证完成，等待用户验收；未执行 summary accept，不结算 TODO 或发布。

## 最终验证选择

本次涉及九包公共接口、锁闭包、Git/版本事务、旧布局清理与安装入口，风险跨越全局。仅定向测试不足以证明迁移等价性，因此执行一次根 workspace 完整单元集及独立原生 fixture 集；对新失败仅重跑受影响范围。受管子包收口中的必需检查对应最终版本候选，不能用升版前输出冒充。

| 风险 | 必需检查 | 当前状态 |
|---|---|---|
| 拆包导致模块、错误和权限行为回归 | 根九包单元集、CLI/Hook 调用 fixture | 首轮唯一失败修复后通过；七包最终候选必需检查全部通过，最终CLI/Hook 53项通过；累计覆盖586个顶层单元用例，16个条件辅助入口忽略 |
| 分发依赖工厂源码或遗漏锁/文件 | 公共八包隔离锁定构建、新装/升级/旧文件保护 fixture | 公共八包最终锁定构建/测试编译通过；fixture 首轮唯一失败修复后通过，累计92个顶层用例、9个条件辅助入口忽略；消费锁检查通过 |
| Skills 安装目标变化或事务恢复失败 | 临时 profile 的 updater 复用、失效与事务恢复 | 三项相关隔离回归通过；junction在Cargo前拒绝；未写真实用户目录 |
| 版本身份或同事务闭包失效 | 七包 develop preview/apply/repeat、baseline 与主版本不变 | 七包受管收口成功，最终候选七项检查通过；重复预览nothing-to-close，release-preview正常待分析，保护文件不变 |
| 工具与源码不同步或 check 写入 | 当前源码三产物准备、自检/收据、热复用及零写入 check | 最终三产物重新准备/自检通过；严格check复用3、构建0、安装字节与时间不变；正常热复用3、写入0 |
| 自审遗漏 | 独立审计真实差异与最终收据 | 整体、增量及最终实物/ledger/日志复核均已完成，未发现阻断 |

## 交付边界与复用经验

工程验证覆盖源码、公共投影、自身 dogfood、版本事务与隔离 fixture；真实下游、正式用户级安装、另一台机器和 Codex 自然生命周期全链路未验证，也未执行。本轮没有提交、推送或发布，不以临时 fixture 中的 Git/release 动作代表真实操作。保留现有工作区与忽略目录内的恢复快照，不自动归档。

本次拆包的三个可复用结论：

1. 混合 workspace 必须显式区分产品版本载体和独立包，首次纳管核实真实历史版本；否则容易把拆分误作新建或在 release 中再次升级子包。
2. 原生版本、投影、锁和 manifest 必须由同一开发候选事务生成。测试应使用隔离、自洽的产物收据，不能依赖尚未刷新到候选版本的活动安装收据。
3. 测试依赖的模板、Skill、探针和 fixture 文件必须进入显式验证输入；源码目录递归边界需拒绝嵌套链接。使用真实负例确认失败发生在 Cargo 或写入之前，不能仅依赖成功路径。

试用入口以根 `INSTALL.md` 为准；本地只读核对可使用 `.codex/bin/bridgeforge-factory.exe develop --root . --preview` 和 `.codex/bin/bridgeforge-factory.exe build-assets --factory --project-root . --check`。开发产物可用不等于用户已验收或正式发布。

最新继续执行自 2026-10-24 12:29:58 UTC 起，至工程收尾约 5 小时 20 分钟，未超出剩余 8–12 小时预算；Token 无可靠实测值，不填报估造用量。预算为上限，不以耗尽额度作为完成条件。

## 2026-10-25 忽略模板路径调整

用户明确要求将忽略模板移回 `templates/hooks/.gitignore`、删除空的 `templates/runtime/`、同步引用和必要验证；仍不提交、推送、发布或修改真实下游。文件已按原始字节移动，`/target/` 内容、稳定资产 id `codex.hooks.gitignore` 和下游目标 `.codex/hooks/.gitignore` 不变。源码投影、工厂 manifest/Git 计划、fixture、来源登记和自身镜像已更新；两处验证输入通过受管配置迁移更新，历史 baseline、主版本及既有收口记录未改写。

验证按局部实现选择：现有源码投影 5 项、实际资产身份/公共边界/零写入渲染 1 项、源码镜像 1 项实际通过。五个源码/fixture/布局文件与任务前快照对照，除路径引用替换外无其他正文变化；配置与原配置对照只替换两处验证输入。模板及自身目标 hash 同为 `306fd52e74fca6746e12acc750f232de15e71da917756a1270d74c91f5eb7368`，manifest 检查 `changed=false`，格式及 diff 检查通过。

受管预览识别 core/factory 的实现变化和其余五包的依赖证据变化。按既有检查合同执行七包完整必需检查，全部通过；实际收据为 `development-closed`、`git_written=false`。core `7.0.0 → 7.0.1`，factory `0.1.0 → 0.1.1`；其余五包仅重新验证，均保持 `0.1.0`，记录 level 为 0。没有配置稳定验收合同，不宣称 0.x 自动达到 1.0.0。主程序及 CLI/Hook 仍为 `6.0.0`。

原生版本、公共投影和相关锁/manifest 的九文件写入集经预览核对后由同一受管事务落地；ledger 共十四条，原有七条逐项原样保留，新增七条检查状态均为 `passed`，journal 已清理。当前 ledger SHA256 为 `8A9A6A8B70CFBD430B64DC06028BAC7896E1B43296E3D3506FBD31BD8AAE3680`；VERSION、CHANGELOG、Git index、HEAD 及不可变 baseline 均保持原状。

此前全局结构、接口和隔离分发证据可复用；本次没有新增独立 Agent 审计，不把首轮审计冒称为本次新增审计，也未重跑无关的完整 fixture 或发布候选准备。执行日志、分析、计划、收口结果和回退快照位于忽略的 `.runtime/hook-ignore-relocation/`。

最终版本三产物已通过受管入口刷新，实际 `prepared=3`、`written_files=6`、`self_test_executed=true`。最终源码镜像测试再次实际选中且通过 1 项。安装 CLI/factory 身份匹配 `6.0.0` / `0.1.1`，manifest `changed=false`；严格检查 `read_only=true`、`reused=3`、`built=0`，热复用 `reused=3`、`prepared=0`、`written_files=0`，安装文件 hash 和修改时间逐项未变。baseline 为 `clean`，结构错误为 0；旧目录和 journal 均不存在，主版本文件、index 及 HEAD 保持原状。最终核对收据为 `final-verification.json`，重复收口预览另保存为 `final-repeat-preview.json`。本次工程修改与必要验证完成，仍待用户验收；未提交、推送、发布或修改真实下游。

## 用户验收（2026-10-25）

用户在本对话明确调用 `$summary accept`，确认 **TODO-032：源码子包、Skills 与分发结构重构** 的当前交付符合需求。本次验收包含 `source/` 九包、独立包版本管理、`templates/skills/` 迁移、公共八包投影、构建分发、自身 dogfood，以及忽略模板移回 `templates/hooks/.gitignore`、删除空的 `templates/runtime/` 和引用同步。TODO-032 标记为已完成，需求状态为 `lifecycle: completed`、`validation_status: verified`。

验收复用上文已有实际验证与独立审计证据，并只读核对最新收据：ledger 共十四条，最新七项受管检查均通过，SHA256 为 `8A9A6A8B70CFBD430B64DC06028BAC7896E1B43296E3D3506FBD31BD8AAE3680`；最终核对收据为 `verified`，重复预览为 `nothing-to-close`、`affected_packages=[]`、`writes=[]`。本次 summary 不重新运行测试、构建、审计或 smoke。

| 对象 | 验收时版本 | 原因 |
|---|---|---|
| core | 6.0.0 → 7.0.0 → **7.0.1** | 公共接口职责拆分不兼容，随后完成忽略模板路径补丁 |
| factory | 新增 → 0.1.0 → **0.1.1** | 工厂独立包及路径引用补丁 |
| checks、lifecycle、memory、skeleton、workflow | 新增 → **0.1.0** | 独立身份与版本首次登记；后续依赖复验不重复升版 |
| 主程序、CLI、Hook | **6.0.0**，未变 | 本轮为开发及验收，未执行主程序发布 |

真实下游、正式用户级安装、另一台机器和自然生命周期全链路仍未验证，属于原授权排除范围，不因验收扩大为通过。原有未提交工作区保留；两个开工前 untracked 文档没有初始正文快照，仍不承诺逐字不变。磁盘上既有可选发布准备记录虽留有 `status=prepared`，本次未重验或刷新，不作为本轮有效准备或发布证据。此次验收不授权提交、推送、发布、正式安装或真实下游写入，也不改 AGENTS、Rule、Hook 或 Memory。

### 验收核对新增事项 TODO-033

读取 TODO 合同时发现 `templates/skills/todo/references/contract.md` 的手工兜底导航仍指向已退役的 `.codex/hooks/crates/bridgeforge-core/src/todo_contract.json`；当前实际投影为 `.codex/hooks/source/checks/src/todo_contract.json`，维护源为 `source/checks/src/todo_contract.json`。只读命令 `.codex/bin/bridgeforge.exe check todo --root . --contract` 实际成功，机器合同和 TODO 功能入口可用，问题限于参考文档中的兜底路径，判定为非阻断文档导航问题。

该问题登记为 **TODO-033，待确认、后续事项**，与已验收的结构和运行功能分开跟踪；本次不修改 Skill，也未授权相关实现。经验补充：目录迁移应同时核对 reference 中的内联路径和显式链接；机器查询入口成功不能代替手工兜底导航有效。此前缺失的用户级 TODO reference 未安装副本不在本轮修复范围，验收读取仓库内的实际参考合同，没有运行安装器。

本次没有新增 Rule、Hook 或 AGENTS 建议：事实源、保护边界及证据要求已由现行约束覆盖，单次文档导航遗漏不足以再增加常驻规则。本 topic 达到归档候选条件；TODO-033 独立跟踪，未自动移动或归档，需另行调用 `$archive-scan`。
