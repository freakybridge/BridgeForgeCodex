---
lifecycle: active
validation_status: awaiting_user_acceptance
---

# 修复普通 git-sync 自动升版

## 需求、授权与范围

- 来源：当前对话 `01a11c32-34c4-7b83-806c-1e868f723081`；用户核对 StratusAgent 对话 `01a11c22-dc82-7080-b99c-d78c2c030e3d` 后确认“那你开始修吧”。
- 目标：普通 `git-sync` 始终只提交和同步；只有显式 `release` 才可自动生成版本、原生 manifest/lock、CHANGELOG 更新。已有累计版本规则、事务回滚和准备式发布继续使用。
- 授权：修复当前 BridgeForge 上游、源码与 dogfood、共享 Skill、验证与发布准备。不提交、不推送，不安装用户级资产，不写真实下游项目或产品 home。
- 规模 M：单一已知缺陷，但工厂要求完整 fixture、workspace、独立审计与发布准备。预算 45 分钟、20k 新增 token（估算，未实测）、最多 1 个 review-auditor、最多 2 轮验证。当前模型无可核实的周额度系数，不编造百分比。预计超额时停止超额动作并报告。

## 已核实原因与决定

StratusAgent 原同步收据为 `release_requested=false`、`release_policy=per_commit`、`version_bumped=true`，业务版本 `1.47.12 → 1.47.13`。原代码与测试明确支持缺省策略自动升版；骨架升级更新工具但项目配置缺少策略，普通同步仍进入旧分支。

1. CLI 从结构上取消普通同步生成发布计划的分支，而非只修改默认配置。
2. 缺配置文件、缺字段、旧 `per_commit`、现有 `explicit_release` 均只允许显式发布；旧配置字节保持原样，无需项目配置迁移。非法值、schema 和重复字段继续报错。
3. 自检新增 `git-sync-release-only-v1`，共享 Skill 在任何同步前核对该能力；旧工具先升级，避免新版 Skill 仍调用会自动升版的旧工具。
4. 上游发布后，旧项目通过既有受管骨架升级获得新工具即可采用新行为。真实下游升级不包含在本次授权中。

## 传播四问

产品层通用修复；源码进入 `templates/`，规则进入共享 `skills/git-sync`；需要后续产品发布与 `[product]` CHANGELOG，由显式发布事务生成，开发不提前修改 VERSION；需要同步自身 dogfood 和分发 manifest。工厂没有 `.codex/skills/git-sync` 镜像，不创建第二份入口。

## 验收与风险

| 场景 | 必须取得的结果 |
|---|---|
| 缺文件、缺字段、per_commit、explicit_release 的普通同步 | 同步成功、clean、0/0；VERSION、manifest、lock、CHANGELOG、策略配置均逐字不变 |
| 上述配置的显式发布 | 发布准备、预览与执行正常，累计规则升一次；不重复升版 |
| 非法配置 | 保留原有拒绝语义 |
| 工具与 Skill 混用 | self-test 能力标记明确阻止旧 CLI 执行普通同步 |
| 完整工厂检查 | dogfood、metadata、manifest、structure、镜像、workspace、fixture 与独立审计通过 |

旧配置 `per_commit` 的行为变化是本次用户明确授权的修复，不继续兼容自动升版。版本写入仍需显式发布授权；未知配置不会被静默纠正。发布准备不等同于已经发布，也不等同于真实下游验证。

## 实施与验证记录

已完成源码、共享 Skill、能力标记、测试和自身镜像修改；通过受管 `manifest` 与 `build-assets` 重建合同和当前版本工具。开发没有修改 VERSION，仍为 `2.0.0`。

| 编号 | 验证命令或收据 | 当前证据 |
|---|---|---|
| 1 | `cargo build --locked --release --manifest-path .codex/hooks/Cargo.toml`；受管 `build-assets --project-root D:/Quant/BridgeForge` | 构建和安装通过；CLI `self-test` 含三项能力，包括 `git-sync-release-only-v1` |
| 2 | `cargo test --locked --manifest-path scripts/tests/Cargo.toml rust_source_is_identical_in_template_and_dogfood -- --test-threads=1` | 1 passed，模板与 dogfood 全树一致 |
| 3 | `cargo test --locked --config scripts/tests/factory-cargo.toml --manifest-path .codex/hooks/Cargo.toml -p bridgeforge-core git_sync::tests -- --test-threads=1` | 初次 36 passed / 1 failed；唯一失败是旧 `release-drift` 用例预期普通同步读取 CHANGELOG 发布计划。新行为不读该文件，已改为同样保护实际快照策略配置的 `policy-drift`，后续完整 workspace 已复测通过 |
| 4 | `cargo test --locked --manifest-path scripts/tests/Cargo.toml real_cli_explicit_release_preview_and_execution -- --test-threads=1` | 1 passed，内部跑四种配置：普通同步版本与四文件及配置逐字不变、clean/0/0；预览零写入；显式累计 minor 发布；重复发布无新提交 |
| 5 | `check baseline`、`self-test --json`、`check factory-version`、`check skill-metadata`、`check project-structure`、`manifest --check`、`git diff --check` | 均通过；仅保留既有归档建议。baseline 初次因沙箱不能创建临时 Git 属性仓库失败，限定权限重试通过 |
| 6 | `/root/release_only_audit`，review-auditor | 独立读取需求、实际 diff、planner、回滚、测试和 hash；未发现需修复的代码缺陷，允许进入验证收口；审计未代替测试 |
| 7 | `git-sync --prepare-release --message-file .runtime/release-only-message.txt --audit-file .runtime/release-only-audit.json` | prepared；7 项检查通过；目标 2.0.1 隔离产物已构建，不改当前 VERSION |

当前 CLI 二进制 SHA-256 为 `a822b3e146dbc6baa4afc43d95fae98eac7583b297f65dcf7742b878a861ccd5`；受管源码树 SHA-256 为 `989dea25939e7950f4f43a55afa184bb63b4d0a9415341f06ddfb7eccd1d663d`。

| 六类证据 | 状态 |
|---|---|
| 源码 | 已实施；普通同步发布计划为空，独立审计通过 |
| 产品传播 | 模板、Skill、分发 hash 已更新；未发布、未安装用户级产品 |
| dogfood | 镜像、受管构建、自检和 baseline 通过 |
| fixture | 四配置真实 CLI 夹具及完整工厂验收通过，89 passed / 4 ignored / 0 failed |
| 真实下游 | 未授权、未写入、未验证 |
| runtime | 临时 CLI 普通同步、预览、显式发布、重复发布通过；真实项目运行未验证 |

独立审计及准备记录位于本机忽略目录，不进入 Git；不以准备代表已发布或用户验收。

完整收口于 2026-10-09 完成：workspace 为 CLI 14、core 183、Hook 22，共 219 passed / 5 ignored / 0 failed；工厂 fixture 为 89 passed / 4 ignored / 0 failed。完整验收共 308 passed，不重复计算定向集和镜像检查。此前唯一失败的旧测试已在完整 workspace 中通过。

忽略项如实保留：workspace 的四个子进程辅助入口和需现场日志的只读高成本检查；工厂 fixture 的显式性能实验、需原生环境变量的高成本检查、子进程 helper 与需单独授权的 Assist 源迁移。没有额外启动这些环境或真实项目测试。

发布准备记录：`.runtime/bridgeforge-codex/release-preparation/current.json`；验证指纹 `sha256:ac59cf8f1172cb141a2eb04370ce17d303edfa494eed7b34f4042d1fd3d0be8a`；`status=prepared`，`checks_passed=7`，`target_version=2.0.1`。检查日志按内容哈希保存在同目录 `blobs/`，实际检查和目标产物均可核对。根 VERSION 与已提交 HEAD 未改变；未提交、未推送。

当前为上游本地待用户验收，未结算为 completed。用户级 Skill、产品 home 与真实下游仍未更新；发布及下游升级需要相应后续授权。使用 1 个独立审计 Agent，2 轮验证；token 未实测。


## 经验沉淀

把要求实现成可选模式会使缺配置的旧项目继续触发旧行为。要保证“只有显式 release 才升版”，必须在执行分支消除普通同步的版本写入，并使用真实 CLI 夹具覆盖缺配置和旧配置；只验证新配置不能证明旧项目安全。
