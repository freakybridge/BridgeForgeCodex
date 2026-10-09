---
lifecycle: completed
validation_status: verified
---

# R04：工厂准备与发布复用独立构建产物缓存

## 原始目标、用户决定与授权

依据[工厂重构扫描](scan_2026-10-09_factory-refactor.md)的 R04，减少同条件候选程序在开发准备
之后实际 release 中再次编译的成本。新增独立、可选的本机构建产物缓存；不是恢复旧 prepared
产物注入分支，不把开发检查、审计或用户验收重新变成 Git 发布门槛。

用户先调用 explain，随后明确“继续confirm”，授权本轮有限事实核验、需求整理和确认卡记录。
三项实际选择已分别明确回答 A：

| 编号 | 已确认决定 | 对实现与验收的约束 |
|---|---|---|
| Q1 | 完整实现缓存并做性能对照 | 初始消费者只覆盖本工厂的开发准备 → release；不是仅出评估报告 |
| Q2 | 自动限量保留 | 每类产物、每个平台保留最近 2 份，持久缓存上限 256 MiB；只清理新缓存中确认归属且未被占用的条目 |
| Q3 | 消除重复编译并记录耗时 | 同条件缓存命中不调用 Cargo 编译；缺失/损坏/失效正确回退，记录同口径对照，不设最低提速百分比或节省秒数 |

Q1 是交付目标选择，不将其推定为绕过 L 级预算确认的开工授权。
Q4 用户明确回答 A，批准本卡预算并开始实施。授权产品源码、受管构建、隔离性能对照、工程
检查及独立审计；不提交、推送、release，不写真实下游或原生 Memory。

## 确认时现场与证据

main，HEAD cbba10c0ec0670370c10ae2ce34b68c03b05c6db，VERSION 2.1.0，暂存为空。
R01–R03 的源码、测试、manifest 和文档仍为未提交改动；已有完整工程验证，均待用户验收。
当前累计准备收据为候选 2.1.1、指纹 90e5f811…37c7；不将它当作 R04 新能力的证明。

| 已核实事实 | 当前代码/数据证据 |
|---|---|
| 已安装产物复用存在 | git_sync_plan.rs 的 reusable_generated 验收实际二进制、收据和自检；未命中进入 pending_assets |
| 源码构建路径存在 | pending_assets 调用 project_sync::generated_writes，仍有真实 Cargo 构建、输入漂移与自检保护 |
| 单次构建内共享 | generated_writes 的局部 snapshots 表让 CLI/Hook 共用同一验证快照和依赖编译；调用创建新临时 target，结束清理 |
| 候选已经保存 | release_preparation 的 save_blob/Prepared.artifacts 保存候选与构建收据，但 direct release 不以 prepared 为前提或消费其认证状态 |
| 当前仍重复构建 | core_git_sync 的 independent_preparation_preserves_worktree_and_direct_release_builds_tools 现有断言：prepare 后同 patch release 为 built=2/reused=0 |
| 五类合同可复用 | schema=2 构建收据绑定 asset id、平台、二进制原始哈希、源码树、锁文件、构建配方和自检哈希 |
| 已有锁与写入设施 | FileLock 是稳定 inode 的非阻塞排他锁；persistence 提供原子写入、目录/链接保护；runtime 保留运行中 Windows 二进制的专门替换路径 |
| 体积基线 | 当前候选 Hook 2,655,232 字节、CLI 5,485,568 字节，加两份收据 1,176 字节；合计约 7.8 MiB |

源码和数据只读核验，不在 confirm 中执行实际发布或性能实验。历史普通同步 29.185 秒、各轮
workspace/fixture 耗时不能拿来计算本项缓存收益。

## 目标数据流与最小实现方案

1. 工厂候选构建完成 → 校验实际产物/收据、自检和输入稳定性 → 发布到独立成品缓存。
2. 工厂 release 的前瞻合同确定 → 优先保留正常已安装产物复用 → 尝试独立缓存 → 缓存无法
   证明可用则走现有真实构建。缓存命中数据仍进入既有写计划、目标快照、漂移阻断与事务回滚。
3. 缓存只能提供已验证的候选字节，不能提供需求验收、审计通过或发布授权；读缓存不访问
   Prepared 的状态、validation_fingerprint 或 audit 字段来决定能否 release。
4. 所有缓存读写/维护都只接入已授权的实际工厂构建/准备/发布路径。status/check/dry-run 等
   只读入口不得为了缓存创建目录、维护时间戳、清理文件或编译。

拟在 core 中形成内聚的构建产物缓存模块（具体命名是实现细节），以类型化合同/字节输入供
Git 写计划使用；不在调用方内联一套缓存算法。实现源进入 templates/hooks 并同步 dogfood。
本轮不改变下游 project-sync 的缓存行为，不跨项目/跨机器共享，不新增独立 Cargo 工程、
脚本包装器或第三方服务；其他产品入口扩展不自动纳入 Q1。

缓存位于本项目 .runtime/bridgeforge-codex 下的独立目录，确切叶名在实施时固定并记录。
它与 release-preparation/blobs、运行中 image 缓存、Cargo target、日志和用户资料分别归属，
不得通过删除旧 prepared blob 来满足新缓存容量限制，也不能依赖已有 prepared 记录存在。
缓存不作为换机交接资产，缺失时正常构建；未支持的共享场景不宣称验证通过。

## 缓存合同、失败处理与保持边界

| 要点 | 实施要求 |
|---|---|
| 键与身份 | 显式 asset id、稳定 schema、当前目标版本/平台、源码树/lock/recipe/self-test 合同；结合实际可证明的编译输入，未知等价性保守不命中，不凭文件名或版本号复用 |
| 字节可信性 | 读取真实二进制与构建收据，核验原始二进制哈希及既有 verify_generated_payload；重新自检并核对执行前后字节/输入，不能只信索引或 receipt |
| 版本与源码变化 | 旧版本或任何相关合同变化不命中；非编译文档变化不能用宽泛 prepared 指纹来误判，但实际 source/lock/recipe 等变化必须失效 |
| 写入 | 未成功构建、自检失败、输入变化或未完整发布的缓存条目不得成为命中；原子发布完整元数据/数据，单一 ownership 与明确路径 |
| 缓存失败 | 缺失、损坏、失效、不可用或容量不足均不能恢复“return to develop”门槛；保持正常已安装复用或真实构建，必要缓存诊断不伪报 hit |
| 必须阻断的失败 | 实际源码/发布输入/自动目标漂移、真实构建或质量失败、工具链缺失/不足/锁文件漂移仍准确停止，不能当作缓存问题吞掉 |
| 路径安全 | 固定 cache root、显式闭合条目结构；拒绝链接/重解析点与越界路径，不执行来源不明的文件，不通过 glob 认领其他内容 |
| 并发与占用 | 复用项目事务保护并为缓存维护定义租用/锁边界；不删除占用中的条目、不 unlink 活跃锁、不中途读取半发布条目 |
| 事务 | 缓存命中与编译产物走同样的 Git 写计划、原子应用、已加载二进制替换、提交失败回滚；缓存不是绕过分支 |
| 输出 | 保持现有命令参数、错误/退出与发布独立性；复用统计不能把 miss/失败当命中，新增诊断需遵守现有消费契约 |

R01 已删除不可达的 prepare_with_artifacts(Some(...))；本项不恢复它，也不恢复 release 与
development-checks/audit/prepared 的强耦合。R02 公共持久化、R03 CLI 分工保持，不顺带重写。

## 已确认资源策略的细化

- “最近”采用成功生成/使用的可验证顺序；具体持久化方式由实现保证，不让损坏元数据获得
  清理其他条目的权限。asset/platform 维度最多保留 2 个完整可用条目，合计受 256 MiB 上限约束。
- 256 MiB 限制新缓存管理的持久成品与元数据，不把既有 Cargo 工作目录或工程证明偷偷移入
  其中。临时文件和失败发布需要及时清理，无法安全释放空间则不写新缓存，正常构建照常继续。
- 自动清理仅针对明确属于本缓存、路径/ownership 可信且无占用的条目；遇未认领文件或活跃
  条目不删除、不扩大目录范围。容量与并发保护不能以回退写真实用户目录解决。
- 缓存清空、只保留一个资产、清理中断及容量受限都属于验证面；只读状态不能触发 GC。

## 性能基线与验收口径

先固定基线，再以同一方法对照：同一平台/工具链、同一目标版本、源码/lock/recipe/self-test
和同一发布工作量，使用隔离工厂 fixture 与本地 bare remote，不访问真实 GitHub，不在原
工作区执行 release。比较前确保相同的已安装产物状态，避免把正常安装复用误当新缓存收益。

| 场景 | 必须断言 |
|---|---|
| 同合同 prepare → release，缓存可用 | 两个资产都可验证复用，真实 Cargo build 调用为 0，产物及实际安装收据正确 |
| 没有 prepared/audit 或其记录损坏 | 缓存是否命中仅取决于独立合同；不因缺失工程记录阻止 release |
| 无缓存/坏缓存/自检失败/半发布 | 正常构建，不用无效字节，不制造虚假复用，真实错误如实停止 |
| 版本/源码/lock/配方/平台等变化 | 对应缓存失效；旧版本不得复用 |
| 仅一个资产命中 | 计数正确，另一个照常构建；提交失败后原资产/收据/版本/index 按既有边界回滚 |
| 并发修改源码、目标或缓存 | 源/目标漂移阻断；缓存半写或不可证明的数据不消费，使用/清理并发不删活跃条目 |
| 清理/容量/链接与未认领文件 | 所有清理留在新缓存范围，容量策略正确，外部文件与原 prepared/log/Cargo 内容保持 |

至少记录同条件有/无缓存的本地 CLI 耗时、编译调用/复用数、计时范围、环境与收据。
测试替身可验证分支与 Cargo 调用数，但不能用假构建时间声称真实提速；实际性能对照使用真实
受管程序和编译/自检路径。不设最低比例，若端到端收益不明显也如实报告原因和边界。
缓存命中不减少其他必要检查，不把 prepare 检查时间或网络环境差异混入 release 提升百分比。
用户端到端时间与本地 CLI 时钟口径分别说明，不把内部数字当用户整个任务用时。

## 拟影响文件与传播四问

主要为 templates/hooks/crates/bridgeforge-core/src 的新缓存模块、lib.rs、git_sync_plan.rs，
必要的 release_preparation/generated_writes 生产集成点及对应 dogfood、受管登记。
tests 在 scripts/tests/**，更新现有 prepare 后 release 的“必定编译 2”断言为真实新行为，
保留无工程记录可发布、无缓存真实构建、漂移及回滚断言，不以删测试掩盖路径。
长期合同设计记录复用项目 doc/0_architecture/design 的现有结构，新增文档时更新唯一索引。

| 问题 | 方案 |
|---|---|
| 层次 | 通用底层缓存能力与产品集成；工厂资源选择/验收记录归本项目配置或文档，不下沉业务事实 |
| 通用/专属 | 实现进入 templates 并镜像，初始消费者限工厂 prepare/release；不给其他项目新增写入入口或自动共享 |
| 版本/CHANGELOG | 新能力发布需按实际变更及受管版本策略处理并标 `[product]`；不沿用 R03 候选版本作硬编码承诺，本次不写 VERSION/CHANGELOG、不发布 |
| dogfood | 需要，全树逐字一致、显式 stable asset id/target/history/单一 ownership、受管 build-assets 与自检 |

## L 级预算草案与最后待确认项

L：新增跨步骤持久缓存合同、生产者/消费者与清理并发，跨 Git 写计划和构建路径，必须新增
真实性能/失败对照并完成工厂完整收口。不是按机械文件数升级，也不是前三项预算的续用。

Q4 已批准：从本次 R04 confirm 开始累计，上限 2 小时、约 60,000 新增 token
（估算，包含主/子 Agent、调查、实现、集成、性能基线/对照、当前/候选构建、完整工程检查、
合理修复重测及环境重试余量，token 未实测）、最多 1 个 review-auditor、最多 3 轮验证。
主对话顺序实现，因项目硬规则由唯一独立 reviewer 审计，必要续核复用同一 Agent；不启用
额外探索、并行 worker、debate 或高成本外援。预计需超额时先报告扩大预算或缩小范围。

预计消耗约 60,000 token，占周额度约 0.9%。沿用当前会话已接受的历史参考
“40,000 token 约占周额度 0.6%”，仅 provisional；无可靠计量不声称精确预算实耗。
每轮为实现/实质修复后完成既定验收集的闭环，多条命令、权限重试和同源码补跑不额外计轮，
但均计成本。预算不因 Skill/Agent/环境切换重置；两次实质修复失败遵守项目升级规则。

开工前 0 个子 Agent、未运行构建/测试/性能实验。实施依据为 Q4 的明确 A，而非仅从 Q1–Q3
的目标选择推定；后续按同一预算执行并补入实际证据。

## 工程验证与交付

| 编号 | 验收 | 当前状态 |
|---|---|---|
| A01 | 独立合同、生产/读取、无工程记录门槛、命中不编译 | 已实现，missing/corrupt prepared 与真实性能对照均证明 warm build=0/reuse=2 |
| A02 | 缺失/损坏/失效/旧版本/自检及真实构建失败处理 | 定向与最终完整工程检查均通过，见文末收据 |
| A03 | 并发、漂移、半写、源/目标保护和回滚 | 精确中断、锁/占用、mixed1+1 回滚与最终完整工程检查均通过 |
| A04 | 2 份/256 MiB 策略，清理边界、占用及只读零写入 | 单调保留、真实磁盘逐出、未知内容/链接保护、只读查询测试通过 |
| A05 | 同口径真实有/无缓存对照和范围说明 | 真实单次本地pair：64.209秒→15.448秒，compile2→0；不外推网络/用户端到端 |
| A06 | 模板/dogfood、登记/基线、完整 workspace/fixture、独立审计与 prepared | 全部通过，7项配置检查退出0，prepared绑定定版审计指纹 |
| A07 | 真实下游、真实机器移植、用户运行/验收、Git 发布 | 工程范围用户验收已取得；真实下游/桌面/runtime/跨机器未验证；正式 Git 发布另获授权，结果以同步收据为准 |

优先复用现有 Rust unit/fixture 与 mock runner，新增测试覆盖缓存真实风险；完整工程条件
遵循 .codex/development-checks.json 和工厂开发收口清单。相同镜像不重复完整 workspace。
先沉淀经验/索引、完成真实审计并固定输入后执行准备，运行期间不改记录；认证后仅声明的
record_documents 更新可轻量复验。prepared 有效才交付工程验证通过，ignored 如实列明。

实施后交付实际代码、当前程序、缓存所有权/关键合同、性能方法/结果、风险和本机收据；
user acceptance 由用户决定，lifecycle 保持 active，未验收不归档或自动结算 R01–R03。
完整方案及 Q4 已明确，携带同一预算进入 develop，不重复既有四项选择。

## 实施、审计修正与实际定向证据

已新增 core/artifact_cache.rs 并只接工厂写计划，复用 R02 persistence 的原子替换函数
（仅开放 crate 可见性，函数体不变）。源与 dogfood 同步，显式资产登记重建；版本未改。
cache root 为 .runtime/bridgeforge-codex/build-artifact-cache，条目与操作均有明确所有权。

独立 reviewer 唯一 `/root/r04_audit`。首轮发现 P1 构建目录工具链差异、P2 中断残留/LRU
系统时间；续审指出 Cargo 合法编译器覆盖和未登记原子临时文件漏口。均按证据修正并经同一
Agent 关闭：等价临时 BuildInputs 快照与祖先配置/实际 compiler 版本、TOML 明确允许表、
stage/retired/touch 精确日志、stage 元数据直接写入、未完成日志保留为未知占用但不毒化发布、
max(used)+1 顺序、根元数据/操作余量和所有未知占用计容量。没有扩展到下游业务或新 Agent。

首次定向工厂测试 3 项旧计数/替身不再符合独立缓存行为：有效缓存可修复坏安装，自检替身需
覆盖缓存路径，Cargo --version 不应计为 build。保留原保护断言后 7 项通过；partial 已安装
命中/编译回滚场景仍保留，另新增真正 cache1+build1 的完整目标/index回滚断言。
新测试 setup 的 Windows mklink 需要规范化反斜线，修正夹具后链接拒绝与外部零写入断言通过；
测试补强中的标准库类型路径编译错误已修正，不归为已通过或隐去失败记录。

| 定向验收 | 实际结果/日志 |
|---|---|
| artifact_cache::tests | 14 passed、0 failed、0 ignored；.runtime/r04-cache-tests.log |
| core cache filter | 17 passed、0 failed；含missing/corrupt prepared与mixed rollback；.runtime/r04-cache-integration.log |
| 原factory_保护集 | 7 passed、0 failed；.runtime/r04-factory-tests-recheck.log |
| 只读status缓存零初始化 | 1 passed、0 failed，VERSION未写入 |
| mixed缓存/构建加强复核 | 1 passed；实际cache selftest=1、Cargo build=1、全部自动targets/index字节恢复 |
| 真实缓存性能对照 | 1 passed、0 failed；.runtime/r04-performance.log |

当前版本受管 build-assets 成功，日志 .runtime/r04-build-assets.log；CLI/Hook 的
schema_version=2 收据源码树为 sha256:33c0986657cad6764db4e79bf7a97c6cb555069c2ee34955413ba2c1adf0d802。
Root CLI 仍为 2.1.0。Windows 自身替换使用校验一致的受管字节副本运行，未替换构建机制。

真实性能单次 cold/warm pair 都是同源码/目标 2.1.0→2.1.1、同文档修改发布工作量、本地 bare
remote、实际受管编译/自检/轻量Hook；各自先做同合同准备，cold 只删除新成品缓存，warm 保留。
两组都故意损坏 prepared/audit 当前记录；最终本地远端 parity 0/0 且工作区 clean。

| 本地CLI计时 | 耗时 | 实际构建/复用 |
|---|---|---|
| cold release | 64.209 秒 | built=2、reused=0 |
| warm release | 15.448 秒 | built=0、reused=2 |

差 48.761 秒，只是此次本地 CLI pair，不外推为稳定百分比、GitHub 网络或用户端到端用时。
实验共 201.52 秒含两组准备、夹具初始化及受管构建，不把预热隐去或混入 release 提速。
本轮没有正式发布；这些提交和 push 仅发生在获准的隔离测试副本/本地 bare remote。

最终源码/定向/性能独立审计 passed，记录 .runtime/r04-artifact-cache/audit.json，绑定
sha256:8e5ba12c35a1ae7b3ddae4183dd296b09f0d5fda178b27480c726679ad0c0bdb。
长期合同设计入口见 [工厂独立构建产物缓存](../../0_architecture/design/factory-artifact-cache.md)，
不在本卡复制第二份机器 schema。已固定源码、设计与记录并执行受管 prepare-release，
日志 .runtime/r04-artifact-cache/prepare.log，完整检查与候选准备一次成功，退出0；运行期间未更新文档。

## 最终工程结果（2026-10-10）

收据 .runtime/bridgeforge-codex/release-preparation/current.json 为 status=prepared，
target_version=2.1.1、checks_passed=7；指纹与上述定版审计一致，绑定当前累计R01–R04工作区。
候选在隔离快照构建，实际工作区 VERSION 和运行CLI仍为2.1.0；未执行正式提交/推送/release。
旧阶段收据与日志保留为历史证据，不误用R03旧指纹作为R04证明。

| 检查 | 实际结果 | 日志SHA256后缀 |
|---|---|---|
| baseline | clean，退出0 | 5146285933a2db9dff13b63b2870b56d28530369e2a37be00c3007b2ebfcf7c3 |
| metadata | issues/warnings为空 | 2f4914ad120aad474ada903be1a4a2b10cd859eab34c0ef951d8ad6c4ca2efdd |
| manifest | changed=false | 4f26cd005522df1df06e7e991cad2699681b25283e63142caef1b1d6c07e26e5 |
| structure | errors为空；既有归档仅advisory | 4dff2f3620921796cd799e7baec44610edce589fdee17a726d6a107fde21c227 |
| mirror-drift | 全树一致，1 passed | a37a467d9f2f969390819235dc7ffb791c22595881f0e149ab8a5ac41c99d4fb |
| dogfood-tests | CLI14、core209、Hook22；245 passed/0 failed/5 ignored | e5cb37fef00f5ba6f381d5d00484527fc8df1b358d6be61de76f6569d0268c6f |
| factory-fixtures | 89 passed/0 failed/5 ignored | e405f3fdd280fc75bfd283bd2fa7232110a4ae75a51cdb9758a15d6912c50c14 |

日志在收据同目录blobs/<后缀>。workspace命令534.659秒、fixture命令379.181秒，是工程命令
计时，不是用户端到端或性能收益。全量覆盖最终新增cache/只读/mixed回滚断言与累计旧功能；
ignored不能当作全部单独执行通过。新增R04性能场景默认ignored，但已按用户授权单独真实运行
并通过；原普通同步性能、high-cost原生第十轮、进程辅助入口、需显式路径的Assist迁移保持ignored。
workspace的辅助子进程入口及live-log检查仍如实标记，不以ignored掩盖缺失的业务验证。

实际使用唯一/root/r04_audit，已有source/targeted/perf结论与同一指纹，完整工程准备一次认证；
审计修正、单测替身/计数调整及测试setup/类型编译修正的重复操作计入成本，不在交接时重置。
token未实测，整体wall time未精确计量，不伪报60k预算实耗或将单次对照宣传为稳定比例。
prepare后仅更新已声明record_documents，按项目原机制轻量核验；最终证据复核继续由同一Agent
核对7日志/4候选blob/输入与性能记录，不重跑全量或新建Agent。

源码、模板传播、dogfood、显式登记、当前产物、风险与集成、真实性能及完整工程收据已取得。
真实下游、真实用户runtime/桌面、跨机器缓存、用户验收与正式Git发布未执行，不计通过。
HEAD保持cbba10c0ec0670370c10ae2ce34b68c03b05c6db，暂存为空；R01–R03验收状态不变，未归档或修改规则。

最终独立工程证据审计 passed：同一/root/r04_audit按现有record归类与序列化算法独立重算开发
指纹，精确等于8e5ba12c…c0bdb；全部非记录类输入实文件无漂移。7份日志、4份候选blob原始
SHA256及候选receipt二进制引用全匹配；实际测试计数、HEAD/VERSION/空暂存与记录一致，
没有剩余工程阻断。工作区source33c098…d802与候选source1638cb…2a14c、lockbe5600…741a
不同是候选版本2.1.1的正常前瞻构建，不把工作区2.1.0收据冒充候选。
workspace stderr有test-receipt辅助状态os error 5/183诊断，Cargo及配置检查均退出0，真实
日志与哈希已核验；这些诊断不阻断本项证据，但不宣称辅助日志系统runtime完全正常。
本次只更新记录文档后轻量核验，不重跑已通过测试；用户验收与正式发布仍分别待明确授权。

## 用户验收与经验收口（2026-10-10）

用户本次明确调用 `$summary 同意验收` 及 `$git-sync release`。按用户明确意图记录当前累计
R01–R04 的工程范围验收，四张交付卡更新为 completed / verified；此前阶段授权及未验收文字
保留为历史事实。另行执行受管 release，不预写提交、推送或版本成功。

本次只读复核当前 prepared 收据及已有独立审计：7 项检查退出码均为 0；7 份日志与 4 份候选
产物的原始 SHA256 全部匹配；298 项非记录类输入逐文件匹配收据，无漂移。
日志实际记录 workspace 245 passed / 0 failed / 5 ignored、fixture 89 passed / 0 failed /
5 ignored。真实本地性能日志仍为 cold 64.209 秒、warm 15.448 秒，构建数 2→0、复用数 0→2。
本轮没有重跑构建、测试、性能实验或独立审计，不把既有实施成果计作本次新增开发。

关键经验已完整覆盖于本卡与[缓存设计](../../0_architecture/design/factory-artifact-cache.md)：
缓存应独立校验编译合同与真实字节，并保留原事务保护；工程收据不能替代发布授权，
单次本地性能对照不能外推稳定提速或用户端到端耗时。没有新的稳定约束需写入 Rule/Hook/AGENTS，
未修改 Memory。既有 TODO-001–005 与本次工程验收无可结算关系，全部保留原状；未新增推测待办。

真实下游、用户桌面/runtime、跨机器缓存仍未验证。候选工程构建版本 2.1.1 属于此前准备记录；
正式 release 根据累计历史与当前 `feat` 改动重新计算版本，不以该候选版本硬编码发布目标。
归档候选为本 topic；未归档，需另行调用 `$archive-scan`。
