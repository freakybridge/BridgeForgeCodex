---
lifecycle: completed
validation_status: verified
topic: gpt6-skeleton-upgrade
date: 2026-09-05
scale: M
---

# 板块二：入口分流与只读副作用

验收收据（2026-09-05）：用户明确“同意验收”，本地入口交付按下列证据收口；未执行场景及完整模型对照仍未验证，统一留板块四跟进；不包含用户级安装或真实下游。用户另行授权 Git 同步，结果以同步收据为准。

## 需求与授权

用户确认上一轮六文件修改范围后明确要求“那你开始改吧”，沿用控制文字膨胀要求；本卡整理已有决定，不重复开工确认。

- 修改 bridgeforge-codex 入口、runtime preflight、用户级维护说明、find-doc、sync-docs 和 INSTALL；配套测试、旧 Map 需求修订说明、版本及分发清单。
- 咨询直接回答；诊断、审计或预览检查现有状态，不运行 updater、不刷新 Map、不安装或修复。正常维护、已授权文档同步仍走现有受管入口。
- Map 仅作定位线索，源文件才是证据；不可用时搜索 fallback，不把维护工作交给用户。
- 通用产品层改动进入共享 Skill，更新 VERSION / `[product]` CHANGELOG 与 dogfood 配套；不增加公共 AGENTS 条款或授权状态系统。
- 不改 updater、Map 算法或生命周期 Hook，不承诺整个 Codex 后台零写入；不安装用户级副本、不修改下游、不提交推送。第一板块及初审记录保留。

## 预算与验收

M 级：45 分钟、20k 新增 token 估算（未实测）、最多 1 个独立 Agent、2 轮验证；预计超额先停止超额动作。六文件逐项统计非空白字符，优先替换和合并，不新增 reference。

- 只读诊断不先升级；运行时缺失或不符只报告，已有状态不得冒称最新。
- 正常维护保留 updater、来源检查与事务回滚；只读 find-doc / sync-docs 跳过刷新并核对原文件。
- 预览只列拟修改文档，正常同步仍可刷新并更新；两类输出不混淆。
- 静态、模型前向检查与实际运行分别留证；不以措辞匹配证明模型选择正确。
- 版本、manifest、metadata、dogfood、完整 Rust 回归与隔离 fixture 通过；真实用户级安装、下游及全生命周期 smoke 不在本轮声明内。

## 实施与验证

六文件修改与 `1.14.10` 发布配套已完成；没有新增 reference。第一板块规则、初审及交付记录共 12 个文件的前后 SHA-256 一致；updater 和 Rust 运行时代码对 Git diff 无变化。

### 文字控制

按 LF 行数和非空白字符统计，不等同 token；六文件 **11,143 → 10,763 字符（−380，−3.41%）**，每份均减少。分流标题使总行数由 334 增至 338。

| 文件 | 行数（前→后） | 字符（前→后） |
|---|---:|---:|
| `bridgeforge-codex/SKILL.md` | 92→98 | 3878→3863 |
| `references/runtime-preflight.md` | 18→19 | 725→647 |
| `references/user-skill-maintenance.md` | 15→13 | 776→743 |
| `find-doc/SKILL.md` | 79→79 | 2080→1983 |
| `sync-docs/SKILL.md` | 48→48 | 1135→995 |
| `INSTALL.md` | 82→81 | 2549→2532 |

旧 Map 需求仅增加修订指针、保留原文；本卡集中记录证据，索引仅链接。版本与 manifest 机械更新，CHANGELOG 新增一条说明；测试文件净减 5 行，不新增措辞匹配测试。

### 前向行为与审计

`review-auditor` `/root/block2_review` 在未读本轮需求卡或 diff 前，实际执行两项只读请求：

| 请求 | 实际动作与结果 |
|---|---|
| 用 bridgeforge-codex 核对 INSTALL 的诊断与升级顺序 | 读取 Skill、公共 AGENTS 和 INSTALL，交付当前文档结论；未运行 updater 或进入维护 |
| 用 find-doc 查 Map 自动生成需求 | 文件名与 README 检索后读取唯一需求及输出 reference；未刷新 Map，也未要求用户维护 |

该过程只用文件读取和限定搜索；未启动其他 Agent、未运行 doctor / planner / 测试。之后同一 Agent 独立读取真实 diff 与调用链，最终无未解决阻断项，确认安全流程和生命周期边界保留。缺失 runtime、sync-docs 预览 / 同步、正常升级仅静态推演，不冒称模型实测；未做旧新规则 A/B。

### 自动验证

| 验证 | 实际命令 | 结果 |
|---|---|---|
| Template | `cargo test --locked --config scripts/tests/factory-cargo.toml --manifest-path templates/hooks/Cargo.toml --workspace -- --test-threads=1` | 137 通过，4 个子进程辅助入口 ignored |
| dogfood | `cargo test --locked --config scripts/tests/factory-cargo.toml --manifest-path .codex/hooks/Cargo.toml --workspace -- --test-threads=1` | 137 通过，4 个子进程辅助入口 ignored |
| 工厂集成 | `cargo test --locked --manifest-path scripts/tests/Cargo.toml -- --test-threads=1` | 81 通过、0 失败；1 个辅助入口及 1 个需额外授权的 Assist fixture ignored |

修订的 `project_maps_are_not_fixed_managed_payloads` 实测通过：当前合同不把运行时目录或旧 Map 路径作为固定分发资产，旧手工提醒 SOP 不存在。它不证明模型分流；刷新正确性、幂等、脏标记及生命周期仍由既有真实函数测试覆盖。

- `cargo run --locked --release --manifest-path .codex/hooks/Cargo.toml --package bridgeforge-cli -- build-assets --project-root .`：`built`，两份收据生成；项目 CLI / Hook 的 `self-test --json` 均 `ok`。
- `.codex/bin/bridgeforge.exe manifest --root . --check`：`changed=false`；同一 CLI 的 `check baseline --root .`：`clean / 1.14.10`；`check factory-version --root .`：`healthy=true`。
- `check skill-metadata --root .`：无 issue / warning；`check project-structure --root .`：无 error、仅既有归档提示；`check instruction-source`、`git diff --check`、`cargo fmt --manifest-path scripts/tests/Cargo.toml -- --check` 均退出 0。

本轮只重建工厂运行文件；用户级安装、真实下游、全生命周期实机 smoke 和用户试用未验证。生命周期自动维护保留，不宣称整个会话或后台零写入。

本轮约 14 分钟、1 个独立 Agent、1 轮完整验证；token 未实测，未超已知预算。源码、自动检查与独立审计完成，待用户试用；未提交、推送或发布。
