---
lifecycle: completed
validation_status: verified
---

# 通用发布核心与 Rust 独立包版本

## 授权、目标与预算

用户先确认“通用发布核心＋语言适配器＋项目配置”，要求先支持 Rust、保留其他项目现有行为；在完整计划后明确“开始推进吧”。本轮授权工厂源码、共享 Skill、dogfood、分发合同、文档、隔离夹具和工程审计。不提交、推送、发布、安装用户级资产或修改真实下游。StratusAgent 接入和实际重编收益单独验证，只能在另行授权的测试 worktree 写入。

任务规模 L：新增策略、配置合同、历史兼容和依赖传播。执行估算 120 分钟、40k 新增 token（估算，未实测）、最多两个子 Agent（implementation-worker 与 review-auditor）、两轮完整验证。原对话的单项目 M 级估算不作为本次扩展后范围的预算。没有可靠周额度换算，不声称周占比。预计超额时报告并停止超额动作。

关联 TODO：TODO-010。本卡是需求、设计和验证记录的唯一主文档。

## 用户可见合同

| 编号 | 行为 | 约定 |
|---|---|---|
| 1 | 普通 git-sync | 不自动升版，保留现有同步、Hook 和失败保护 |
| 2 | 未启用新策略 | schema 1／无配置项目保留现有行为，不自动迁移 |
| 3 | 产品版本 | 根 VERSION，沿用累计 Conventional Commits 最高级别 |
| 4 | Rust 包版本 | 各包 Cargo.toml；自身改动按相关提交最高级别，反向依赖跟升 patch，每包一次 |
| 5 | 首次拆分 | 继承版本物化为原有效版本，不因机械迁移全包升版，也不跳过未发布提交 |
| 6 | 兼容性 | 版本写入与构建、测试、DLL 配套验证分别报告 |
| 7 | 预览 | 零写入、无构建、无联网和锁文件更新 |
| 8 | 事务 | 使用现有受管 Git、index、tree guard、快进、回滚和推送边界 |

依赖跟升 patch 是本计划采用的首期默认值，显式写入项目配置；不把版本号作为接口兼容已验证的证据。纯产品／文档改动可通过显式 product_inputs 只升级产品。

## 架构和配置

通用核心负责发布边界、逐提交事件、产品版本、计划合并和事务；Rust 适配器只读取输入并返回包清单、依赖图和精确文件计划，不运行 Git 或测试。静态 Rust 模块实现，不引入动态插件系统。源码落在 templates/hooks，逐字同步 .codex/hooks。工厂自身继续使用 schema 1，验证旧策略兼容。

显式启用示例（新 schema，旧二进制必须拒绝）：

```json
{
  "schema_version": 2,
  "release_policy": "explicit_release",
  "strategy": "rust_independent",
  "rust": {
    "workspaces": ["Cargo.toml"],
    "dependency_bump": "patch",
    "shared_inputs": [
      {"path": "build-common/", "packages": ["crates/core/Cargo.toml", "crates/app/Cargo.toml"]}
    ],
    "product_inputs": ["README.md", "doc/"]
  }
}
```

配置不保存第二份版本。workspace 和包身份是明确 manifest 路径；共享路径用文件或以斜线结尾的目录，不作为受管资产 glob ownership。映射重叠、未知字段、重复字段、越界、链接和未知策略阻断。

支持多个 workspace、本地路径依赖、重命名、workspace 依赖继承、普通／构建／开发／可选／目标条件依赖；传播采用声明依赖集合，不等同于某次构建启用集合。锁文件按本地包身份更新，第三方包不跟着批量改版。Cargo.lock 自身改动需显式归属；复杂且不能可靠解析的配置（如 patch/replace）阻断，禁止猜测。

首次迁移保留版本值；新增包的受管首次纳入规则见[新增包修复](requirements_2026-10-19_package-additions.md)。删除／改名及无法识别的身份变化仍阻断，当前无通用身份迁移命令；不自动 cargo publish。预览保留新旧版本和 first_release／direct_change／dependency_change 原因。实际执行 fetch／快进后重新计算，并核对输入内容、包／文件清单和 HEAD。

## 实施顺序与验收

| 编号 | 优先级 | 任务与验证 |
|---|---|---|
| 1 | P0 | schema 2 配置及旧分支兼容；旧工具能力不足阻断 |
| 2 | P0 | Rust 发现、归属和传播；私有包、公共包、传递／循环依赖、无关包 |
| 3 | P0 | 精确 manifest、约束和锁文件更新；Cargo locked/offline 接受生成结果 |
| 4 | P0 | 按历史配置验证边界，机械迁移不全升、不遗漏未发布改动 |
| 5 | P1 | 输入及 inventory 漂移阻断，事务、Hook、提交及推送失败保护复用 |
| 6 | P1 | CLI 能力、预览和最终收据，Skill 与 dogfood 同步 |
| 7 | P1 | 定向测试、完整 workspace／factory fixture、独立 review-auditor |
| 8 | P2 | 正式受管发布／用户级安装／真实下游／runtime 另行授权及验收 |

开发检查与 git-sync 使用资格分离；新增策略不要求业务测试、开发 JSON 或审计收据作为每次 release 的前置。工程收口遵循 development-completion.md，在当前工作区不提前修改 VERSION。

## 传播四问

通用产品能力，进入 templates/ 和 skills/；同步自身 dogfood 与分发哈希。正式 release 时由受管事务升级产品版本并写 [product] CHANGELOG，当前不预写。下游名称、Hook、DLL 和 UI 规则不进入公共模板。

## 实施和验证记录

实施、审计修正和工程验证已完成，等待用户验收。首轮模板定向集 `cargo test --locked --config scripts/tests/factory-cargo.toml --manifest-path templates/hooks/Cargo.toml -p bridgeforge-core release -- --test-threads=1` 在隔离 Git 权限重试后 53 passed、0 failed；该结果先于后续 Cargo 边界修正，最终以 dogfood 和完整工厂检查为准。首次 sandbox 临时 Git 初始化失败是 `.git/config: Permission denied`，未进入功能断言；相同命令获得窄范围权限后通过，不改 ACL 或改用手工同步。

独立 review-auditor 指出的混合骨架／业务提交、外部 path 包 dev 依赖不传递、误将 workspace 成员登记为根、无计划时策略收据及未启用 optional 锁闭包均已修正并补测试。稳定差异最终源码审计为 passed、findings=[]；7 个变更模板与 dogfood 源码 SHA-256 一致。审计不包含测试执行、完整工厂检查、真实下游或用户安装。

dogfood 定向命令 `cargo test --locked --config scripts/tests/factory-cargo.toml --manifest-path .codex/hooks/Cargo.toml -p bridgeforge-core release:: -- --test-threads=1` 最终 49 passed、0 failed。其中 23 项 Rust 专项通过，真实 Cargo 离线夹具覆盖 dev 不传递、optional 关闭、关闭 default、显式特性开启、默认开启五种场景。

### 最终工程收据

受管 `.codex/bin/bridgeforge.exe git-sync --prepare-release --message-file .runtime/rust-independent-message.txt --audit-file .runtime/rust-independent-audit.json` 最终返回 `status=prepared`、`checks_passed=7`、`target_version=2.4.0`；输入指纹 `sha256:df18ad4c12610ace24ea0df2d47b9aa233a51b6b736b31e96440ef01cb8f4a26`。真实独立审计记录保存在本机忽略目录，不进入 Git。准备过程中不修改根 VERSION、暂存区或提交历史。

| 编号 | 检查 | 最终结果 |
|---|---|---|
| 1 | baseline | 通过，包含源码与受管 CLI/Hook 构建收据核验 |
| 2 | skill metadata | 通过；首轮因入口 124 行超 120 行阻断，合并新增段落且保持正文后通过，未放宽硬闸 |
| 3 | manifest --check | 通过 |
| 4 | project structure | 通过，包括文档、索引和 TODO 合同 |
| 5 | mirror drift | 1 passed，源码全树逐字一致 |
| 6 | 完整 workspace | CLI 15、core 259、Hook 24，共 298 passed、0 failed、8 ignored |
| 7 | 完整 factory fixture | 90 passed、0 failed、6 ignored；包含真实 CLI 新旧策略同步/发布、重复 release 及工程失败保护 |

workspace 日志：`sha256:ad4945655ac4359ae536d58d95652215c8ad20a8865ba530c4de4d7ce05f2739`；factory 日志：`sha256:f0553cc51dee9170e0b0bc5a146f6f4d6db27cb0031d452dd183275be3b930ce`。ignored 是既有子进程辅助入口、显式性能/live log/高成本环境及另需授权的迁移夹具，不视为通过。

当前版本 2.3.1 的 CLI 和 Hook 已通过受管 build-assets 刷新并自检。隔离目标 2.4.0 的 CLI/Hook 及收据已经构建并按内容哈希保存；本机准备记录位于 `.runtime/bridgeforge-codex/release-preparation/current.json`。准备完成时根 VERSION 仍为 2.3.1，HEAD 仍为 `916a4074df65cf9877883eaaf7c627efa490811d`。未提交、推送或发布，未安装用户级 Skill、未接入真实下游。TODO-010 保持进行中，由用户验收流程结算。

执行使用 implementation-worker 和 review-auditor 共两个子 Agent；完整准备首轮在 metadata 阶段阻断，修正后第二轮七项全部通过。token 未实测，不声称精确消耗或周额度比例。

### 支持边界与经验

1. 版本传播图与锁文件必需依赖闭包职责不同。版本按所有声明的本地依赖保守传播；外部 path 包的 dev 依赖不传递，optional 依赖按本地默认／显式特性激活状态验证。不做 registry 特性解析或实际目标平台构建判断。
2. workspace 配置必须指向真正的 Cargo 根。祖先 manifest 和缺失状态纳入输入守卫，防止误用成员目录的锁文件边界。
3. 同一提交混合业务和骨架升级时，逐路径剔除纯受管骨架；保留项目专区的实际业务变化，不能要求项目给每个骨架文件增加业务归属。
4. 首期仅稳定三段版本；Cargo.lock 支持格式 3/4。缺失锁保持缺失，不生成或重新解析第三方依赖。复杂成员 glob（**、[]、{}）、patch/replace、未登记的本地包、无法安全拓宽的复杂内部约束明确阻断。
5. 新策略除了校验已有脏文件内容，也核对完整文件清单和工作区状态，防止计划后新增其他脏路径被提交而未计算包版本。预览的 status 使用 --no-optional-locks，避免刷新 Git index。

以上是本次实现经验，未修改 AGENTS、Rule 或原生 Memory。真实下游、自然 runtime、用户级安装、正式发布及实际重编收益均未验证。

## TODO-010 当前验收

2026-10-19，用户通过 `$summary accept` 明确验收扩展后的 Rust / Python 通用发布交付；本卡首期需求由该交付及当前完整回归覆盖。TODO-010 的正式验收、最新工程证据和发布授权统一见[当前验收记录](requirements_2026-10-19_polyglot-lifecycle.md#用户验收与发布授权2026-10-19)。以上保留首期历史过程，不将其测试结果作为当前验证；真实下游、用户级安装与业务运行仍未验证。
