---
lifecycle: active
validation_status: implementation_verified_live_smoke_pending
record_type: debate
confirmed_at: 2026-09-04
---

# Native Memory Hook 自动触发断点辩论

## 依据与边界

- 确认卡：`../requirements_2026-08-31_asset-migration-sync.md`
- 目标：定位已授权 Native Memory 在 Codex Desktop 0.153.0 中未留下生命周期 Hook runtime 收据的根因，并收敛为可验证、无感的产品修复。
- 边界：只读检查真实用户配置与运行收据；隔离测试不得读写真实 Memory 正文或真实 GitHub；用户确认辩论结论前不改产品代码。
- 验收：至少解释升级状态目录断档和当前机器 Hook 未触发两层现象；修复后用自动测试、隔离 runtime smoke 和真实生命周期 smoke 分层验证。

## 已有证据入口

- `templates/hooks/crates/bridgeforge-core/src/memory/user_config.rs`
- `templates/hooks/crates/bridgeforge-cli/src/main.rs`
- `scripts/tests/unit/cli.rs`
- `doc/0_architecture/design/codex-native-memory-sync.md`
- 用户级 `hooks.json`、`config.toml`、受管 ledger 与只读 `memory-sync status`
- 新任务 `01a06a1f-8024-7951-a093-5059dd7c3b16` 的原始 rollout
- Codex 0.153.0 官方 Hook discovery、dispatcher 与 Windows command runner 源码

## 现场事实

1. 旧状态目录有授权与同步收据，新目录最初没有迁移；隔离 A/B 已证明这会让 `hook-run` 在授权校验前失败。
2. 当前新目录已有匹配的 `remote.txt`，但 `hookRuntimeVerified=false`、`lastReceipt=null`。
3. 新任务中项目级 `SessionStart` 已执行并注入输出；用户级 Memory `SessionStart` 没有 runtime 收据。
4. 用户级 Memory handler 已配置、启用且有匹配信任哈希；无 matcher 按官方实现应匹配全部 SessionStart。
5. 当前 `commandWindows` 经 Codex 0.153.0 同款 `cmd.exe /C` 包装隔离运行，退出码 0，并生成 runtime 与 pending；Windows 引号假说被排除。

## Agent

- A（implementation-worker）：`/root/memory_hook_implementation`
- B（review-auditor）：`/root/memory_hook_audit`

## 辩论记录

### 第一轮

#### A：实现视角

- 最高概率根因是新 Rust Hook 定义改写后精确哈希信任失效；今天 trust 写入晚于 Codex 进程启动，当前运行面仍未装载新 handler。
- 旧 Hook 在相同 ACL 下已有三类真实 receipt，故 ACL、输出协议、Windows 引号、handler 覆盖和 detached worker 均不是当前主因。
- 必修旧状态目录迁移，并把配置存在、信任状态、当前 runtime 激活拆开报告；禁止自动写 Codex trust。
- 可实现的无感边界是一次显式信任和必要重启后冻结 handler 定义，后续只更新稳定路径下的二进制。

#### B：审计挑战

- 不能笼统表述为“所有新任务复用进程启动缓存”。Codex 0.153.0 在 `thread/start` 构造配置，但单个 Session 的 Hook engine 构造后不会自动追补；本次更准确是创建时撞上 09:53:59 的信任写入竞态。
- `hookInstalled=true` 只检查 JSON 内容，不检查 Codex exact hash trust，也不证明当前 Session 已装载。
- 直接新增 `UserPromptSubmit` 或把 `Stop` 改同步都会再次改变信任哈希；前者每次输入增加进程/队列开销且异步可能在 Session 结束时取消，不能作为当前根因修复。
- 状态迁移只能迁 durable 状态；不得复活旧 worker、锁或临时目录，旧 pending 应转换为新的 migration pending。

#### 第一轮共识与待答

- 共识：已实证升级状态目录断档；当前现场主因是 Rust handler 改写后的 trust/Session 装载时序，而非引号、ACL 或生命周期 matcher。
- 待答：当前现场应先以“trust 落盘后的第二个全新 root task”还是“完整重启后新任务”作为最小闭环；状态字段和迁移合同如何避免再次改变 handler 哈希。

### 第二轮

#### A：回应与修正

- 接受 B 的时序修正：现有证据只支持本次 `thread/start` 捕获了 modified/untrusted 快照，不能断言所有后续线程都复用进程启动缓存。
- 现场验证应先创建 trust 落盘后的第二个全新 root task；仍失败才完整退出 Desktop 并重启，以区分单 Session 竞态与进程级缓存。
- 最小产品改动不再改变 Hook 定义，也不写 `[hooks.state]`；新增 durable 状态迁移、配置与 runtime 状态拆分、Hook attempt/completed 收据。
- 旧 worker、锁、临时目录、last-worker 和旧 runtime 收据不得进入新状态；旧目录保留以便审计。

#### B：终审挑战

- 不在收据中声称证明 Codex 实际执行定义的 trust fingerprint；当前 argv 没携带该值，从磁盘重算不能证明 Session 内快照，往 argv 加值又会改变 handler hash。
- 收据拆成 `hook-attempt` 与 completed 两层：event 校验后先记录 attempt，只有迁移、授权、pending 和 worker 启动/复用全部成功才标 runtime verified；失败进入 health。
- 状态迁移必须按类型验证和转换，不能 raw-copy：remote 与 ledger 一致；last-synced 与 snapshot 原子成包；pending 解析重写并追加 migration trigger；active conflict 只迁引用包并改新路径；health/alert 从迁移结果重算。
- 迁移完成标记必须包含旧状态 source fingerprint，防止保留的旧 active conflict 在后续被再次复活。
- `status` 必须纯只读；保留 `hookInstalled` 兼容字段，同时新增 `hookConfigured`、`hookDispatchObserved`、`hookRuntimeVerified` 与迁移状态。
- 本轮完全冻结现有 SessionStart/Stop/SessionEnd 定义，并用 golden regression 防止无意改 hash；不新增 UserPromptSubmit，不调整 Stop async/timeout。
- 新 root task 会启动真实 worker 并可能 push 私有 GitHub，必须在用户明确授权真实 smoke 后执行。

## 收敛方案

### 根因结论

1. 已实证产品 Bug：Rust 改版切换状态目录时没有迁移旧 durable 状态，旧授权/基线/pending 被新实现忽略。
2. 当前机器第二断点的高置信根因：9 月 2 日 Rust handler 改写使 exact-hash trust 失效；今天信任写入与新任务创建发生竞态，本次 Session 没有装载并追补 Memory handler。最终闭环仍需一次经授权的真实 root-task smoke。
3. 已排除 Windows 命令引号、matcher、同事件 handler 覆盖、输出协议、ACL 和 detached worker 作为当前无 receipt 的主因。

### 推荐实施

- 新增幂等、fail-closed 的旧状态迁移与一次性完成标记；只迁并验证 durable 状态，旧目录不删。
- `status` 改为纯只读，并区分 configured、dispatch observed、runtime completed 与 migration 状态；兼容保留 `hookInstalled`。
- `hook-run` 在 event 校验后写 attempt，完整登记 pending 并启动/复用 worker 后才写 completed；失败记录 health，不计 verified。
- 冻结当前 Hook JSON 精确定义并增加 golden regression；不自动写 trust，不新增事件。
- 同步 Template/dogfood、测试、VERSION、CHANGELOG、受管资产与 manifest；不 commit、不 push。

### 待用户选择

- A（推荐）：实施上述最小修复，隔离验证零网络；暂不触发真实 GitHub。
- B：实施 A，并授权实施后创建一次全新 root task，允许 worker 对既有私有 Memory 仓库完成真实自动同步 smoke。
- C：扩大为增加 UserPromptSubmit 或同步 Stop；接受 handler 再次失信、重新 review/trust 和每轮额外开销。

## 实施结果

- 用户指示“开始修吧”；因未明确授权真实 GitHub 写入，本轮按 A 边界实施，不执行真实 push。
- 已新增旧 durable 状态一次性迁移、迁移锁与 source fingerprint 完成标记；remote 必须与 ledger 一致，基线必须整包验证，pending 保留最早时间，active conflict 只迁当前引用包，旧目录不删除。
- 已新增 Hook attempt/completed 收据；只有迁移、授权、pending 与 worker 启动/复用成功后才写 runtime verified。`status` 已改为纯只读，并拆分 configured、dispatch observed、runtime verified 与 migration 状态。
- Template/dogfood、架构事实源、1.12.6 版本、受管资产和 manifest 已同步；既有 SessionStart/Stop/SessionEnd 命令、timeout 与 async 合同未改变。
- 自动验证：迁移 2 项、Memory CLI 5 项、Hook 定义 1 项通过；完整工厂首轮 77 通过、4 失败、2 忽略，修正测试版本与下游测试门控后 4 个失败项逐项复跑通过；真实 GitHub/全新 root task smoke 仍未执行。
