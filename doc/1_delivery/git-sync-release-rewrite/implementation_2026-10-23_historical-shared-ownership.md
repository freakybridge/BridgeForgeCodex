---
lifecycle: completed
validation_status: verified
---

# 历史共享源码归属补正

## 范围与原因

开发阶段用户授权仅修复 Forge，运行相关定向验证；不跑全量回归、不修改下游、不提交或推送。后续验收与发布授权见文末。

StratusAgent 对话中的骨架 4.0.0 更新已经成功；后续业务 release 因 `stratus/apps/backtest/app/src/gui.rs` 同时归自然包与外置 Cargo 测试消费者所有而阻断。此前补正只修复静态源码路径/hash，未生成缺失的共享归属。本次不以修改下游配置绕过歧义。

传播四问：这是通用产品能力，落在模板 Rust 发布核心和共享 git-sync reference；同步工厂 dogfood。将来获准 release 时由受管事务处理产品版本与 CHANGELOG，本轮不预写版本。

## 实现

1. 从 Cargo 显式外置入口与静态 mod/path/include 图找出确有字节内容的共享文件，仅为多个 Rust 包补精确路径声明。普通 owner 仍拒绝未声明歧义。
2. 基线补正分别重建历史与当前消费者，计划列出两组新增共享声明及内容 hash、包 manifest/source_roots 证据。既有显式输入角色保持优先；已发布基线、伪造历史、宽目录声明与输入漂移仍阻断。
3. 推导的共享声明属于迁移记账。仅当当前配置等于不可变初始配置加上当前静态图重算出的声明时，忽略这一配置差异；真实其他配置变动仍参与发布分析。

## 验证记录

验证按风险选取：历史证明与事务边界用基线补正夹具；消费者变化用历史/当前分离场景；普通归属边界用外置入口与不透明宏场景；迁移记账用无业务变化及真实配置变化对照。不运行完整 workspace 或发布准备流程。

- 首轮 `cargo test --locked --config scripts/tests/factory-cargo.toml --manifest-path .codex/hooks/Cargo.toml -p bridgeforge-core schema4_baseline_repair -- --test-threads=4`：5 项通过，458 项过滤；测试执行 179.03 秒。
- 补充运行 `cargo test --locked --config scripts/tests/factory-cargo.toml --manifest-path .codex/hooks/Cargo.toml -p bridgeforge-core --lib -- schema4_baseline_repair_sharing schema4_external_ schema4_opaque_include schema4_real_configuration_changes schema4_schema1_migration --test-threads=4`：外置入口/模块移动、路径歧义、不透明 include、schema 1 迁移共 5 项通过；4 项暴露初始配置省略 shared_inputs 的兼容错误。
- 修正缺省字段后，仅重跑 `--lib -- schema4_baseline_repair_sharing schema4_real_configuration_changes --test-threads=4`：4 项通过，459 项过滤，124.23 秒。包括推导共享不能掩盖其他配置变化的断言。复用未受影响结果，共 11 个不同定向场景通过；未跑全量回归。
- 独立 review-auditor：发现上述缺省字段问题；修正后复核通过，无剩余阻断项。静态审计与实际测试分别记录。
- `manifest --root .` 刷新后 `manifest --root . --check` 返回 ok / changed=false；`git diff --check` 通过。
- 最终仅执行一次受管 `build-assets --project-root .`，CLI 与 Hook 均构建成功并通过构建内自检；收据位于 `.runtime/historical-shared-ownership/build-receipt.json`，共同源码 hash 为 `sha256:9a00dbab57661b4a4b1f4efb0f6eed2a813e577f595274dc580ed9557081db18`。
- 新产物执行 `check baseline --root .` 返回 `state=clean`；收据位于 `.runtime/historical-shared-ownership/baseline-receipt.json`。首次因沙箱临时 Git 目录权限失败，原样提权重试成功，无重建。
- 四组修改的源码/reference 镜像 hash 一致。HEAD 仍为 `aaeea59fb39f35de43a0a63b2c5226cb174e01d7`；VERSION / CHANGELOG 无改动，版本仍为 4.0.0。本轮变更留在工作区，未提交、推送或发布。

## 证据边界

源码、模板传播、dogfood 镜像与临时 Git 夹具分别核验。夹具中的迁移、发布与提交仅发生在测试临时目录。未执行真实下游补正、真实业务发布或自然 runtime 验证，不能宣称 StratusAgent 现场已经恢复。

## 经验沉淀

1. 外置入口路径与内容 hash 齐全，不等于共享消费者归属齐全。自然包与外置测试同时消费文件时，应从相应快照的静态图补精确共享声明，不能用当前消费者覆盖历史事实。
2. 补正产生的配置差异应与真实配置改动分开判断；必须同时验证“仅推导共享不升版”和“推导共享加真实配置变动仍可见”。旧 JSON 允许省略字段，不能按数组必然存在处理。
3. 增量审查发现缺省字段问题后只复验受影响的 4 项，其余证据复用；源码稳定后集中构建一次。总结与验收不再触发构建或回归。

历史核对：已阅读原生 Memory 的历史 Cargo 归属及同环境发布记录（2026-10-22）。旧记录只证明 paths/hash 的补正范围，本次 shared_inputs 增量以本文与当前代码为准；旧测试与旧发布状态不作为本轮证据。现行 AGENTS、开发收口合同及 git-sync reference 已承载对应约束，不新增 Rule / Hook / AGENTS 建议，不写 Memory。当前 topic 尚有关联未完成后续事项，无归档候选。

## 用户验收与发布授权（2026-10-23）

用户明确调用 `$summary accept` 与 `$git-sync release`，确认本次 Forge 修复符合需求，并授权发布本批改动。TODO-017 的历史共享源码增量按 11 个定向场景（含有效证据复用）、独立审查、受管构建与基线一致性证据验收；原有验收记录保留。

仅更新 TODO-017 的本次进展与验收链接，不改变无关 TODO，不新增推测事项。真实下游 apply/release 与自然 runtime 仍未验证。正式版本、提交与推送结果以本次受管发布收据为准；本段记录授权与验收，不预报发布成功。
