---
lifecycle: completed
validation_status: verified
---

# 单次升级与历史 Cargo 归属补正

## 目标与授权

2026-10-22 用户明确授权修复：bridgeforge-codex 一次调用、最多一次确认完成升级；git-sync release 补齐历史外置源码归属及已有迁移基线的受管补正。完成验证，先不提交推送。关联 TODO-016、TODO-017。

开发阶段只修改工厂产品源、共享 Skill、dogfood、测试与必要文档，不写真实下游、不安装到用户级、不升版、不提交推送。2026-10-22 用户随后以 `$summary accept` 验收，并以 `$git-sync release` 明确授权本轮工厂升版、提交和推送；用户级安装与真实下游写入仍不在本次范围。原始证据来自 Stratus 对话 `01a11e39-3861-7031-823f-c26ac0cdedf0`。

## 最终工程结果（2026-10-22）

开发实现与工程验证已完成，用户已明确验收，TODO-016、TODO-017 按下节验收记录结算。下表是升版前的工程快照；下文早期中断及旧源码记录保留为历史，以本节及最终受管收据为准。

| 证据层 | 最终结果 |
|---|---|
| 源码 | 单次 Skill 内固定提交自动衔接、一次汇总确认，以及历史外置源码与基线补正实现完成；独立审计通过 |
| 产品传播 | templates、共享 Skills、updater 与分发 manifest 已同步；未发布、未安装到用户级 |
| dogfood | 源码全树一致；baseline、skill-metadata、manifest --check、project-structure、mirror-drift 全通过；完整 workspace 为 456 passed / 0 failed / 13 默认 ignored |
| fixture | 完整 factory fixture 为 96 passed / 0 failed / 7 默认 ignored；包含原失败的三个 Cargo manifest/lock 同步用例 |
| 真实下游 | 同源码 debug 诊断 CLI 的 Stratus 只读 repair plan 成功，原 all_inventory 模块归属与历史 hash 已证明；配置和索引不变；未 apply、未 release |
| runtime | 工厂受管 release CLI 自检通过，含 git-sync-baseline-repair-v1；用户级自然升级与业务 runtime 未执行 |
| 工程准备 | 七项检查全通过，隔离目标 3.0.1 构建成功，CLI 返回 prepared；工作区 VERSION 仍为 3.0.0，无提交推送 |

定向验证还包含 5 项静态图及 43 项 schema4 回归，均 0 failed。完整 workspace 命令为 `cargo test --locked --config scripts/tests/factory-cargo.toml --manifest-path .codex/hooks/Cargo.toml --workspace -- --test-threads=4`，耗时约 59 分钟；完整 fixture 命令为 `cargo test --locked --manifest-path scripts/tests/Cargo.toml -- --test-threads=4`，耗时约 17.6 分钟。默认 ignored 不计为已执行，真实用户级升级和下游发布不计为已验证。

最终产品源码树 SHA-256 为 `c79cd060a4df7bbccb1c51568dd321c732ce65f01acc85059608311c29fa6491`；审计工程输入指纹为 `87cb3c0a249668886f48434d53e902e97a2855378fa973ff64ca158f360d92b9`。受管收据为 `.runtime/bridgeforge-codex/release-preparation/current.json`；workspace 日志为 `0fb61ef83378600079ac6fa4171ef74a0b34feffad74f832d2641a59b6cb4d78`，fixture 日志为 `f2e742da79bd70a0723f3107aef8b2dadf77d174f575d529f3ebf59d12ff3884`，均位于同目录 blobs/。

本轮最终准备前曾被 analysis stale 校验阻断，未启动测试，原因未确认；在两次稳定当前预览后重新绑定分析，严格指纹校验通过，未放宽保护。以上七项结果均来自之后的成功准备。记录类文档更新后仅复用同一源码的检查与隔离产物，重新绑定最终记录；不再组织全量重测。

静态归属图不执行构建脚本或动态宏，不保证识别全部动态消费者；动态共享源码仍需真实 shared_inputs。历史悬空声明可以被记录或跳过，归属分析通过不代表历史源码可编译。

## 用户验收与发布授权2026-10-22

用户在获知上述完整工程结果及未验证边界后，明确调用 `$summary accept` 与 `$git-sync release`，确认本次开发符合需求，并授权受管工厂发布；这替代开发阶段“先不提交推送”的限制，不延伸到用户级安装或真实下游升级。

1. TODO-016：用户验收同次 Skill 自动衔接旧协议、固定源码提交及最多一次汇总确认的实现。既有 updater 夹具、完整工程检查和独立审计已通过，状态更新为已完成；用户级自然升级仍未执行。
2. TODO-017：用户验收历史 Cargo 外置入口与静态引用归属、已有未正式发布迁移基线的受管补正。43 项 schema4 定向回归、完整 workspace/fixture、隔离构建和独立审计已通过；真实 Stratus 只读计划证明原报错模块归属与历史 hash，状态更新为已完成。下游 apply、真实 release 和业务 runtime 仍未执行。

本次只结算上述原需求，不结算无关记录、不修改 Rule、AGENTS、Hook 或 Memory，不自动归档。发布只走项目受管 git-sync release，保留轻量 Hook；不为验收或发布重跑完整测试、开发准备或独立审计。发布阶段新出现的未完成事项见下节 TODO-018，不能将工程验收当作发布成功。

## 发布预览中断2026-10-22

TODO-018：已授权的受管 release 尚未完成。无分析预览返回 `analysis-required`，版本为 3.0.0、受影响包为空，指纹为 `sha256:d1a679a8a6ca2a27bb0fc52346ac34dd63cd5bd52c69d5e08c434bfb782dea00`；对应 schema 1 分析文件的带分析预览返回 `release analysis is stale or has an unsupported schema; inspect final inputs again`，退出码 2。具体输入或环境变化原因未确认，不能据此断言源码发生变化或兼容性失败。

用户中断后再次调用 `$summary accept`，本轮完成验收记录，不自动恢复 Git 写入。只读核对 HEAD 仍为 `b2461baa798114c2a635438ba97a7b0a76deefb0`、暂存区为空、VERSION 仍为 3.0.0；尚未执行实际 `git-sync --release`，未提交、未推送。原工程收据仍保存七项检查通过及独立审计结果，不能将发布分析失效误写为测试失败。

后续先核对当前预览与分析绑定的输入/环境，再生成当前分析；只有 `release-planned` 才恢复受管发布。不绕过指纹、不手改 baseline，不以重跑完整测试替代身份核对。该事项以“后续事项”独立保留，不推翻 TODO-016、TODO-017 的源码与工程验收。

## 范围与实现决定

1. 旧 updater 成功但缺少新协议字段时，在同一次 Skill 内最多自动衔接一次新版 updater，固定首份成功收据的源码提交；失败、空字段和来源漂移不得重试循环。所有风险与逐源迁移包汇总一次确认，未确认不写项目。
2. Rust 适配器从当前/历史 Cargo manifest 读取显式 lib、bin、test、bench、example、build 路径，从外置入口追踪静态 `mod`、`#[path]` 与静态 include 文件图。只按实际声明记录精确文件，不把父目录或无引用兄弟文件推定归包；共享文件须由 shared_inputs 消歧。包内目标保持元数据快照的既有目录归属，不要求快照包含源码。
3. 已有 schema 4 在首次正式发布前可用 plan/apply 补正历史归属。只允许同一不可变 Git 历史能证明的 additive source_roots 和可达文件内容 hash；原包身份、版本、发布边界、已有 input_hashes 的全部键值、既有归属和生命周期均不变。已提交但未正式发布的迁移也可补正；已正式发布的基线不可改写。
4. plan 零写入；apply 重新规划、核对指纹、文件与 Git 状态，并用现有锁与原子写入保存。拒绝任意配置提案或顺带改策略。

## 规模与验证预算

M：边界明确的升级协议衔接与历史派生数据修复，复用现有事务。初始按 develop/confirm 默认预算执行；2026-10-22 用户先批准 2 小时，随后明确回复“允许，扩至3小时并完成验证”，总时间预算为 3 小时，约 20k 新增 token 估算、1 个 review-auditor、最多两轮验证；token 未实测。本机周额度样本与本轮模型配置无法可靠对应，不声称周额度精确消耗；不新建计量或付费校准任务。

验证包含外置目标移动、无关同目录文件不误归属、多包冲突、旧基线补正、既有提交迁移、篡改/发布后拒绝、计划漂移零写入、升级固定源码衔接，以及项目要求的完整 workspace、factory fixture、镜像/manifest/结构检查、受管构建和独立审计。真实下游与用户级自然升级未在授权范围，单独标为未验证。

## 传播四问

产品层通用能力，进入 templates/、skills/、scripts/ 并同步 dogfood。需要产品版本和 CHANGELOG，但仅在之后显式 release 中生成；本轮保持 VERSION。验证期间将工厂自身 `.codex/development-checks.json` 的两项完整测试并发从 1 改为 4，属于 repo 层执行配置，不下沉模板；保留全部用例和原超时。独立审计检查隔离目录及 Hook 环境锁后未发现确定竞争。

## 实施与验证记录

源码及 Skill 已实现。独立 review-auditor 首轮发现跨包自然目录归属冲突和 TOML 内联 target 兼容问题，两处均已修正并补测试；二次只读复核通过。完整工程验证仍在执行，尚未声明用户验收。

前序定向 schema4 回归 32 passed / 0 failed；固定源码提交的 updater 衔接夹具通过；实际安装 CLI 自检含 `git-sync-baseline-repair-v1`，已发布基线及缺确认调用被拒绝且配置 hash 不变。最终边界修正后的专门回归 1 passed / 0 failed，受管 CLI/Hook 已重新构建，最终源码树 SHA-256 为 `0112af0d9fdd7ebcbb39dc1be38b6ca4dc725816480dca9377cdf4a8ba5fb3d7`。

首次完整准备前五项通过，串行 workspace 触发 3600 秒超时；日志 `139c010f58e8113f0303e1723f3f5740fd27039989de91577955c89dfeb82f33` 中没有 FAILED，完成项为通过，停在多语言发布矩阵。此轮不计为全量通过。后续保留完整用例，以 4 线程重跑，不因超时修改产品逻辑或跳过断言。

补充自查与独立复核确认：初版显式入口集合会改变 Python 精确版本源的既有优先级。已将新增处理限定 Rust，同时保留跨语言冲突检查，并补两项 DTO 断言。进行中的旧源码准备被主动终止，后续验证须绑定修正后的源码；此前 32 项通过不能替代最终源码的全量证据。

## 前次停止状态（2026-10-22；后续见继续开发）

修复与独立源码审计已完成；完整工程验证未通过，未验收、未提交、未推送，VERSION 保持 3.0.0。用户批准的总预算为 3 小时；追加等待授权尚未收到，因此在收口边界主动停止了 factory fixture 子进程，让受管父进程保存日志。停止后读取完整已捕获日志，发现 `factory_release_updates_all_three_cargo_manifests_and_locks` 已报告 FAILED；不能把这项失败归为预算中断。失败详情尚未完整输出，根因未确认，需后续定向复现与核查。

| 证据层 | 本轮结果 |
|---|---|
| 源码 | 两项修复及 Rust/Python 兼容边界完成；最终独立审计通过 |
| 产品传播 | templates、共享 Skills、updater 与分发 manifest 已更新；未正式分发 |
| dogfood | 源码全树一致、baseline、metadata、manifest、structure 通过；本机 CLI/Hook 已构建 |
| workspace | 最终源码完整运行：447 passed / 0 failed / 13 默认 ignored，约 48.2 分钟；日志 `cccda8143460135f2717771a028beed358d1cb0b154057dacf924b77efd6af24` |
| factory fixture | 至少一项 `factory_release_updates_all_three_cargo_manifests_and_locks` 报告 FAILED，其余执行在预算边界中断；日志 `ef1cc3998c7428b7c335d734016ca4512fb749845a455b4a0911eba669551f87`，进程 code=-1、timeout=false；失败根因未确认 |
| 真实下游 | 未写入，未验证 |
| runtime | 本机受管 CLI 自检、拒绝已发布基线/未确认调用已核验；用户级自然升级与业务 runtime 未验证 |
| 工程准备 | 六项检查已有成功收据；factory 失败需先核查，完整 fixture 与隔离目标构建待续，不能声明 prepared |

前次缓存指纹 `d7e62b421563a78c3a8d8226bf1b65d4719f32b29a136ea5a6a99418a1e5e829` 只对应旧源码，继续开发的修复已使其失效。适用入口和结果以“继续开发”最新记录及受管收据为准；不要手改缓存或把中断、失败检查标为通过。消息、分析和审计文件均在 `.runtime/upgrade-ownership-*`，不进入 Git。

## 继续开发（2026-10-22）

用户在明确获知完整验证未完成后，要求“那我需要你继续完成开发”。继续授权覆盖同一目标下的 factory 失败定位、必要修复、重测及剩余工程准备；仍不提交、不推送、不写真实下游、不升级用户级。此前按预算停止的记录保留为历史，不将“已停止”视为任务完成，也不重复索取已授权步骤的许可。已有通过证据按实际输入与受管缓存条件复用。

首次定向复现受沙箱临时 Git 配置写权限限制；以相同命令提权重试，稳定复现为 `.codex/hooks/crates/bridgeforge-cli/src/main.rs` 不在元数据 fixture 的 Git 清单中。新增检查误覆盖包内显式入口；该 fixture 原本只复制 manifest/lock，以验证三个 workspace 的同步版本与锁合同。

修复将存在性检查限定到新增的外置入口，包内入口继续由既有目录根归属。补充元数据快照的包内 lib/bin/test/bench/example/build 兼容、仓库根包，以及外置缺失拒绝/存在时精确归属的成对回归。未放宽外置入口的 Git 证明或修改原 factory 断言。

两项定向复验均通过：`rust_source_ownership_preserves_metadata_only_local_targets` 为 1 passed / 0 failed，原 `factory_release_updates_all_three_cargo_manifests_and_locks` 为 1 passed / 0 failed。增量独立审计通过，确认规范化、包目录前缀及精确 inventory 约束仍生效，模板/dogfood 一致。最终受管产物更新与完整工程验证正在继续；此前 447 项结果保留为历史，不冒充修改后全量通过。

此阶段受管 CLI/Hook 构建的源码树 SHA-256 为 `7e52e8be4029ab9fcdfa2d4e0fefb3495d0370a3ebcd33e86f3d1fc37c6c392c`，审计工程输入指纹为 `7758eac1984accfd5d5e3add0324189f0336b629278bcfee0b95212c5ad923aa`。以下静态图修正已使该阶段证据过时，不能作为最终源码通过凭据。

真实历史覆盖核对更正：只读检查 Stratus 不可变提交 `f7d21877bd1f6002e9bdddf655e8e616afb5224d` 确认，Cargo `[lib]` 指向 `../tests/market_loader/lib.rs`，该文件第 16 行是 `pub mod all_inventory;`。原报错文件是可达模块，不是显式 Cargo bin。此前同名 bin 的合成回归不足以覆盖真实触发链，相关“已完整实现”结论撤回；进行中的测试已停止，其部分日志为 `f44e452f66b7eece94fca0f4bf558eaef6dfcb9d26437d539badf5b17b2578ab`。

已补全外置入口的静态 Rust 文件图，使用已有锁定版本 syn 2.0.119 的 AST（只新增直接依赖，未升级现有包）。当前与历史始终复用调用方 read provider；可达文件进入精确 source_roots 及 inputs。支持普通/嵌套模块、inline/path 属性、cfg 静态分支并集、静态 include 的 item/表达式及文本/二进制资源；不执行宏或根据本机 cfg 裁剪，不支持的嵌套条件路径明确阻断。旧基线补正只允许新增同一历史图证明的 Some(hash)，已有 hash 键值不能删改；不读取工作树来补历史。

静态图的四项定向回归已通过；独立增量审计指出的条件语句作用域、限定标准库 include 与 cfg_attr 属性资源均已修正并补断言。基线补正的两项回归通过，包含已有 hash 改写、删除及伪造新增的拒绝。

真实下游只读 plan 进一步证明：上述历史提交的 scripts/database/Cargo.toml 声明了已缺失的 scripts/tests/strategy_contract_encoding.rs。归属提取不能替代 Cargo 编译检查；调整为保留历史 manifest 证明的精确入口声明，缺失时不读取、不递归、不生成 hash。Rust 只有目录根可匹配后代，精确文件不能拥有同名目录下的文件；Python 原语义保留。新增缺失/恢复后的可达内容与 hash 回归，真实下游配置及索引前后哈希不变。

进一步核对历史中的悬空 `#[path] mod` 和 `include!(env!(...))` 后，静态图合同更新为：静态路径先规范化，再仅遍历同一 inventory 中实际存在的文件；缺失引用不造归属或 hash，inventory 有记录而读取失败仍阻断。非字面量 include 与其他宏一样保持不透明，不读取本机环境变量或运行构建脚本。计划收据披露 static 模式及动态共享需显式 shared_inputs；全量 diff 仍逐条核对归属，但不能以此宣称发现所有动态消费者。越界、歧义、内容漂移及嵌套 cfg_attr 拒绝规则保留。

最终增量独立审计通过，未发现新的确定阻断问题；五项最终静态图回归为 5 passed / 0 failed。完整 schema4 定向回归、最终受管产物、真实历史只读 plan 和全量工程准备另取实际结果，不能以审计通过替代测试通过。

真实 Stratus 只读 `--repair-baseline-plan` 已成功，退出码 0，status 为 `baseline-repair-planned`，指纹 `58fe74e0ef4cdb7f03df946625703d8ef833701c6830db8b88fd5995abbf4607`。原报错 `scripts/tests/market_loader/all_inventory.rs` 在固定历史提交中被唯一归属 `stratus-market-loader`，内容 hash 为 `6ea268671191c64489c41d321ed5e903ebe2f61105f74d2a43ebe95b2c5ea1f3`；配置和 Git index 前后 SHA-256 一致。原始计划保留在忽略目录 `.runtime/stratus-readonly-repair-plan.json`。这证明真实历史可规划，不代表已对下游 apply、已 release 或业务 runtime 通过。

## 本轮经验

1. updater 自身升级会让“正在执行的旧脚本”和“更新后的 Skill”处于不同协议代次。对旧成功收据的字段缺失应走固定源码提交的一次性自动衔接；已有字段为空或失败不可当作旧协议重试。
2. 外置入口仅能证明该文件归属，不能证明父目录的所有文件都属于该包。显式路径也不能覆盖另一包的自然目录归属；冲突必须使用真实 shared_inputs 消歧。
3. 派生基线的受管补正应从同一不可变历史重建，并逐字段限制允许差异，而不是允许用户提交替换 baseline。已普通提交与已正式发布要分别判断。
4. 测试和审计均限定于工厂及隔离仓库。历史 Stratus 发布错误已用于设计回归，但没有写入真实下游，也没有执行真实用户级升级。
