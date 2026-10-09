---
lifecycle: active
---

# BridgeForge 工厂重构候选扫描

> 本文保留扫描时事实。后续用户已授权 R01 开发，最新范围与验证见
> [R01 开发确认卡](requirements_2026-10-09_r01-cleanup.md)。R02 已授权实施，最新范围与证据见
> [公共持久化确认卡](requirements_2026-10-09_r02-persistence.md)。R03 也已授权实施，最新范围及证据见
> [Memory CLI 确认卡](requirements_2026-10-09_r03-memory-cli.md)。R04亦已获预算与实施授权并取得工程证据，
> 最新累计工作区结果与 2026-10-10 的 R01–R04 工程范围用户验收见 [独立缓存确认卡](requirements_2026-10-10_r04-artifact-cache.md)。下文候选表保留扫描时状态。

## 范围、结论与授权

基准 HEAD 为 0425b2da9ed5bf285ff6268ccd55b68e0d846c86，版本 2.1.0。
扫描前后工作区干净。先梳理全项目目录，深入 CLI、Git/构建、Memory 文件操作及相关测试；
不是全仓穷尽扫描，未逐项审查全部 Skill、历史文档、迁移流程和用户级分发器全部实现。
templates/ 为产品事实源，.codex/ 为受管镜像，不将镜像计为业务重复。

用户调用 refactor-scan，允许现有定向测试和离线性能基准；随后要求分别白话解释 R01–R04。
用户表示“我明白了”，不等于选中候选或实施授权。
本报告仅保存已取得的扫描证据和建议；四项全部未决定、未授权实施。
建议优先评估 R01、R02，未发现已确认 P0/P1 缺陷。

扫描按 M 级推进：45 分钟、约 20,000 新增 token、0 个子 Agent；实际约 15 分钟，token 未实测。
测试仅写缓存和隔离临时夹具，未修改源码，也未向真实远端提交或推送。
扫描范围与后续实施预算分别确认，不将本报告当作开发确认卡。

## 职责概览

| 编号 | 当前区域 | 职责与边界 |
|---|---|---|
| M01 | templates/ | 公共骨架和 Rust 产品源码；与 .codex/ 的受管镜像保持一致 |
| M02 | bridgeforge-cli | 命令分发与适配，当前也包含 Memory 命令编排 |
| M03 | bridgeforge-core | Git、构建、项目同步、Memory 等业务能力及共享进程执行 |
| M04 | bridgeforge-hook | Codex 生命周期与门禁入口，通过 core 复用能力 |
| M05 | skills/ 与用户级分发 | 通用流程源及受管安装；普通源码目录不自动等于菜单安装 |
| M06 | scripts/tests/ | 单元、隔离 fixture 与已有离线性能基准 |

## 稳定编号清单

| 编号 | 优先级/类型 | 最小建议 | 预期收益 | 规模/风险 | 用户决定 |
|---|---|---|---|---|---|
| R01 | P2/结构清理 | 移除没有有效调用方的旧 prepared 产物注入分支 | 减少无用路径和维护干扰 | M/低；逻辑小改，包含工厂审计与工程检查 | 未决定，未授权 |
| R02 | P2/职责调整 | 将通用文件持久化工具从 Memory 领域模块中归位 | 减少其他业务模块对 Memory 命名空间的依赖 | M/中 | 未决定，未授权 |
| R03 | P3/模块整理 | 集中 Memory 命令适配与编排，主入口保留分发 | 缩小命令修改与审查范围 | M/中 | 未决定，未授权 |
| R04 | P2/性能候选 | 评估工程准备与 release 共用独立的构建产物缓存 | 可能减少重复编译；收益待测 | L/较高；需要新缓存合同与验证 | 未决定，未授权 |

实施规模和预算以 confirm 对选中范围的判定为准，本表不复制其阈值。

### R01：清理不用的旧路径

证据位置：templates/hooks/crates/bridgeforge-core/src/git_sync_plan.rs:53、56、193。
prepare 只调用 prepare_with_artifacts(..., None)。全量源码/测试引用搜索没有传入 Some 的调用方；
write_plan 为 git_sync 的私有模块，不是外部公开入口。
当前 git_sync.rs:701 与 release_preparation.rs:520 都调用 WritePlan::prepare。

旧 Some 分支仍包含 prepared 产物注入及“return to develop”的旧错误路径，但没有参与当前发布。
最小动作是合并包装并清除不可达分支，保留安装产物复用、真实构建、漂移检查与回滚。
主要影响 git_sync_plan.rs、dogfood 镜像、相关测试和受管登记。
不删除独立开发准备收据，不恢复 release 对开发验收的依赖，也不删除正常复用逻辑。

白话：一条旧选项保留在代码里，但现行调用都没有使用它；清掉后少读、少维护一条路径。
本轮实际 git-sync release 使用的是检查现有程序、无法复用则构建的流程。
收益主要是可读性和维护成本，不宣称明显提速。

### R02：把公共工具放到公共位置

证据位置：templates/hooks/crates/bridgeforge-core/src/memory/mod.rs:1015、1044、1051，
以及 git_sync、release_preparation、batch、project_sync、runtime、file_lock、project_hooks、
high_cost 和 Hook util 的实际调用。
atomic_write、atomic_write_json、is_link_or_reparse 等通用能力位于 Memory 领域命名空间。
算法已经共享，候选针对职责归属，不把现状误报成重复实现。

建议归入中性的持久化能力，同步直接消费者；现有公开调用路径需先核查消费者，
必要时保持委托入口，不能直接删除未知外部契约。
必须保留 JSON 排序/换行、原子替换、长路径支持、链接拒绝、失败清理和 MemorySyncError 映射。
未来通过既有文件/恢复/路径测试及实际调用方验证等价性；当前没有性能收益证据。

白话：Memory 保存健康状态、batch 保存运行进度、Git 保存发布保护记录，
都在借用同一可靠写文件工具。工具集中到公共位置，各业务继续使用原有可靠语义。

### R03：集中 Memory 命令代码

证据位置：templates/hooks/crates/bridgeforge-cli/src/main.rs:420、617、1068、1133、1312。
主入口除路由外，还承担 Memory 路径与授权、健康结果、迁移、worker 启动和通知编排。
按同一业务职责移动相关函数到独立 CLI 模块，保留主入口路由和公共参数工具，
不引入新命令框架，也不因为文件长就拆分。

影响主入口、Memory 命令模块、单测私有访问位置、dogfood 和登记。
保留所有命令参数、JSON 字段、错误信息、退出码、只读查询和后台启动语义。
验收应覆盖状态、授权、通知、worker 分支及其他命令分发，不把局部状态测试当作全部覆盖。

白话示例（尚未实施）：以后调整 memory-sync status 的显示字段，主要查看 Memory 命令模块；
主入口只负责把命令交过去。预期改善是定位和审查，不宣称运行更快。

### R04：评估复用提前编译好的程序

证据位置：git_sync_plan.rs:193、209、267；project_sync.rs:2642；
release_preparation.rs:520，以及本机已有工程准备和实际 release 收据。
准备阶段构建 2.1.0 候选产物，后续实际 release 收据仍为复用 0、构建 2。
本轮比对两个资产的源码、lock、配方、自检和平台合同，五项均相同。
缓存文件只在本机 .runtime 中，报告不复制二进制、凭证或完整日志。

现有正常复用针对已安装产物；可以评估跨步骤的独立、可选构建缓存。
合同相同只是评估条件，命中仍须校验实际二进制哈希、运行自检和输入稳定性。
缺失/损坏/失效时回到真实构建，release 不依赖 prepared、审计或验收记录。
保留旧版本不得复用、漂移阻断和事务回滚等现有边界。

这是新增性能能力候选，不是已确认功能 Bug。尚未实测“准备后 release”缓存命中收益。
需在既有隔离 fixture 上增加同合同准备→发布、缺缓存、损坏缓存、输入变化等对照场景。
不承诺具体节省时间，不把普通同步的基准当作此项收益。

R04 涉及 R01 旧接口的处理，若选择 R04，应合并设计，避免重复实施 R01。
R02、R03 不自动成为 R04 的实施依赖；未授权独立条目不能顺带加入。

白话：已经编译过同一目标版本，可以考虑严格检查后沿用；条件不符合仍照常编译。
清理旧的无调用分支与设计新的独立缓存需要协调，不能恢复旧的发布前置门槛。

## 实际验证与基线

| 编号 | 命令/场景 | 实际结果 | 限制 |
|---|---|---|---|
| V01 | core git_sync::tests::factory_ | 7 passed、0 failed；68.33 秒测试执行 | 当前行为回归，覆盖复用、失效、漂移、回滚，不证明新方案已实现 |
| V02 | CLI memory_status_ | 4 passed、0 failed；0.25 秒 | 状态字段、损坏状态、路径别名和只读边界，不覆盖全部 worker/通知流程 |
| V03 | core memory_atomic_write_handles_long_windows_paths | 1 passed、0 failed；0.12 秒 | Windows 长路径写入、替换和快照字节；不是全部持久化安全验收 |
| V04 | factory_ordinary_sync_performance | 1 passed、0 failed；CLI 定时 29185 ms | 单次文档修改、本地 bare remote、普通同步；不是 GitHub 或用户端到端耗时 |
| V05 | manifest --check / skill-metadata | changed=false，issues/warnings 为空 | 当前清单与元数据，不证明重构建议实施等价 |

V01–V03 使用受管入口及锁定 workspace：

```powershell
cargo test --locked --config scripts/tests/factory-cargo.toml --manifest-path .codex/hooks/Cargo.toml -p bridgeforge-core git_sync::tests::factory_ -- --test-threads=1
cargo test --locked --config scripts/tests/factory-cargo.toml --manifest-path .codex/hooks/Cargo.toml -p bridgeforge-cli memory_status_ -- --test-threads=1
cargo test --locked --config scripts/tests/factory-cargo.toml --manifest-path .codex/hooks/Cargo.toml -p bridgeforge-core memory_atomic_write_handles_long_windows_paths -- --test-threads=1
```

默认执行因 .cargo-lock 权限失败，以完全相同命令窄范围提升权限后取得上述结果；
权限重试不是代码修复，没有修改权限、源码或切换非受管入口。

V04 的环境变量只在该进程生效；夹具复制工厂、使用本地 bare remote，未访问 GitHub：

```powershell
$env:BRIDGEFORGE_PERF_EXPECT_REUSE = '1'
cargo test --locked --manifest-path scripts/tests/Cargo.toml git_sync_runtime::factory_ordinary_sync_performance -- --ignored --exact --nocapture --test-threads=1
```

收据 version_before/version_after 均为 2.1.0，version_bumped=false、复用 2、构建 0、
夹具工作区 clean、ahead/behind=0/0。夹具测试执行总计 54.67 秒、编译 34.09 秒，
这些不属于上述 29.185 秒的 CLI 定时口径。该结果说明普通同步已有有效复用，不重复列为待优化问题。
未执行完整回归、新缓存对照、真实下游或桌面运行验证。

## 经验沉淀与后续决定

1. 不能把函数包装入口被调用，等同于其中每个可选分支都被调用；清理前核对完整调用链和可见性。
2. 已共享算法仍可能归属不清，但重构应围绕真实职责，不人为复制一套公共实现。
3. 合同相同不是直接复用授权；二进制、自检、漂移和失败路径仍需验证。
4. 性能基准先固定普通/发布、真实/本地远端及定时范围，不用不同场景的数字计算提升百分比。
5. 模板镜像、文本规范化哈希与原始二进制哈希、前后漂移检查都可能是必要机制，不能按外观相似删除。

经验已由当前 Skill 与既有项目约束覆盖，不新增 Rule/Hook/AGENTS/Memory 建议。
本轮不是验收或归档；既有 refactor-scan Skill 开发验收仍由原需求卡记录，不扩大到 R01–R04。
接续入口见 [换机交接](handoff_2026-10-09_factory-refactor.md)。
