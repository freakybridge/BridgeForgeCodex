---
lifecycle: completed
validation_status: verified
---

# 唯一发布模型与一次性历史分界

## 已确认目标与授权

TODO-022。用户确认：整体发布一次，哪些包有实际变化，就更新哪些包的版本；下游更新实现本方案的新版本后默认使用唯一模型，日常 release 不再考虑旧格式兼容。

沿用已确认边界：规则分界与正式发布基线分开；已有未发布改动保留；不删除 Git 历史、不强推、不借分界自动升级业务版本。普通 git-sync 不升版，无发布变化不制造空版本。只支持现有已实现的 Rust/Python 适配能力，不借本轮扩展新语言。

用户已授权生成计划并开发，拒绝原 240 分钟提案后明确本轮硬上限为 1 小时。实施窗口为北京时间 2026-10-23 19:32–20:32；最多 1 个独立审计 Agent、2 轮相关验证，token 估算上限 30k、未实测。到点保存实际成果与缺口，不自动追加，不改变下游及 Git 授权边界。

## 现状与最小方案

现有 `schema4/plan.rs` 的独立发布路径已能按主程序和包分别计算版本；Rust/Python 原生版本及锁文件改写、生命周期、漂移检查和 Git 事务已有实现。本轮复用这些能力，不重写发布引擎，不以全面文件清理或架构改造作为交付前提。

当前 `configuration.rs` 接受 synchronized / independent 两种策略；工厂自身也使用 synchronized。`lineage.rs` 从 migration_head 之后要求每条提交为 schema 4，Stratus 的 44d7e871 → 21684676(schema 3) → 496937eb(schema 4) 因此阻断。仅改当前配置或手改 migration_head 不能构成安全切换。

| 顺序 | 优先级 | 最小改动 | 验证 |
|---|---|---|---|
| 1 | P0 | 日常发布只接入已有按包变化升版路径，删除双策略选择入口；保留主程序版本与必要原生版本载体的明确映射 | 主程序和受影响包升版，未改包不升版；无包项目、无变化 release 仍有效 |
| 2 | P0 | 增加一次性受管历史分界：封存可验证的既有发布事实，分界后只走新合同；正式发布基线保持独立 | 重放已知 schema 3 中间提交；累计未发布变化不消失，版本不被重置或降低 |
| 3 | P0 | 将切换接入现有项目更新 plan/apply 事务，保留项目配置与包身份；不提供长期旧模式回退 | 更新和切换共同校验输入、原子应用/回滚，不出现工具已更新但发布配置仍未接续的半完成状态 |
| 4 | P1 | Forge 自身通过同一合同适配；同步模板、dogfood、必要 Skill/文档和分发清单 | 工厂自身不保留另一套发布策略；镜像与受管产物一致 |
| 5 | P1 | 只验证受影响的发布、切换和更新保护，复用不受影响的已有证据 | 定向检查、必要隔离端到端与独立审查；不默认跑全量回归 |

预计涉及：`release/schema4/{configuration,migration,lineage,inputs,plan}.rs`、`release_history.rs`、`release/setup.rs`、CLI 模式入口、`project_sync` 更新事务接线、工厂发布配置与对应测试。原生语言适配器及 Git 冲突/stash/回滚实现以复用为主，只有直接受影响时修改。

旧格式转换仅属于一次性切换，不进入日常发布的兼容分支。复用现有内部解析/原生改写辅助函数不等于继续提供旧发布模式；不为“删光旧文件”额外重写稳定能力。

## 验收与停止点

1. 下游更新后只得到一个发布模型，不再询问 synchronized/independent；不需要再手工修改 schema 或迁移边界。
2. 受影响包分别升版，未变包保持；只因依赖包版本变化不得机械带升所有调用方，实际依赖约束变化继续按已有规则分析。
3. 分界不能吞掉未发布变化；不能允许未来旧格式提交跨过新边界，也不能无限自动移动边界来掩盖问题。
4. 基线、归属、完整指纹、版本、锁、Hook、提交内容和远端竞态保护保留。无法证明的输入阻断，不伪造通过或降级普通同步。
5. 工厂内的定向/隔离验证与真实下游现场验证分别记录；本轮不修改真实下游、不提交推送、不部署。
6. 具体配置封存格式、初始化入口和更新事务接线由实现前的直接调用核对确定；涉及新信任假设或额外业务改写时先明确，不以“无兼容”代替证据。

## 规模与传播

规模 L：跨发布合同、历史边界和更新事务，不能当作只改一个默认值。缩减的是不必要的重写和无关验证，不删验收要求。预算按上述 1 小时窗口执行。

属于通用产品层，源改动进入 templates/shared skills 并同步自身 dogfood。取消旧策略/配置合同可能是不兼容变更；正式版本和 CHANGELOG 在另行授权 release 时处理，本轮不预写 VERSION。

## 当前进度

源码与模板/dogfood、Skill 及合同修改完成。主程序版本映射命名为 product_manifests，和独立包发现范围不得交叉；不保留 synchronized 日常发布分支，也不允许显式目标为未变包制造升版。

配置仍使用 schema 4 容器，以 release_model=product-packages 明确新合同。切换时保留正式发布 revision，把 migration_head 设为固定 HEAD；旧 schema 4 同样执行一次性切换。分界之后的普通提交和正式发布边界均要求新模型，已存在新模型的历史不允许被新的分界覆盖。

Update/Adopt 将切换配置、输入指纹、读集合和回滚快照纳入既有原子事务。全新 Init 若没有正式 Git 基线，不伪造发布事实，收据保留 migration-required；首次 release 前经受管配置入口建立基线。旧项目缺完整历史、未忽略 .runtime 或原生版本/身份无法证明时仍阻断，不能默默跳过。

### 定向验证记录

使用 `cargo test --locked --config scripts/tests/factory-cargo.toml --manifest-path .codex/hooks/Cargo.toml -p bridgeforge-core -- --test-threads=4 --exact <测试全名>`，只运行所列测试集，不执行 workspace 全量回归。

| 检查 | 结果及覆盖 |
|---|---|
| 第一轮选定 13 项 | 11 通过、2 失败，耗时 284.99 秒。事务测试暴露 baseline 顶层字段白名单缺 release_model；正式边界测试的 CHANGELOG 标题缺方括号。对应实现和夹具分别修正 |
| 第二轮 6 项 | 全部通过，耗时 89.65 秒；重测上述 2 项、补齐显式目标及依赖约束断言，并覆盖合同字段严格校验、全新 Init 延后初始化 |
| 合计 15 项唯一相关检查 | 通过；第二轮复用第一轮不受修正影响的 9 项证据。包括 Rust/Python/混合升版、workspace 继承拆分、未变包不升版、净撤销无变化、旧4→旧3→旧4分界且不吞未发布修改、旧格式重新引入拒绝、产品映射与注册包不得交叉、文件/HEAD 漂移、确认缺失、更新失败回滚 |
| 独立审计 | review-auditor 发现产品 workspace 间接覆盖注册包、正式边界漏检模型两项 P1；修复并补测试后复核无剩余阻断。审计只读，没有代跑测试或访问下游 |

首次以模块名筛选到 45 项时，发现包含耗时较高的历史归属修复集，主动中断并收窄到上述 13 项；该中断运行不算通过，不作为完整回归证据。一次测试编译因新增 SyncPlan 字段未补夹具而失败，修正后重新编译；未隐去失败。

复用既有 Git 冲突、stash、提交树、回滚和远端竞态实现；这些路径本轮没有更改，不组织重复远端测试。首次新合同正式发布前仍从不可变 Git 对象核验封存的原生历史事实，这不提供旧发布模式或旧算法回退。真实下游升级、真实远端发布及业务 runtime 未执行。

### 最终产物与工厂自身接续

2026-10-23 20:02（北京时间）完成，实施窗口约 30 分钟，低于用户指定 1 小时硬上限；1 个独立审计 Agent、2 轮定向验证，token 未实测。未追加预算。

| 证据 | 结果 |
|---|---|
| 受管 `build-assets --project-root D:\Quant\BridgeForgeCodex` | CLI/Hook 均 built；原始收据 `.runtime/single-release-model/build-receipt.json`，共同 source_tree_sha256 为 `sha256:95aae51282e1b795d94474c02aea02d3710f3c7cc93627af49e52e5a5e5755f7` |
| 新 CLI `self-test --json` | status=ok，具备 git-sync-product-packages-v1；本轮不预写版本，二进制仍报 4.0.2 |
| Forge `git-sync --migration-plan/apply` | 经真实指纹核对应用，不手写 baseline；收据 `.runtime/single-release-model/forge-migration-receipt.json`：migrated、sync_performed=false、version_changes=false |
| Forge 配置 | release_model=product-packages、strategy=independent、source_schema=4；正式发布与规则分界均为 `e534f568ee4afd29c54683b01c3094f3bce17229`（工厂当前 HEAD 恰为正式发布）；3 个主程序 Cargo 映射保留，不存在另一套工厂发布策略 |
| 新 CLI `git-sync --release-preview` | status=analysis-required、model=product-packages、VERSION=4.0.2、affected_packages=[]、writes=[]；收据 `.runtime/single-release-model/forge-preview.json`。这是零写入输入预览，不是正式发布或发布就绪声明 |
| `check baseline --root .` | state=clean，验证模板、dogfood、真实 CLI/Hook 收据；`.runtime/single-release-model/baseline-receipt.json` |
| manifest / factory-version / TODO / Skill metadata / diff | `manifest --check` changed=false；factory-version healthy=true；TODO errors=[]；Skill issues=[]、warnings=[]；git diff --check 通过。Skill 首次误把单个 Skill 当根目录，修正为 skills 后检查通过 |

上述开发收口时，实现和工程验证完成；根 VERSION、CHANGELOG 和 Git HEAD 未改变，没有对工厂执行下游 project-sync adopt/apply，没有修改骨架版本 stamp。真实下游、用户级分发和业务 runtime 未执行，不能宣称 StratusAgent 已获得本次修复。验收及发布授权见下节，实际 Git 发布以受管终态收据为准。

## 用户验收与发布授权2026-10-23

用户明确调用 `$summary accept`，确认本轮 TODO-022 开发交付符合需求；据已有 15 项相关检查、独立审查、受管构建、工厂自身切换和零写入预览记录验收，TODO-022 标记已完成。此次总结不重新编译、测试或审计，不把真实下游与业务 runtime 未验证改写为通过。

用户同时明确调用 `$git-sync release`，授权本轮 Forge 改动通过当前受管入口提交、升版及推送；此授权不包括真实下游升级、用户级分发或部署。旧发布模式退役属于明确要求的不兼容变更：用户此前确认“从当前版本后不再考虑历史兼容性，下游更新当前版本后就默认主程序＋多语言包这一种模式”，并确认“整体发布一次，哪些包有实际变化，就更新哪些包的版本”。因此本次产品按 major 分析，预期 4.0.2 → 5.0.0；不再重复索取相同业务决定，实际目标和结果以受管预览及发布收据为准。

### 经验沉淀

1. 发布被旧 schema 中间提交阻断时，当前配置已升级不代表历史链满足新合同。规则切换点和正式发布基线必须分开，不能直接移动正式基线来消除报错，否则会漏掉未发布改动。
2. 主程序 Cargo 映射必须检查实际 workspace 成员与注册包是否相交，不能只比较根 manifest 路径；正式发布边界本身也须核验新模型，不能仅检查边界之后的普通提交。
3. 先复用已有发布路径，再按风险选择定向测试。预算上限应有范围依据，不能把保守估算当必需工时；耗时测试筛选过宽时及时收窄，保留中断和失败事实。本轮约 30 分钟完成，不以重跑全量或发布准备作为开发完成前置。

已复用本会话读过的 Native Memory 中历史 Cargo 归属约束，并与现行源码和本次证据核对；没有新增 Rule/Hook/AGENTS 建议，现有分层收口与证据规则已覆盖这些风险。未修改 Memory。当前 topic 同时承载发布接入合同和其他关联事项，本轮不提出整目录归档候选，不移动文档。
