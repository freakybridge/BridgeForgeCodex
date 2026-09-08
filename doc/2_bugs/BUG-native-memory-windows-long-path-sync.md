---
lifecycle: completed
validation_status: verified
---

# Windows 长路径阻断 Native Memory 同步

## 用户现象与授权

2026-09-08 用户报告 Memory 同步不正常，授权修复。随后确认合并本机与既定 GitHub Memory 仓库：两边独有文件全部保留，同名不同内容的 `skills/bridgeforge-downstream-sync/SKILL.md` 采用本机版本，保留冲突备份。不提交或推送本工厂代码，不直接覆盖用户级程序。

## 事实与根因

用户级 1.16.0 Hook 配置和运行收据有效；真实 reconcile 报 Windows os error 3，旧状态迁移记录为 baseline-incomplete，当前没有有效 lastReceipt。已保存冲突快照为本机独有 88、远端独有 61、相同 8、同名不同 1；执行前必须重新捕获，不把历史快照当作当前目录。

源码 `memory/mod.rs::atomic_replace_file` 把普通路径直接传给 MoveFileExW。Rust 文件读写支持长路径，但该原生调用未转换成 Windows 扩展绝对路径；深层快照写入失败。新增隔离回归在大于 260 字符路径上先复现相同 os error 3。

修复规范化源文件与目标父目录，再拼接目标文件名，支持目标尚不存在及覆盖既有文件；不改变 Memory 内容、冲突策略和原子替换标志。传播四问：通用产品层，进入 templates 并同步 dogfood；版本 1.17.3 与 CHANGELOG [product]；刷新受管清单和本仓库程序。S 级定向修复，复用用户授权，未委派 Agent。

## 后台测试的独立问题

原后台测试单独运行通过，在 Memory 模块并行运行时仍报 sentinel 被占用。测试故意创建的可继承句柄处于整个测试进程中，其他并行测试通过 std::Command 启动的子进程可能继承它，无法据此证明 Memory worker 泄漏句柄。

将原探针放到仅运行这一用例的独立测试进程；探针 ready 文件通过重命名发布，父进程通过进程句柄等待终止后清理。原有无控制台、无标准句柄、输入输出、参数与额外句柄隔离断言保留，不修改产品后台创建方式。

## 验证记录

- 源码：长路径回归修复前失败、修复后通过，覆盖首次写入、覆盖与完整快照生成/读取。
- fixture：`cargo test --locked --config scripts/tests/factory-cargo.toml --manifest-path .codex/hooks/Cargo.toml -p bridgeforge-core memory:: --no-fail-fast` 最终 31 passed、0 failed，包含并行后台测试、冲突保护及同步期间并发写入保护。
- 产品传播：模板修复已落盘；未发布或安装到用户级。
- dogfood：源码、manifest 和 Hook/CLI 产物已同步；1.17.3 baseline clean，factory-version、manifest --check、project-structure 和 diff --check 通过。产物源码树收据为 `sha256:858b9b848123f6618875aa0ee000f0e582cd7059c51b78f24c074bad4756f6d0`。
- 真实下游：未修改其他项目。
- runtime：已复现已安装程序 os error 3；用户已授权真实合并，结果待补。使用相同用户级受管入口和同一 Codex home 的扩展长路径写法恢复当前操作，不修改持久 Hook 配置。

保留失败和冲突现场。只有取得同步完成收据后才能声称本机与远端已合并；用户级旧程序未更新前，不声称后续自动同步的长路径问题已永久消除。

## 真实合并与完整 workspace 复核

相同用户级 1.16.0 受管入口，显式指定同一 Codex home 的 Windows 扩展长路径后，reconcile 成功返回 conflicted，确认之前错误来自路径处理。新快照与已批准范围一致，继续通过 resolve 为所有独有文件选择其存在的一侧，唯一同名差异选择 local；受管程序返回 healthy / resolved。

真实 GitHub Memory 合并提交为 `07395d5b6ba5af1d4e6d13270d63cca499440d20`，revision 33。合并后共 158 个文件；逐文件核对 last-synced manifest、预期两边并集和本机文件 SHA-256 全部一致。冲突双方备份保留。该提交只属于用户已批准的 Memory 仓库，不是本工厂代码提交。

`cargo test --locked --config scripts/tests/factory-cargo.toml --manifest-path .codex/hooks/Cargo.toml --workspace --no-fail-fast` 最终退出 0，原后台句柄用例在完整并行运行中通过。另一项工厂 git-sync runtime 发布测试故障未在本轮修复或重跑，不扩大结论。

随后使用本仓库已构建的受管 1.17.3 CLI，以普通路径执行真实 reconcile，返回 healthy / noop，验证长路径源码修复在真实同步目录有效。最后用户级 status 为 healthy，pending 和 activeConflict 均为空、workerActive=false，lastReceipt 仍指向 revision 33 的合并提交。实际本机与远端已经收敛；用户级自动 Hook 仍使用 1.16.0，持久修复需要后续正式发布与安装，本轮未覆盖用户级程序。

## 后续可靠性排查

用户继续要求检查所有可能原因，见 [自动同步可靠性闭环](../1_delivery/current-baseline-project-asset-migration-and-native-memory-sync/requirements_2026-09-08_auto-sync-reliability.md)。旧用户级 1.16.0 随后再次触发相同长路径故障，证明单次手动恢复不能替代正式安装。1.17.4 又修复预约损坏、PID 身份、健康提示及基线中断恢复；05:57:22 UTC 使用项目修复版临时启动的真实后台 worker 成功清空队列并返回 healthy/noop。正常会话使用修复版的生命周期验证仍待正式发布安装，Bug 不据此关闭。

## 最终验收

2026-09-08 用户明确同意验收并授权 git-sync、正式安装和实测。修复版 1.17.5 已经官方 updater 从发布提交 `3e2d7e9b0639e6f2a2bed7ecf58d7e524db7d2d8` 安装到用户级；self-test、doctor 通过。真实 app-server 生命周期观察退出 0，覆盖 SessionStart、Stop 和 SessionEnd；宿主退出后后台继续完成，06:28:15 UTC 最终 healthy/noop、pending/worker 清空。完整收据与六类证据见关联需求卡，源码、传播、dogfood、fixture、正式 runtime 均已验证；真实下游业务升级不在授权范围，未执行。长路径造成的真实自动同步阻断据此关闭，保留冲突备份和原生本机索引。
