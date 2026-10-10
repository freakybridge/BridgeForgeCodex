---
title: R04 自动索引刷新性能核验
lifecycle: completed
validation_status: verified
date: 2026-10-11
---

# R04 自动索引刷新性能核验

## 确认与授权

用户要求“看看是否需要 confirm，没有的话可以开始了”。保留索引结果和及时性，不改变当前授权、Hook 入口或错误语义，没有需要补问的业务决定。先取得基线，收益成立且不改变合同则实施；不为性能目标削弱正确性。

保留当前未提交 R01–R03，HEAD 为 `1978b367de7376b34e644ea84d6985dea5d12f58`。本轮 R04 区别于历史 R04 独立产物缓存。授权为本地测量、必要优化、验证与文档，不提交、推送、发布或写入真实下游。

## 已核实事实与技术选择

- Stop 在 dirty 时遍历并重建两个索引，最终比较字节决定是否写入；无 dirty 且索引存在时已有跳过路径。
- `project_map_dirty_tracking_is_scoped_and_strict_route_repairs_maps` 明确要求源码事件触发时，外部 AGENTS 修改也应被发现。因此本次不直接按源码正文编辑跳过重建，不改变脏标记范围。
- `reference_source` 对每条 inline/link 引用重新编译行号后缀和 Windows 盘符正则；这是本次保持语义的性能候选，先测量再决定。
- 不引入输入缓存、持久化状态或新的索引 schema；保持 SessionStart/显式刷新、目录发现、排序、输出、错误与副作用顺序。

## 预算和传播

M：45 分钟 / 约 20k 新增 token（估算，未实测）/ 1 名 review-auditor / 最多 2 轮验证，包含基线、同法复测、构建和工厂必需完整检查。无适用周额度系数，不编造百分比；预计超额或必须改变索引合同则暂停受影响动作。

若优化成立，属于产品层通用能力，修改 templates/hooks 并同步 dogfood、manifest 和相关说明；将来显式 release 按 [product] 升版。本次不预写 VERSION、不对工厂执行下游 project-sync apply。若成本不足以支持改动，则保留基线结论，不为数量制造优化。

## 基线与验收

新增可重复的显式离线 benchmark，位于 scripts/tests，使用实际 release Hook 子进程与隔离项目；不把临时诊断脚本变为产品入口。两档确定性文件/文档规模，测无变化、源码正文、文件新增、文件删除、引用修改五种场景，各 5 次，记录中位数与范围。准备输入不计时；只测 Stop 索引路径及其进程开销，不能外推完整 Codex 回复延迟。

基准比较同一 binary profile、同一 fixture 构造和相同样本数；检查输出哈希与无变化场景完全一致、增删和引用变更确实反映到索引。保留现有 scoped dirty/严格刷新/失败传播用例，新增正则匹配边界样本。fixture 自动清理，结果保存在本机 .runtime；不测真实下游、不临时改变业务数据。

## 结果与经验

- 已运行 release Hook 基线，结果与准确的 `binary_sha256` 保存在 `.runtime/R04-before/benchmark.json`，不手抄维护第二份哈希。
- 1000 源码/50 设计文档、每文档 21 引用：no-op 中位数 60.16 ms，正文编辑 dirty 刷新 485.71 ms；5000 源码/200 文档：分别为 76.20 ms 与 1580.85 ms。正文编辑后输出字节完全一致，证实重复刷新有成本，但未据此削弱现有新鲜度合同。
- 选择仅复用 reference_source 内两条固定 Regex，正则文字、归一化顺序和所有扫描/dirty 行为不变；无持久化缓存或新增状态。下节记录同法复测观察值与限制。
- 新增 12 个匹配边界样本已在旧实现通过，包含 Unicode 数字、行列号、Windows 路径、标点、越界和通配引用；保留 `:12.` 原有不匹配行为，不顺带修正语法。

## 复测结果与边界

相同命令 `cargo test --locked --manifest-path scripts/tests/Cargo.toml project_map_refresh_performance -- --ignored --nocapture --test-threads=1`；仅通过 `BRIDGEFORGE_MAP_BENCH_OUTPUT` 选择本机结果位置。前后均使用实际 release Hook、两档相同 fixture、每场景 5 次；报告见 `.runtime/R04-before/benchmark.json` 与 `.runtime/R04-after.json`。10 组两份索引哈希全部一致，增删和引用变更确实更新索引，fixture 自动清理。

| 源码文件/设计文档 | 场景 | 修改前中位数 ms | 修改后中位数 ms |
|---|---|---:|---:|
| 1000/50 | 无变化 | 60.16 | 55.62 |
| 1000/50 | 正文编辑 | 485.71 | 280.64 |
| 1000/50 | 新增文件 | 465.94 | 280.54 |
| 1000/50 | 删除文件 | 486.71 | 268.98 |
| 1000/50 | 引用修改 | 447.63 | 271.02 |
| 5000/200 | 无变化 | 76.20 | 56.76 |
| 5000/200 | 正文编辑 | 1580.85 | 1073.06 |
| 5000/200 | 新增文件 | 1530.06 | 969.14 |
| 5000/200 | 删除文件 | 1466.90 | 975.93 |
| 5000/200 | 引用修改 | 1498.12 | 1012.29 |

正文编辑样本范围：小档前 478.19–531.86 ms、后 267.98–292.68 ms；大档前 1519.24–1760.42 ms、后 976.90–1241.61 ms。完整样本保留在 JSON。no-op 未执行改动路径也有下降，说明系统负载/缓存等噪声存在，不能将全部降幅归因于改动；本次只报告观察值，不承诺普遍提速比例。

测试直接标记 dirty 后以空 stdin 调用 Stop，不计 PostToolUse、高成本观察或完整用户交互延迟；并非真实下游/用户端到端性能验收。基准是显式、可复用的测试入口，默认忽略；没有遗留临时诊断脚本或合成项目。

新实现索引定向回归 4/4 通过。当前版本受管 release build 与 build-assets 安装通过，工作区版本仍为 2.2.0。独立审计 `review-auditor:r01_audit` 返回 passed，确认匹配语义、扫描与 dirty 合同保持一致，复测配置和二进制身份有效；初审指出的手抄哈希错误已改为引用原始 JSON，问题关闭。完整工程检查已通过。

## 最终工程证据

执行 `.codex/bin/bridgeforge.exe git-sync --prepare-release --message-file .runtime/R04-message.txt --audit-file .runtime/R04-audit.json`，覆盖 R01–R04 当前工作区；返回 prepared、checks_passed=7、隔离目标版本 2.2.1。工作区 VERSION 仍为 2.2.0，未提交、推送或发布。

| 检查 | 本轮结果 |
|---|---|
| baseline、skill metadata、manifest --check、project structure | 全部通过 |
| 模板/dogfood 源码全树一致 | 1 项通过 |
| dogfood workspace | CLI 14、Core 215、Hook 23，共 252 项通过，5 项条件忽略；约 723 秒 |
| 完整工厂 fixture | 89 项通过，6 项条件忽略；约 432 秒 |
| 性能基准 | 前后各 1 次显式入口通过，合计 100 个计时样本，10 组输出哈希一致 |
| 独立审计 | passed，建议已关闭 |
| 临时项目清理 | 实际检查 `bf-map-bench-*` 遗留目录为 0 |

开发指纹为 `sha256:46b4fb5a5360b280c5fee2aab4e3fbf94b91e6a79e1b237125a1be19140ca683`；本机收据为 `.runtime/bridgeforge-codex/release-preparation/current.json`。workspace 日志为 `sha256:b60d0563d0d091d27aa7cec7737fb4a0d0d5bdcb414051182e8c66b72eedfea4`，factory 日志为 `sha256:10513cd5ba52e258d5aa29e086c9ba1c6af7dec8a13bc008c392c49560f97ba2`，位于同目录 blobs。

workspace 条件忽略为 4 个子进程 helper/fixture 和显式 live log 检查。factory 条件忽略包括两项既有性能实验、原生 Codex 高成本提醒 smoke、进程 helper、需单独授权来源的 Assist 迁移 fixture，以及本轮索引性能基准；最后一项已单独执行前后测量，其余不计为本轮通过。

本轮在 45 分钟上限内完成，使用 1 名独立审计 Agent，未发生源码实质修复失败；token 未实测。工程验证完成，用户于 2026-10-11 通过 `$summary accept` 验收；真实下游与用户端到端性能未验证的边界保留。

## 累计验收与发布授权

2026-10-11，用户明确调用 `$summary accept` 和 `$git-sync release`：前者确认本会话 R01–R04 已满足约定的本地开发、验证和交付要求，后者单独授权这些累计改动的受管升版、提交与推送。四项需求卡均更新为 completed / verified，发布结果以受管 Git 收据及 CHANGELOG 为准。

验收复用各卡中的真实证据；最后一次工程准备覆盖 R01–R04 当前工作区，指纹为本卡记录的 `46b4fb5a...`，7 项检查全部通过，workspace 252 通过 / 5 条件忽略，factory fixture 89 通过 / 6 条件忽略。验收记录更新只做项目允许的轻量文档核验，不重跑测试、构建或审计。

沉淀的主要经验沿用各卡事实源：R01 用旧行为样本保护不同解析策略；R02/R03 集中共享能力但保留调用方的事务与并发责任；R04 先测量再优化，保留输出等价性及 no-op 噪声边界。现有 AGENTS/Skill 已覆盖这些原则，本次不另提或写入治理规则，不修改原生 Memory。

TODO-004 补充工厂真实使用 refactor-scan 到验收的进展，真实业务下游验证仍为待验证；其他 TODO 保留，无新增明确未解决缺陷。四个已验收 topic 可作为后续 archive-scan 候选，本次不归档、不移动文件。真实下游升级、真实 Memory/runtime 及完整用户端到端性能不包含在本次已完成证据内。
