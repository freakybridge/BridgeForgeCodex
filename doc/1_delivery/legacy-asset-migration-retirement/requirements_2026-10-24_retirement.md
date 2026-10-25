---
lifecycle: active
validation_status: verified
---

# 旧 Rule 与项目 Memory 语义迁移退役

## 来源与授权

关联 TODO-026；来源为[骨架对标](../../3_reference/skeleton-benchmark_2026-10-24.md)。用户在核实迁移手册用途后明确要求：“直接退役这个skill，因为这是历史遗留的一个功能”。本决定取代第 1 项原先“统一恢复说明”的建议，授权退役该语义迁移功能；其余五项仍待讨论。

## 目标与边界

删除迁移 reference、调用分支、专用 CLI 输入、语义迁移模块及专用测试，更新分发登记、dogfood 与当前说明。旧 CLI 参数明确拒绝，不能静默忽略后继续 Apply。

旧 `.codex/rules/*.md`、`.codex/memory/**` 不再作为语义迁移源。兼容更新保留它们；adopt/低基线重建沿用现有普通项目资产的保留/删除决策，不新增特殊转换或默认删除。Codex 原生 Memory、项目 Hook 独立迁移、版本模型升级、普通同步事务和项目资料保护保持原行为。

本次不修改真实下游、用户级安装、原生 Memory，不提交、推送、发布或归档历史文档。产品功能退役不等于旧文件删除授权。

## 规模、预算与传播

按单一功能退役组织为 M；沿用本轮累计 45 分钟、20,000 token 估算上限、0 个子 Agent，最多两轮实现验证；token 未实测，不报告无依据的周额度占比。预计超额或出现无法在范围内处理的事务回归时停止新增动作，说明实际进度，不静默扩额。

属于通用产品层修改，需要同步模板、共享 Skill、工厂 dogfood 和分发清单。当前 `.codex/bridgeforge-version.json` 的 packages 为空，不涉及独立子包升版；主程序版本与正式 CHANGELOG 留待另行授权的 release。本次需求与调研记录是元文档。

## 验证

- 静态：无悬空 reference、模块和分发登记；历史记录可保留，明确退役后的新决定。
- 同步器：旧文件兼容更新保留；重建按普通资产逐项决定；未决定时零写阻断；显式保留和删除、源漂移及失败回滚继续生效。
- 回归：运行同步器相关单元集、分发和 runtime_flows 集成集；退役专用测试删除，混合测试保留非退役覆盖。
- CLI：旧参数在读取清单或修改项目前拒绝；正常 project-sync 仍可规划。
- 传播：受管构建、manifest 与 dogfood 一致性；不以离线夹具冒充真实下游验证。

## 实施记录

已删除迁移 reference、Rust 模块和调用链，移除 plan/receipt 的专用迁移字段；旧 CLI 参数在读取 stdin、清单或项目路径前明确拒绝。模板与工厂 dogfood 同步，受管 manifest 已重建；当前安装说明、同步设计与上游更新 Playbook 已同步退役决定。

专用迁移测试随功能删除；混合测试保留非退役行为，新增普通资产保留、显式删除、漂移与回滚覆盖。首次同步器测试发现一个既有发布状态断言与 fixture 不符：fixture 有 VERSION、没有 product-packages 配置，应为 `migration-required`；已更正断言，未修改发布逻辑。

已完成检查：

| 命令 | 结果与范围 |
|---|---|
| `cargo test --locked --config scripts/tests/factory-cargo.toml --manifest-path .codex/hooks/Cargo.toml -p bridgeforge-core project_sync::tests -- --test-threads=1` | 43 项通过；覆盖项目同步、资产保护、漂移和事务回滚 |
| `cargo test --locked --config scripts/tests/factory-cargo.toml --manifest-path .codex/hooks/Cargo.toml -p bridgeforge-cli retired_asset_migration_arguments -- --test-threads=1` | 1 项通过，检查三种旧参数输入，拒绝发生于 stdin 或项目读取之前 |
| `cargo test --locked --manifest-path scripts/tests/Cargo.toml distribution_regressions:: -- --test-threads=1` | 20 项通过；首次受限环境运行有 8 项权限错误，原命令提权重跑全部通过 |
| `cargo test --locked --manifest-path scripts/tests/Cargo.toml runtime_flows:: -- --test-threads=1` | 首轮 11 通过、1 失败、1 忽略；失败为组合升级用例读取尚未刷新的工厂生成产物，报 `generated asset receipt drifted: codex.hooks`。完整初始化、共享产物安装、项目 Hook 保留、Memory 隔离同步等其余场景通过 |
| `cargo test --locked --manifest-path scripts/tests/Cargo.toml runtime_flows::project_sync_combined_upgrade_preserves_project_assets_and_is_idempotent -- --exact --test-threads=1` | 工厂受管构建完成后单独复测 1 项通过；未修改该测试逻辑。未为此重复整组长测试 |
| `cargo run --locked --release --manifest-path .codex/hooks/Cargo.toml --bin bridgeforge -- build-assets --factory --project-root .` | `status=prepared`、`prepared=2`、`written_files=4`、`self_test_executed=true`；刷新本工厂 CLI/Hook |
| `.codex/bin/bridgeforge.exe build-assets --factory --project-root . --check` | `status=current`、`read_only=true`、`reused=2`、`built=0`；临时 Git 属性沙箱权限失败后原命令提权重跑通过 |
| `.codex/bin/bridgeforge.exe check baseline --root .` | `state=clean`；临时 Git 属性沙箱权限失败后原命令提权重跑通过 |
| `.codex/bin/bridgeforge.exe manifest --root . --check` | `status=ok`、`changed=false` |
| `.codex/bin/bridgeforge.exe check skill-metadata --root .`、`check project-structure --root .`、`check todo --root .`、`check factory-version --root .` | 无错误；目录检查仅给出既有归档候选和排除目录提示，未执行归档 |
| `.codex/bin/bridgeforge.exe project-sync --asset-migration-manifest=- --apply` | 实际 CLI 返回 BLOCKED，明确说明迁移已退役，没有等待 stdin 或进入 apply |
| `git diff --check` | 通过 |

本地工程验证完成：上述测试按独立用例计共 76 项通过，1 项需要显式真实业务 fixture 授权的集成用例保持默认忽略。初次把工厂产物刷新与读取这些产物的集成集并行，造成旧产物收据失败；受影响项已在刷新后顺序复测通过，后续应先完成准备再运行此类测试。编译仅保留既有 release.rs 的三条 dead_code warning。

用户验收尚未发生；TODO-026 的原始总表条目按现有合同留待验收维护，本需求记录保存最新决定和实施证据。

未执行用户级安装、真实下游升级或实际会话生命周期验证；不把隔离夹具或工厂构建当成这些证据。兼容更新不主动清理下游未登记的历史源码文件；新产品不再分发或加载退役模块。主程序版本与 CHANGELOG 未改，未提交或发布。

## 冻结与恢复记录（2026-10-24）

用户要求暂时冻结并调用普通 `$summary` 保存进度。当前停在“本地实现与工程验证完成、等待用户验收”，上述测试属于退役实施阶段的证据，本次总结未重跑测试或构建。保留现有 `lifecycle`、`validation_status` 和 TODO 状态，不把冻结记成验收或完成归档。

代码、模板、共享 Skill、dogfood、manifest 和文档改动均留在 `main` 未提交工作区；未提交、推送、发布、分发到用户级安装或升级真实下游。恢复时先检查这些输入是否发生变化，再判断原有证据是否仍有效；任何安装、下游或生命周期结论都需要对应实际证据。

本轮可复用经验已在实施记录中保留：退役语义迁移不等于授权删除用户文件；依赖工厂二进制的集成测试必须在受管产物准备完成后运行；权限失败应与功能失败区分，修复执行环境后仅复测受影响项。上述经验不自动写入 AGENTS、Hook 或 Memory。

整个对标任务及剩余五项的恢复位置见[调研冻结点](../../3_reference/skeleton-benchmark_2026-10-24.md#当前进度与冻结点2026-10-24)。本主题尚未用户验收，无归档候选。
