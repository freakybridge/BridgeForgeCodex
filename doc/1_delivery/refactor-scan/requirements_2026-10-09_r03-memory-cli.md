---
lifecycle: completed
validation_status: verified
---

# R03：集中 Memory 命令适配与编排

> 本卡保留 R03 阶段证据，最新累计工程收据见 [R04 工程结果](requirements_2026-10-10_r04-artifact-cache.md)；用户验收见下节。

## 用户验收（2026-10-10）

用户明确调用 `$summary 同意验收`，确认当前累计 R01–R04 交付符合需求。R03 工程范围已验收，
状态更新为 completed / verified；下文阶段记录的未验收、未授权 R04 等措辞保留为历史事实。
累计源码的工程证据与本次只读复核见 [R04 验收记录](requirements_2026-10-10_r04-artifact-cache.md)。
真实下游、用户桌面/runtime 未执行，不计通过。另获 `$git-sync release` 授权，结果以受管同步
收据为准；没有关联的既有 TODO 可结算，未归档。

## 目标与授权

依据[工厂重构扫描](scan_2026-10-09_factory-refactor.md)的 R03，按同一 Memory 命令职责集中
CLI 代码，缩小修改、定位和审查范围；main.rs 保留命令分发和公共参数工具。
这是内部模块整理，不引入命令框架，不改变用户行为，不承诺性能提升。

用户询问 R03 作用和 confirm 是否必要后，明确“那继续 $confirm”，授权事实核验、需求整理、
确认卡及索引记录。随后用户明确“确认，开始”，授权按本卡实施 R03、受管构建、工程检查和
独立审计；不包含提交、推送、release、真实下游或原生 Memory 写入。
下面采用 memory_commands.rs 作为普通实现建议，不将技术建议或保存记录改写成用户开工决定。

## 确认时现场与已核实事实

main，HEAD cbba10c0ec0670370c10ae2ce34b68c03b05c6db，VERSION 2.1.0。
工作区包含先前 R01/R02 源码、manifest、测试和文档改动；保留原状，不归为 R03 新增成果。
R02 已有完整工程准备记录（候选 2.1.1、指纹 ec470e90…222547），R01/R02 均待用户验收，未发布。
以下文件位置及函数边界是 R03 实施前事实，实施时只复核相关变化。

| 事实 | 当前证据 |
|---|---|
| CLI 尚未分出 Memory 模块 | templates/hooks/crates/bridgeforge-cli/src 目前只有 main.rs |
| 公共参数工具在根入口 | main.rs:25–46 的 value、values、has、path_value；blocked 是共享错误格式入口 |
| Memory 代码集中但职责混在入口 | main.rs:420–1162 含 13 个 Memory 路径/授权、结果、迁移、worker 和通知函数 |
| 命令分发唯一入口 | main.rs:1261 的 run，其中 memory-sync 分支只向 memory_sync 传入去掉顶层命令后的 args |
| 单测包含混合职责 | scripts/tests/unit/cli.rs 共 14 项：10 项 Memory 场景与 4 项通用入口/参数场景 |
| 私有调用边界 | 13 个 Memory 函数均为私有 fn；实际调用在当前 main.rs 与 CLI 单测，没有公开库 API |
| 底层业务位置 | 原生 Memory 同步、授权、恢复、锁及后台启动基础能力仍在 bridgeforge-core::memory |
| R02 结果需保留 | CLI 的 hook-attempt/hook-runtime 写入已改用公共 persistence，不恢复旧工具依赖 |

13 个拟迁移函数：memory_paths、memory_path_identity、authorized_memory_remote、authorized_remote、
memory_operation_outcome、memory_failure_outcome、migrate_memory_state、record_hook_attempt、
hook_failure_outcome、launch_memory_worker、memory_sync、memory_state_error、memory_hook_notice。

已核对子命令分支：setup、decline、maintain/repair-hook、mark、status、ack-alert、reconcile、
resolve、kick、worker、hook-run；未知或缺失子命令的错误路径也属于保留范围。
memory_hook_notice 复用 status 查询结果；worker 启动使用当前可执行文件、保留 token/路径参数，
spawn 失败会释放 reservation。固定 Codex home 路径与授权 remote 检查必须保留。
不从函数名推定通知、worker 或其他分发已经被状态查询测试全部覆盖。

## 拟定模块边界

1. 新增 `templates/hooks/crates/bridgeforge-cli/src/memory_commands.rs`，集中上述 13 个函数及其
   必要 imports。按原行为迁移，不在移动过程中更改条件、顺序、字段或错误处理。
2. main.rs 注册私有模块，在既有 memory-sync 分支转交命令参数；保持顶层 argv 切片语义、
   run/emit/main、自检、其他命令处理与 value/values/has/path_value/blocked 等共享工具。
3. 仅将命令处理入口开放到父模块所需可见性，其他 Memory helper 保持私有；通过同 crate 的
   共享工具使用既有参数/错误格式，不另建一套解析器或第二张角色/命令路由表。
4. 对 tests 的私有访问采用对应模块测试布局。拟将 10 项 Memory 场景保留原断言移入
   scripts/tests/unit/cli_memory.rs，4 项通用场景留在 cli.rs；Memory 测试继续通过真实主入口 run
   核对分发，并在所属模块访问私有 helper。具体拆分是实现安排，不为测试公开业务内部函数。
5. 同步 `.codex/hooks/crates/bridgeforge-cli/src` 镜像和受管登记；测试仍在 scripts/tests/**，
   不向下游分发工厂专属测试文件，不对工厂执行下游 project-sync adopt/apply。

用户命令 → main.rs 的 run 路由 → memory_commands → core 的 Memory/persistence 服务。
这是一项模块提取，不移动底层 Memory 业务、不改 R02 公共持久化实现，也不增加异步编排层。

## 必须保持的行为与不做

| 用户动作/数据 | 保持要求 |
|---|---|
| 相同命令与参数 | flag、默认值、重复参数处理、argv 切片和参数报错相同 |
| 查询状态 | 同样的 JSON 字段、类型、路径别名、损坏状态处理；只读查询不新增落盘动作 |
| 授权、迁移与同步 | consent、固定路径范围、私有远端和 approved remote 检查，确认标记及恢复顺序相同 |
| 通知与确认告警 | 中文通知正文、SessionStart 条件、alertId、首次/重复/恢复后的告警行为相同 |
| kick/worker/hook-run | reservation/token/PID 与后台无窗口启动、重复 worker 复用、失败释放和健康记录相同 |
| CLI 结果 | CommandOutcome 的 code/stdout/stderr/receipt、错误文本与退出码相同 |
| 其他命令 | 自检、doctor/check、project-sync、git-sync、manifest/build-assets、batch 等路由保持 |

不因为文件长而任意拆其他命令，不引入第三方 CLI 框架或新 crate，不改命令名称或状态展示，
不重写 Memory 数据结构、同步算法、通知规则、原子持久化或隐藏进程实现。
不扩大到 R04 缓存；不运行真实 setup/reconcile/worker 来写用户 Memory 或访问生产远端。
不新增 AGENTS/Rule/Skill/Memory、不自动验收或归档 R01/R02。

## 影响文件与传播四问

主要为模板 CLI main.rs、新 memory_commands.rs、对应 dogfood、现有 cli.rs 和拟新增
cli_memory.rs 的测试归属，以及受管 manifest。CLI Cargo manifest 与依赖原则上不需变化；
若实际需要额外依赖或框架则超出本卡，应先说明证据和范围。

| 问题 | 方案 |
|---|---|
| 哪一层 | 通用产品层 CLI 内部整理；确认卡/索引为元文档 |
| 通用或工厂专属 | 通用 CLI 行为进入 templates，工厂测试仍在 scripts/tests，不污染下游 |
| 版本与 CHANGELOG | 发布时按实际变更记录 `[product]` 并交由受管 release 处理；本次确认不写版本或发布记录 |
| dogfood | 需要，源码全树逐字一致、manifest 登记和受管产物验证 |

## 规模、预算与停止点

M 级：有明确模块提取边界，但涉及多个命令分支、私有测试访问和后台/通知副作用；
需要工厂独立审计与完整工程收口。镜像及登记数量不单独抬高规模。

R03 从本次确认开始累计：45 分钟、约 20,000 新增 token（估算，含主/子 Agent、实现、受管
构建、项目强制检查、合理修复和环境重试缓冲，token 未实测），最多 1 个 review-auditor、
最多 2 轮验证。当前仅主对话有限读取与记录，0 个 Agent、未运行构建或测试；后续交接不重置。
R01/R02 历史预算、测试和认证收据不能直接替代 R03 实施验收。

预计消耗约 20,000 token，占周额度约 0.3%。沿用当前会话已接受的历史参考
“40,000 token 约占周额度 0.6%”，标为 provisional，不是官方扣减或实测结果。
缺可靠计量不伪报精确已用/剩余 token。预计超额时暂停超额动作并说明选择。

发现需要改变任何参数/输出/错误/授权/通知/后台语义、另建框架或补未知消费者契约时，
仅暂停受影响动作；普通 imports、模块可见性和测试归属自行处理，不强行重访谈。

## 验收与验证计划

| 编号 | 断言/场景 | 方法与当前状态 |
|---|---|---|
| A01 | 13 个函数按职责归位，根入口保留路由和共享工具 | 已实现，当前版本构建及独立源码审计通过 |
| A02 | 14 项既有 CLI 测试的断言和覆盖保持，无漏掉通用分发 | 14 项原测试通过，函数体/属性/断言迁移审计通过 |
| A03 | 状态/路径别名/损坏状态/只读边界保持 | 原有 CLI 状态/文件断言与完整 workspace 均通过 |
| A04 | 授权、通知、worker、重复触发、恢复与失败分支保持 | 原有 CLI/Core 行为测试、完整 workspace/fixture 及源码等价审计通过 |
| A05 | 非 Memory 命令、缺参数及未知命令分发不回归 | 原有通用测试通过；根入口及原错误分发代码等价审计通过 |
| A06 | 模板/dogfood 一致，登记、当前产物及完整工程证明有效 | 7 项配置检查退出 0，prepared 收据绑定同一审计指纹 |
| A07 | 真实用户环境、runtime、验收与发布 | 未授权或未执行，不计为通过 |

现有 CLI 场景实际核验过源码：授权范围拒绝、各生命周期排队/复用、不重复通知、失败与 busy、
损坏状态与缺 baseline、冲突/滞留告警及 acknowledgement、repair-hook 事前授权、状态字段、
Windows 分隔符别名、只读状态和告警重置；另有预览写入拒绝、metadata gate、自检和重复 batch
参数测试。不把局部状态测试当作所有 worker/通知覆盖，也不把历史 14 项通过写成 R03 通过。

实施后按项目清单证明镜像一致，再跑 dogfood 完整 workspace 与工厂 fixture；不在相同模板上
重复完整 workspace。构建必须关联当前源码，真实审计绑定当前输入，prepared 成功后才交付。
准备运行前固定源码、需求及经验记录，运行期间不改文档；记录类更新按既有机制做轻量核验。
不为纯搬移新建测试框架或只检查“符号换位置”的测试，不伪造 passed 或准备记录。

## 假设、风险与交接

采用 memory_commands.rs 名称、最小父模块入口和按职责拆测试的方案；不视为用户逐项命名决定。
主要风险是移漏 helpers/imports、argv 传递、测试私有访问及 worker/通知失败路径变化；使用
原函数体迁移、真实主入口测试、完整工程检查和独立审计验证等价性。

目前没有需要用户额外选择的业务未决项。实施授权已取得，携带本卡、当前 R01/R02 改动和同一
R03 预算进入 develop。
不重复确认已有目标，不因确认卡存在推定开工；不提交、推送、发布或结算其他条目。

## 实施记录

已将原 13 个函数提取到私有 memory_commands 模块，仅 memory_sync 对父模块可见；
主入口在原分发位置转交同样的 args[1..]，其余命令、共享工具和 emit/main 保持。
10 项 Memory 测试迁入 cli_memory.rs，4 项通用测试留在 cli.rs，保留原函数体及断言，
Memory 测试仍通过真实根入口 run 核对分发，不为测试公开 helper。
模板与 dogfood 同步修改，未触碰 R02 persistence/core 业务实现。
移动通过文件工具直接写入，中文通知正文未嵌入 shell 写入命令或动态执行；实施前快照仅保存于本机
忽略目录 .runtime/r03-memory-cli，用于本轮等价审计，不属于产品资产或新事实源。

受管 manifest 重建成功，后续 --check changed=false；完整既有 CLI 测试已运行：

```powershell
cargo test --locked --config scripts/tests/factory-cargo.toml --manifest-path .codex/hooks/Cargo.toml -p bridgeforge-cli -- --test-threads=1
```

退出 0，14 passed、0 failed、0 ignored，测试执行 0.40 秒（含编译命令总计约 3.58 秒）。
10 个 Memory 和 4 个通用测试的原有断言全部保留，无缺失测试或为搬家新增的符号镜像测试。
源码、新测试布局和受管登记已经修改。当前版本受管构建成功（退出 0），日志为
`.runtime/r03-memory-cli/build-assets.log`，CLI/Hook 两份 schema_version=2 收据的源码树哈希为
`sha256:8f8da79d082b669e6231f15cee56d07ff074cc1a4ebe085be85a44e5f91f50eb`。
用已核验字节一致和自检的受管 CLI 副本运行 build-assets，避免 Windows 自身替换占用，
未改构建逻辑或依赖、没有构建失败或源码返工。

唯一 reviewer `/root/r03_audit` 的独立源码审计 passed：基准快照与 HEAD 加恰好两处 R02
persistence 引用变化一致；13 个原函数逐字保持，唯一入口仅增加 pub(super)；根入口其余
代码不变。14 项旧测试的函数体、属性、断言逐项一致；50 个受管资产、全树镜像和两份
manifest 一致，新模块的显式 ID/target/whole/hash 正确。审计指出的 R03 索引旧授权状态已更新。
真实审计记录 `.runtime/r03-memory-cli/audit.json` 绑定当前开发指纹
`sha256:90e5f811da9ef624994df69b0a5ba9750b635652a8f626ab1b120bbe84fd37c7`。
旧 R02 准备收据因本轮源码变化已 stale，符合预期，不当作 R03 认证。

按项目配置执行受管
`git-sync --prepare-release --message-file .runtime/r03-memory-cli/message.txt --audit-file .runtime/r03-memory-cli/audit.json`，
日志为 `.runtime/r03-memory-cli/prepare.log`，实际一次执行成功，退出 0。运行前固定记录文档，
期间未更新，无输入漂移、无源码返工。

经验：混合测试不能整份移走后让通用分发测试失去原入口。按职责归位，并继续从真实 run
调用 Memory 分支，在所属模块访问私有 helper，可同时保留入口覆盖与模块封装。
该原则与既有扫描边界一致，不新增 AGENTS/Rule/Hook/Memory。

## 最终工程结果（2026-10-09 准备，2026-10-10 复核）

收据 `.runtime/bridgeforge-codex/release-preparation/current.json` 为 `status=prepared`，
`target_version=2.1.1`、`checks_passed=7`，开发指纹为
`sha256:90e5f811da9ef624994df69b0a5ba9750b635652a8f626ab1b120bbe84fd37c7`，与真实审计一致。
它绑定当前累计 R01–R03 工作区；旧阶段记录保留为历史证据，不替代 R03 本轮认证。
2.1.1 是隔离快照候选，工作区 VERSION/运行 CLI 仍为 2.1.0，没有执行发布。

| 检查 | 实际结果 | 日志 SHA256 后缀 |
|---|---|---|
| baseline | clean，退出 0 | 5a81c1d27042e8decc7942825bd406a0ec7cb53260b226d6a9b79bea094e8258 |
| metadata | issues/warnings 为空 | 2f4914ad120aad474ada903be1a4a2b10cd859eab34c0ef951d8ad6c4ca2efdd |
| manifest | changed=false | 4f26cd005522df1df06e7e991cad2699681b25283e63142caef1b1d6c07e26e5 |
| structure | errors 为空；旧归档候选仅 advisory | 4dff2f3620921796cd799e7baec44610edce589fdee17a726d6a107fde21c227 |
| mirror-drift | 全树逐字一致，1 passed | 4668d1b3ce42d1e38b10b3f0bc4fffc3a9e17eea144d56db659700ee858859e3 |
| dogfood-tests | CLI 14、core 192、Hook 22 passed；合计 228 passed、0 failed、5 ignored | 2ffd77d71d89ff671666962bb3e487d472883f462cd47344f8aaaf4cad57bec8 |
| factory-fixtures | 89 passed、0 failed、4 ignored | f219d1cbfb661a8188bbf4e22245c1dccc625dac9d6744d9ca35619cb5014e58 |

日志位于收据同目录的 `blobs/<后缀>`。workspace 命令耗时 572.242 秒，fixture 命令 423.241 秒，
不将工程检查命令时间冒充端到端总耗时或本次性能收益。14 项 CLI 原测试包含 10 Memory 与
4 通用入口场景，断言和 Windows 条件未删改；全量还验证累计 R01/R02 行为及其他命令流程。

workspace 5 项 ignored 为子进程辅助入口和显式 live log 检查；fixture 4 项 ignored 为普通
同步性能实验、high-cost 原生第十轮试验、进程辅助入口和需显式源路径授权的 Assist 迁移。
不把 ignored 当作全部单独执行通过，也不顺带运行这些实验或真实项目迁移。

实际使用唯一 `/root/r03_audit`，没有实质代码修复，准备执行一次；从本轮确认累计预算，未因
Skill/Agent 切换重置。token 未实测，整体 wall time 未精确计量，不伪报预算实耗。
收据认证后只更新 record_documents，再用原机制核验有效性；同一 Agent 最终复核日志、审计和
候选 blob，保持源码审计与工程证据一致，不重复测试或另开 Agent。

源码提取、产品传播、dogfood、登记、当前产物、14 项 CLI 测试、完整工程检查与独立源码审计
均已取得证据；真实下游写入、真实 runtime smoke、用户桌面试用/验收及提交/推送/发布未执行。
HEAD 保持基准，暂存为空，R01/R02 验收状态不变，R04 未授权，不归档或修改规则。

2026-10-10 最终独立收据复核通过：同一 `/root/r03_audit` 核对审计字段、7 份检查日志和
4 个候选产物 SHA256 及候选收据引用，实际测试计数吻合；299 项非记录输入当前字节无漂移，
HEAD/VERSION/空暂存与记录状态一致，原索引 P3 已修。没有工程收口阻断，未重跑测试。
准备记录实际落盘时间为 2026-10-09 23:57:49（Asia/Singapore）；本卡文件名保留确认启动日期。
