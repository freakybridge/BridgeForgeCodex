# Codex 项目操作指南

## AGENTS 项目专区

项目级约束只写入根 `AGENTS.md` 的 `BRIDGEFORGE:PROJECT` 区域，或写入对应目录的嵌套 `AGENTS.md`。禁止修改 `BRIDGEFORGE:PUBLIC` 区域；编辑后 hook 会提示，pre-commit 会同时检查工作树与 staged blob，因而“先暂存公共区修改、再恢复工作树”不能绕过硬闸。

项目专区固定包含项目架构红线、业务与安全红线、目录地图、快速命令和目录级 AGENTS 索引。嵌套 `AGENTS.md`、项目自有 hook、项目 handler 与 pre-commit extension 均是项目资产，bridgeforge-codex 不覆盖、不删除、不重新格式化。

## Hook 检查边界

Shell 路径防护识别已支持写命令的实际目标：复制只检查目标，移动同时检查源和目标；相对路径从工具声明的工作目录解析，已有链接按真实位置核对。已识别的写入若含动态目标、未支持参数或工作目录变化，要求拆分为直接命令。它是有限命令识别器，不执行 Shell，也不代替平台对未知程序和外部副作用的权限控制。

普通编辑只检查本次涉及文本的编码，多文件补丁覆盖全部目标；修改 AGENTS、受管合同、Hook/配置入口、规则或文档索引等指令来源时才执行完整指令检查。完整遍历在入口剪去原已排除的 `.git`、`.runtime` 和 `doc/4_archive` 子树。无关存量问题不再由普通编辑即时报告，提交前 index 编码硬闸和项目 Map 新鲜度机制保持原职责。

SessionStart 查询 Git 的有效 `core.hooksPath`（包含 global、local、worktree 来源），保留已有配置；查询失败不按未配置处理。仅当确实未配置且默认 hooks 目录没有非 sample 文件时初始化本地 `.githooks`。项目自有 Hook 不会被自动接管。

## 文档治理

文档布局、各层职责与当前索引以 `doc/README.md` 为唯一事实源。完成的 delivery 或 Bug 需要归档时调用 `$archive-scan`；本文不复制各层含义或索引同步红线。

## 换机与依赖

clone 后先按项目主语言恢复受版本控制的依赖。BridgeForge 骨架只恢复锁定的 Cargo workspace；业务 Python 项目才建立自己的 `.venv`，Node 项目使用 `package.json`。语言环境不得混入骨架 Hook 运行链，目录改名后也不得复用含旧绝对路径的环境。

禁止在用户目录保存项目才能运行的关键配置，禁止在依赖清单中写本机绝对路径。机器特定凭据只提供受版本控制的示例与重建步骤，不提交真实凭据。

## 版本域隔离

项目根 `VERSION` 表示主程序整体发布；项目自有子包各自登记独立版本及唯一原生来源。`.codex/.bridgeforge_codex_version` 只表示 bridgeforge-codex 骨架版本，不能代替主程序或子包版本。

版本红线以根 `AGENTS.md`「版本与发布」「骨架与 Memory」为准。develop 验证后升子包并自动判定稳定版，release 核验收口后只升主程序并公告累计包变化；入口、能力和验证状态见[版本生命周期设计](../0_architecture/design/package-version-lifecycle.md)。普通 git-sync 不主动升版，不受子包收口门槛限制。

骨架更新的 preflight、事务、回滚和骨架版本戳写入顺序仍归 `$bridgeforge-codex` 及统一同步器，不与业务版本收口混用。骨架更新不替项目生成业务验证证据。

## 大版本依赖升级 Spike

因性能、UI 行为、字体等可感知体验而跨大版本升级依赖前，必须先在主项目外建立最小复现，限定 2–4 小时验证核心诉求。使用新版依赖与最小代码，同现状做截图、benchmark 或可复核的主观对比；只有用户确认体验确实改善后才启动全项目升级。若没有改善，停止升级并保留 spike 结论。

禁止先修改主项目或 lockfile 再验证；禁止只依据 CHANGELOG 宣称体验改善；禁止把“升级完成”当作“诉求已验证”。
