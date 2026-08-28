<!-- BRIDGEFORGE:README:BEGIN -->
## BridgeForge 项目协作说明

> 本区由 bridgeforge-codex 管理。项目自己的介绍、安装方法和业务说明放在标记外；骨架更新只替换本区。

这是一份“遇到事情去哪里”的使用说明。强制红线由 `AGENTS.md`、对应 Skill 和机器硬闸承载；这里负责把它们讲清楚。

### 指令和说明分别放在哪里

| 你要找的内容 | 位置 |
|---|---|
| 全项目红线 | 根 `AGENTS.md` 的 BridgeForge 公共区 |
| 项目特有红线和工作地图 | 根 `AGENTS.md` 的“项目级专区” |
| 某个目录的红线 | 该目录的嵌套 `AGENTS.md` |
| 一次主动操作的完整流程 | 项目 `.codex/skills/<name>/SKILL.md`，或当前会话提供的用户级 Skill |
| 可自动判断的规则 | Hook、pre-commit 或测试 |
| 原理、参数、案例和历史 | `doc/README.md` 指向的专题 |

“项目级专区”固定保留项目架构、业务与安全、目录地图、快速命令和嵌套 `AGENTS.md` 索引。

项目专区、项目自有的嵌套指令、Hook 处理器和提交前扩展都属于项目。同步时会逐字保留，不覆盖、不删除、不重新格式化。

公共区由 Hook 和 pre-commit 同时检查工作树与已暂存版本。暂存后恢复工作树不能绕过检查。

常用入口按用途分组：

- 确认与方案：`$confirm`、`$develop`、`$plan`、`$collab`、`$debate`。
- 记录与查找：`$summary`、`$todo`、`$find-doc`、`$archive-scan`。
- 升级与接续：`$escalate`、`$snapshot`、`$resume`、`$git-sync`。

### 文档怎么放

`doc/README.md` 是文档总入口，其中的 `delivery_layout` 记录当前真实交付布局：

| 目录 | 放什么 |
|---|---|
| `0_architecture/` | 当前架构、关键接口、数据流和长期决策 |
| `1_delivery/` | 尚未验收的需求、计划、实现和验收记录 |
| `2_bugs/` | 未关闭 Bug 的现象、根因、修复和证据 |
| `3_reference/` | 操作参考、外部资料和可复核样例 |
| `4_archive/` | 已完成、已替代或只供追溯的材料 |

一个主题只有一份文档时直接放入对应层；需要 README、辩论、证据等多份材料时，建立专题目录，例如 `doc/2_bugs/BUG-<topic>/README.md`。完成事项通过 `$archive-scan` 检查和归档。

### 项目 Memory 怎么用

| 类型 | 位置 | 用途 |
|---|---|---|
| 项目 Memory | `.codex/memory/` | 随项目 Git 保存项目经验 |
| Codex 原生 memories | `~/.codex/memories/` | Codex 用户级记忆能力 |

两套 Memory 的隔离和强制检索顺序见根 `AGENTS.md`；这里仅说明两类 Memory 和常用入口。

沉淀本轮成果使用 `$summary`；它每次更新一个当前主 Memory。用户明确说“同意验收”表示结算当前交付。

模块 Memory 记录模块长期怎样工作；Topic Memory 记录一次独立交付为何做、做到哪里、是否关闭。Topic 适用于用户已确认、同时具有独立目标、独立验收和独立生命周期的交付；普通子任务留在当前主 Memory。完成或废弃的 topic 保留原目录，并进入 `MEMORY_COLD.md`。

### 第一次 clone、换机或重建环境

1. clone 仓库并进入项目根。
2. 按根 `AGENTS.md` 的环境红线建立项目 `.venv`。
3. 从版本控制内的依赖清单安装依赖；Windows 使用 `.venv/Scripts/pip.exe`，POSIX 使用 `.venv/bin/pip`。
4. 核验项目测试、Git Hook、UTF-8 编码和“项目快速命令”。
5. 根据受版本控制的示例重建机器特定配置；真实凭据留在版本控制外。

Python、Node 和 Rust 的可复现入口通常分别是项目依赖清单、`package.json`、`Cargo.toml` 和 `rust-toolchain.toml`。可重建环境的合格状态是：项目运行所需的关键配置不只存在用户目录，依赖清单不含本机绝对路径。项目移动或改名后应重建 `.venv`，避免继续使用旧绝对路径。

### 业务版本和骨架版本不是一回事

根 `VERSION` 记录业务版本；语言原生清单（manifest）由业务发布流程同步。`.codex/.bridgeforge_codex_version` 只记录安装到项目里的骨架版本。

初始化、接管或更新骨架时调用 `$bridgeforge-codex`。它会先说明准备修改什么和是否需要确认，最后用白话报告是否完成。完整操作合同以该 Skill 为准。

### 跨大版本升级先做小实验

根 `AGENTS.md` 规定了何时需要这个实验和放行条件。本节只解释做法：限定 2–4 小时，使用新版依赖和最小代码，与当前实现做截图、benchmark 或可复核的体验对比，并记录改善了什么、还有什么没有改善。放行证据是用户确认目标体验确实改善；没有改善时保留实验结论并停止。

实验只证明核心诉求，不替代正式迁移、回归测试或用户验收。
<!-- BRIDGEFORGE:README:END -->
