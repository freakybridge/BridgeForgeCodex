# schema4 autopilot 工作交接

唯一需求事实源为 [确认卡](requirements_2026-10-21_schema4.md)，产品接口见 [接入合同](schema4-contract.md)。本文件记录运行位置与恢复证据，不复制需求正文；用户已于 2026-10-22 验收，决定以[正式记录](requirements_2026-10-21_schema4.md#用户验收2026-10-22)为准。

## 现场与授权

- 仓库 BridgeForge，分支 main；保存时基准 HEAD 为 `5f2b1350ce39ad4ec470c3178bcc470f1a0ce81a`，不是任何最终提交。
- 有本任务未提交改动：模板与 dogfood Rust release/CLI/事务接入、共享 git-sync Skill及镜像、测试、schema4 工厂配置、清单和文档。暂存区无业务变更，未提交、未推送、未发布；当前 VERSION 为 2.7.0。
- 用户已批准 autopilot，时间仅估算；1 名只读 review-auditor、最多 2 轮完整验证；禁止真实下游与用户级安装写入。
- 同一 Goal 所属线程为 `01a15ecc-fe52-7650-b9fb-ad104e389499`，状态以原生接口为准；App Server 调整后的结构化 tokenBudget 为 10000000，未重置创建时间或用量。Goal 前成本未实测，子 Agent 是否完整纳入计量未验证；跨机器不能默认 Goal、进程或剩余额度自动恢复。

## 任务位置

| 编号 | 状态 | 内容与边界 |
|---|---|---|
| S4-1 | 实现与本地工程验证完成 | 两种策略共用 Rust/Python、最终差异与逐对象分析、接口转发影响、范围化证据、目标与不兼容接受、历史迁移与生命周期 |
| S4-2 | 已完成本机迁移和构建 | 工厂 schema4 synchronized；产品映射三个 Cargo workspace，保留原生继承；未提前改 VERSION |
| S4-3 | 已通过 | 第二轮 7/7 必需检查与隔离目标 3.0.0 prepared；首轮旧测试失败经过修正及复验 |
| S4-4 | 已交付并获用户验收 | 2026-10-22 `$summary accept`，TODO-015 已完成；文档、清单与镜像已同步；未提交推送、真实下游或用户级分发 |

核心生产实现位于 `templates/hooks/crates/bridgeforge-core/src/release/schema4/`、`release/schema4.rs`，复用同目录 Rust/Python 原生适配器及 Git 事务层；`.codex/hooks/` 是逐字镜像。Stratus 只读成果以确认卡指定提交和测试场景复用，BT 业务缓存与账本没有搬入 Forge。

## 已有证据

| 验证 | 实际结果 |
|---|---|
| schema4 定向 | 27 passed；含接口转发、局部证据组合、基线篡改、Python 环境字节漂移、两策略两语言与混合 |
| Git 同步事务定向 | 59 passed；含回滚、并发、Hook 改写、缓存、目标远端与预检 |
| Rust 原生 | 32 passed；含实际 Cargo locked/offline 与跨 workspace 锁边界 |
| 首次配置/迁移 | 6 passed；含单引号 TOML、零写入、指纹漂移 |
| 工厂映射 | 正常同步、成员 literal 漂移、锁陈旧版本、缺失本地边通过 |
| 隔离工厂真实 CLI | 自动构建、pre-commit 与本地测试远端同步链路通过；不是当前仓库或真实下游发布 |
| 独立审计 | `/root/schema4_audit` passed，无剩余 P1/P2；指纹 `sha256:0b7b23ec42b4cbf61be3c3042dde73e8b3dc52b39560787fe2e6f20a13b82274` |
| 最终受管产物 | 已构建，baseline clean；source tree `sha256:cdf25fb8be02607a1ad5639bdf520f80ab548753e7ee3c4257b4aebbee6f1a8d` |
| 完整 workspace | 442 passed / 0 failed / 13 默认 ignored；日志 `285f4b014e93f6c64c7df121ee210b07ea8dd72a879411cb785377616d449c6a` |
| 完整工厂 fixture | 96 passed / 0 failed / 7 默认 ignored；日志 `6f3207183658211aa3562489a742277e432867c5c31514f5e7e81693ff7fb1f7` |
| 实际 CLI 补验 | 33 passed；V2/V4/V5 证据缺口已补齐并经独立复核；安装 CLI SHA-256 `88b184936909b5615a2dc155b2211e54870e040ef5590592bbbf1adde6857c84` |
| 目标准备 | 第二轮 7 checks executed，0 reused，target 3.0.0 prepared；当前 VERSION 未变；最终文档按既有缓存机制重新绑定 |

完整七项检查和隔离目标版本构建已完成。四项固定版本 uv/Poetry/PDM 真实锁工具用例、显式性能/现场日志、原生 Codex Stop Hook 十轮及真实 Assist fixture 未执行；ignored 中也有由父测试调用的子进程 helper，不混称为功能全部通过。真实下游升级、用户安装、业务 runtime 和真实发布仍未验证。2026-10-22 用户验收本次工程范围，TODO-015 已完成；本次总结未重跑测试或构建。

最终交付就绪独立审计 passed，无剩余 P1/P2。主任务原检查环境返回 prepared；默认沙箱与受管执行的 LOCALAPPDATA、TEMP、TMP 摘要不同，子 Agent 另有线程身份差异，因此其他环境现场可返回 stale。缓存绑定完整环境，不跨环境承诺复用，也不因此自动重跑完整测试；恢复时保留这一条件。

## 恢复与本机依赖

本机 `.runtime/schema4-validation/` 保存定向日志、33 项补验、最终审计记录、消息与分析文件、App Server 调额收据及原始 `prepared-round2.json`；`.runtime/bridgeforge-codex/release-preparation/` 保存当前准备记录、按内容哈希存储的日志/目标产物和缓存。这些忽略文件、Rust 工具链和 `.codex/bin/` 产物不随本交接自动迁移。其他输入的旧 prepared 不代表本轮通过；必须按真实输入核对或恢复。不要复制凭据、直接修改 Goal 数据库或手工运行真实 Git 提交。

截至工程结果保存时，完整与定向测试进程均已结束；只读审计 Agent 没有写入权限，没有其他写入 Agent。不要跨机器假定这一进程快照仍有效。

1. 核对当前分支、HEAD、Git 差异与原生 Goal，保持原任务及累计预算，不重建目标。
2. 使用 `--development-status` 核对现有准备与开发指纹；同机同输入的完整结果可复用，缺失缓存或输入漂移不能冒充通过。
3. 若仅变更已声明记录文档，按现有准备缓存轻量复核并重新绑定完整事务清单；不要无依据重跑长回归或重置验证轮次。
4. 本次工程范围已获用户验收；只有明确的新授权才提交、推送、发布或修改真实下游，不因恢复任务自动重启已完成 Goal。

接续提示：读取 `doc/1_delivery/git-sync-release-rewrite/handoff_2026-10-22_schema4.md`，先核对当前分支、代码与文档差异，再在已有授权范围内从下一步继续；缺失证据如实标明。

尚未提交和推送。换机需要用户另行授权 Git 同步；交接文档不等于另一台机器已收到未提交改动或本机缓存。
