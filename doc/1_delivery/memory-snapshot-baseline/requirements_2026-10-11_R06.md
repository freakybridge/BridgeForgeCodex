---
title: R06 Memory 快照加载基线
lifecycle: completed
validation_status: verified
date: 2026-10-11
---

# R06 Memory 快照加载基线

## 授权与范围

用户要求“先测量基线，再决定是否优化”，在计划后明确回复“开始吧”。本轮只实施第一阶段：增加显式离线基准、测量旧实现并给出结论；不优化生产实现、不读真实 Memory 正文、不连接 GitHub、不提交或推送。

保留未提交的 R05。基准 HEAD 为 `97a7624dd8410e137e7f6671ee49a28a0adc2ead`，版本 2.2.1；起点包含 R05，开发指纹为 `sha256:2e316e66038b5dc5ff053f2a835d02c13167e1d0d7b02217d91469d8d6743016`。

## 规模、预算与传播

M：45 分钟 / 约 20k 新增 token（估算、未实测）/ 最多 1 名 review-auditor / 最多 2 轮验证。包含基准、测量、独立审计与项目强制工程检查；后续优化需根据本轮结论决定，不自动追加范围。无适用周额度校准系数，不编造百分比。

本轮是工厂专属测试与测量记录，不修改产品运行源码；不需要 dogfood 源码传播，不预写 VERSION。测试归 scripts/tests，记录归本主题与 doc/README.md。新增测试使开发检查输入变化，需重新工程准备；不将 R05 的已通过结果冒充 R06 新基准通过。将来显式 release 再由受管流程判定版本及 CHANGELOG 分类。

## 测量合同

1. 使用受管 release Cargo 测试入口，默认忽略的显式基准；编译、生成数据、快照构造与清理不计入目标函数耗时。
2. 合成数据固定为 100×4 KiB、2,000×8 KiB、20×1 MiB；不称为真实用户规模。
3. 独立子进程分别测 verify_snapshot 和 snapshot_files。首次调用单列，1 次额外预热，随后 10 次正式样本，报告中位数及范围；不人为清理系统文件缓存，不称为冷缓存。
4. 加载子进程不构造数据；记录初始工作集和进程峰值工作集，包含运行库与结果容器，不解释成文件缓存或独占堆大小。
5. 本地 Git 场景为 100×4 KiB，从本地 bare 仓库恢复到全新目录，重复 3 次。复用现有 LocalGit 夹具，计时包裹真实 reconcile，单独记录同步调用中 ProcessRunner 耗时和次数；不把不同测量入口的耗时相减当作重复读取成本。
6. 记录 OS/CPU、Rust 版本、构建模式、源码/测试/二进制哈希与原始样本。结果写本机 .runtime/R06，关键数据进入本文。

## 决策标准

基线只证明成本，不证明优化收益。若加载成本很小、波动较大，或实际离线同步主要耗时在其他步骤，则保留现状。成本明显且范围可控时才提出最小优化试验；最终保留与否仍需同条件 A/B、内存及正确性验证。

保持损坏快照分类、旧清单顺序、本机专属文件过滤、链接拒绝、额外/缺失文件检测、推送前本地变化检查。尤其不合并 read_remote 两个失败处理阶段。

## 实施与结果

第一阶段测量已完成，生产优化未实施。建议暂缓 R06 优化：小快照加载仅十余毫秒；2,000 个小文件虽达到约 0.21 秒，但本轮没有证明它是实际用户路径的瓶颈。小规模本地恢复的主要耗时在 ProcessRunner 调用内，不能仅因存在重复读取就认定优化值得实施，也不扩展为 Git 性能改造。

基准为 `scripts/tests/unit/core_memory_performance.rs`，通过现有 Memory remote 单元测试模块登记；3 个新增入口默认忽略（两个基准、一个子进程 helper）。首次函数测量通过后，本地 Git 场景改为每个文件有独立内容标识以避免相同 blob 影响解释，并完成格式整理；正式函数数据已在最终固定版本重新测量。以下仅记录最终版本，不混用初测数字。

## 正式基线

环境：Windows 11 Pro 10.0.26300 x86_64，AMD Ryzen 7 9700X（8 核、16 逻辑处理器），Rust 1.94.1，release + bridgeforge_factory_tests。合成快照函数场景的每个文件填充 0x5a，本地 Git 场景每个文件前 8 字节为独立编号。未清理 OS 缓存，首次调用也不属于冷缓存测试。

| 场景 | 校验中位数 ms | 加载中位数 ms | 加载样本范围 ms | 加载子进程峰值工作集 MiB |
|---|---:|---:|---:|---:|
| 100 × 4 KiB（0.391 MiB） | 9.873 | 13.029 | 12.108–14.156 | 6.73 |
| 2,000 × 8 KiB（15.625 MiB） | 177.544 | 213.219 | 208.542–234.242 | 23.31 |
| 20 × 1 MiB（20 MiB） | 14.341 | 17.376 | 17.218–18.355 | 26.03 |

峰值是整个测量子进程的历史峰值，包含运行库、清单、结果容器、断言和多轮调用，不是单次函数增量或纯堆大小。上述校验和加载是独立入口，不相减推算优化收益。

| 场景/入口 | 首次 ms | 初始工作集 bytes | 最终峰值 bytes | 10 次正式样本 ms |
|---|---:|---:|---:|---|
| small / verify | 13.6535 | 5783552 | 6238208 | 10.0369, 9.7192, 9.8592, 9.5551, 8.8087, 9.8999, 9.8313, 9.8860, 10.1007, 10.1936 |
| small / load | 14.7394 | 5836800 | 7057408 | 13.2560, 12.2971, 12.3167, 13.1137, 12.5055, 13.3281, 13.9947, 12.9443, 12.1075, 14.1559 |
| many-small / verify | 194.9312 | 6434816 | 7593984 | 185.2342, 188.2734, 178.3347, 168.1849, 163.9132, 162.5662, 170.7239, 181.3016, 178.7329, 176.7535 |
| many-small / load | 244.6161 | 6422528 | 24440832 | 221.2837, 215.8307, 210.4237, 210.9250, 213.4074, 208.5421, 210.3301, 213.0303, 216.6011, 234.2418 |
| few-large / verify | 15.2064 | 5861376 | 7249920 | 14.4243, 14.2569, 15.9268, 14.1486, 14.0335, 14.7738, 14.1431, 14.5866, 14.2538, 14.7436 |
| few-large / load | 19.1545 | 5849088 | 27295744 | 18.3546, 17.4348, 17.2176, 17.2658, 17.6212, 17.6896, 17.5341, 17.3060, 17.3178, 17.2452 |

本地 bare 仓库恢复 100×4 KiB 到全新目录：3 次总耗时为 8845.8826、8838.4763、8602.3996 ms，中位数 8838.4763 ms；各次 ProcessRunner 累计耗时为 8261.1793、8301.7765、8062.7991 ms，每次 109 次调用，占各自总时间约 93.4%–93.9%。计时不含最初 seed push，调用中包含既有夹具模拟的私有仓库查询和真实本地 Git 命令。该数值是离线函数流程，不是 GitHub 网络时间或用户指令到回复的总耗时；不能将单次 snapshot_files 的独立测量直接折算成整个流程可节省的时间。

## 复现与证据

```powershell
$env:BRIDGEFORGE_R06_OUTPUT = 'D:\Quant\BridgeForge\.runtime\R06'
cargo test --locked --release --config scripts/tests/factory-cargo.toml --manifest-path .codex/hooks/Cargo.toml -p bridgeforge-core memory::remote::tests::performance::r06_snapshot_baseline -- --ignored --exact --nocapture --test-threads=1
cargo test --locked --release --config scripts/tests/factory-cargo.toml --manifest-path .codex/hooks/Cargo.toml -p bridgeforge-core memory::remote::tests::performance::r06_local_restore_baseline -- --ignored --exact --nocapture --test-threads=1
```

两项显式基准均通过，每个加载样本校验完整路径集合对应的内容哈希，本地恢复逐文件比较全部内容。正式函数基准测试执行 9.55 秒，本地恢复基准执行 43.82 秒；这些是包含夹具准备的测试耗时，不混入上表函数计时。

本机原始记录为 `.runtime/R06/snapshot-baseline.json`、`local-restore-baseline.json`、`environment.json` 与六个子进程 `child.json`，关键样本已在本文保存。

| 身份 | SHA-256 |
|---|---|
| memory/mod.rs 原始字节 | 4bb6746995325ddf616d3e1b487cdc7f1b9774f73e3b7bf3d635c81add2c6366 |
| memory/remote.rs 原始字节 | f1331ede2f5b5dc8d93cc650ac9281dbab6a223e44b017e46e05b4834ad0e13d |
| core_memory_performance.rs | dee2fa82ff156a24f24beb5800d6f6ddac29b08ac58bf9fe9b2f4a77f7d59370 |
| 最终 release 测试二进制 | e880c3baada58073161d2cb3cd1ecb4c3da9a7151d112742fdafd24635c78998 |

独立 `review-auditor:/root/r05_audit` 已确认计时、隔离、正确性断言、Windows FFI 与本地 Git 边界，无待修复发现；审计指纹为 `sha256:2469b6dc1cf9f8c82358db1c5bc01c6ada869335eb127d3b26d71639f8775025`。显式测量后检查临时目录，未残留本轮 bf-restore 夹具。

## 工程收口

受管 `git-sync --prepare-release --message-file .runtime/R06/message.txt --audit-file .runtime/R06/audit.json` 返回 `prepared`，7 项检查 exit_code=0，审计指纹一致。首次调用仅因准备说明的 `test:` 类型不被工具支持而前置拒绝，改为支持的 `chore:` 后执行；不是测试失败。

| 检查 | 本轮实际结果 |
|---|---|
| baseline、Skill 元数据、manifest、项目结构 | 通过；结构仅含已有归档提示 |
| 模板与 dogfood 全树一致 | 1 项通过 |
| 完整 workspace | CLI 14、Core 218、Hook 23，共 255 项通过，0 失败，8 项条件忽略；625.983 秒 |
| 完整工厂 fixture | 89 项通过，0 失败，6 项条件忽略；356.542 秒 |
| 独立审计 | passed，无待修复问题 |
| 受管准备 | prepared，checks_passed=7；累计工作区隔离目标版本 2.2.2 |

workspace 的 8 项忽略包含原有 5 个条件入口和新增 3 个 R06 入口；R06 两个基准已显式通过，其中快照基准成功调用了 6 次子进程 helper。fixture 的 6 项忽略为已有 3 个性能实验、原生 Stop smoke、子进程辅助入口和需授权来源的 Assist 迁移；不计为本轮执行通过。

workspace 日志：`sha256:9bcbbf9a1bf0d50519468494569ccbc0d1880bb78973a68c9147cbf1b80fff4c`。
fixture 日志：`sha256:c799357d1e018fc95f5d3043e1bc05e5a215dd88652fca5d73e44cd8e58da178`。
当前本机准备收据：`.runtime/bridgeforge-codex/release-preparation/current.json`，与同目录 blobs 配合核验。

本轮在 45 分钟预算内完成，使用 1 名独立审计 Agent，token 未实测。工程准备结束时 R05 改动保留，Memory 生产源码 diff 为空，VERSION 保持 2.2.1，没有提交、推送或发布。准备目标 2.2.2 包含累计未提交的 R05，不代表 R06 已优化或当时工作区已升版。第一阶段测量与工程验证完成，用户决定见下节。

## 经验与后续边界

基线应区分“代码存在重复工作”“函数耗时可见”“真实流程值得优化”。本轮具备前两类证据，尚不足以证明生产优化收益。内存峰值须标明整个子进程口径，独立校验与加载的差值不是优化上限。

进一步讨论 token 后核对实际调用链：本轮重复读取发生在 Rust 快照处理内部，不会把每次读取的正文逐次送入模型；同步命令返回 status/action，正常 Hook 无消息，异常提示有去重。文件 I/O 次数不能当作模型 token 次数。Agent 重复读取并向上下文返回正文、日志或提示是另一类成本；本轮没有实测其 token 消耗，也不宣称优化可节省 token。

本记录沿用 summary 的同主题沉淀方式；既有规则已覆盖证据与范围要求，不新增 AGENTS、Rule、Hook 或 Memory 约束。真实 Memory 规模、GitHub 网络和 runtime 用户路径未验证。

## 用户验收与结论

2026-10-11，用户在理解基线及 token 边界后明确表示“本轮优化都可以收口了”，调用 `$summary accept` 和 `$git-sync release`。本次验收的是 R06 基线工具、测量、审计与工程验证，以及暂缓生产优化的结论；不是验收某个已经实施的性能优化。R06 按该范围更新为 completed / verified，不保留自动继续优化任务。

未来只有出现实际规模、频率或用户体验的新证据时，才另行决定是否重启优化；当前不将潜在优化记为已确认缺陷或新增 TODO。本次发布授权覆盖本轮基准与验收记录，实际版本和 Git 同步结果以受管 release 收据为准。
