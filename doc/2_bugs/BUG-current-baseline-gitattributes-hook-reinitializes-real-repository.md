---
status: source-fixed-awaiting-release-and-downstream-recovery
severity: critical
scope: downstream pre-commit current-baseline gitattributes validation
reported_at: 2026-08-27
factory_head: 734b9544bd2e27884a655a6808c32bf518ed065a
product_version: 1.5.6
downstream: D:\Quant\CodexWorktree\1d62\StratusAgent
affected_common_git_dir: D:\Quant\StratusAgent\.git
---

# BUG：pre-commit 行尾校验把真实共享仓库重置为 bare

## 结论

BridgeForgeCodex `1.5.6` 新增的 `.gitattributes` 默认 LF 校验会在
`$git-sync` 的真实 pre-commit 生命周期中继承调用进程携带的仓库环境变量。校验器原本要在临时
目录初始化隔离仓库，却没有为 `git init` 传入隔离环境，导致该命令命中真实下游的 common
Git dir，并把共享仓库配置写成 `core.bare=true`。

结果不只是一次提交被 fail-closed：当前 checkout 及所有共享该 common Git dir 的 worktree
都会失去 worktree 身份，`git status`、`git diff` 和后续标准提交均无法运行。`$git-sync`
虽然报告自动写入和原 Git index 已恢复，但 common Git config 不在当前事务回滚边界内，损坏会
留在现场。

这是产品级、跨下游的共享 Git 元数据破坏缺陷。当前不得直接重试 `$git-sync`，不得跳过 hook，
也不得把手工 `core.bare=false` 当作产品修复。

## 用户可见影响

- `$bridgeforge-codex` 可以把下游成功升级到 `1.5.6`，no-op replan 也能通过。
- 用户随后运行官方 `$git-sync` 时，pre-commit 首先报告默认 LF 策略缺失或被覆盖。
- 失败后 Git 不再把当前目录识别为 worktree；常规状态检查直接报错。
- 共享 common Git dir 的主 checkout 和其他 linked worktree 可能同时受影响。
- 本次真实事故发生在 commit 和 push 前，业务文件与 8 个骨架变更仍在，远端没有新增提交。
- 只把 `core.bare` 改回 `false` 可以恢复仓库身份，但未修复校验器；再次提交仍可能复发。

## 真实下游复现

### 环境

- 工厂仓库：`D:\Quant\BridgeForge`
- 工厂 HEAD：`734b9544bd2e27884a655a6808c32bf518ed065a`
- 产品版本：`1.5.6`
- 真实下游：`D:\Quant\CodexWorktree\1d62\StratusAgent`
- 下游分支 / upstream：`codex/m2` / `origin/codex/m2`
- 下游 common Git dir：`D:\Quant\StratusAgent\.git`
- 下游骨架版本：`1.5.6`
- 工厂 Template、工厂 dogfood 与真实下游三份 `current_baseline.py` SHA-256：
  `86C1648F0C7142E2EB0FAFEC62F36C1DBC160D8EFD4C746C1A3EB38115105A22`

### 命令

在已完成 `1.5.4 -> 1.5.6` 骨架升级、尚未提交的真实下游运行官方入口：

```powershell
.venv\Scripts\python.exe .codex\scripts\codex_git_sync.py --message "chore: 升级 BridgeForge Codex 骨架至 1.5.6"
```

### 原始失败收据

```text
[git-sync] git commit failed: [current-baseline] BLOCKED: .gitattributes default LF policy is missing or overridden
[git-sync] automatic writes and the original Git index were restored
```

没有改走手工 `fetch/add/commit/push`，没有跳过 pre-commit，也没有 reset、rebase、force push
或 ACL 修改。

## 失败后的现场证据

### 1. 真实共享配置被改为 bare

`D:\Quant\StratusAgent\.git\config` 当前包含：

```ini
[core]
    bare = true
```

而 `$git-sync` 前同一 worktree 的 `git status --short --branch` 能正常返回
`codex/m2...origin/codex/m2` 与 8 个骨架变更，证明提交前有效状态不是 bare。

### 2. worktree 身份丢失

失败后只读检查：

```text
git rev-parse --is-inside-work-tree
false

git status --short --branch
fatal: this operation must be run in a work tree
```

`git rev-parse --git-dir` 仍指向
`D:/Quant/StratusAgent/.git/worktrees/StratusAgent1`，说明 Git 管理目录仍存在，问题是共享
repository mode 被改变，不是工作目录或提交对象被删除。

### 3. 行尾规则因此完全不可见

对四个校验探针执行 `git check-attr text eol` 后，`text` 与 `eol` 均返回
`unspecified`。这与 pre-commit 的“默认 LF 策略缺失”报错一致，但它是仓库已被切成 bare 后的
次生结果，不代表工作树里的 `.gitattributes` 缺少 `* text=auto eol=lf`。

### 4. 远端没有新增提交

- 失败后的 HEAD：`aa424847aa1a6c281261ea48e63ec89559058727`
- HEAD / upstream：`0 0`
- commit / push：均未发生

## 源码根因

`templates/scripts/current_baseline.py::_gitattributes_default_state()` 的执行顺序是：

1. 创建 `TemporaryDirectory`；
2. 写入临时 `.gitattributes`；
3. 直接运行 `subprocess.run(["git", "init", "-q"], cwd=root, ...)`；
4. 直到 `git init` 完成后才执行 `environment = os.environ.copy()`；
5. 该 `environment` 只传给后续 `git check-attr`。

因此 `git init` 完整继承调用进程环境。事故现场的 hook 进程环境包含 `GIT_DIR`、
`GIT_WORK_TREE`、`GIT_INDEX_FILE`、`GIT_COMMON_DIR` 等仓库本地变量；仅改变 `cwd` 不能隔离
这些变量，继承的 `GIT_DIR` 优先于临时目录，使所谓的隔离 `git init` 写入真实 common Git
dir。回归同时确认普通 Windows Git hook 不保证自动导出 `GIT_DIR`，所以根因是“未隔离调用
环境中的仓库变量”，不是依赖某一种 hook 启动实现。

Template、dogfood 与真实下游安装副本字节一致，因此这不是 StratusAgent 本地定制，也不是下游
同步漂移。

## 事务缺口

`codex_git_sync.py` 能恢复它自己计划内的工作树写入和原 Git index，但没有把共享
`<common-git-dir>/config` 纳入事务快照。于是 pre-commit 子进程对 Git config 的越界写入不会被
检测或回滚，最终收据中的“original Git index was restored”不能解释为仓库整体已恢复。

## 推荐修复

### 1. 从根源隔离临时 Git 子进程

在执行任何临时仓库命令前构造一份统一的 isolated Git environment：

- 从环境中移除 Git 报告的全部 repository-local 变量，至少包括 `GIT_DIR`、
  `GIT_WORK_TREE`、`GIT_INDEX_FILE`、`GIT_PREFIX`、`GIT_COMMON_DIR`、
  `GIT_OBJECT_DIRECTORY` 与 `GIT_ALTERNATE_OBJECT_DIRECTORIES`；
- 保留当前 `GIT_ATTR_NOSYSTEM=1` 与空 global attributes 约束；
- 同一份隔离环境必须同时传给 `git init` 和 `git check-attr`；
- 禁止只清一个已知变量，避免 linked worktree、quarantine 或 alternate object 环境再次穿透。

优先把该逻辑收口为可测试的单一 helper，供今后所有临时 Git 仓库调用。

### 2. 增加共享 Git config 越界防线

`$git-sync` 在进入 commit / hook 前后应核对 repository identity：common Git dir、
`core.bare`、worktree identity 与目标分支必须保持不变。任何漂移都要给出独立的高严重度错误，
不能被普通“index 已恢复”收据掩盖。

该防线是事故检测，不应替代第 1 项隔离修复；禁止静默把任意用户配置强制重写为默认值。

### 3. 受影响下游恢复顺序

1. 发布包含隔离修复的新 BridgeForgeCodex 版本；
2. 再次核对 common Git dir、原普通 worktree 证据与当前 `core.bare=true`，取得窄范围确认后
   受控恢复 `core.bare=false`；
3. 核对所有 linked worktree、工作区、index、HEAD、upstream 与 common Git config，确认 M2
   原有改动仍在；
4. 下游重新运行 `$bridgeforge-codex` 安装修复副本；
5. 再运行官方 `$git-sync` 完成原骨架提交，并复核所有 worktree 与远端同步状态。

禁止在仍安装 `1.5.6` 缺陷副本时直接重试提交。

## 回归与验收标准

1. 新增真实 pre-commit 回归：在临时普通仓库和 linked worktree 中安装产品 hook，通过真实
   `git commit` 触发两次 current-baseline 检查。
2. 测试必须在 hook 环境中确认 `GIT_DIR` 等变量确实存在，并证明临时 `git init` 未写入原仓库。
3. commit 前后 common Git config 字节一致，`core.bare=false`，两个 worktree 均能正常执行
   `git status`。
4. `.gitattributes` 的默认 `text=auto/eol=lf`、项目例外、嵌套路径与 attribute macro
   覆盖语义继续通过现有回归。
5. 增加反例：显式注入 `GIT_DIR`、`GIT_WORK_TREE`、`GIT_INDEX_FILE`、alternate object 与
   quarantine 变量时，隔离仓库仍不得触碰 sentinel repository。
6. `$git-sync` 的失败事务测试必须区分“工作树/index 已恢复”与“repository identity 漂移”；
   不得输出会让用户误以为整个仓库已恢复的收据。
7. Template 与 dogfood 副本、两份 managed contract、manifest、VERSION 与 CHANGELOG 同步，
   CHANGELOG 标记 `[product]`。
8. 完整 unittest、downstream fixture、factory dogfood、manifest、mirror、instruction、skill
   metadata、project structure 与 `git diff --check` 全部通过。
9. 独立审计必须复核临时 Git 隔离边界、linked-worktree 共享配置、事务回滚与错误语义。
10. 修复发布后，在真实 `D:\Quant\CodexWorktree\1d62\StratusAgent` 恢复并重新执行
    `$bridgeforge-codex -> $git-sync`；只有 clean worktree、push 成功且 HEAD/upstream 为 `0 0`
    时，才可关闭本 Bug。

## 六类证据

| 类别 | 当前状态 |
|---|---|
| 源码 | 已修复；临时 `git init` 与 `git check-attr` 共用隔离环境，project-sync 同型入口一并修复 |
| 产品传播 | Template、dogfood 与两份 managed contract 已同步；修复版尚未发布到真实下游 |
| dogfood | 工厂镜像与 Template 字节一致；工厂正式 `$git-sync` 尚未执行 |
| fixture | 核心 81 项测试通过；真实 checkout/linked-worktree pre-commit 各提交一次且 common config 字节不变 |
| 真实下游 | StratusAgent linked worktree 已稳定复现，并留下 `core.bare=true` 现场 |
| runtime | 缺陷版真实 `$git-sync` 已复现；修复版 StratusAgent 运行时收据尚未取得 |

## 传播四问

1. 层级：产品层；主修 `templates/scripts/current_baseline.py`，同步 dogfood 副本与受管合同。
2. 通用性：影响所有通过 Git hook 调用该默认 LF 校验的下游，必须下沉 Template，不能做
   StratusAgent 特例。
3. 版本与 CHANGELOG：必须 bump 根 `VERSION`，并在 `CHANGELOG.md` 标记 `[product]`。
4. dogfood：必须同步 `.codex/scripts/current_baseline.py` 并用真实 pre-commit 流程验证，而不只
   直接调用 Python 函数。

## 范围与非目标

- 本报告只记录缺陷、恢复边界与验收要求，不实施源码修复。
- 不修改 StratusAgent 业务文件、Git 历史、分支、index 或远端。
- 不把跳过 hook、手工 commit、删除 `.gitattributes`、关闭 LF 校验或固定某个下游绝对路径作为
  解决方案。
- 不宣称受影响下游已经恢复，也不宣称修复版已经通过任何自动或真实下游验收。
