---
lifecycle: completed
validation_status: verified
---

# 项目 Hook 独立依赖

## 目标与授权

用户指出下游新增 Rust Hook 不应因额外依赖而修改公共骨架，并在核对改动范围后明确“开始改吧”。本次允许修改工厂产品源码、自身镜像、测试及说明；不提交、推送、安装用户级产品或写真实下游。

## 范围与决定

- 在现有项目 Hook 目录中支持独立 Cargo.toml、Cargo.lock 和源码；无 Cargo.toml 的既有 Hook 保持原行为。
- 项目维护依赖和锁文件，骨架仍负责隔离构建、注册、Windows 无窗口入口及原子安装。
- 项目工程全部输入纳入指纹、漂移检查和升级保留；锁文件缺失或过期必须停止。
- 下游工程为自包含包，路径依赖必须位于该 Hook 目录内。注册格式及 run(args) -> i32 入口保持兼容。
- 非目标：迁移 StratusAgent 的业务 Hook、自动生成或升级依赖锁、放宽真实下游写入授权。

## 规模、预算与传播

M：扩展既有构建/同步接口，不替换旧路径。45 分钟 / 20k 新增 token 估算（未实测）/ 最多 1 个子 agent / 最多 2 轮验证。预计超额时停止超额工作并报告。按 develop M 级实施，复用已有授权。

属于通用产品层，修改 templates，同步自身 dogfood，更新 VERSION 与 CHANGELOG [product]。文档以现有项目 Rust Hook 手册为使用事实源。

## 验收与风险

验收覆盖旧 Hook、项目额外依赖、锁文件缺失/过期、源码及文件集合漂移、非法路径依赖、构建失败不安装、同步升级保留。运行针对性单测、真实临时 Cargo 工程 smoke、工厂一致性检查；不把临时 fixture 当真实下游验收。

项目源码及依赖是受信可执行代码，构建脚本不是安全沙箱；隔离快照保证构建输入和产物核验，不声称隔离任意外部副作用。第三方依赖首次使用可能需要 Cargo 缓存或网络。

## 实施与验证记录

已实现独立工程模式及旧单文件兼容；新增 `project_hooks/package.rs` 负责输入捕获和构建，`project_sync.rs` 负责完整输入指纹及漂移阻断，`baseline.rs` 负责仅从暂存区读取工程。同步公共 AGENTS、项目 Hook 手册、自身镜像、版本 1.17.0 和生成清单。

- 源码：已实现，`git diff --check` 通过。
- 产品传播：模板源码、公共约束和手册已同步；未发布、未安装用户级产品。
- dogfood：源码镜像与最终生成资产已同步；`bridgeforge check baseline --root .` 返回 clean、版本 1.17.0，包含 Hook/CLI 生成收据和全部受管源码；版本、manifest --check、project-structure、skill-metadata 通过。
- fixture：最终固定源码执行 `cargo test --locked --config scripts/tests/factory-cargo.toml --manifest-path .codex/hooks/Cargo.toml -p bridgeforge-core project_ --no-fail-fast`，53 项通过。覆盖旧 Hook、独立路径依赖、真实 toml_edit 解析、锁文件缺失/过期、源码/文件集合漂移、越界依赖、暂存区、同步及事务回滚。公共 AGENTS 模板/dogfood 一致性定向测试通过。
- 真实下游：未执行，本轮未授权 StratusAgent 等真实下游写入。
- runtime：临时独立 Hook 已实际运行，验证 TOML 解析输出 42，以及另一个 Hook 的输出 independent:37 和退出码 7；未验证 Codex 真实事件或 StratusAgent 业务行为。

完整安装升级 fixture 首次因本轮在编译期间修改源码触发 `generated source inputs changed during build` 而中止；固定源码后重跑通过：`cargo test --locked --manifest-path scripts/tests/Cargo.toml project_sync_real_init_builds_and_applies_generated_assets -- --nocapture`，1 项通过，耗时 376.94 秒。断言覆盖真实临时项目初始化、旧 Hook 安装、切换独立依赖、构建后锁文件漂移阻断、安装后运行输出 42、再次同步 current、暂存区不借用工作区锁文件，以及下游普通 Cargo 测试不依赖工厂测试代码。

生成资产曾因从 `.codex/bin/bridgeforge.exe` 运行导致 Windows 无法覆盖当前进程而自动回滚；改用 `.codex/hooks/target/release/bridgeforge.exe build-assets --project-root D:/Quant/BridgeForgeCodex` 成功刷新。最终源码树 hash 为 `sha256:2877279782554a8ec9cd85117b4845eb8b8eb989041b71e60964f530f9ca7ff0`，Hook 与 CLI 收据一致。临时验证目录由测试清理。

预算保持 M 级、0 个子 agent；完成两轮内的定向验收，token 未实测。未执行全仓发布回归或独立发布审计，未发布、未提交、未推送。用户试用路径：下游取得本版本后，在项目 Hook 目录维护 Cargo.toml / Cargo.lock，通过既有 project-sync 或已注册项目的 build-assets 构建；使用说明见 `doc/3_reference/project-rust-hooks.md`。真实下游升级与业务 Hook 迁移仍待用户另行授权。

## 2026-09-08 发布检查与验收阻断

以上预算及验证描述为实施阶段收据。随后用户明确调用 git-sync，授权当前提交和推送；主对话按项目发布规则补充完整测试，并委派 1 个 review-auditor 独立审计。用户再次明确输入 `$summary 同意验收`，但下列必要条件尚未满足，因此保留 lifecycle: active，改为 awaiting_validation，不关闭交付，不执行发布。此前的定向测试和基线通过仍有效，但不能代替完整发布检查。

- 完整 workspace 测试：CLI 9 项通过；core 135 项通过、1 项失败、3 项忽略，命令退出 1。失败为 `memory::background::tests::background_does_not_inherit_extra_handles_and_preserves_native_contract`：`core_memory_background.rs:113` 报文件占用（os error 32）。单独串行复测在同一测试的第 119 行清理临时目录时报拒绝访问（os error 5）。该模块无本轮源码改动，根因未确认，不能把不同错误当作已修复。
- 完整工厂测试：86 项通过、1 项失败、3 项忽略，命令退出 1。`git_sync_runtime::real_factory_cli_sync_builds_new_runtime_and_commits_through_precommit` 在隔离测试仓库的自动写入规划阶段失败；构建 `codex.bridgeforge-cli` 时无法写入临时输出目录内的 `.fingerprint/bridgeforge-cli-eeffb4de970093bf/invoked.timestamp`，报路径不存在（os error 3）。未确认根因。项目 Hook 完整安装升级 fixture 在这次全量运行中仍通过。
- 独立审计 P2（已修复并获用户验收）：`project_hooks/package.rs` 和 `baseline.rs` 原先对 `.cargo` 与保留入口名的比较区分大小写。该项已完成 Windows 回归复现、共享校验修复与 54 项定向测试，见下方“大小写绕过定向修复”及本次验收记录。
- 独立审计 P2（已定向修复并获用户验收）：独立工程原先复用仅允许预先捕获文件的依赖清单校验，正常 `build.rs` 在 OUT_DIR 生成并由 include! 引入的源码被拒绝。现已真实复现并修复，见下方“生成源码定向修复”及验收记录；两项完整发布测试失败仍未解决。

本次验收调用只更新此需求卡，不重新运行测试、不修改产品代码或新增规则。已阅读原生 Memory 中生成资产失败与同步边界的相关记录；既有 AGENTS / git-sync 已覆盖发布前检查，不建议重复增加规则。当前交付无归档候选。实际项目未暂存、未提交、未推送；原有 stash 保留。恢复条件为处理上述阻断、取得对应验证与审计收据后，再收口验收并走受管 git-sync。

## 2026-09-08 大小写绕过定向修复

用户在逐项解释 Windows 大小写绕过问题后明确“开始修”。本次仅修该项，其他生成源码能力及发布测试失败保持未解决；本轮未授权提交或推送。按 S 级已知问题直接修复，20 分钟 / 8k token 估算（未实测）/ 0 个子 agent / 1 轮定向验证及必要修复重测，复用本需求卡，不新建交付文档。

传播四问：通用产品层修复，进入 templates；同步自身源码及手册镜像；版本更新到 1.17.1 并写入 CHANGELOG [product]；刷新生成清单与本仓库产物。

修复前新增回归测试已在 Windows 复现：`project_package_reserved_paths_reject_case_variants_and_preserve_similar_names` 发现 `.CARGO/config.toml` 被 capture_input 接受，测试按预期失败。

修复采用共享的 `reserved_package_component`，按 ASCII 大小写无关方式识别 `.cargo` 和 `.bridgeforge-main.rs`；工作区扫描、待写入内容验证及暂存区读取共用该规则。测试覆盖大小写/混合写法、嵌套路径、保留入口名，以及 `.cargo-notes`、`.bridgeforge-main.rs.txt` 等正常相似名称。

修复后执行 `cargo test --locked --config scripts/tests/factory-cargo.toml --manifest-path .codex/hooks/Cargo.toml -p bridgeforge-core project_ --no-fail-fast`，54 项全部通过。manifest --check、factory-version、project-structure 和 git diff --check 通过。新增测试由修复前失败变为修复后通过；对应大小写检查遗漏已修复并有定向证据，未声称真实 Codex 事件或下游升级已经验证。

模板、手册及自身源码镜像已同步。通过构建目录内受管 CLI 的 build-assets 刷新 Hook/CLI 产物，二者源码树收据均为 `sha256:46bc0e54b7e487c733b313ed232e61eee9e3d7e2ea264441a7b5278c29247ba7`；随后 `.codex/bin/bridgeforge.exe check baseline --root .` 返回 clean、版本 1.17.1。未执行真实下游写入、用户级安装、提交或推送。其余发布阻断仍待处理，本交付保持 awaiting_validation。

### 大小写修复验收记录

用户在上述定向修复完成后明确调用 `$summary 同意验收` 和 `$git-sync`。依据已有复现、修复后 54 项通过及 1.17.1 基线 clean 收据，记录“大小写绕过”子项已获用户验收；本轮只读核对确认共享校验仍在，没有为总结重跑测试。

整个独立依赖交付仍存在生成源码审计问题与两项完整发布检查失败，不能把子项验收扩大为整个 topic 已完成。顶层状态保持 active / awaiting_validation，无归档候选。已收到本轮提交同步授权，但项目发布前完整测试及审计条件未满足，未执行受管 git-sync，未暂存、未提交、未推送。复用已读 Memory 与现行约束，不新增重复 Rule、Hook 或 AGENTS 建议。

## 2026-09-08 生成源码定向修复

用户在解释生成源码误拒绝后明确“开始修吧”。本轮仅修此项，不处理其余两项完整发布测试故障，不提交或推送。按 S 级已知问题直接实施，20 分钟 / 8k token 估算（未实测）/ 0 个子 agent / 1 轮定向验证及必要修复重测，复用本卡。

传播四问：通用产品层，修改 templates 中的项目 Hook 构建器；同步自身源码及手册镜像；更新版本 1.17.2 与 CHANGELOG [product]；刷新生成清单和本仓库产物。

新增真实 Cargo 回归测试先复现旧实现失败：Cargo 编译成功，但后续校验报 `project Rust hook used uncaptured dependency: .../output/release/build/.../out/generated.rs`。修复后，仅捕获本次临时构建 OUT_DIR 下、依赖清单实际引用的生成文件，自检后复核其内容及依赖清单未变化；不扩大旧单文件模式。独立构建模式标识更新到 project-package-v2，使既有收据触发重建。

执行 `cargo test --locked --config scripts/tests/factory-cargo.toml --manifest-path .codex/hooks/Cargo.toml -p bridgeforge-core project_ --no-fail-fast`，最终 57 项全部通过。覆盖生成源码编译并实际运行输出 42、自检期间生成文件变化被拒绝、绝对路径外部 include 被拒绝、输出目录边界、Windows junction 被拒绝及旧模式不放宽。第一次修复后测试中 56 项通过，junction 用例因 mklink 不接受正斜杠路径而未能创建夹具；改为 Windows 路径后重测全部通过。

源码和临时 runtime 已定向验证；真实下游升级、Codex 实际事件、用户级安装及本轮独立审计未执行。此子项尚未获用户验收，整体仍为 active / awaiting_validation，两项完整发布测试失败保留。

产品模板、手册及自身镜像已同步，受管 build-assets 成功更新本仓库 Hook 与 CLI；两项产物源码树收据均为 `sha256:f8baa56d5bf2b334f24047d9dc759d038bed0858b003d99ba236c3bf7220022a`。随后 `.codex/bin/bridgeforge.exe check baseline --root .` 返回 clean、版本 1.17.2；manifest --check、factory-version、project-structure 及 git diff --check 均通过。未暂存、未提交、未推送，原有 stash 未改动。

### 生成源码修复验收记录

用户随后明确调用 `$summary 同意验收` 与 `$git-sync`。依据上述 57 项定向测试通过、真实生成源码运行及 1.17.2 基线 clean 收据，记录生成源码误拒绝子项已获用户验收；本次仅更新既有验收记录，不为总结重跑测试。

整体保持 active / awaiting_validation：先前 memory background 与 git-sync runtime 两项完整测试失败尚未处理，真实下游与 Codex 事件验证仍未执行。本次未发现新阻断。已收到本轮同步授权，但项目发布前完整测试条件未满足，未执行受管 git-sync；main 工作区变更未暂存、未提交、未推送，原有 codex_git_sync_autostash 保留，未 fetch 因而不声称最新远端同步状态。

已检索并阅读原生 Memory 中受管同步和基线证据边界，结合现行项目约束，不新增重复 Rule、Hook 或 AGENTS 建议。当前交付无归档候选。恢复条件为处理两项已有完整测试失败并补齐发布验证与独立审计收据，再执行受管同步。

### 后续 Memory 阻断处理

用户随后授权先修 Memory 同步问题，并确认真实本机/远端并集合并及唯一同名冲突采用本机。处理记录见 [Windows 长路径同步 Bug](../../2_bugs/BUG-native-memory-windows-long-path-sync.md)。原后台测试并行干扰与退出清理竞争已修正，最终完整 workspace 测试退出 0，先前 Memory 测试阻断已解除。用户实际同步另有 Windows 长路径缺陷，源码与 dogfood 已修复到 1.17.3；真实 Memory 合并已获受管成功收据。用户级旧程序尚未升级，不宣称长期自动同步问题已完全交付。

本 topic 的其他发布条件仍需分别核对：完整工厂 git-sync runtime 失败未处理，本轮未做独立发布审计。保持 active / awaiting_validation；未提交或推送本工厂代码。

### 发布前复核与本次验收

2026-09-08 用户再次明确 `$summary 同意验收` 与 `$git-sync`。后续 Memory 修复阶段已完成 core 147 项、Hook 24 项及 CLI 13 项通过；完整工厂回归退出 0，87 passed、0 failed、3 ignored，包含此前失败的真实工厂 git-sync runtime 和完整项目 Hook 初始化安装夹具。原 background 并行干扰已修复，git-sync invoked.timestamp 故障本次未复现，不把未复现写成根因已修。前期独立审计指出的大小写保留路径和生成源码问题均已有修复与回归；本次 Memory 独立审计也已完成。

用户已验收源码及既有 fixture 结果，授权当前 Memory 与此前 Rust Hook 改动统一受管发布。真实下游业务迁移不在本次范围；正式产品安装与关联 Memory 生命周期验证仍待执行，保留 active / awaiting_validation，最终收据写入关联的 [自动同步可靠性闭环](../current-baseline-project-asset-migration-and-native-memory-sync/requirements_2026-09-08_auto-sync-reliability.md)。不新增规则，不归档。

### 最终收口

上述发布和关联 Memory runtime 条件现已完成：修复提交 `3e2d7e9b0639e6f2a2bed7ecf58d7e524db7d2d8` 已推送 origin/main，用户级官方产品安装为 1.17.5；真实 Memory 三生命周期实测退出 0，最终 healthy/noop 且队列清空，详细收据见关联需求卡。结合此前独立审计、57 项项目 Hook 专项及本轮完整工厂 87 项通过，用户明确验收，本交付关闭。此结论不包含未授权的 StratusAgent 等真实下游升级或业务 Hook 迁移；无自动归档或新增规则。
