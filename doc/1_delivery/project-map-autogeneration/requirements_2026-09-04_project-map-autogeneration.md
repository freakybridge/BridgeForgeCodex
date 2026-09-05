---
lifecycle: active
validation_status: awaiting_user_acceptance
record_type: requirements
confirmed_at: 2026-09-04
source: $confirm
---

# 项目 Map 自动生成与静默维护需求

> 2026-09-05 修订：下述使用前刷新规则不适用于明确只读、审计或预览；这些场景直接读取线索并核对源文件或搜索 fallback。生命周期自动维护保持不变，详见[板块二](../gpt6-skeleton-upgrade/requirements_2026-09-05_block2.md)。原始决定保留供追溯。

## 原始需求摘要

用户最初要求把 `.codex/find-doc.map.md` 与 `.codex/sync-docs.map.md` 从用户手工维护的数据表升级为骨架自动维护的机器索引。2026-09-04 用户进一步确认：两份 Map 是会实时变化的骨架内生数据，不应进入 Git，正式位置改为 `.runtime/bridgeforge-codex/`；旧 `.codex` 路径在升级事务中精确退役。正常运行必须对用户无感；文件缺失、输入变化或内容漂移时由程序创建或重建。

调用来源：用户给出 StratusAgent 对话 `codex://threads/01a069fb-59db-7b12-b8fc-3f9255f28987`，由 `$bridgeforge-codex` 与 `$develop` 路由进入 `$confirm`。

后续交接目标：`$develop` 在用户开工确认后完成实现、验证、版本升级与产品记录。

## 目标

- 由骨架确定性生成并维护两个项目 Map，用户不再创建、编辑或修复它们。
- 在 `SessionStart`、相关编辑后的 `Stop` 以及 Skill 使用前保证索引为当前状态。
- 输入指纹未变化时不重写文件，避免无意义 Git diff。
- 无法从当前项目事实确定的映射不得编造，Skill 继续使用现有搜索 fallback。
- 生成结果只存在于项目 `.runtime/bridgeforge-codex/`，不进入 Git；旧 `.codex` Map 由项目同步事务精确删除。

## 不做

- 不修改、升级、提交或推送 StratusAgent 及其他下游项目。
- 不在本工厂自动执行 `git add`、commit 或 push。
- 不保留或迁移旧手写 Map 的任何条目；首次运行按当前事实完全重建。
- 不把业务语义写入公共 Template，也不因缺少确定映射而新建业务文档。
- 不把两个 Map 加入 Template 的固定内容覆盖面、Git 跟踪或 project-owned 保留清单；它们是可随时重建的本地运行时数据。

## 规模与预算

| 项目 | 预算 |
|---|---|
| 规模 | M |
| 判定依据 | 核心逻辑跨公共 Rust Hook、两个共享 Skill、Template/dogfood 镜像和发布测试，但目标、迁移规则与下游边界已锁定 |
| 时间 | 45 分钟 |
| Token | 估算不超过 20k；平台无可靠计量器，未实测 |
| Agent | 0 个；由主对话完成 discovery、实现与验证 |
| 验证轮次 | 最多 3 轮完整验证；用户于 2026-09-04 明确同意从 2 轮扩至 3 轮 |
| 超预算停止点 | 时间、估算 token、agent、验证轮次或范围任一预计越界时，停止并让用户选择扩大预算或缩小范围 |

## 已核实事实

| 事实 | 收据 |
|---|---|
| 工厂当前不存在两份 Map | 工厂精确路径只读检查 |
| 项目同步器只在 Map 已存在时把精确路径登记为 `required-preserve`，不会创建文件 | `templates/hooks/crates/bridgeforge-core/src/project_sync.rs` |
| `$find-doc` 当前读取 `topic_to_rules`，缺失或未命中时走搜索，并通过 SOP 提醒用户补 Map | `skills/find-doc/SKILL.md` 与 `references/map-reminder-sop.md` |
| `$sync-docs` 当前读取路径映射，缺失时寻找候选并提醒用户补 Map | `skills/sync-docs/SKILL.md` |
| 公共 Hook 已有 `PostToolUse`、`Stop`、`SessionStart` 路由；编辑 payload 能取得目标路径 | `templates/hooks.json` 与 `templates/hooks/src/lib.rs` |
| Windows Hook 二进制使用无窗口子系统 | `templates/hooks/src/main.rs` |
| StratusAgent 现有两份 Map 包含人工语义，且 `find-doc` Map 仍含已退役 `.codex/rules/*.md` 链接 | 两个下游 Map 的只读核对；未修改下游 |
| Template 与工厂 dogfood Rust 源码必须逐字一致 | `scripts/tests/src/lib.rs` |

## 已确认规则

1. 两份 Map 顶部必须声明“由 BridgeForge 自动生成，禁止手工维护”，并记录生成格式版本与输入指纹。
2. `SessionStart` 在 Map 缺失或输入指纹变化时静默重建。
3. `PostToolUse` 对相关 `AGENTS.md`、源码结构、Cargo manifest 与 `doc/**` 编辑只记录索引已脏，不向用户输出维护提醒。
4. `Stop` 对脏索引统一静默重建；无变化时不写盘。
5. `$find-doc` 与 `$sync-docs` 读取前调用受管 Rust 入口执行 `ensure-current`，再消费 Map。
6. 已有手写 Map 首次进入新机制时完全重建；旧内容不作为证据或输入。
7. 生成器只输出能由当前文件、目录或明确引用证明的关系；不确定项省略，由 Skill fallback 搜索。
8. 正常创建、no-op、判脏和重建均不显示给用户；只有生成失败并影响当前任务时才报告真正阻断。
9. 两份 Map 固定写入 `.runtime/bridgeforge-codex/` 且禁止进入 Git；同步器仅精确退役旧 `.codex/find-doc.map.md` 与 `.codex/sync-docs.map.md`，禁止用 glob 扩大删除边界。

## 拟修改组件

| 组件 | 修改方向 |
|---|---|
| `templates/hooks/src/` | 增加 Map 生成、指纹、判脏、原子写入与 `ensure-current` 路由 |
| `.codex/hooks/src/` | 同步工厂 dogfood 镜像 |
| `skills/find-doc/` | 使用前确保索引当前，删除用户维护提醒 |
| `skills/sync-docs/` | 使用前确保索引当前，删除用户维护提醒 |
| 项目同步与架构文档 | 将 Map 定义为 `.runtime` 中的骨架内生数据，并在两条升级路径中精确退役旧 `.codex` 路径 |
| `scripts/tests/**` | 覆盖生成、触发、幂等、完全重建、fallback、镜像和分发行为 |
| `VERSION`、`CHANGELOG.md`、manifest 与生成资产 | 第一轮完整测试通过后升级产品版本、记录 `[product]`，按官方流程刷新并终验 |

最终精确文件清单以实施时当前代码为准；机械镜像、manifest、二进制和收据不单独抬高规模。

## 验收标准

| 验收面 | 通过标准 |
|---|---|
| 缺失文件 | `ensure-current` 或生命周期事件能在 `.runtime/bridgeforge-codex/` 创建两份合法 Map |
| Git 边界 | 新 Map 位于既有 Git 忽略区；旧 `.codex` Map 在兼容更新与重建中均由事务删除 |
| 输入变化 | 相关事实变化后 Map 确定性重建 |
| 无关编辑 | 不判脏、不改写 Map |
| 幂等 | 相同输入连续执行时文件字节和修改时间保持不变 |
| 旧 Map | 任意手写内容被完全重建，不泄漏到新格式 |
| 防编造 | 无明确关系时不生成猜测条目，Skill fallback 仍可工作 |
| 用户体验 | 正常 Hook 与 Skill 路径不输出 Map 维护提醒 |
| 工厂一致性 | Template 与 dogfood 源码逐字一致，受管 manifest 无漂移 |
| 发布前验证 | 第一轮完整 Rust、结构、mirror、fixture 与自动测试通过后才修改版本 |
| 发布终验 | 版本、`[product]` CHANGELOG 和正式生成资产刷新后，再通过完整发布级测试 |

## 合理假设与风险

- 完全重建会有意丢弃旧手写关系；这是用户明确选择，不属于回归。
- 只凭目录名不能证明业务文档关系；生成结果允许比旧手写表更稀疏，fallback 是正常路径。
- Hook 写入只发生在项目既有 Git 忽略区，不形成工作树 diff，也不主动暂存、提交或推送。
- 本轮不做真实下游升级或 runtime smoke；相关证据必须在用户另一个对话完成前标为未验证。

## 自动化边界

- 可自动：读取项目事实、计算指纹、标脏、确定性生成、原子写入、幂等检查、Skill 前置刷新和测试 fixture。
- 必须停止：生成器无法读取必需输入、输出路径不是安全普通文件或原子写入失败并影响当前任务。
- 禁止自动：继承旧手写条目、猜测业务关系、创建业务文档、扩大 Map ownership、修改或发布下游项目。

## 实施记录

- 2026-09-04：用户确认范围仅限工厂，选择旧 Map 完全重建，并确认本需求卡。
- 2026-09-04：用户授权 `$develop` 开工；锁定 M 级精简路径，由主对话实施，不使用子 agent。
- 2026-09-04：用户修订存储边界；两份 Map 改入 `.runtime/bridgeforge-codex/`，旧 `.codex` 路径不再跟踪并由升级事务精确退役。

### 实施计划

1. 在公共 Rust Hook 中实现安全扫描、输入指纹、确定性渲染、脏标记、原子写入与 `ensure-current` 命令。
2. 把生成器接入 `PostToolUse`、`SessionStart` 与 `Stop`，成功路径保持静默。
3. 更新 `$find-doc`、`$sync-docs` 的前置刷新和 fallback 合同，删除用户维护提醒。
4. 同步 Template/dogfood 源码与架构事实，补齐专项、镜像和分发回归。
5. 执行版本前完整工厂验证；全部通过后才升级版本、记录 `[product]`、刷新正式生成资产并进行最终发布终验。

## 验证记录

- 首轮完整工厂回归暴露既有并行 fixture 临时目录名冲突：79 项通过、2 项忽略、2 项失败；使用进程内原子序号消除同时间戳碰撞后，相关 `build_provenance` 11 项专项通过。该轮未用于版本升级依据。
- 版本前稳定验收通过且输入无漂移：CLI 9 项通过；`bridgeforge-core` 100 项通过、2 项忽略；`bridgeforge-hook` 15 项通过、2 项忽略；工厂回归 81 项通过、2 项忽略；manifest、factory-version 与 `git diff --check` 通过。完成该轮后才把版本从 `1.12.6` 升至 `1.13.0`。
- `cargo build --locked --release --manifest-path .codex/hooks/Cargo.toml` 成功，三个 crate 均为 `1.13.0`。首次 `build-assets` 因 manifest 源码树哈希陈旧被硬闸零安装拦截；使用同一 release CLI 正式刷新 manifest 后重试成功。
- 正式 `build-assets` 生成 schema 2 收据：Hook `sha256:f8b835d2f32f884732ef557cb8fd743acee425fd840c6c9e4f3c3cabbe41268f`，CLI `sha256:9b90617a37a09f1dfe61f21a024d51d1b67d0a82fcef8410e884fec2e176ea60`；共同源码树为 `sha256:d220929ffdda4be2b2edfb78f4692fb0c1d0262a12d5e6c892a10176f820601e`。
- 真实 dogfood Hook 使用 `Start-Process -Wait -WindowStyle Hidden` 串行执行两次 `project-map ensure-current`：两次退出码均为 0，两张 Map 的 SHA-256 与修改时间均不变；内容检查未把 Template/交付/Bug 的非生效 AGENTS 或目录同名猜测混入索引。
- `1.13.0` 最终发布级验收通过且输入无漂移：CLI 9 项通过；`bridgeforge-core` 100 项通过、2 项忽略；`bridgeforge-hook` 15 项通过、2 项忽略；工厂回归 81 项通过、2 项忽略；`MAP_RUNTIME_OK=True`；manifest `changed=false`；factory-version 返回 `version=contract_version=1.13.0`、`healthy=true`；`git diff --check` 通过。
- 本轮未修改、升级、提交或推送任何下游项目；真实下游与下游 runtime smoke 仍未验证，由用户在另一个对话完成。
