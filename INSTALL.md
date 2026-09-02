# bridgeforge-codex 安装与迁移

bridgeforge-codex 只支持 Windows、Rust/Cargo 1.88+ 和 Codex。Cargo 在安装、升级及工厂同步提交时构建
受管 Rust 工具；日常 Hook 不调用 Cargo 或脚本解释器。请按自己的情况直接进入对应章节：

- 第一次使用：阅读“首次安装”。
- 使用过旧 `$bridgeforge`：阅读“旧 BridgeForge 用户重新安装”。
- 项目更新被阻断：阅读“项目升级与异常诊断”。
- 维护产品或同步器：阅读“维护者协议”。

## 首次安装

1. 克隆产品仓库：

   ```powershell
   git clone https://github.com/freakybridge/BridgeForgeCodex.git "$env:USERPROFILE\tools\bridgeforge-codex"
   Set-Location "$env:USERPROFILE\tools\bridgeforge-codex"
   ```

2. 用锁定的 Cargo workspace 构建工厂 dogfood 工具，再安装用户级入口和通用 Skills：

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
资产由产品管理，避免覆盖来源不明或被人工修改的文件。

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

项目骨架版本记录在 `.codex/.bridgeforge_codex_version`。每次 `$bridgeforge-codex` 先刷新官方产品 home，再按身份和固定升级基线分流：

- 没有骨架身份和骨架资产的空白项目：直接初始化，不要求预先存在版本戳。
- 低于修复首版 `1.8.6`：盘点并确认项目资产后重建最新骨架，不读取旧 schema、旧 manifest 或逐版本兼容链。
- 等于或高于 `1.8.6`：兼容更新并保留项目定制；升级分界线固定，不随最新发布版本自动上移。比产品 home 更新的版本拒绝降级。
- 已存在骨架资产但没有版本戳：进入受控接入；双戳或非法戳在写入前停止。

两条升级路径都会逐文件整理 `.codex/rules/*.md` 和 `.codex/memory/**`。每个源文件展示完整迁移包并由用户确认：红线进 `AGENTS.md`，命令策略进 `.rules`，流程进 Skill，机械约束进 Hook / test，工作与设计资料进 `doc/`。`MEMORY.md`、`MEMORY_COLD.md`、`_stats.json` 逐个确认固定退役，不做语义转换。

确认必须在一次连续流程中完成，中断后不保存选择并从第一个源重来。全部确认前项目零写入；确认后的新资产、最新基线、旧 Rule / Memory 删除和验证属于同一事务，失败时完整回滚。逐文件迁移确认已经授权删除对应源，不会再弹出第二次清理确认。

其他可选项目资产仍通过一次性 `PreservationManifest` 明确保留或删除；它和迁移 manifest 都不长期写入项目。同步器最后才写当前版本戳，`.codex/managed-skeleton.json` 只保存当次最新 schema 4 的资产归属和内容哈希。

## 维护者协议

本页只说明用户可观察到的安装、迁移和阻断行为，不重复维护同步器内部合同。维护者按以下
单一事实源继续阅读：

- [Codex 项目同步设计](doc/0_architecture/design/codex-project-sync.md)：版本路由、规划、确认、事务和回滚合同。
- [设计依据](doc/0_architecture/design/design-rationale.md)：资产归属与安全取舍。
- [原生指令架构](doc/0_architecture/design/codex-native-instruction-architecture.md)：AGENTS、Hook、Skill 与文档的职责边界。

## 已退役能力

bridgeforge-codex 不再安装或维护 `CLAUDE.md`、`.claude/`、Claude 入口、host switch、
project finalizer、setup junction 或 harness parity。项目中发现 Claude 遗留时只报告存在，
不读取、不迁移、不删除。

Markdown `paths:` 不是 Codex 的指令加载机制，安装过程不会建立自研 Markdown Rule 加载器。
