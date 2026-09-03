---
lifecycle: active
validation_status: awaiting_user_acceptance
---

# 项目 Hook 登记兼容性修复确认卡

## 本次确认

用户要求先修骨架，再修 Assist，并以“开始吧”确认实施。M 级；预算 45 分钟、约 20k 新增 token（估算，非计量），最多 1 名 review-auditor、2 轮验证。开始时间 2026-09-03 14:54:40 +08:00。

## 范围与验收

- 将项目自有 Rust Hook 元数据迁到项目所有的 `.codex/project-hooks.json`；Codex 原生 `.codex/hooks.json` 不再含 `bridgeforgeProjectHooks` 顶层字段。
- 修复读取、生成、升级事务、暂存区校验与迁移校验；兼容旧登记迁移，双份冲突必须停止，原命令和业务源码保持。
- 同步 Template、dogfood、文档、回归测试、产品版本与清单。
- 骨架验证后，在隔离测试目录验证 Assist 的真实登记与源码；不执行真实 vault 操作，不更改业务源码或资料。
- 不修改用户级 hooks，不自动信任 Hook，不提交、推送或发布。Assist 正式更新必须遵守官方更新入口和发布条件；未满足时明确留待后续。

## 传播四问

产品层通用修复；不是工厂专用；需要 patch 版本及 `[product]` CHANGELOG；需要同步自身 dogfood，但禁止对工厂执行下游 apply 或写骨架版本戳。

## 验证记录

2026-09-03 完成两轮验证；执行约 28 分钟，token 未独立计量。第一轮完整升级测试在编译期间遇到主线程补丁，正确返回 `generated source inputs changed during build`，没有完成安装；冻结源码后的第二轮通过。没有为此放宽输入漂移检查。

| 证据类别 | 当次结果 |
|---|---|
| 源码 | 核心 93 项通过、2 项子进程辅助测试按设计 ignored；覆盖冲突、孤立登记、幂等、输入漂移、事务回滚和既有命令保留 |
| 产品传播 | 本地版本 1.12.1；Template 文档、读取/迁移/校验链、锁文件产品版本及两份受管清单已同步；manifest 检查 `changed:false`；尚未提交或发布 |
| dogfood | 模板与自身 Rust 源码完全一致；官方 `build-assets` 重建两项产物；含运行产物的 baseline 为 `clean` |
| fixture | 分发回归 20 项通过；完整 init/build/apply fixture 通过，包括暂存区独立登记读取、工作区损坏不影响已暂存内容、未暂存登记拒绝通过和重复升级 `current` |
| 真实下游 | 只读复制 Assist 的原生登记和入口源码到一次性隔离目录；迁移、锁定编译、基线和幂等均通过；真实 Assist 目录未修改，正式更新尚未执行 |
| runtime | 本地 CLI 自检版本 1.12.1；demo 程序 stdin/stdout 路径通过；Assist 只运行保留的包装器自检，不调用业务 run；真实 vault junction/snapshot 与 Codex UI 加载未验证 |

Assist 入口源码 SHA-256：`5908b2faea976959a25f6f35b904ba38d5b379be6996b7c8d98a7f5d883873b4`。隔离测试逐项断言原生命令、事件、参数不变，新登记内容等于原登记，真实配置和源码读前读后逐字一致；未复制 vault、vault-mirror 或 vault_node_map。

## 当次验证命令

均在工厂根目录运行。下列测试只在自身创建的临时目录产生测试数据，不提交或推送真实仓库。

```powershell
cargo build --locked --release --manifest-path templates/hooks/Cargo.toml
cargo test --locked --config scripts/tests/factory-cargo.toml --manifest-path templates/hooks/Cargo.toml -p bridgeforge-core -- --test-threads=1
cargo test --locked --manifest-path scripts/tests/Cargo.toml distribution_regressions:: -- --test-threads=1
cargo test --locked --manifest-path scripts/tests/Cargo.toml tests::rust_source_is_identical_in_template_and_dogfood -- --exact
cargo test --locked --manifest-path scripts/tests/Cargo.toml tests::managed_manifests_are_current_and_python_free -- --exact
cargo test --locked --manifest-path scripts/tests/Cargo.toml project_sync_real_init_builds_and_applies_generated_assets -- --test-threads=1 --nocapture
$env:BRIDGEFORGE_ASSIST_FIXTURE_SOURCE = 'D:/Quant/BridgePersonalAssist'
cargo test --locked --manifest-path scripts/tests/Cargo.toml assist_registry_migration_in_isolated_fixture -- --ignored --test-threads=1 --nocapture
templates/hooks/target/release/bridgeforge.exe manifest --root D:/Quant/BridgeForgeCodex
templates/hooks/target/release/bridgeforge.exe build-assets --project-root D:/Quant/BridgeForgeCodex
.codex/bin/bridgeforge.exe self-test --json
.codex/bin/bridgeforge.exe check baseline --root D:/Quant/BridgeForgeCodex
.codex/bin/bridgeforge.exe check factory-version --root D:/Quant/BridgeForgeCodex
.codex/bin/bridgeforge.exe check project-structure --root D:/Quant/BridgeForgeCodex
.codex/bin/bridgeforge.exe check skill-metadata --root D:/Quant/BridgeForgeCodex
.codex/bin/bridgeforge.exe manifest --root D:/Quant/BridgeForgeCodex --check
git -c safe.directory=D:/Quant/BridgeForgeCodex diff --check
```

## 独立审计与待办

本轮唯一 `review-auditor` 审计发现并复核修复了两个边界：迁移记录的目标哈希必须对应最终原生配置；重建库存必须将独立登记列为 required-preserve。两项均有当次通过的回归。审计结束时无未解决 P1/P2；独立审计没有代替主线程重复运行完整测试。

本轮未运行全部发布测试矩阵，不宣称发布完成。项目结构检查只有一条既有归档候选提示，没有错误；不顺带归档。Git 只读检查存在用户全局 ignore 文件权限警告，不修改其权限。

下一步：用户自行提交/发布骨架后，按 `$bridgeforge-codex` 正式入口更新真实 Assist，再在 Codex 界面复验加载告警。正式更新未完成前，原截图问题仍可能出现。三个用户级待审核 Hook 不在本次修复范围，未改变信任状态。用户验收前保持 active，不关闭 Bug。

## 后续正式执行授权与发布前检查

用户随后明确要求按顺序修复真实 Assist、Git-sync Forge、Assist 安装最新版骨架。本节补充该授权，不追改上一轮的隔离验证事实。

真实 Assist 已迁移项目登记，并同步既有架构说明。与其 HEAD 比较：原生命令和登记内容一致、原生顶层仅保留合法字段；vault 业务源码 SHA-256 不变。新版只读检查器核对其现有 1.12.0 资产与产物返回 clean；旧版项目运行器仍须由后续正式升级替换。本次没有执行真实 vault 业务命令，没有提交 Assist。

发布前额外执行：

- `cargo test --locked --config scripts/tests/factory-cargo.toml --manifest-path templates/hooks/Cargo.toml --workspace`：CLI 8、Core 93、Hook 11 项通过，4 项子进程辅助用例按设计 ignored。
- `cargo test --locked --manifest-path scripts/tests/Cargo.toml -- --test-threads=1`：完整集成测试 78 项通过、2 项按设计 ignored，包含真实隔离发布与项目初始化/升级。
- 首次默认并发集成测试为 76 通过、2 失败、2 ignored：`build_assets_holds_project_lock_during_build_and_releases_after_error` 的模拟调用次数为 0，`legacy_receipt_delete_failure_rolls_back_installed_assets` 发现临时文件缺失。两项单独复跑、所属 11 项测试组并发复跑、完整串行复跑均通过；未修改断言或跳过用例。首次并发失败根因未确认，不能宣称已修复其不稳定性。

Forge 推送范围为骨架代码、测试和说明，远端已实查为公开仓库 `freakybridge/BridgeForgeCodex`；不含 Assist 活库、镜像或私人资料。最终同步版本及 Assist 安装结果以此次正式工具收据为准。
