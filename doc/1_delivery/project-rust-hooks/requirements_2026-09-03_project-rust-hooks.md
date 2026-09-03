---
lifecycle: active
validation_status: awaiting_user_acceptance
size: L
---

# 项目自有 Rust Hook 构建与 Assist 迁移

## 已确认需求

用户以“A”确认：补齐骨架对项目自有 Rust Hook 的构建、安装、注册和校验能力，再将 Assist 的活库连接检查与镜像快照两个 Hook 移植为 Rust；既有功能、配置、数据链、失败语义必须保持等价。旧 Python Hook 与已确认废弃资产只允许在最终可回滚迁移事务中删除。

通用构建能力属于产品层，进入 templates 并同步工厂 dogfood；Assist 业务逻辑仅留在 Assist 的授权测试 worktree。产品版本升至 1.11.0，CHANGELOG 标记 [product]，兼容性基线仍为 1.8.6。

## 范围与预算

90 分钟（2026-09-03 09:48:26 至 11:18:26，Asia/Shanghai）、40k token 估算、最多 1 个 review-auditor、最多 2 轮验证。用户已确认范围及开工，不重复访谈。

不提交或推送 Git，不发布用户级安装，不写真实活库或镜像正文，不越过尚未确认的下游资产。真实 Assist 的正式升级必须在新产品发布且剩余资产确认完成后，使用官方同步器执行。

## 实施与验收

1. 项目 Hook 使用显式注册、固定 project_* Rust 入口和受管锁定 workspace；在隔离快照中构建，不改项目受管 Cargo 文件，不在 Hook 触发时编译。
2. 二进制、注册、构建收据与源码迁移纳入同步计划；漂移、构建失败、校验失败必须零写或回滚，版本戳最后写入。
3. Windows 入口无可见控制台；子进程保留 stdin/stdout/stderr、退出码与 timeout，禁止回退 Python。
4. Assist 两个事件可共享一个项目自有 Rust 程序，保持主机/IP/本地路径解析、缺失目录跳过、普通目录不覆盖和 robocopy 排除规则；仅临时 fixture 验证镜像删除行为。
5. 覆盖注册与路径拒绝、锁文件/源码/二进制漂移、失败回滚、重复升级、真实临时构建及独立审计；真实活库和 Codex 交互未执行时明确标记未验证。

## 进度

已完成调用链调查、通用实现、独立代码审计、最终完整回归和隔离环境实际程序验证，待用户验收。本轮开发与验证于 2026-09-03 10:54（Asia/Shanghai）完成，未超出 90 分钟预算。Assist Memory 确认 33/33，其他资产决定 5/47，尚余 42 项；本卡不代替或重置已有逐项决定。

## 实现与审计记录

通用能力由 `project_hooks.rs` 承载，project-sync 将注册、显式源码、程序及收据纳入原有回滚事务。Hook 的 identity 绑定单入口、事件参数、受管源树、Cargo.lock 和构建方案；Cargo 依赖清单拒绝未捕获文件及其他项目 Hook 的源码。项目源码不纳入公共源树 ownership；既有同名产物无合法收据时列为风险。工作区与暂存区均核验新注册，Windows PE subsystem 必须为 GUI。自检不调用项目 run，但项目 Rust 源码不是安全沙箱。

review-auditor 已独立复核并关闭同名产物误列 safe、跨 Hook 隐式依赖漏重建、暂存区漏校验、build-assets 回滚遇外部目标异常提前终止等问题。其后针对注册 JSON 末尾换行补充红转绿专项用例，避免第一次安装后再次误报更新。

Assist 源码保留在授权测试 worktree `.runtime/assist-rust-hooks-test/.codex/hooks/project_vault/entrypoint.rs`；两个事件共用路径解析。旧 Python bundle、主项目的旧源、原注册、活库及镜像均未改动。测试源码位于该 worktree 的 `scripts/tests/vault_hooks.rs`，临时测试 harness 位于工厂忽略目录 `.runtime/assist-hook-test-harness/`，不是下游正式分发文件。

## 当前验证收据

- `cargo test --locked --config scripts/tests/factory-cargo.toml --manifest-path .codex/hooks/Cargo.toml --workspace -- --test-threads=1`：109 passed，0 failed，4 ignored。随后唯一产品修正为注册 JSON 补 LF；`cargo test --locked --config scripts/tests/factory-cargo.toml --manifest-path templates/hooks/Cargo.toml -p bridgeforge-core project_hook_registration_serialization -- --test-threads=1` 先失败再通过，最终为 1 passed。完整 workspace 与后补专项分别计数，不合称一次全量运行。
- `cargo test --locked --config scripts/tests/factory-cargo.toml --manifest-path templates/hooks/Cargo.toml -p bridgeforge-core project_hooks -- --test-threads=1`：6 passed；覆盖锁定隔离构建、失败零安装、锁漂移、外部/跨 Hook 依赖拒绝、所有权及 GUI 检查。
- `cargo test --locked --manifest-path .runtime/assist-hook-test-harness/Cargo.toml --lib -- --test-threads=1`：最终源码 6 passed；覆盖主机/IP/local 顺序、仓库标记缺失时按程序位置定位项目、缺失路径跳过、普通目录不覆盖、临时目录下真实 junction 与 robocopy 镜像/删除/排除行为、退出码和超时。仅测试夹具正文发生复制与删除。
- 首轮完整 fixture 为 70 passed / 5 failed / 1 ignored：运行产物/清单尚未一致、测试误用 init，以及产品扫描误包含忽略的 `.runtime`。测试准备已修正；产品扫描只排除既有忽略的运行目录，不放宽产品 Python 禁令。
- 工厂 `manifest --check` 返回 changed=false；factory-version 为 1.11.0 healthy；project-structure 0 errors（保留一条既有归档建议）；skill-metadata 0 issues / warnings；`git diff --check` 通过。
- 最后一轮 `cargo test --locked --manifest-path scripts/tests/Cargo.toml -- --test-threads=1`：75 passed / 0 failed / 1 ignored，556.60 秒；包括从零初始化编译、兼容升级真实构建项目 Hook、未确认同名产物禁止覆盖、构建后源码漂移拒绝、stdin/参数/stdout、重复计划 current、基线源码漂移检测和下游 Cargo 测试。
- `templates/hooks/target/release/bridgeforge.exe build-assets --project-root D:/Quant/BridgeForgeCodex`：最终工厂 CLI 与 Hook 构建成功；`.codex/bin/bridgeforge.exe check baseline --root .` 返回 clean。受管源树 hash 为 `sha256:b063d99cb7fb21d20bdd5d91688232a898f4ee43d1e25c3b07c29cab312f7168`，Cargo.lock hash 为 `sha256:f517c9db76dc1a702e77a1bc16eb75d2c1e0dea28b36987fe7c6129127d2987b`。曾从待替换 CLI 自身发起构建而被 Windows 文件占用拒绝，事务回滚；最终使用独立 CLI 成功完成。
- `cargo run --locked --manifest-path .runtime/assist-hook-test-harness/Cargo.toml --bin assist-hook-install-smoke`：最终 Assist 源码真实编译、注册、安装成功；收据为 `applied / succeeded / 1.11.0 / ready`，版本戳最后写入。安装后的 EXE 的 print_target、junction、snapshot 三个入口均退出 0，重复计划为 current。该场景复用已核验的工厂核心产物，项目 Hook 重新编译；从零核心构建由上一项完整 fixture 单独覆盖。

## 最终隔离运行证据与边界

最终临时夹具为 `C:/Users/bridg/AppData/Local/Temp/assist-rust-install-16516-1788403614226829200`，仅其内部新建的 `fixture live source` 和镜像用于实际操作。项目 Hook 输入 hash 为 `sha256:8fb319ccc89eb19a2ce1c2eb420d4741a4128b2f64fb893c9946e6f741bcc383`，Windows 二进制 hash 为 `sha256:1d735db75a0700fa477c36efa074fac957d2c69f5e6a6d2972d59220e44997ae`。

独立 review-auditor 已复核最终换行修正与 Assist 项目根路径回退语义，无未解决的 P0/P1/P2 发现。仅使用该一个审计角色，未扩张并行实施范围。

六类传播证据：源码已实现；产品 Template/manifest 已更新但未发布；工厂 dogfood 与基线已通过；完整 fixture 已通过；真实下游主项目未应用（仅授权测试 worktree 留存新源码）；runtime 为隔离夹具中的实际 EXE 通过，真实活库、真实镜像和 Codex 会话触发未验证。

收尾只读核验：`git -c safe.directory=D:/Quant/BridgePersonalAssist -C D:/Quant/BridgePersonalAssist status --short --branch` 仅输出 `## master...origin/master`，真实骨架戳仍为 1.5.8。Git 同时提示不能读取用户级 ignore 文件；未修改权限或配置，不把该检查视为忽略文件内容的完整盘点。

## 交接与后续试用

本轮按 develop 流程保留 active，等待用户验收，不自动归档。未 commit、push、发布用户级骨架或清理下游旧资产。

下一步须由用户明确发起 Git 同步/发布，再继续按每批 5 项确认 Assist 剩余资产。全部确认后才由官方同步器在同一可回滚事务中安装新骨架、迁移新资产并删除已确认旧源。正式安装后，再在用户授权范围内验证真实会话的连接检查与快照；未获该授权前不得操作真实活库或镜像。
