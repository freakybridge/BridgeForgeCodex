# Codex 项目操作指南

## AGENTS 项目专区

项目级约束只写入根 `AGENTS.md` 的 `BRIDGEFORGE:PROJECT` 区域，或写入对应目录的嵌套 `AGENTS.md`。禁止修改 `BRIDGEFORGE:PUBLIC` 区域；编辑后 hook 会提示，pre-commit 会同时检查工作树与 staged blob，因而“先暂存公共区修改、再恢复工作树”不能绕过硬闸。

项目专区固定包含项目架构红线、业务与安全红线、目录地图、快速命令和目录级 AGENTS 索引。嵌套 `AGENTS.md`、项目自有 hook、项目 handler 与 pre-commit extension 均是项目资产，bridgeforge-codex 不覆盖、不删除、不重新格式化。

## 文档治理

文档布局、各层职责与当前索引以 `doc/README.md` 为唯一事实源。完成的 delivery 或 Bug 需要归档时调用 `$archive-scan`；本文不复制五层含义或索引同步红线。

## 换机与依赖

clone 后先按项目主语言恢复受版本控制的依赖。BridgeForge 骨架只恢复锁定的 Cargo workspace；业务 Python 项目才建立自己的 `.venv`，Node 项目使用 `package.json`。语言环境不得混入骨架 Hook 运行链，目录改名后也不得复用含旧绝对路径的环境。

禁止在用户目录保存项目才能运行的关键配置，禁止在依赖清单中写本机绝对路径。机器特定凭据只提供受版本控制的示例与重建步骤，不提交真实凭据。

## 版本域隔离

项目根 `VERSION` 表示下游业务版本，`.codex/.bridgeforge_codex_version` 表示 bridgeforge-codex 骨架版本；两者属于独立生命周期，不能互相代替。

版本域红线以根 `AGENTS.md`「信息放置与指令承载」为准。实际 preflight、事务、回滚和版本戳写入顺序唯一归 `$bridgeforge-codex` 及其统一项目同步器，本文不复制操作规则。

## 大版本依赖升级 Spike

因性能、UI 行为、字体等可感知体验而跨大版本升级依赖前，必须先在主项目外建立最小复现，限定 2–4 小时验证核心诉求。使用新版依赖与最小代码，同现状做截图、benchmark 或可复核的主观对比；只有用户确认体验确实改善后才启动全项目升级。若没有改善，停止升级并保留 spike 结论。

禁止先修改主项目或 lockfile 再验证；禁止只依据 CHANGELOG 宣称体验改善；禁止把“升级完成”当作“诉求已验证”。
