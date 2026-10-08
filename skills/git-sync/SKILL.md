---
name: git-sync
description: 分析当前 Git 变更，生成简体中文提交消息与代码变动效果摘要，并安全完成 fetch、必要的快进更新、commit、push 和最终同步核验；用户明确调用 /git-sync 或 $git-sync 时使用。
user_invocable: true
argument: 可选：release
---

# git-sync — 提交并推送

## 定位与边界

用户显式调用即授权本轮执行 Git 同步闭环。优先使用项目提供的确定性脚本；自动化只覆盖安全机械步骤，任何分叉、冲突、缺 upstream 或破坏性恢复都必须停止并交回主对话。

## 同步与发布

先读取项目 `.codex/bridgeforge-version.json` 的 `release_policy`：`explicit_release` 表示普通同步不升版；缺省或 `per_commit` 保持原有每次业务提交升版行为。不得因用户调用普通同步而擅自切换项目策略。

使用显式发布或 `explicit_release` 策略前，先运行受管二进制 `self-test --json`，确认 `capabilities` 包含 `git-sync-explicit-release-v1`。旧二进制可能忽略未知参数并执行普通同步，不能直接试跑新参数探测能力；缺能力时停止并提示先升级骨架。

| 用户用法 | 执行行为 |
|---|---|
| `$git-sync` | 提交并同步；启用 `explicit_release` 的项目不自动修改版本、原生 manifest/lock 或 CHANGELOG |
| `$git-sync release` | 先只读预览累计发布计划，再使用 `--release` 升版并同步 |

显式发布取上次 VERSION 发布边界以来全部未发布业务改动的最高级别，只升一次：`!` 或 `BREAKING CHANGE:` 升 major；否则含 `feat` 升 minor；其他合法 `fix/perf/refactor/docs/chore` 升 patch。major/minor 后低位归零。纯骨架升级不参与；当前未提交业务改动使用经 diff 审查的消息参与；干净工作区不能用发布命令的消息覆盖历史级别。

工作区干净但存在未发布业务提交仍可发布；没有未发布业务改动不升版、不创建空提交。浅历史、合并历史边界、非规范业务提交或手工版本变化导致计划不可可靠判定时停止并展示原始原因，不擅自降为 patch。

## 输入

- 当前仓库、分支、upstream、工作区与暂存区状态。
- `git diff` / `git diff --cached` 的真实变更。
- 当前宿主目录内受管 `.codex/bin/bridgeforge[.exe]`。

## 核心流程

### 1. 只读判断、效果摘要与提交消息

1. 检查状态、diff 和当前分支，概括本轮实际变更。
2. 在运行同步脚本前，根据真实 diff 固化 1-3 条“代码变动效果”：说明增加的功能、修复的问题或改变的系统行为，以及它们带来的实际结果；禁止只罗列文件名、提交类型或“更新了代码”等空话。无法可靠判断时明确标记“未能从 diff 可靠判定”，禁止猜测。
3. 生成简洁的简体中文消息：`<类型>: <描述>`；类型限 `feat`、`fix`、`refactor`、`perf`、`docs`、`chore`。

### 2. 当前宿主确定性 Rust 入口

Codex 项目使用 `.codex/bin/bridgeforge[.exe] git-sync`。二进制存在时，主 agent 完成只读范围审查和提交消息决策后，必须直接且只运行该入口：

```text
.codex/bin/bridgeforge git-sync --message "<类型>: <描述>"
```

用户调用 `$git-sync release` 时，先执行零写入预览并展示目标版本、累计改动及升级依据。`release` 是 Skill 参数，底层 CLI 仍使用 `--release`：

```text
.codex/bin/bridgeforge git-sync --release-preview --message "<类型>: <描述>"
.codex/bin/bridgeforge git-sync --release --message "<类型>: <描述>"
```

预览无需再次索取已有发布授权；只在结果暴露新的范围或兼容性疑问时澄清。预览返回 `nothing-to-release` 时说明不升版，仍可按原授权完成同步。实际同步在 fetch/快进后重新计算发布计划，以最终收据为准。多行消息优先写入临时文件并传 `--message-file`，预览与执行使用同一消息。

需要审批时只为该项目脚本申请合理前缀，不分别为 fetch、add、commit 和 push 申请持久规则。脚本可执行 fetch、ahead / behind 判断、安全 stash、`pull --ff-only`、按项目策略或显式发布请求升级版本与同步原生版本、CHANGELOG 和衍生产物刷新（工厂提交先在仓库外临时目录构建 Hook / CLI 并生成实测收据，将版本、manifest、二进制和收据纳入同一可回滚事务；pre-commit 只读验证）、add、commit、push 和最终检查。纯 `$bridgeforge-codex` 骨架更新不升级项目版本。

若首次运行在 `git fetch`、`.git/FETCH_HEAD`、`Permission denied` 或 `Access is denied` 阶段失败，主 agent 必须立即以**完全相同的 repo-local 脚本命令**、`require_escalated` 重试。审批说明仅限：允许 Git 更新当前项目的 `.git/FETCH_HEAD` 等元数据，以完成用户已授权的同步。不得改走手工 Git 命令、修改 `.git` ACL 或扩大到无关目录。重试仍失败时保留原始错误与现场并停止；不得把网络、分叉或凭据错误伪报为权限恢复成功。

除上述确定性的权限恢复外，任何分叉、冲突或失败必须返回主对话处理。

受管二进制不存在、自检失败或当前项目 baseline 不健康时必须停止并报告；禁止回退 Python、PATH 脚本或手工 Git。即使用户要求逐条执行，也不得退化为手工 fetch、add、commit 或 push。

## 输出与收据

- 当前分支、upstream、同步前后的 ahead / behind。
- 实际提交消息、commit id 和 push 目标。
- 项目策略、是否要求发布、是否实际升版，以及升版前后版本；普通同步完成不能表述为版本发布完成。
- 收据提供 `generated_assets_reused` / `generated_assets_built` 时，报告工厂产物复用和重建数量，便于区分无构建同步与实际重建。
- 同步成功后输出此前固化的“代码变动效果”，最多 3 条；显式发布概括累计改动。若既无本地变更也无累计发布，则说明本轮没有新提交，并按收据说明远端同步结果。
- 工作区最终状态；只有状态干净且实际 push 目标的 ahead / behind 为 `0 0` 才报告同步完成。upstream 与 push 目标不同时，禁止以上游一致代替推送完成。
- 失败时给出原始错误阶段和保留的现场状态。

## 停止条件

- diverged、缺 upstream 或无法可靠判定远端状态时，停止并由主对话决定。
- `pull --ff-only` 失败时停止，不改变历史。
- `stash pop` 冲突时保留 stash 和冲突现场，交给用户处理。
- push 失败时重新 fetch 并判定一次；若出现竞态或分叉，停止，不强推。
- pre-commit 或衍生产物刷新失败时停止，不绕过检查。

## 禁止事项

- 禁止自动 rebase、merge、`reset --hard`、force push 或丢弃 stash。
- 禁止在 `0/0` 且无变更时创建空提交。
- repo-local 确定性脚本存在时，禁止主 agent 把 fetch、add、commit、push 拆成手工 Git 命令，或把脚本执行再次委派给其他 agent。
- 禁止脚本或其他 agent 处理分叉、冲突或失败后的决策；这些决策始终留在主对话。
- 禁止用文件清单代替代码变动效果，或因提交后 diff 已清空而省略成功收据中的效果摘要。
- 禁止只说“已同步”而不提供最终干净状态和 `0 0` 收据。

$ARGUMENTS
