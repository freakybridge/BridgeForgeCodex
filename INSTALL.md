# bridgeforge-codex 安装与迁移

当前候选工具仅支持骨架 8.0.0 及以后。更早项目的升级/格式迁移已退役，须在项目写入前拒绝；新项目初始化、8.x 当前配置与恢复保持。业务 VERSION 和独立子包版本不受该数字下限限制。

bridgeforge-codex 只支持 Windows、Rust/Cargo 1.88+ 和 Codex。Cargo 在安装、升级及工厂运行时准备时按需构建
受管 Rust 工具；日常 Hook 不调用 Cargo 或脚本解释器。请按自己的情况直接进入对应章节：

- 第一次使用：阅读“首次安装”。
- 使用过旧 `$bridgeforge`：阅读“旧 BridgeForge 用户重新安装”。
- 项目更新被阻断：阅读“项目升级与异常诊断”。
- 维护产品或同步器：阅读“维护者协议”。

## 工厂本地运行时准备

工厂源码可由 Git 更新，但 `.build/bridgeforge/bin` 是被忽略的本地产物。需要显式刷新工具或完成开发必要构建时执行下列入口；普通 `$git-sync` 和 `$git-sync release` 使用已安装工具，不将准备或独立 self-test 作为日常前置：

```powershell
cargo run --locked --release --manifest-path Cargo.toml --target-dir .build/bridgeforge/cargo/local --bin bridgeforge-factory -- build-assets --factory --project-root .
```

该入口不依赖已安装的 CLI，不提交或发布，不安装下游骨架。
仓库不设置根 `.cargo` 目录。所有构建、检查和测试入口显式传入 `--target-dir`，输出必须位于本仓库 `.build/`；从仓库根目录执行本文命令。不要省略该参数：直接运行未指定输出目录的原生 Cargo 命令仍会使用其默认 `target`，受管入口不能替任意手写命令改变默认行为。
它先核验已安装基线，再从当前产品源生成候选 dogfood、manifest 和入口，并核对 CLI、Hook、工厂工具的源码、锁文件、配方、产物
哈希、自检与收据；健康产物不改写，失效产物复用经验证的缓存或执行锁定构建。
候选投影、入口、三种产物及收据共用安装回滚事务。旧 `.codex/bin/` 文件仅在历史收据及实际字节可证明归属时退役；未知文件保留并阻断。维护锁仍协调构建与 develop；git-sync 不再持有该锁或支持同仓库并行同步。输入漂移、构建失败、
文件占用或安装校验失败均停止；并发外部修改保留并报告，不覆盖其他进程的文件。

Cargo 引导可能产生增量构建及缓存写入，只用于已授权维护。引导产物未建立完整输入
身份时不冒充正式产物复用；首次准备可能仍需隔离构建，实际耗时以执行结果为准。
当前产品源可以生成新的候选投影；已有镜像被外部改坏或旧文件归属不明时阻断，不自动改写产品源。单独 `manifest` 不得在新工具尚未就绪时切换运行入口。

项目目录遵守三目录合同：工厂生成工具及编译缓存放在 `.build/bridgeforge/`，消费项目的已安装公共工具放在 `.deps/bridgeforge/bin/`；用户级 `$env:USERPROFILE/.codex/bin/` 属于独立安装作用域，保持不变。项目本机配置、下载和测试结果分别使用 `.runtime/config/`、`.runtime/download/`、`.runtime/tests/<任务>/`。三个根全部由项目 `.gitignore` 忽略，受版本管理的配置继续留在原所有权位置。

只读诊断使用已支持 `factory-runtime-prepare-v1` 的工具：

```powershell
.build/bridgeforge/bin/bridgeforge-factory.exe build-assets --factory --project-root . --check
```

`--check`（以及同义 `--dry-run`）不创建锁、缓存或收据，不启动 Cargo 或产物自检进程；
它只证明当前源码/镜像与产物哈希、收据一致。缺失或旧工厂工具不支持此入口时报告检查不可用，
不得改用 `cargo run` 声称零写入。直接绕过准备入口运行旧 EXE 仍可能使用旧程序。
不新增拉取后构建 Hook，pre-commit 保持校验职责。下游继续使用原有受管安装升级流程。

严格只读检查对 `.gitattributes` 只接受能够直接证明的默认 `* text=auto eol=lf` 规则及注释；复杂属性语法报告无法证明，不创建临时 Git 仓库兜底。普通基线核验保留完整 Git 属性语义检查。

## 首次安装

1. 克隆产品仓库：

   ```powershell
   git clone https://github.com/freakybridge/BridgeForgeCodex.git "$env:USERPROFILE\tools\bridgeforge-codex"
   Set-Location "$env:USERPROFILE\tools\bridgeforge-codex"
   ```

2. 用锁定的 Cargo workspace 构建工厂 dogfood 工具，再安装用户级入口、通用 Skills 和统一用户指令：

   ```powershell
   cargo build --locked --release --manifest-path .\Cargo.toml --target-dir .build/bridgeforge/cargo/local --bin bridgeforge
   & powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\install-shared-skills.ps1
   ```

3. 新开 Codex 会话，在目标项目根目录运行：

   ```text
   $bridgeforge-codex
   ```

安装器会把完整产品仓库原子安装到 `~/.bridgeforge-codex`，只在
`~/.codex/skills/bridgeforge-codex` 保留供 Codex 发现的薄入口，并把其他受管 Skills
写入 `~/.codex/skills/`。`~/.codex/bridgeforge-codex-managed.json` 记录哪些用户级
Skills 由产品管理，避免覆盖来源不明或被人工修改的 Skill 文件。

安装和获准维护同时分发 `templates/user/AGENTS.md` 到默认 `~/.codex/AGENTS.md`。
文本相同则不写；本地有不同内容（包括个人编辑）时，原始文件先保存为
`~/.codex/.AGENTS.bridgeforge-backup-<operation_id>.md`，再覆盖为统一标准版。
备份成功后保留，收据给出具体路径；需要恢复时可用对应备份替换 AGENTS.md，但下次维护仍会恢复标准版。
这不改变项目专区保留规则，也不删除 `AGENTS.override.md`；现有会话不保证重载，请新开会话。
自定义 `CODEX_HOME` 不在本功能管理范围。旧薄入口首次刷新缺少用户指令收据时，按新版 Skill 补跑一次入口完成过渡。

项目侧运行面包括 `AGENTS.md`、`.codex/` 和 `.githooks/pre-commit`。项目同步器会先给出
计划，需要决定的项目会停下来确认；无法安全判断的冲突直接停止，应用失败时回滚。项目
指令由根或嵌套 `AGENTS.md` 原生加载，机器可判约束由 Hook 或 pre-commit 执行，操作流程
由 Skill 执行，长说明放在 `doc/`。

## 旧 BridgeForge 用户重新安装

旧 `$bridgeforge` 到 `$bridgeforge-codex` 的自动迁移已经退役。请直接执行上面的“首次安装”
流程。

当前安装器不会读取、接管或删除旧 Codex/Claude 受管记录、旧 `.bridgeforge` 产品目录或
旧用户级 Skill。如果正式 Skill 的目标路径已被旧安装占用，安装器会把它报告为“不属于
当前产品管理的冲突”并停止，不会根据旧记录猜测文件归属。

## 项目升级与异常诊断

项目骨架版本记录在 `.codex/.bridgeforge_codex_version`。咨询、诊断或只读预览不更新安装；明确维护（含无附加限制的 `$bridgeforge-codex`）才刷新官方产品 home，随后分流：

- 没有骨架身份和骨架资产的空白项目：直接初始化，不要求预先存在版本戳。
- 低于修复首版 `1.8.6`：盘点并确认项目资产后重建最新骨架，不读取旧 schema、旧 manifest 或逐版本兼容链。
- 等于或高于 `1.8.6`：兼容更新并保留项目定制；升级分界线固定，不随最新发布版本自动上移。比产品 home 更新的版本拒绝降级。
- 已有骨架资产但无戳：受控接入；双戳或非法戳在项目写入前停止。

旧 Markdown Rule／项目 Memory 的语义迁移已经退役。`.codex/rules/*.md` 和 `.codex/memory/**` 不再生成迁移包，也不会被固定删除；兼容更新保留这些文件，低基线重建或 adopt 时按普通项目自有资产的精确路径确认保留或删除。原生 `~/.codex/memories/` 不属于本操作范围。

可选项目资产通过一次性 `PreservationManifest` 明确保留或删除；确认前项目零写入，不长期保存该 manifest。资产处理、最新基线及验证属于同一项目事务；可捕获失败恢复本事务能够证明归属的改动，外部编辑保留并报告未恢复项。同步器最后才写当前版本戳，`.codex/managed-skeleton.json` 只保存当次最新 schema 4 的资产归属和内容哈希。

[子包开发收口与主程序发布](doc/0_architecture/design/package-version-lifecycle.md)要求 `package-development-v1`、`package-stability-v1`、`git-sync-development-closure-v1`；任务检查选择和版本覆盖还须核验 `task-selected-development-checks-v1`、`version-coverage-v1`。直接 apply 和阶段间复用分别核验 `direct-development-apply-v1`、`staged-development-validation-v1`，优先消费实际入口收据，不默认另跑 self-test。安装或骨架升级成功不等于项目已配置业务检查或完成版本收口。配置及实际入口只维护在[版本生命周期合同](templates/skills/git-sync/references/schema4.md)，保留原发布基线和下游 Python 环境支持，不借安装说明自动收口或写入真实下游。

## 维护者协议

本页说明安装、迁移与阻断行为；内部合同见：

- [Codex 项目同步设计](doc/0_architecture/design/codex-project-sync.md)：版本路由、规划、确认、事务和回滚合同。
- [设计依据](doc/0_architecture/design/design-rationale.md)：资产归属与安全取舍。
- [原生指令架构](doc/0_architecture/design/codex-native-instruction-architecture.md)：AGENTS、Hook、Skill 与文档的职责边界。

## 已退役能力

bridgeforge-codex 不再安装或维护 `CLAUDE.md`、`.claude/`、Claude 入口、host switch、
project finalizer、setup junction 或 harness parity。项目中发现 Claude 遗留时只报告存在，
不读取、不迁移、不删除。

Markdown `paths:` 不是 Codex 的指令加载机制，安装过程不会建立自研 Markdown Rule 加载器。
