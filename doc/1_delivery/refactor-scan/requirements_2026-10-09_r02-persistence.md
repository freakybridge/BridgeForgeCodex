---
lifecycle: completed
validation_status: verified
---

# R02：将通用文件持久化能力归位

> 本卡记录 R02 阶段证据；最新累计工程证据见 [R04 工程结果](requirements_2026-10-10_r04-artifact-cache.md)。用户验收见下节。

## 用户验收（2026-10-10）

用户明确调用 `$summary 同意验收`，确认当前累计 R01–R04 交付符合需求。R02 工程范围已验收，
状态更新为 completed / verified；下文各阶段未验收措辞属于历史记录。
累计源码的工程证据与本次只读复核见 [R04 验收记录](requirements_2026-10-10_r04-artifact-cache.md)。
真实下游、用户桌面/runtime 未执行，不计通过。另获 `$git-sync release` 授权，结果以受管同步
收据为准；没有关联的既有 TODO 可结算，未归档。

## 原始目标与本次授权

依据[工厂重构扫描](scan_2026-10-09_factory-refactor.md)的 R02，将已经共享、但位于 Memory
领域模块内的写文件工具归入公共位置，降低 Git、批处理、运行时等模块对 Memory 命名空间的依赖。
收益是职责清晰与维护定位，不是新增持久化算法，不承诺性能提升。

用户先询问 R02 的作用、confirm 是否必要和公共模块位置；主对话提出
`bridgeforge-core/src/persistence.rs`。随后用户明确“继续 $confirm”，授权本次事实核验、
需求整理、确认卡及索引记录。随后用户明确“开始吧”，授权按本卡实施 R02、受管构建、完整工程
检查与独立审计；不包含提交、推送、release、真实下游或原生 Memory 写入。
本卡按此前提出的最小归位方案整理，兼容策略及模块名字属于下述实施方案，
不将主对话技术建议改写成用户逐项选择。

## 已核实事实与现场

当前分支 main，HEAD cbba10c0ec0670370c10ae2ce34b68c03b05c6db，VERSION 2.1.0。
工作区包含 R01 源码、manifest、需求及文档改动；保留原有改动，不把它们归为 R02 成果。
R01 已取得有效工程准备记录（候选 2.1.1），但尚未取得用户验收或 Git 发布授权。

下表记录确认时、R02 实施前的源码事实；实施后的结果见文末记录，不把历史位置描述成当前状态。

| 事实 | 当前证据 |
|---|---|
| 公共位置尚不存在 | core 的 lib.rs 模块表与源码文件列表没有 persistence 模块 |
| 原子写入已共享 | memory/mod.rs:1015 的 atomic_write、1044 的 atomic_write_json；不是多份算法重复 |
| 路径检查已共享 | memory/mod.rs:1051 的 is_link_or_reparse，当前为 pub(crate) |
| 最小依赖链 | 写入依赖 ensure_real_directory、temporary_sibling、atomic_replace_file；JSON 写入依赖 sort_json_value |
| 附属能力也被 Memory 使用 | 目录检查与临时路径命名同时用于快照、恢复和冲突目录，不能直接删除或复制一份算法 |
| Windows 特殊处理 | 原子替换使用规范化长路径和 MoveFileExW；已有长路径创建/替换及快照字节测试 |
| 当前错误契约 | MemorySyncError 保存 message，From<io::Error>/From<serde_json::Error> 保留 Display 文本；MemoryResult 是其 Result 别名 |
| 旧公开入口 | atomic_write 与 atomic_write_json 是 pub，不能仅凭仓内消费者清单删除未知外部调用契约 |

直接消费者已核对：core 的 Git 同步、Git 写计划、开发准备、批处理、项目同步、项目 Hook 及
package、runtime、file_lock、high_cost 和 release；CLI 的 Memory 授权/worker 编排；Hook util。
Memory 自身的健康状态、授权、快照、远端同步、迁移、worker 和配置也依赖上述工具。
当前引用搜索未发现 MemorySyncError 的具体 downcast 检查，但这不证明仓库外没有消费者。

## 目标行为与不做

用户执行相同命令、保存相同状态或发布保护记录时，落盘字节、写入保护、输出、错误信息、
退出码和失败清理保持不变。Memory 的授权、隐私远端验证、冲突处理和不透明字节快照不改变。

不拆分独立 crate，不新增通用 utils 集合，不实现新缓存，不优化写入算法或调整安全策略。
不移动 Memory 业务编排、Git 事务回滚、正在运行二进制的替换策略或项目安装事务。
不实施 R03/R04，不以此清理 R01 之外的历史代码；不修改 AGENTS/Rule/Skill 或用户级安装、授权与原生 Memory 正文。

## 实施方案与兼容边界

1. 在 `templates/hooks/crates/bridgeforge-core/src/persistence.rs` 集中公共实现，并在 core/lib.rs
   注册模块；新位置与 memory/git_sync/batch 同级。同步对应 `.codex/hooks` dogfood 镜像。
2. 迁移 atomic_write、atomic_write_json、is_link_or_reparse 及其最小配套能力；目录检查、临时
   文件命名、JSON 排序和平台原子替换各保留一个实现。Memory 的其他消费者必要时继续委托。
3. 公共模块使用中性的错误契约，不反向依赖 MemorySyncError 或 Memory 领域模块。
   Memory 入口负责转换回现有 MemorySyncError；保留原 Display 文本和现有 MemoryResult 签名。
   新错误类型的具体名字是普通实现细节，不为改名字改变错误或清理语义。
4. 保留 Memory 已公开的 atomic_write/atomic_write_json 为薄委托兼容入口；仓内其他业务调用方
   改用 persistence。Memory 内部按实际需要委托公共目录/路径能力，不保留第二份算法。
5. 原样保留递归 JSON 键排序、pretty 格式与末尾换行；保留长路径、链接/重解析点拒绝、文件
   同步、create_new 临时文件、防重名及失败清理。临时名称模式和异常路径信息不借重构改写。
6. Hook util 对外仍返回原有 io::Result；CLI 命令参数、JSON 字段、错误返回及后台启动语义保持。
   不把“算法移动”误当作“所有业务逻辑都需要搬到 persistence”。

调用关系：Memory/Git/Batch/CLI/Hook → persistence → 文件系统。
Memory 兼容入口 → persistence → Memory 错误转换；公共实现不得反向调用 Memory。
具体可见性按实际消费者确定：跨 crate 的写入能力需可访问，内部路径辅助不无故公开。

## 影响文件与传播四问

主要影响模板的 core/lib.rs、core/persistence.rs（拟新增）、core/memory/mod.rs、上表直接
消费者、CLI main.rs 和 Hook src/util.rs；测试按需要放在 scripts/tests/**。
机械传播包括对应 dogfood、受管 manifest 与本机运行产物。实施时只改实际调用路径，不扩展
其他文件职责。公开接口委托兼容与附属能力归位同步完成，不能先删入口再用临时绕过补救。

| 问题 | 方案 |
|---|---|
| 属于哪一层 | 通用产品层内部重构；确认卡及索引是元文档 |
| 通用还是工厂专属 | 公共可靠写文件能力进入 templates，不下沉工厂专属事实 |
| 是否需要版本/CHANGELOG | 发布时按实际变更确认版本类型并记录 `[product]`；本次 confirm 不写 VERSION/CHANGELOG |
| 是否同步 dogfood | 需要，源码和模块登记同步；受管 manifest/build-assets 完成登记和产物，不对工厂运行下游 adopt/apply |

## 规模、预算与停止点

M 级：跨多个实际业务消费者，涉及共享错误契约和原子写入的配套依赖，但目标与行为边界明确，
不做数据迁移或新架构。镜像和登记文件数量不单独抬高规模。

R02 从本次确认开始累计，后续 confirm/develop/审计不重置：默认 45 分钟、约 20,000 新增 token
（包含主/子 Agent、构建、完整工程检查、合理修复及环境重试缓冲，token 未实测），
最多 1 个 review-auditor、最多 2 轮验证。确认阶段仅主对话有限读取和记录，未使用子 Agent 或运行测试。
R01 的历史测试耗时不作为 R02 性能收益或预算实耗；它的实现授权和收据不替代 R02 验收。

预计消耗约 20,000 token，占周额度约 0.3%。沿用用户在当前会话明确接受的历史参考
“40,000 token 约占周额度 0.6%”，仅为 provisional 估算，不是官方或实测扣减。
预算具体消耗尚无可靠计量，不声称精确剩余 token。

预计超额、发现需要更改现有安全/错误语义或未知公开接口消费者时，仅暂停受影响动作，
说明证据及扩大预算/缩小范围的选择，不静默增加候选、Agent 或验证轮次。

## 验收与验证计划

| 编号 | 断言与场景 | 方法/当前状态 |
|---|---|---|
| A01 | 非 Memory 消费者直接依赖公共模块，公共模块不依赖 Memory | 已实现，独立源码审计通过 |
| A02 | 旧 Memory 公开入口、返回类型与错误文本保持 | 已实现，真实错误场景及独立源码审计通过 |
| A03 | JSON 字节、长路径、原子替换、链接拒绝与失败清理保持 | 新增 5 项风险测试及完整 workspace/fixture 均通过 |
| A04 | Git 复用/构建、输入漂移、回滚；worker 状态/锁及 CLI/Hook 消费者保持 | 完整 workspace 和隔离 fixture 均通过 |
| A05 | 模板全树与 dogfood 逐字一致，manifest/metadata/结构/当前版本基线健康 | 受管登记、当前版本构建与配置检查均通过 |
| A06 | 完整 workspace、factory fixture、独立审计及输入绑定准备收据 | 源码审计 passed；7 项检查退出 0，prepared 收据绑定相同指纹 |
| A07 | 真实下游、真实 runtime、用户验收与发布 | 未授权或未执行，不计为通过 |

复用测试入口包括 scripts/tests/src/memory_sync.rs 的原子写入内容替换与临时文件清理，
scripts/tests/unit/core_memory_remote.rs 的 Windows 长路径写入、替换和快照字节验证，及现有
Git/Memory worker/Hook/进程等行为测试。历史通过只证明既有场景存在，不证明 R02 新方案通过。
源码变更后必须取得本轮工程证据；不在模板和 dogfood 两份相同源码上重复跑完整 workspace。
准备运行前固定源码、确认卡和经验记录，运行期间不更新记录文档，避免重现 R01 的完整输入漂移。

## 合理假设、风险与交接

采用此前推荐的 persistence.rs 路径和最小兼容方案；公开兼容委托是保持已有契约的实施安排，
不是承诺永久双轨算法。可复用的可靠写入只有一个实现。主要风险是错误转换、共享临时命名/
目录辅助遗漏及 Windows 失败路径变化，通过真实消费者、字节断言和独立审计控制。

实施已获授权，沿用本卡、现有 R01 改动和同一 R02 预算进入 develop。
不重新访谈已明确内容，不因本卡存在推定开工，不将 R01 结算为用户已验收。

## 实施记录

已新增 persistence 公共实现及中性错误契约，将 Memory 入口改为委托与错误转换，迁移所有
已核实的非 Memory 写文件/链接检查调用方。JSON 排序、临时文件计数器、目录检查和 Win32
原子替换各保留一个实现；临时名称中的历史 fallback 保持不变。
测试新增于 scripts/tests/unit/core_persistence.rs，覆盖 JSON 字节与旧入口兼容、序列化失败、
替换失败清理、链接父目录拒绝和 Windows 长路径替换。已实际执行以下命令，退出 0，
5 passed、0 failed、0 ignored（0.06 秒测试执行，含编译命令总计约 5.24 秒）：

```powershell
cargo test --locked --config scripts/tests/factory-cargo.toml --manifest-path .codex/hooks/Cargo.toml -p bridgeforge-core persistence::tests -- --test-threads=1
```

dogfood 已同步，受管 manifest 重建成功。源码引用核验表明非 Memory 业务没有继续调用
memory::atomic_write/json/is_link_or_reparse；公共模块无 Memory 依赖。
当前开发指纹为 `sha256:ec470e90d948069f4212dea6fb18a65c5c60ffe26e86c33da634dc27bd222547`；
R01 旧准备记录因源码变化已 stale，符合预期，不把旧记录冒充本轮认证。
唯一 `/root/r02_audit` 独立只读源码审计通过，未发现需修复的问题：6 个提取函数仅错误类型名及
可见性变化，两个平台的原子替换函数逐字一致；49 个受管 hooks 源文件 inventory/hash、两份
manifest 和新 persistence asset 的显式 ID/target/whole ownership 均匹配。没有重复实现。
实际审计记录为 `.runtime/r02-persistence/audit.json`，绑定上述当前开发指纹。

当前版本受管构建成功，退出 0，日志为 `.runtime/r02-persistence/build-assets.log`，返回 CLI/Hook
两份 schema_version=2 收据，源码树哈希为
`sha256:e4dfacb76b37a81dfa4168870a904373635059aee8c2190b967beb4b8312ee87`。
采用同一受管 CLI 的已校验字节副本运行 build-assets，避免直接占用替换目标；没有切换语言、
包装器或修改构建逻辑。本轮没有复现 R01 的工具替换失败。

按项目清单运行受管
`git-sync --prepare-release --message-file .runtime/r02-persistence/message.txt --audit-file .runtime/r02-persistence/audit.json`，
日志为 `.runtime/r02-persistence/prepare.log`。实际一次执行成功，退出 0；准备运行期间未更新
本卡或索引，没有输入漂移或源码返工，不将旧 R01 收据或新增 5 项测试替代完整工程证据。

经验：公共写入不仅是三个入口；目录安全和临时文件名称也被快照/恢复复用。它们跟随公共实现
归位，Memory 保留转换边界，避免新公共模块依赖 Memory 或形成两套临时命名/写入算法。
此经验与既有扫描约束一致，不增加 AGENTS/Rule/Hook/Memory。

## 最终工程结果（2026-10-09）

实际收据 `.runtime/bridgeforge-codex/release-preparation/current.json` 为 `status=prepared`，
`target_version=2.1.1`，`checks_passed=7`，开发指纹为
`sha256:ec470e90d948069f4212dea6fb18a65c5c60ffe26e86c33da634dc27bd222547`，与独立审计一致。
该收据绑定当前累计 R01/R02 工作区；不把先前 R01 的指纹/日志当作 R02 通过证据。
2.1.1 仅为隔离快照候选；根 VERSION、工作区 CLI 自检仍为 2.1.0，未执行 release。

| 检查 | 实际结果 | 日志 SHA256 后缀 |
|---|---|---|
| baseline | clean，退出 0 | 9f7d85f54814dfa17a18ff0576c1f41314aaafa176439f6f622830751f90db6b |
| metadata | issues/warnings 为空 | 2f4914ad120aad474ada903be1a4a2b10cd859eab34c0ef951d8ad6c4ca2efdd |
| manifest | changed=false | 4f26cd005522df1df06e7e991cad2699681b25283e63142caef1b1d6c07e26e5 |
| structure | errors 为空；既有归档候选仅为 advisory | 4dff2f3620921796cd799e7baec44610edce589fdee17a726d6a107fde21c227 |
| mirror-drift | 全树一致，1 passed | 3380d8b44cd53018c8187a650273304937708d8c444307bb2c67eedf289a50c7 |
| dogfood-tests | CLI 14、core 192、Hook 22 passed；合计 228 passed、0 failed、5 ignored | 19774f6dd36a10cd4777f1d594f8e4b8f3a431d1b4ae3ccc3439123bedf3b2a1 |
| factory-fixtures | 89 passed、0 failed、4 ignored | 3550084fe8449bb84f3b6c0ddc6dbc3ce0a4722ec717e8cce60c22d79b84b364 |

日志位于收据同目录 `blobs/<后缀>`。workspace 命令耗时 519.104 秒，fixture 命令耗时 468.522 秒；
这是工程检查的命令口径，不是端到端总时间，也不是本次性能收益。
完整 workspace 包含新增 5 项风险测试、既有长路径/快照、Git 复用/失效/漂移/回滚、Memory
worker/锁与 CLI/Hook 行为；ignored 不算执行通过，子进程辅助入口与 live log 检查如实保留。
fixture 的 4 项 ignored 是普通同步性能实验、high-cost 原生第十轮试验、进程辅助入口，以及
需显式源路径授权的 Assist 迁移；不顺带执行这些显式实验或真实项目迁移。

实际使用唯一 reviewer `/root/r02_audit`，没有实质代码返工，完整工程准备执行一次，验证
预算未因 Skill/Agent 交接重置。token 未实测，整体 wall time 未做精确计量，不伪报预算实耗。
准备结束后只更新已声明的记录文档，再按项目机制核验收据有效性；最终证据复核由同一 Agent
核对审计、检查日志与候选 blob，不重复源码审查或全量测试。

源码、产品模板、dogfood、登记、当前版本产物、风险测试、完整工程检查和独立源码审计均已
取得证据。真实下游写入、真实 runtime smoke、用户验收、Git 提交/推送及发布尚未执行。
HEAD 保持基准，暂存区为空；R01 验收状态保持原样，不因 R02 工程收口自动验收或归档。

最终独立证据复核已通过：同一 `/root/r02_audit` 核对审计字段、指纹、7 份检查日志和 4 份
候选 blob 哈希及候选收据引用，当前运行产物与 build-assets 记录匹配；483 项输入中的非记录
输入无字节漂移，HEAD/VERSION/空暂存及确认卡/索引一致。未发现收口阻断，没有重跑测试。
