---
lifecycle: completed
validation_status: verified
topic: gpt6-skeleton-upgrade
date: 2026-09-05
scale: M
---

# 板块一：任务推进与授权

验收收据（2026-09-05）：用户明确“同意验收”，本地规则交付按下列证据收口；未执行场景及完整模型对照仍未验证，统一留板块四跟进；不包含用户级安装或真实下游。用户另行授权 Git 同步，结果以同步收据为准。

## 已确认范围

用户已同意上一轮修改计划，并要求控制所有修改文件的文字膨胀。本卡直接整理已有决定，不重复访谈；初审历史保留在 [audit-01](audit-01_2026-09-05.md)。

- 修改公共 AGENTS、六个流程 Skill、develop 两份 references 和澄清手册，共 12 个主要文件；同步自身镜像与发布配套。
- 已授权的有限只读取证先行；需求确认与实施授权分开记录；跨 Skill 继承决定、授权与预算，仅重核实质变化。
- 同一问题连续两次实质修复失败停止盲修；环境重试不计修复次数；用户主动求外援无需次数达标；恢复不重跑已完成阶段。
- 暂停说明真实原因、影响与恢复条件，引用实际触发的 Skill 条款。
- 不新增授权状态系统；不修改个人 AGENTS、其他板块、真实下游或用户级安装；未授权 commit / push。

## 预算与文字约束

M 级：45 分钟、20k 新增 token 估算（未实测）、1 个独立审计 Agent、最多 2 轮验证；按原任务累计，预计超额时报告并停止超额动作。

12 个主要文件以非空白字符总量净减少为目标，逐文件统计；公共新边界用替换和合并容纳，不把重复说明搬到新 reference。新增记录仅保留决定、证据和未验证项。

## 验收

- 只计划不实施、模糊大需求先取证、仅确认需求不编码、已授权开工不重复问、无卡不重访谈。
- collab 继承授权；debate 不把讨论自动转编码；主动 escalate 无最低次数；失败与环境重试正确区分；范围扩大仍受授权约束。
- 静态检查覆盖 metadata、引用、镜像、manifest、版本及受管 runtime；行为检查与静态检查分开记录，不以字符串匹配自证模型行为。
- 独立 review 读取真实改动；完整回归按发布要求执行；真实下游与用户级安装不在本次验收声明内。

## 实施与验证记录

通用产品规则已修改，版本为 `1.14.9`；Template 与工厂镜像同步，根项目专区和初审历史未改。未修改 Agent 配置或其他板块。

### 文字控制

按 LF 行数及非空白字符统计，不等同于 token。12 文件合计 **874 → 842 行，24,429 → 23,663 字符（−766，−3.14%）**；没有新增 reference 承接重复条款。

| 文件 | 行数（前→后） | 字符（前→后） |
|---|---:|---:|
| `templates/AGENTS.md` | 141→144 | 5121→5122 |
| 根 `AGENTS.md` | 166→169 | 6794→6795 |
| Template / 工厂 `codex-hook-signals.md`（每份） | 29→26 | 683→603 |
| `skills/plan/SKILL.md` | 44→41 | 733→672 |
| `skills/confirm/SKILL.md` | 104→95 | 2894→2743 |
| `skills/develop/SKILL.md` | 83→72 | 1820→1708 |
| `skills/collab/SKILL.md` | 77→74 | 1643→1481 |
| `skills/debate/SKILL.md` | 73→71 | 1351→1320 |
| `skills/escalate/SKILL.md` | 73→71 | 1080→1019 |
| `develop/references/ml-delivery.md` | 28→28 | 908→907 |
| `develop/references/agent-execution.md` | 27→25 | 719→690 |

版本、Cargo 与 manifest 只做机械配套，CHANGELOG 增加一条发布说明；本卡集中记录需求及证据，索引仅保留入口，不新增重复报告。

### 独立审计与行为证据

`review-auditor` `/root/block1_review` 独立读取真实 diff，发现并复核关闭 1 个 P2：confirm 的“升档必停”与 develop 的“原预算覆盖则继续”冲突。最终无未解决 P0/P1/P2；公共区、镜像及项目专区保留核验通过。

- 实际前向检查：独立 Agent 使用待测 plan 与公共 AGENTS，只读核对本话题索引及两份文档后交付计划；未写盘、未实施、未追加开工确认。
- 静态推演覆盖无卡已有授权、仅确认需求、只读协作、自由文本、预算继承、collab 分派、debate / escalate 及范围扩大；不得等同实际模型运行。
- 未做旧新规则对照或其余场景的独立运行；真实下游、用户级安装和用户试用未验证。

### 自动验证

| 验证集 | 实际命令 | 结果 |
|---|---|---|
| Template | `cargo test --locked --config scripts/tests/factory-cargo.toml --manifest-path templates/hooks/Cargo.toml --workspace -- --test-threads=1` | 137 通过，4 个子进程辅助入口 ignored |
| dogfood | `cargo test --locked --config scripts/tests/factory-cargo.toml --manifest-path .codex/hooks/Cargo.toml --workspace -- --test-threads=1` | 137 通过，4 个子进程辅助入口 ignored |
| 工厂集成 | `cargo test --locked --manifest-path scripts/tests/Cargo.toml -- --test-threads=1` | 重跑 81 通过、0 失败；1 个辅助入口及 1 个需额外授权的 Assist fixture ignored |

覆盖锁定构建、迁移事务、项目所有权保留、Hook、临时 Git 收发及隔离 fixture；不访问真实 Memory 远端。首轮工厂测试复制旧运行文件导致 1 项失败；重建后该项用 `runtime_flows::project_sync_real_contract_preserves_project_owned_zones_rows_and_hooks -- --exact --test-threads=1` 单独复测及完整重跑均通过。

本地受管构建：`cargo run --locked --release --manifest-path .codex/hooks/Cargo.toml --package bridgeforge-cli -- build-assets --project-root .` 返回 `built` 和两份资产收据。直接从目标 `.codex/bin/bridgeforge.exe` 启动时曾替换失败并回滚；改从构建目录运行同一官方实现后成功，未修改运行时代码。

最终门禁均实际执行，退出码 0：

- `.codex/bin/bridgeforge.exe manifest --root . --check`：`changed=false`。
- 同一 CLI 的 `check baseline --root .`：`clean`、`1.14.9`，包含产物及收据；`check factory-version --root .`：`healthy=true`。
- `check skill-metadata --root .`：无 issue / warning；`check project-structure --root .`：无 error，仅既有归档提示；`check instruction-source` 与 `git diff --check`：通过。
- `.codex/bin/bridgeforge.exe self-test --json` 与 `.codex/bin/bridgeforge-hook.exe self-test --json`：均 `status=ok`。此前错误自检参数的输出不计通过。

收据在 `.codex/bin/build-receipt-cli.json`、`build-receipt-hook.json`，来源与 lock hash 由 baseline 核验；不手写 hash。CLI 自检不等于完整 Codex 生命周期实机 smoke。

本轮约 26 分钟、1 个独立 Agent，未超过 2 轮验证；token 未实测。源码、自动检查与独立审计完成，待用户试用；未提交、推送或发布，未安装用户级 Skill 或更新真实下游。
