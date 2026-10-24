# bridgeforge-codex 安装与迁移

bridgeforge-codex 只支持 Windows、Rust/Cargo 1.88+ 和 Codex。Cargo 在安装、升级及工厂运行时准备时按需构建
受管 Rust 工具；日常 Hook 不调用 Cargo 或脚本解释器。请按自己的情况直接进入对应章节：

- 第一次使用：阅读“首次安装”。
- 使用过旧 `$bridgeforge`：阅读“旧 BridgeForge 用户重新安装”。
- 项目更新被阻断：阅读“项目升级与异常诊断”。
- 维护产品或同步器：阅读“维护者协议”。

## 工厂本地运行时准备

工厂源码可由 Git 更新，但 `.codex/bin` 是被忽略的本地产物。维护本工厂时，在普通
`$git-sync`、`$git-sync release` 的能力检查之前，或需要显式刷新工具时执行：

```powershell
cargo run --locked --release --manifest-path .codex/hooks/Cargo.toml --bin bridgeforge -- build-assets --factory --project-root .
```

该入口不依赖旧 `.codex/bin/bridgeforge.exe`，不提交或发布，不安装下游骨架。
它先校验产品源、dogfood 和派生 manifest，再核对 CLI/Hook 的源码、锁文件、配方、产物
哈希、自检与收据；健康产物不改写，失效产物复用经验证的缓存或执行锁定构建。
两种产物及收据共用安装回滚事务，并与 git-sync 共用仓库锁。输入漂移、构建失败、
文件占用或安装校验失败均停止；并发外部修改保留并报告，不覆盖其他进程的文件。

Cargo 引导可能产生增量构建及缓存写入，只用于已授权维护。引导产物未建立完整输入
身份时不冒充正式产物复用；首次准备可能仍需隔离构建，实际耗时以执行结果为准。
源码和镜像/manifest 漂移时先完成开发同步，准备入口不会自动修复源文件。

只读诊断使用已支持 `factory-runtime-prepare-v1` 的工具：

```powershell
.codex/bin/bridgeforge.exe build-assets --factory --project-root . --check
```

`--check`（以及同义 `--dry-run`）不创建锁、缓存或收据，不启动 Cargo 或产物自检进程；
它只证明当前源码/镜像与产物哈希、收据一致。缺失或旧 CLI 不支持此入口时报告检查不可用，
不得改用 `cargo run` 声称零写入。直接绕过准备入口运行旧 EXE 仍可能使用旧程序。
不新增拉取后构建 Hook，pre-commit 保持校验职责。下游继续使用原有受管安装升级流程。

## 首次安装

1. 克隆产品仓库：

   ```powershell
   git clone https://github.com/freakybridge/BridgeForgeCodex.git "$env:USERPROFILE\tools\bridgeforge-codex"
   Set-Location "$env:USERPROFILE\tools\bridgeforge-codex"
   ```

2. 用锁定的 Cargo workspace 构建工厂 dogfood 工具，再安装用户级入口、通用 Skills 和统一用户指令：

   ```powershell
   cargo build --locked --release --manifest-path .\templates\hooks\Cargo.toml
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

两条升级路径都会逐文件整理 `.codex/rules/*.md` 和 `.codex/memory/**`。每个源文件展示完整迁移包并由用户确认：红线进 `AGENTS.md`，命令策略进 `.rules`，流程进 Skill，机械约束进 Hook / test，工作与设计资料进 `doc/`。`MEMORY.md`、`MEMORY_COLD.md`、`_stats.json` 逐个确认固定退役，不做语义转换。

确认必须在一次连续流程中完成，中断后不保存选择并从第一个源重来。全部确认前项目零写入；确认后的新资产、最新基线、旧 Rule / Memory 删除和验证属于同一事务，失败时完整回滚。逐文件迁移确认已经授权删除对应源，不会再弹出第二次清理确认。

其他可选项目资产仍通过一次性 `PreservationManifest` 明确保留或删除；它和迁移 manifest 都不长期写入项目。同步器最后才写当前版本戳，`.codex/managed-skeleton.json` 只保存当次最新 schema 4 的资产归属和内容哈希。

[子包开发收口与主程序发布](doc/0_architecture/design/package-version-lifecycle.md)要求 `package-development-v1`、`package-stability-v1` 和 `git-sync-development-closure-v1`。安装或骨架升级成功不等于项目已配置业务检查或完成版本收口。复用 schema 4 配置提案登记项目检查，保留原发布基线；具体入口见[版本生命周期合同](skills/git-sync/references/schema4.md)。不将未发布改动默认认证为已收口，不借安装说明自动写入真实下游。

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
