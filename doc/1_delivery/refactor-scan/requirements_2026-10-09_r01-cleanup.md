---
lifecycle: completed
validation_status: verified
---

# R01：清理无调用方的旧产物注入路径

> 本卡保留 R01 阶段证据。后续 R02/R03 源码变更已重新执行累计工作区的完整工程检查，
> 最新累计工程证据见 [R04 工程结果](requirements_2026-10-10_r04-artifact-cache.md)；用户验收见下节。

## 用户验收（2026-10-10）

用户明确调用 `$summary 同意验收`，确认当前累计 R01–R04 交付符合需求。R01 工程范围已验收，
状态更新为 completed / verified；下文各阶段的未验收措辞保留为历史事实。
累计源码的工程证据与本次只读复核见 [R04 验收记录](requirements_2026-10-10_r04-artifact-cache.md)。
真实下游与用户桌面/runtime 未执行，不计为通过。另获 `$git-sync release` 授权，正式发布
结果以受管同步收据为准；没有关联的既有 TODO 可结算，未归档。

## 目标、事实与授权

候选依据为[扫描报告](scan_2026-10-09_factory-refactor.md)的 R01。用户在解释及 confirm/develop
流程说明后明确“开始吧”，授权 R01 的需求落盘、实现、受管构建、工程检查及独立审计。
不包含提交、推送、release、真实下游写入或用户验收；R02–R04 均不在本次范围。

实施基准：main，HEAD cbba10c0ec0670370c10ae2ce34b68c03b05c6db，VERSION 2.1.0，工作区干净。
实施前源码和测试引用核验表明：`WritePlan::prepare` 只向 `prepare_with_artifacts` 传入 `None`；
没有其他调用方。`write_plan` 是 git_sync 私有模块。Git 同步与独立开发准备均调用 `prepare`。
旧分支未参与当前运行，清理不改变现有调用方的输入、返回值或错误语义。

## 规模、预算与自动化边界

逻辑为单一可逆清理；机械镜像和登记文件不单独抬高规模。工厂要求独立审计与完整工程收口，
按 M 级路径推进：45 分钟、约 20,000 新增 token（估算，含主/子 Agent，token 未实测）、
最多 1 个 review-auditor、最多 2 轮验证。完整测试由项目 `.codex/development-checks.json` 明确要求，
不以通用 M 级默认规则替代项目清单。时间预算包含构建、完整 fixture 与环境重试缓冲；预计超额时
停止新增工作并报告扩大预算或缩小范围的选择，不静默追加。

本机缺少当前模型周额度系数；用户已明确选择沿用历史参考“40,000 token 约占周额度 0.6%”，
标为暂定估算。本次预计消耗约 20,000 token，占周额度约 0.3%；不是官方扣减或实测结果。

## 传播四问

| 问题 | 决定 |
|---|---|
| 层次 | 通用产品层内部重构；需求、经验记录属于元文档 |
| 通用/工厂专属 | 产品 Git 写计划内部清理，进入 templates；不增加工厂专属规则 |
| 版本/CHANGELOG | 发布时按 patch 和 `[product]` 记录；本轮不改 VERSION/CHANGELOG，不执行发布 |
| dogfood | 同步 `.codex/hooks` 对应源码及受管 manifest，证明源码全树逐字一致 |

## 拟修改与不做

1. 将实际实现保留在 `WritePlan::prepare`，移除 `prepare_with_artifacts` 包装及 `prepared` 参数。
2. 删除 `Some(prepared)` 的注入与旧报错分支，原样保留已安装产物复用及未命中构建路径。
3. 同步模板与 dogfood；通过受管 `manifest` 重建资产登记、受管 `build-assets` 构建运行产物。
4. 复用已有覆盖，不增加只检查“符号被删除”的实现镜像测试。

不实现新缓存，不移动 Memory 工具，不拆 CLI 模块；不删除独立开发准备和其收据。
保留哈希和自检验证、旧版本失效、源/lock/二进制/收据失效、漂移阻断、原子写入及事务回滚。
用户可见的命令参数、输出、发布与普通同步行为均不改变；不承诺性能提升。

## 验收与验证安排

| 编号 | 验收 | 方法/当前状态 |
|---|---|---|
| A01 | 无消费者被删除，调用接口不变 | 实现已完成，独立源码审计通过 |
| A02 | 模板与 dogfood 全树一致，登记和基线健康 | 当前版本受管构建、基线、metadata、manifest、structure、mirror-drift 均通过 |
| A03 | 复用、各类失效、旧版本、漂移和回滚行为保持 | 完整 workspace 中 7 项 `git_sync::tests::factory_` 均通过 |
| A04 | 完整自动测试及工厂 fixture | workspace 223 passed/0 failed/5 ignored；fixture 89 passed/0 failed/4 ignored；ignored 不算通过 |
| A05 | 独立审计和准备记录匹配当前输入 | `/root/r01_audit` 静态审计通过；受管 prepare-release 为 prepared，7 项检查通过 |
| A06 | 发布/真实下游/runtime/用户验收 | 本轮未授权或未执行，不计为通过 |

合理假设：当下无隐藏消费者的引用证据与私有模块边界有效；实施时若发现矛盾则重新核实，
不顺带扩大到 R04。主要风险是误动正常复用或失败保护，使用既有行为测试和真实 diff 审计验证。
完成后交付工程证据与待验收结果；`lifecycle` 保持 active，由用户明确验收后另行结算。

## 实施、验证与经验记录

已合并 `prepare` 的实现并删除无消费者的可选注入分支；模板与 dogfood 同步修改。
正常复用、未命中构建、输入核验与回滚消费者保持原样。基础检查、镜像及完整 workspace 已通过，
工厂 fixture 与最终准备记录均已取得，工程验证完成，等待用户验收。
后续只在本卡追加实际结果，不将扫描阶段验证归为本轮完成。

经验：私有 `write_plan` 模块中的包装方法只有 `None` 调用时，内联实际实现、删除 `Some` 分支
即可保持消费者行为。不要删除同名概念在独立开发准备中的真实用途，也不要把这次清理扩大为
新缓存设计；该原则与扫描报告现有经验一致，不增加 Rule/Hook/AGENTS/Memory。

环境记录：受管 `manifest` 首次部分写入后因保护目录权限失败（os error 5），以完全相同命令
窄范围提升权限重试成功；随后受管 `build-assets` 首次 Cargo 临时目录写入也遇到 os error 5，
同命令提升权限后完成编译，但事务替换阶段仍因 os error 5 回滚。源码显示 build-assets 直接
原子替换运行目标；从 `.codex/bin/bridgeforge.exe` 调用会占用自身替换目标。
改为将同一受管二进制复制到本项目 `.runtime/r01-cleanup/`，逐字节 SHA256 核对和自检成功后
执行受管 build-assets，退出 0，返回 CLI/Hook 两份 schema_version=2 收据。
当前源码树哈希为 `sha256:882ad2707d8185ac90273416643d3727bf200555b77b0ff01b4561f39399e00f`。
不修改构建逻辑，不切换语言或包装器。
权限重试不计为实质代码修复，不改变构建入口或依赖。

独立源码审计：`/root/r01_audit` 未发现需修复的问题；核对原 `None` 路径等价性、私有边界、
生产调用方、既有行为测试、46 个 Rust 源文件的镜像哈希和两份 manifest 登记。
审计 JSON 为本机 `.runtime/r01-cleanup/audit.json`；当前开发指纹为
`sha256:03eed873476266f9be07fbe9382606679fdb05465feb000f32b1255c60113e7f`。
审计未代替执行构建或测试。已开始受管
`git-sync --prepare-release --message-file .runtime/r01-cleanup/message.txt --audit-file .runtime/r01-cleanup/audit.json`，
日志为 `.runtime/r01-cleanup/prepare.log`；依次运行项目清单，不提交、推送或预写 VERSION。
第一次准备的完整 workspace 已执行通过：CLI 14 passed、core 187 passed/3 ignored、
Hook 22 passed/2 ignored，合计 223 passed、0 failed、5 ignored；core 执行 485.83 秒。
7 项 factory 行为测试全部通过。日志 blob 为
`.runtime/bridgeforge-codex/release-preparation/blobs/119849303694a34ece82772f34158a79670ba3e36192cdba2c2611ddefcfd010`。
ignored 包括子进程辅助入口和显式 live log 检查，不能记成这些入口全部单独实测通过。

第一次准备未认证：主 Agent 在完整检查运行期间更新了本卡，导致受管过程对完整输入快照的
漂移检查报 `development checks changed inputs; no preparation was certified`，退出 2。
开发指纹仍为同一值，源码和配置没有变；测试通过不等于 prepared 成功。
已固定文档内容，重新执行同一准备流程，期间不写记录文档。第二次准备已成功，结果见下方。
代码未变的重试不单独消耗验证轮次，但重复运行计入时间/token 成本。

补充经验：record_documents 在已认证准备记录的后续查询中可做轻量验证；准备正在运行时，
工具仍绑定完整输入快照。因此不能边跑准备边更新记录，即便开发指纹不变也会触发保护。

## 最终工程结果（2026-10-09）

第二次命令参数相同，输出日志为 `.runtime/r01-cleanup/prepare-retry.log`，退出 0。
7 项配置检查全部通过，真实收据为 `.runtime/bridgeforge-codex/release-preparation/current.json`，
`status=prepared`、`target_version=2.1.1`，开发指纹与上述独立审计完全一致。
2.1.1 仅为隔离快照中的候选构建；工作区 VERSION 和 CLI 自检仍为 2.1.0，没有执行发布。

| 检查 | 实际结果 | 收据中的日志 SHA256 后缀 |
|---|---|---|
| baseline | clean，退出 0 | f3f5f0876cb419e1b0c241fed43d9c879ff773ae16d107d7a6dd1059cf9e94b1 |
| metadata | issues/warnings 为空 | 2f4914ad120aad474ada903be1a4a2b10cd859eab34c0ef951d8ad6c4ca2efdd |
| manifest | changed=false | 4f26cd005522df1df06e7e991cad2699681b25283e63142caef1b1d6c07e26e5 |
| structure | errors 为空；既有归档候选仅为 advisory | 4dff2f3620921796cd799e7baec44610edce589fdee17a726d6a107fde21c227 |
| mirror-drift | 全树逐字一致，1 passed | ed424f1a79120b5f322d3ff9eef8169e603e63be09964a1a107afa0ad2a4fb57 |
| dogfood-tests | CLI 14、core 187、Hook 22 passed；0 failed、5 ignored | bc5146a79ebfbdec39eda154ea472715e48df9891a820568601f85e688477a40 |
| factory-fixtures | 89 passed、0 failed、4 ignored | 9789e7654df202d420078581c50dae6fdba9ebdcf2edb73b312cfc0beffebe36 |

日志位于收据同目录的 `blobs/<后缀>`。第二次 workspace 命令 496.302 秒，fixture 命令 441.020 秒；
这两项耗时不代表用户端到端总耗时，也不计为 R01 性能收益。
实际使用 1 个独立 reviewer；没有实质代码返工，准备期间记录漂移导致同源码重跑计入成本。
token 未实测，整体 wall time 未做精确计量，不伪报预算实耗。

fixture 的 4 项 ignored 为普通同步性能实验、high-cost 原生第十轮试验、进程辅助入口和需显式
源路径授权的 Assist 迁移。不把本轮隔离 fixture 的 CLI/runtime 行为测试当作真实下游或用户桌面验收。
源码、模板传播、dogfood、登记、当前版本产物、完整测试与独立源码审计均已有证据；
真实下游写入、真实 runtime smoke、Git 发布和用户验收仍未执行。

收据认证后仅更新已声明的记录文档，再用 `git-sync --development-status` 核验有效性；
最终证据复核使用同一 `/root/r01_audit`，不启动第二个 Agent，不重跑已通过测试。
HEAD 保持上述基准，暂存区为空；本轮未提交、未推送、未升级工作区版本、未归档。

最终独立证据复核已通过：同一 reviewer 核对收据、审计记录、输出日志的指纹与 reviewer 一致，
7 份检查日志 blob 和 4 份候选产物 blob 的 SHA256 全部匹配，实际测试计数及 HEAD/VERSION/
空暂存均吻合；没有收口阻断。其指出的历史事实措辞已改为“实施前”，未修改源码或配置。
