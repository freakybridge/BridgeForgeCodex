# git-sync 历史锁迁移换机交接

保存日期：2026-10-17，Asia/Shanghai。

接续更新（2026-10-17）：用户已另行授权 120 分钟／30k 新增 token 的历史锁迁移修复、必要验证与独立审计，不提交推送、不发布、不修改真实下游。当前实施与最终证据统一补回[原需求卡“本轮历史锁迁移修复”](requirements_2026-10-16_first-use-and-rust-members.md#本轮历史锁迁移修复2026-10-17)。下文保留交接时快照，不能用它覆盖后续授权或把旧工程结果当成本轮验证结果。

## 恢复入口与唯一事实源

先读[原需求与本轮复核](requirements_2026-10-16_first-use-and-rust-members.md)，尤其“发布后真实下游复核”。需求、现场数据、根因和测试遗漏以该卡为主要事实源；本文只维护接续入口，不另建需求卡。

目标：解决首次从旧统一升版切换到 Rust 独立包发布时，新增 workspace 的旧锁滞后导致历史回放失败的问题。2.5.0 的首次配置引导和 Cargo 隐式成员修复已发布，不能把它们重新当成未实施功能；但真实下游的独立发布仍未打通。

## 仓库、版本和授权

| 编号 | 项目 | 交接时事实 |
|---|---|---|
| 1 | 仓库 | BridgeForgeCodex；origin 为 `https://github.com/freakybridge/BridgeForgeCodex.git` |
| 2 | 分支／upstream | `main`／`origin/main` |
| 3 | 基准 HEAD | `0e649b1486a88f3beb75af10fecff25019cea2a3`，是本次文档同步前的基准，不是随后普通 git-sync 的最终提交 |
| 4 | 当前产品版本 | `2.5.0`；本次普通 git-sync 不升版 |
| 5 | 初始工作区 | 干净，ahead/behind=0/0；有 1 条 `codex_git_sync_autostash`，禁止擅自清理 |
| 6 | 本次待保存 | 仅原需求卡、本文和 `doc/README.md`，无代码修复 |
| 7 | 有效授权 | summary switch 保存文档；普通 git-sync 提交推送本轮文档；既有 2.5.0 发布授权已完成 |
| 8 | 剩余修复 | 已只读调查，尚未获新的实施授权；换机不能把本文当开工、发布、用户级安装或真实下游写入授权 |

不是 autopilot，没有为此创建 Goal。原先实施包曾估算 120 分钟／30k 新增 token，实际 token 未实测且经历中断；不能把旧估算当可用余额，换机不重置累计。剩余修复时间估计 90–120 分钟，尚未形成新的完整预算。`review-auditor /root/historical_lock_audit` 已完成只读复核，不假设其进程或会话可在另一台机器恢复。

## 已有证据与阻塞

1. 下游“确认 Cargo.toml 版本升级的编译影响”对话（`01a14421-0203-7633-9d00-2a5aa9e52c49`）确认已用 2.5.0；主对话现场只读 release-status 仍 blocked，错误指向 `strategies/template/blackswan/Cargo.lock` 的 stratus-clock。
2. 当前锁已是 1.51.0；历史 schema 1 只管理两个主 manifest，策略锁仍为 1.50.0，而当时有效包版本已为 1.51.0。新策略拿当前注册范围严格校验旧快照，导致迁移阻断。
3. 源码路径：`templates/hooks/crates/bridgeforge-core/src/release/independent.rs` 的 historical_workspace、validate_current、中间提交回放；`release/rust.rs` 的 load/validate_locks；`release_history.rs` 的历史自身合同校验。产品源修改时须同步 `.codex/hooks/` 镜像及受管 manifest。
4. 缺失 fixture：`scripts/tests/unit/core_release.rs` 的单 workspace、锁始终一致的迁移夹具，未覆盖额外 workspace 的旧锁滞后。setup 和 Rust 适配器相关测试位于同目录 `core_release_setup.rs`、`core_release_rust.rs`。
5. 2.5.0 原工程证据为 workspace 312 passed／8 ignored、factory 90 passed／6 ignored、7 项检查和目标构建通过；它们只证明原输入和已有覆盖。新迁移问题没有修复代码、针对性测试或新的工程通过收据。

## 按顺序接续

1. 同步本仓库后核对实际分支、HEAD、VERSION、工作区差异及配置；本文基准 HEAD 不能用来 reset。核对新环境是否仍能访问真实下游；路径或文件变化时不照搬旧现场结论。
2. 先承接用户的新实施指令；没有该授权时仅允许只读核验与讨论，不修改产品或真实下游。对已明确的首次选择不重新访谈：工厂选择保留统一规则；StratusAgent 已选独立包，当前尚待受管保存确认。
3. 获准实施后，先建立历史 boundary 和中间提交的最小回归：旧 schema 1 管理 A/B，C 的旧锁滞后，当前修好 C 后切换 schema 2。核对新规则生效边界，不以整体关闭锁检查绕过。
4. 使历史加载区分当时配置和校验范围；保持当前锁、schema 2 历史锁、包身份和有效版本保护，覆盖 boundary 与中间提交。模板与 dogfood 同步，保留预览零写入、无 Cargo/网络解析的合同。
5. 定向测试必须覆盖合法迁移通过以及当前锁滞后、schema 2 历史锁滞后、身份/版本漂移仍拒绝；再按项目开发收口清单组织必要构建、强制工程检查与独立审计，不复用旧输入的 passed 冒充新修复通过。
6. 在另行授权和可访问环境中，验证真实下游只读 release-status/release-preview；配置落盘、真实下游测试 worktree 更新及正式发布分别判断授权。没有真实下游证据时明确未验证，不用更多离线测试替代。
7. 将实际修复结果补回原卡。只有用户验收才能结算相关 TODO；summary switch 不代替 accept，后续修复的提交推送也不能沿用本次文档同步授权。

## 不随 Git 携带的内容

- `.runtime/` 中的工程准备收据、完整日志、审计 JSON、预览及缓存是本机忽略产物；另一台机器缺失时如实说明，文档不是构建产物或 prepared 收据。
- `.codex/bin/` 的当前 CLI/Hook、受管构建缓存和 Cargo 缓存不能假设随 clone 存在。按根 AGENTS 的项目快速命令及 `INSTALL.md` 恢复 Rust/Cargo 和受管工具；不得回退脚本或手工伪造收据，也不得对工厂执行下游 project-sync adopt/apply。
- StratusAgent 的未提交接入改动不在本仓库；需在它自己的仓库／机器核对，本交接没有复制或修改这些文件。
- 不包含凭证、秘密、完整日志、原生 Memory 快照或本机进程恢复状态。

## 可复制接续提示

> 读取 doc/1_delivery/git-sync-first-use/handoff_2026-10-17_historical-lock-migration.md，先核对当前分支、代码与文档差异，再在已有授权范围内从下一步继续；缺失证据如实标明。历史锁迁移修复目前仅调查完成，未获新的实施授权；不要把 2.5.0 已发布、旧工程检查通过或本次文档同步当成新修复已完成。
