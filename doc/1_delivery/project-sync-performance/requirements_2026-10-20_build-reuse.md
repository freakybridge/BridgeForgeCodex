---
lifecycle: completed
validation_status: verified
---

# 下游维护耗时与构建复用（R04、R02、R01）

关联 TODO-013。2026-10-20 用户授权按 R04 → R02 → R01 实施；R05、R03 暂缓。不提交、不推送、不发布、不写真实下游或用户级安装。

## 范围、预算与传播

当前有效上限：360 分钟、45000 新增 token（估算，未实测）；以下保留各次预算变更原因。工程验证已完成，用户验收尚未发生。

通用产品层：事实源为 templates/hooks、scripts、skills；同步自身 dogfood、manifest。保持版本 2.6.0，正式发布由受管 release 处理 [product] 版本和 CHANGELOG，本轮不预写。

M 级跨模块任务，上限 45 分钟、20000 新增 token（估算，未实测）、一名 review-auditor、两轮验证，含完整工厂检查和收尾。本机 provisional 系数 0.152 点/万 token，预计周额度约 0.3%，不作为实测扣减。超额前停止新增工作并保存进度。

2026-10-20 用户明确选择“扩至 90 分钟，完成验证”：累计时间上限调整为 90 分钟，其余预算和授权不变。原因是 D 盘空间耗尽后的构建输出迁移与重编译；不扩大功能范围或 Git 授权。

随后实际 release CLI 的小型对照发现：长缓存路径（264 字符）的程序可读但 Windows 不能启动自检，导致回退冷构建。用户再次明确授权“扩大预算，完成修复与验证”：累计上限 150 分钟、30000 新增 token（估算，未实测，约周额度 0.5%），仍一名审计、最多两轮验证、不扩大 R04/R02/R01 或 Git 范围。短路径隔离副本自检属于 R02；原件与副本都需验证字节，自检后删除仅本轮创建的副本，未知占用保留。

第二轮完整回归发现两项测试模拟器未识别短路径 Probe，适配后定向通过。用户明确授权“扩大预算，完成最终完整验证”：累计执行时间上限 220 分钟、35000 新增 token（估算，未实测），允许第三轮完整验证，沿用同一名独立审计。等待用户预算决定期间未运行验证，不计执行时间；功能、Git 和外部项目授权不变。

第三轮 fixture 的四项失败经验证环境调整后均定向通过；用户明确授权“扩大预算，完成正确环境的完整验证”：累计执行时间上限 300 分钟、40000 新增 token（估算，未实测），允许在正确环境重跑完整检查与隔离构建。仍仅 R04/R02/R01、同一名独立审计、不提交、不推送、不发布。

## 事实、目标与保留契约

基准 main@76643c9e。四个下游对话中单次 plan/Apply 约 251–450 秒；Stratus、Causis、Holdem 的重复 plan 与 Apply 指纹各自相同。updater 编译准备漏分项计时。

1. R04：补齐 updater 准备、编译、自检和项目同步规划、内置产物、项目 Hook、Apply 耗时及实际 Cargo 构建/验证缓存命中计数。
2. R02：复用现有受管缓存能力，跨 plan/重规划/Apply 复用产物，包含项目 Hook。保留源码、lock、配方、自检、环境身份、实际字节检查、依赖验证、漂移阻断、锁、回滚、最后写戳和收据字段语义。计划不写项目目标；派生缓存放受控临时目录。无法建模的外部输入正常构建。
3. R01：updater 仅构建 CLI；以已验证的来源、环境、字节和自检复用已安装产物，缺少证据正常构建。不缓存远端来源、不自动修复 runtime。

## 验收与边界

定向测试覆盖重复调用不重复构建、输入/环境变化、损坏与自检失败、项目 Hook、单任务编译设置及计时计数。按 development-completion 完成 dogfood 一致性、受管构建、完整测试/fixture、独立审计与本地准备；准备不得提交、推送或改变当前版本。

真实下游、自然 Hook、用户级安装未授权，明确未验证。优化收益必须以同方法复测，不能把模拟构建次数或历史耗时作为提速实测。

## 实施与验证

实现、独立源码审计、完整工程检查和隔离目标构建已完成。当前等待用户验收；此前失败与环境恢复证据按时间顺序保留如下。

独立 review-auditor 已复核实际差异，初审发现并修正了 lock 身份遗漏、updater 编译 cwd 与探测不一致、本地过程宏闭包和失败阶段计时。最终源码审计无剩余阻断项；工程检查与真实下游仍单独判定。

验证遇到 D 盘空间为零、链接 PDB FILE_SYSTEM 和 fixture 编译 no space on device。只删除已逐项证明在本轮开工后创建、无 reparse point 的 17 个 incremental s-* 子目录；历史缓存与源码保留。随后测试输出改到当前任务 C 盘目录并禁用增量缓存。模板定向测试首次因 fixture Git init 权限失败，不计为功能失败或通过；完整测试使用相同受管命令及必要权限继续。

一次 build-assets 由运行中的安装目标 CLI 启动，Windows 替换自身失败并回滚；改用同一受管 workspace 的独立 release 构建入口重试，未改变 ACL 或结束其他进程。

长路径修复前的两组旧输入验证已通过执行会话 Ctrl-C 正常中断，避免继续认证已失效输入；已有通过子项不冒充最终结果。最初小型基准合同没有普通资产，Apply 被 baseline 拒绝并回滚，补足合法基准输入后再对照。

## 修复后定向证据

Windows x64、Rust/Cargo 1.94.1、release CLI 2.6.0，代码为本轮未提交工作区；产物源码指纹 `sha256:3301635471086333654434d90d2dc503cc6575ed61fa866969470bb1d00314a2`。基线 clean。缓存机制 17 项及 Windows 超 260 字符缓存启动 1 项通过；后者实际执行短路径副本并确认缓存原件不变。独立源码审计无剩余阻断项。

小型离线对照（每阶段 1 个样本）：1 个无依赖独立 package Hook、1 个普通受管文件，全新项目 fixture。使用实际 `project-sync --mode update --output-format combined`，两次 plan 后 Apply，三阶段 aggregate fingerprint 相同。

| 阶段 | 外部进程耗时 ms | CLI elapsed_ms | Cargo 构建调用 | 验证命中 |
|---|---:|---:|---:|---:|
| 首次 plan | 3570.7 | 3053 | 1 | 0 |
| 紧邻 replan | 1321.1 | 1296 | 0 | 1 |
| Apply | 1409.0 | 1382 | 0 | 1 |

计时与断言保存在本任务 C 盘 fixture 的 `reuse-benchmark/measurements-fixed.json`。这是小型离线场景，不代表四个真实下游或用户指令端到端提速；真实安装与耗时复测仍未验证。

后续使用同版本已安装旧 CLI 与工作区新版 CLI，在同一机器上交替执行三组全新 fixture；未更新用户级安装。外部进程耗时中位数如下，原始结果为本任务 C 盘 `reuse-benchmark/measurements-alternating.json`。

| 阶段 | 旧 CLI 中位数 ms | 新 CLI 中位数 ms |
|---|---:|---:|
| 首次 plan | 1224.8 | 2320.9 |
| 紧邻 replan | 1149.0 | 1150.3 |
| Apply | 1275.3 | 1368.0 |

这个极小、无依赖 Hook 场景的首次 plan 多约 1.1 秒，warm 阶段接近。新版三阶段实际 Cargo 构建为 1/0/0、验证命中为 0/1/1，证明重复构建被消除，但缓存验证有成本，不能据此宣称普遍提速；四个真实下游的重型流程仍未复测。

## 第二轮完整检查结果（未通过）

2026-10-20 北京时间 11:54，正式受管准备在 dogfood-tests 失败后停止；baseline、metadata、manifest、structure、mirror-drift 五项已通过。CLI 16 项通过；core 383 项通过、2 项失败、11 项条件忽略。factory-fixtures、后续 Hook 测试与隔离目标构建未取得本轮完整通过收据，当前为 not-prepared。

失败日志：`.runtime/bridgeforge-codex/release-preparation/blobs/267151766e67fdaefbb8c7bf2e97a6c42a3284a1ba1fe825afda7efd003e6a89`。

两项失败为 `factory_reuse_self_test_failure_rebuilds_and_drift_blocks_apply` 与 `mixed_cache_hit_and_build_rejected_commit_restore_all_targets`：旧测试模拟器仅识别安装位置和原缓存目录，短路径 Probe 不触发故障注入与命中计数。仅适配测试识别本机临时目录直接子目录中的 `bf-probe-*`，保留原有重建、漂移阻断、构建/命中计数与完整事务回滚断言，生产源码未再改动。

适配后两项分别定向通过（4.83 秒、34.60 秒）；同一名独立审计复核测试适配通过，未运行测试。最终开发输入指纹为 `sha256:0bc2d4117b45edfc973b5c4c7bc32a1a1892c7d83b98b114b44ac3c06159c8ee`。定向通过不能替代最终完整回归；用户已授权预算调整，第三轮从此指纹继续。

本轮经验：Windows 可读取超长路径文件不代表可启动；短路径副本需要与原缓存双向核对字节，并只清理能够证明归属的副本。源码、Cargo.lock、实际编译 cwd 和环境必须共同绑定复用身份。自检路径改变时，测试的故障注入也应沿真实执行路径更新，不能删除失败或回滚断言。父子阶段计时存在包含关系，不可直接相加。

## 第三轮完整检查结果（环境阻塞，未 prepared）

2026-10-20 北京时间 13:40，完整 workspace 通过：CLI 16 项、core 385 项、Hook 24 项，共 425 项通过、0 项失败；另有 13 项条件忽略。忽略项包括子进程 helper、显式性能/现场日志实验及未单独配置的 PDM/Poetry/uv 适配验证。dogfood-tests 收据耗时 2327671 ms；前五项基础检查同样通过。

北京时间 14:00，完整 factory fixture 返回 89 项通过、4 项失败、6 项条件忽略，准备因此停止，隔离目标构建未执行。日志为 `.runtime/bridgeforge-codex/release-preparation/blobs/fc1b07e5a45e77eed13940a54ea3df7335e75c7c1eb52e79954a981d504ea80a`。

三项失败分别在隔离工厂 Git add、Memory fetch、Memory hash-object 报 `Filename too long`；本轮迁移后的 C 盘临时目录过深。另一项 updater 隔离测试找不到 `Get-FileHash`：以 .NET Process 直接启动 Windows PowerShell 实证，继承 PowerShell 7 模块路径时失败，仅为该子进程移除 `PSModulePath` 后恢复系统默认 Windows PowerShell 模块并成功。

恢复环境仅限验证进程：继续使用原 C 盘 Cargo target；TEMP/TMP 改为仓库内短目录 `D:\Quant\BridgeForgeCodex\.runtime\t`；移除当前进程继承的 `PSModulePath`。未改生产源码、全局 Git/PowerShell 配置或真实用户安装。updater 隔离复用、Memory 远端推拉、Memory 同路径冲突、真实隔离工厂事务四项定向复测均通过，分别耗时 8.65、4.30、18.42、358.17 秒。

正式检查缓存绑定环境，不能将旧环境通过记录改写为新环境通过。用户已将预算扩到 300 分钟，正确环境下重新运行完整检查与隔离目标构建，预计 60–75 分钟；最终收据取得前仍不记为工程验证完成。

## 仓库内短 TEMP 的反例与独立环境研判

北京时间 14:51，本轮 CLI 16 项、core 385 项再次全部通过；Hook 22 项通过、2 项失败、2 项条件忽略。首个失败为 `session_failed_git_query_is_unknown_not_clean`：fixture 本身没有 Git，但 TEMP 位于工厂仓库内部，Git 向父级发现真实工厂仓库，不能代表“无 Git 仓库”场景。随后 `ENV_LOCK` 中毒导致另一项失败。未改 Hook 产品代码。日志为 `.runtime/bridgeforge-codex/release-preparation/blobs/22379defb110f76a6b1ca6b15220b5a131fb5b1cde63d756742707ae2e00f04c`。

主对话承认此前环境判断不完整，按 escalate 停止修改与长检查，并由同一名 `/root/review_build_reuse` 独立只读复核。被挑战的核心假设是“换成较短 TEMP 即能隔离”：仓库内 TEMP 会继承工厂 Git；另查明 `C:\Users\bridg\.git` 存在，用户 Temp 也不能直接视作无 Git 的隔离位置。

独立预检候选 `D:\bfc-01a156`：目标尚不存在，D 根非 reparse，根下无 `.git`、Cargo 配置或 Rust toolchain 覆盖。以现有复制规则扫描，工厂最长相对路径 135 字符，候选 fixture 合计约 205 字符，低于 260。检查时 C 盘约余 4.05 GB、D 盘约余 37.50 GB；保留已编译的 C Cargo target，受管隔离构建使用显式 target 并落到 D TEMP。

推荐在用户明确同意该外部临时目录与预算后，先验证 Git 祖先隔离、原生 Windows PowerShell 模块、完整 Hook 测试和四项 fixture 回归；全部通过后才启动一次完整 prepare。备选为保存当前实现与分项证据，停止验证。候选目录未创建，后续验证未启动，当前仍未 prepared。

用户已明确选择采用独立建议并扩大预算：仅新建 `D:\bfc-01a156` 为专用隔离验证临时目录，累计执行上限 360 分钟、45000 新增 token（估算，未实测，按既有系数约周额度 0.7%），按 E01 环境探针 → E02 完整 Hook → E03 四项 fixture → E04 完整 prepare 顺序继续。任一前置验证失败即停在该项；不修改生产代码、全局配置或真实下游，不提交、不推送、不发布。

E01 实际通过：D 根祖先覆写检查通过；专用目录记录本任务 owner；Git `rev-parse --show-toplevel` 返回 128 且明确为 `not a git repository`；直接 .NET 子进程中的 Windows PowerShell Desktop 能计算 owner 文件 SHA256，TEMP/TMP 与 Cargo target 均符合计划。E02 完整 Hook 通过（24 项通过、2 项子进程 helper 忽略，4.99 秒）。E03 四项均通过（9.61、4.38、17.13、353.93 秒）。前置检查通过后，按同构环境启动 E04 完整 prepare。

## 最终工程交付（2026-10-20）

北京时间 16:30，E04 正式返回 `status=prepared`、`checks_passed=7`、`checks_executed=7`、`checks_reused=0`。开发指纹仍为 `sha256:0bc2d4117b45edfc973b5c4c7bc32a1a1892c7d83b98b114b44ac3c06159c8ee`，独立审计 verdict 为 passed，无剩余源码阻断项。

| 检查 | 最终结果 |
|---|---|
| baseline、metadata、manifest、structure、mirror-drift | 全部通过，含模板与 dogfood 源码全树一致 |
| 完整 workspace | CLI 16、core 385、Hook 24 项通过；0 失败、13 条件忽略；2442591 ms |
| 完整 factory fixture | 93 项通过；0 失败、6 条件忽略；1602083 ms |
| 隔离目标构建 | CLI、Hook 与两份构建收据均生成、核验，准备目标 2.6.1 |
| 独立审计 | 同一名 review-auditor 完成源码、测试适配和环境假设复核；不冒充审计者执行了测试 |

两组完整测试合计 518 项通过、19 条件忽略。忽略项为子进程 helper、显式性能/现场实验、未单独配置的 PDM/Poetry/uv 工具，以及未授权的 Assist fixture；不记作已执行。正式收据与内容哈希日志位于 `.runtime/bridgeforge-codex/release-preparation/current.json` 及同目录 blobs。最终记录类文档补充后，使用同构环境轻量复核并重新绑定收据，不重复完整回归。

源码、通用产品事实源与自身 dogfood 均已同步；变更保持在本地工作区。HEAD 仍为 `76643c9e8c2c405ac24709987684a74542d04e94`，根 VERSION 仍为 2.6.0，CHANGELOG 未预写。2.6.1 只存在于隔离准备产物，不代表已发布。本轮未暂存、提交、推送、发布、更新用户级安装或写真实下游。

R04 已提供准备/构建/自检/规划/Apply 分项计时和实际构建、缓存计数；R02 已在离线真实 CLI 证明 plan/replan/Apply 构建为 1/0/0，并覆盖 Hook、损坏、漂移和回滚；R01 已覆盖仅编译 CLI、验证既有产物复用与来源/lock/环境失效。极小 Hook 的首次 plan 额外约 1.1 秒、warm 阶段接近的边界仍成立，不宣称普遍提速。

真实四个下游的升级耗时、用户级实际更新、自然 Hook/桌面运行仍未验证；R03/R05 暂缓。用户尚未验收，TODO-013 保持进行中，不结算完成。已授权测试临时根 `D:\bfc-01a156` 保留本轮归属标记与派生缓存，不触碰其他临时目录或业务项目。

当前授权为累计 360 分钟、45000 token 估算上限；工程收口在时间上限内完成，token 使用未实测。此次耗时主要来自多轮完整回归、磁盘迁移及验证环境纠正；不将工厂验证耗时算作下游维护提速收益。

## 用户验收与发布授权（2026-10-20）

用户本次明确调用 `$summary accept` 与 `$git-sync release`，验收当前会话已交付的 R04/R02/R01，并授权连同随后 R10/R11/R08 的已验证改动，经受管 release 升版、提交、推送。此前“不提交/不推送/不发布”是实施阶段边界，由本次明确授权更新；不扩展到真实下游写入、用户级安装或暂缓的 R03/R05。

TODO-013 已达到约定的工程交付条件并取得用户验收，状态更新为已完成。原工程收据及离线对照见上文；最新覆盖两轮改动的完整检查为 workspace 435 通过、13 跳过，factory fixture 93 通过、7 跳过，均无失败，七项检查及隔离目标构建 prepared，独立审计 passed。本次总结不补跑测试或构建。

验收不等于真实四个下游的升级耗时、用户级实际更新、自然 Hook/桌面运行已验证；极小 Hook 首次 plan 较慢的实测边界继续保留。缓存验证、短路径副本、临时目录隔离与 PSModulePath 经验已在本卡沉淀，不新增重复 Rule/Hook/AGENTS 约束，不写 Memory。该 topic 可作为归档候选，依据为 TODO-013 工程验收完成；仅另行调用 `$archive-scan` 处理，本次不移动文档。实际发布版本、提交和推送状态以受管同步最终收据为准。
