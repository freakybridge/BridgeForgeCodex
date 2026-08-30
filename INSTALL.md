# bridgeforge-codex 安装与迁移

bridgeforge-codex 只支持 Windows、Python 3.11+ 和 Codex。请按自己的情况直接进入对应章节：

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

2. 在仓库根目录安装用户级入口和通用 Skills：

   ```powershell
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

项目骨架版本记录在 `.codex/.bridgeforge_codex_version`。同步器先区分空白项目和已有骨架：

- 没有骨架身份和骨架资产的空白项目：直接初始化，不要求预先存在版本戳。
- 版本 `>=1.4.31`：按当前资产归属清单格式（schema 3）更新受管资产，不重放历史迁移链。
- 版本 `<1.4.31`：先独立审计旧项目，再进行一次确认式重建。
- 已存在骨架资产但缺少可识别版本戳、同时存在多个版本戳或版本戳非法：在创建 `.venv` 和生成更新计划前停止，保持零写入。

旧项目重建时会逐项检查项目 Rules、`AGENTS.md` 项目区、pre-commit 项目扩展和
`.codex/hooks/project_XXXX/` 自包含 Hook 目录。散落 Hook 必须先由独立 Agent 在临时副本
或受控步骤中整理；项目 Memory 和 Skills 自动保留并检查。

每个可选项目资产都必须明确选择保留或删除。这个临时确认单在内部称为
`PreservationManifest`：它只服务本次事务，不会长期写入项目。所有写入和验证通过后，
同步器才删除旧戳并最后写入当前版本戳；`1.4.31+` 项目的
`.codex/managed-skeleton.json` 只保存当前 schema 3 的资产归属和内容哈希。

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
