---
lifecycle: completed
validation_status: verified
---

# Native Memory 自动同步可靠性闭环

## 目标与授权

用户明确要求彻底排查并解决 `~/.codex/memories` 无法自动同步，包含本次失败及其他可能原因。复用已授权的固定 GitHub Memory 仓库、两边独有文件并集及唯一同名冲突采用本机的决定；新出现的同名冲突必须保留证据，不扩大旧选择。原生三个本机索引保持本机所有。

本次允许修复产品、测试和文档并验证真实 Memory 自动同步。根项目 Git 提交/推送仍须用户当次明确授权；不擅自发布工厂代码。修复版安装与真实生命周期验证是交付必要条件，不能由手动 reconcile 成功替代。

## 规模、预算与传播

M 级：既有同步链的跨模块可靠性修复，不新建调度架构。45 分钟 / 20k 新增 token 估算（未实测）/ 最多 1 位 review-auditor / 最多 2 轮验收，包含合理重测与环境缓冲；预计超限时报告，不隐瞒未完成项。复用前期扫描，主对话实现。

通用产品层改动进入 templates，同步 dogfood；更新 VERSION 与 CHANGELOG [product]、manifest 与受管产物。测试放 scripts/tests。长期设计以 codex-native-memory-sync.md 为事实源。

## 已确认事实与待查项

- 已安装 1.16.0 自动 Hook 正常触发，但在深层路径写快照时 os error 3；本仓库 1.17.3 长路径修复已通过真实 reconcile。
- 2026-09-08 revision 33 合并后 158 个文件逐个 hash 一致、健康；随后旧版自动 worker 再次失败，pending 重新积压。冲突未重新出现。
- 独立审计确认：损坏 worker.json 无法自愈；PID 复用可能误认为旧 worker 存活；失败提醒函数没有接入 SessionStart。
- 当前为事件驱动同步，失败留待下一事件，不是独立常驻定时服务；不擅自新增永久轮询。
- 继续核对状态损坏、锁竞争、尾部事件、失败收据、基线提交中断、临时目录清理、授权/网络/远端漂移、安装与触发链。

## 验收

1. 已确认代码缺陷有回归证据：损坏/截断预约、并发恢复、PID 身份、失败/冲突/陈旧pending提醒及去重、无副作用status、保留原有数据冲突和锁边界。
2. 通过相关 Memory/CLI 测试、完整 workspace（原并行失败面）、必要工厂发布检查、基线与独立审计；无关测试失败分别记录。
3. 正式用户级程序与被测源码版本一致；正常启动、回复结束、关闭会话分别取得自动触发和同步完成收据，pending/worker清空、远端一致。
4. 未取得证据的异常条件明确列为未验证，不能承诺涵盖所有外部故障；不把当前恢复等同长期可靠。

## 前期记录

长路径、后台测试修复及真实冲突合并见 [Bug 记录](../../2_bugs/BUG-native-memory-windows-long-path-sync.md)。本卡只管理新增可靠性闭环，不重算已完成修复。

## 本轮排查与实施证据

版本 1.17.4；以下区分现场故障、已复现的潜在缺陷和外部条件，不将它们全部归因为本机已经发生的故障。

| 环节 | 结论与处理 | 验证或边界 |
| --- | --- | --- |
| 正式入口与传播 | 现场自动 Hook 仍运行用户级 1.16.0，项目源码已修并不能改变它 | 2026-09-08 05:49:44 UTC Hook 收据记录用户级路径和 1.16.0；05:50:20 再次 os error 3。必须正式发布、安装并重新观察生命周期 |
| Windows 深层路径 | 已确认本次直接故障；原生 MoveFileExW 未使用扩展绝对路径，1.17.3 已修 | 大于 260 字符的首次写入、替换及完整快照回归；普通路径真实 reconcile 已通过 |
| 任务预约损坏 | 空文件、截断 JSON、非法状态可让旧实现持续无法预约；1.17.4 在锁内保留原件并恢复 | 8 个并发请求仅一个取得预约，原始损坏字节保留；目录不当作普通状态文件替换 |
| PID 复用 | 仅检查 PID 存活可能认错工作进程；1.17.4 校验进程创建身份 | 正确与错误身份、旧状态启动时间窗口、旧 token 不能释放新预约均有回归 |
| 待办与尾部事件 | 坏 pending 保留原件后由新事件恢复；已有 drain 在释放预约后再次检查尾部事件 | 状态损坏回归及 lifecycle_queue_drains_events_arriving_during_sync；不删除用户 Memory 文件 |
| 基线提交中断 | 快照与 last-synced.json 是两项写入；1.17.4 保留旧快照到收据提交，按实际 hash/revision 恢复 | 注入旧快照移开、新快照就位、收据提交后三种阶段；没有可信收据时保留现场并报错 |
| 健康状态与提醒 | 旧读取吞掉损坏状态，busy 可擦掉失败，提醒未接入；1.17.4 修复 | status 零写并显式失败；busy 保留失败；SessionStart 失败/冲突/陈旧待办提醒、去重、确认及恢复后重发有 CLI 测试。非零 Hook 的提示能否被宿主展示尚未实测 |
| 锁与后台生命周期 | 同步锁、队列锁、带 token 释放及后台脱离已有实现；此前 sentinel 失败属于并行测试自身干扰 | 修正后的隔离后台测试在完整 workspace 核验通过；本轮没有改为常驻服务 |
| 配置、授权与安装工具 | 现场 approved/enabled、Hook 配置与历史运行收据有效；用户级 doctor 通过 | HookConfigured 依赖被查询程序自身路径：项目 CLI 查用户级配置为 false 不能证明用户 Hook 丢失；用户级 CLI 查为 true |
| 新出现的账本告警 | 托管账本前缀 EF-BB-BF；Skill 检查直接反序列化导致误报。Memory 的 ownership JSON 读取已去 BOM | 同一账本实际 schema=1、platform=codex、19 条记录；用户级 status 授权仍 approved。官方 updater 的 PowerShell UTF8 写入可能产生 BOM；此独立提示缺陷未在本次同步修复中修改 |
| 网络、凭据与远端变化 | 既有链会核对私有仓库、远端地址，Git 操作有超时，push 不强推；网络失败保留待办 | 此前真实 revision 33 已完成；不能据此保证未来网络和凭据永不失效。失败等待下一生命周期事件重试，没有新事件时不会独立定时重试 |
| 内容冲突与并发写入 | 原生三个本机索引排除；只同步其余不透明字节。同名新冲突不沿用旧选择 | 已有冲突、新本机文件、push 前后并发写入、校验失败拒绝恢复及字节/换行保持回归；本次没有扩大内容授权 |
| 临时目录与异常退出 | 正常收尾清理当前操作临时目录；异常现场与冲突备份保留 | 未实现全盘扫描清理历史临时目录；磁盘满、ACL 改变、杀毒拦截和系统断电未做真实故障注入，不宣称全部覆盖 |

## 本轮验证收据

- `cargo test --locked --config scripts/tests/factory-cargo.toml --manifest-path .codex/hooks/Cargo.toml --workspace --no-fail-fast`：core 147 passed、3 ignored；Hook 24 passed、2 ignored。原命令退出 1，唯一失败是新增 CLI 测试夹具未启用 Memory，走了正常禁用分支。
- 只修夹具并补齐冲突、陈旧待办和已确认提醒覆盖后，`cargo test --locked --config scripts/tests/factory-cargo.toml --manifest-path .codex/hooks/Cargo.toml -p bridgeforge-cli`：13 passed、0 failed、退出 0。产品源码未改，不重复已通过的 core/Hook 全量。
- 独立 review-auditor 已复核四项模板与 dogfood 实现，无新增源码阻断；提示失败分支、状态读取竞争与提醒测试覆盖反馈均已处理。真实宿主展示仍单列未验证。
- `build-assets --project-root D:/Quant/BridgeForgeCodex`：status=built；源码树 `sha256:80690525fa8224d0a2c477902a1dd23f601da8771065851cdb25a170bdf39801`，Hook `sha256:4237a6e771774c75835bbfa55142ac364ffdeb44b413fc0381089add2dcfeadd`，CLI `sha256:ba56b011d4bafeb17ab7f99b75826f41fb01c7d70a9d7affdf288e890fa2fa76`。
- 1.17.4 `check baseline` 为 clean；`manifest --check` 无变化；factory-version healthy；skill-metadata 无问题；project-structure 通过（只有其他主题归档建议）。
- `cargo test --locked --manifest-path scripts/tests/Cargo.toml -- --test-threads=1`：退出 0，87 passed、0 failed、3 ignored，测试阶段 859 秒。跳过项为需要专用环境的高成本提示宿主测试、由其他用例显式运行的子进程辅助项，以及需要单独授权的真实 Assist 迁移。前次失败的 `real_factory_cli_sync_builds_new_runtime_and_commits_through_precommit` 本次通过，包含新版本构建提交、运行产物损坏修复与健康状态快速路径；真实初始化构建与全部默认 fixture 通过。前次 invoked.timestamp/os error 3 未重现，未据此宣称确认并修复其独立根因。
- 真实后台专项：使用项目已构建 1.17.4 `memory-sync kick --trigger repair-validation --codex-home C:/Users/bridg/.codex` 临时启动后台 worker，未替换用户级程序或修改持久 Hook。05:56:44 UTC 记录新 PID 与 processIdentity；05:57:22 完成 healthy/noop、刷新 revision 33 收据，pending/worker/activeConflict 均为空。没有冲突写入或新增远端提交。这证明修复版真实后台链可运行，不代表正常会话已切换到修复版。
- 源码、dogfood 与一次真实后台专项已完成；正式产品传播、修复版真实生命周期与其他下游尚未验证，整项保持 in_progress。用户级当前仍旧版，后续自动事件仍可能重新触发已确认故障，不能宣称自动同步已永久恢复。

## 当前交接点

发布前源码、产物和默认回归准备完成，未提交或推送工厂代码；原有 codex_git_sync_autostash 保留。官方 updater 要求从干净的 canonical origin/main 临时 clone 安装，不能把未发布工作树直接装成正式产品。需要用户当次授权将 Memory 修复及此前已验收的 Rust Hook 改动经受管 git-sync 发布到 `https://github.com/freakybridge/BridgeForgeCodex.git` 的 main，再运行官方安装入口和真实生命周期观察。不得用当前项目 exe 持久代替用户级安装。

发布安装后必须重新核对实际版本与三事件收据；若自动版本流程改变版本，应记录最终实装版本，不能预先固定为本卡的源码 1.17.4。临时后台 noop 仅证明本轮真实数据可收敛；正常启动、回复结束、关闭会话的自动完成仍是未完成验收项。

## 用户验收与发布授权

2026-09-08 用户明确调用 `$summary 同意验收` 与 `$git-sync`，批准上述修复，并确认前一答复提出的提交推送、正式安装及真实生命周期验证路径。复用上一阶段的测试和独立审计收据，不为 summary 重跑测试。整项在实装与三事件完成前保持 active / awaiting_validation；发布和 runtime 验证由已授权的后续动作继续，不以用户验收代替证据。

本次检索并阅读 2026-09-04 原生 Memory 状态迁移与受管发布历史，当前证据仍以本卡为准。现行 AGENTS 已要求区分源码、传播和 runtime，不新增重复 Rule / Hook / AGENTS 建议；不写入原生 Memory，不执行归档。

## 正式安装与自动生命周期验收完成

2026-09-08 受管 git-sync 发布修复版 1.17.5，提交 `3e2d7e9b0639e6f2a2bed7ecf58d7e524db7d2d8`，推送 origin/main，工作区 clean、ahead/behind=0/0；原有 codex_git_sync_autostash 保留。发布后 baseline clean。

官方 updater 返回 `status=completed`、`source_commit=3e2d7e9b0639e6f2a2bed7ecf58d7e524db7d2d8`、`mode=updated`、`action_count=2`；用户级 CLI self-test 显示 1.17.5，doctor 通过。旧程序备份因文件占用留下 cleanup_pending=true，安装事务已提交且新程序生效；此非阻塞清理项留给下一次官方维护，未手工删除或重复运行 updater。

执行 `scripts/tests/fixtures/native_memory_lifecycle_observer.ps1 -ConfirmAuthorizedMemorySync`，原始观察收据保留在 `.runtime/native-memory-lifecycle-1.17.5.jsonl`，退出 0，耗时约 122 秒。真实 app-server 临时会话验证：

- SessionStart 用户级 Memory Hook 完成并启动后台任务，宿主 hook/completed 实际展示此前失败的 warning；本轮首次取得正常退出路径的失败提醒宿主证据。非零退出 Hook 的展示仍未注入验证。
- Stop 实际触发，用户级 hook-runtime 记录 executablePath 为用户 `.codex/bin/bridgeforge.exe`、binaryVersion=1.17.5、lastEvent=Stop；启动与回复结束合并的队列于 06:27:27 UTC 清空。
- 06:27:33 关闭 app-server stdin，SessionEnd 于 06:27:35 启动新 worker；app-server 随后退出，后台 worker 继续存活。06:28:15 worker 完成并退出，pending/worker 均清空，final-health=healthy/noop，app-server 退出 0。

本次仍为 revision 33 的无变化同步，没有新增 Memory 内容提交；自动同步的真实调用链已恢复。用户既有验收授权有效，源码、产品传播、dogfood、默认 fixture、正式安装与真实 runtime 必要验收完成，因此本卡关闭。真实下游业务升级不在本次范围；网络中断、磁盘满和系统断电等未实测条件仍按前文边界保留，不承诺永不失败。独立 Skill 账本 BOM 提示及旧安装备份清理不属于本次自动同步阻断。

本节是对已安装并实测的 1.17.5 及其源提交的验收。随后受管同步保存验收文档可能自动递增仓库版本；该元数据提交不改变本次测试指向的安装版本，不把未再次安装的新版本写成已安装。
