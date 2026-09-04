---
name: find-doc
description: 定位主题相关文档、TODO、当前进展、设计计划、验收方案及关联 rules；用户询问“相关文档在哪、还有什么未解决、现状如何、查一下某主题”时主动使用。精确路径、纯代码搜索或本轮已查过同一主题时跳过。
user_invocable: true
argument: 主题关键词（中英混合，例 "auth oauth" / "数据库 schema"）
---

# 文档综合检索

## 定位与边界

只检索 `doc/` 和 BridgeForge 自动生成的项目指令索引，不扫描源代码、项目 Memory 或原生 Memory，也不先读取完整文档。目标是用少量高信号命中回答“在哪、现状、TODO、设计和计划”。

## 输入

从 `$ARGUMENTS` 或用户问题提取主题：中文按 `/` 和空格分词；英文 `_` 不拆。多关键词用 OR 检索，并增加一路共现交集。

## 核心流程

### 0. 确保项目索引为最新

先运行当前平台的受管入口：Windows 使用 `.codex\bin\bridgeforge-hook.exe project-map ensure-current`，其他平台使用 `.codex/bin/bridgeforge-hook project-map ensure-current`。成功路径无输出。

入口缺失或失败时，不要求用户创建或修复 Map；跳过索引并继续下面的文档搜索 fallback。只有 fallback 也无法完成当前检索时，才报告真正的任务阻断。

### 1. 分流意图

| 意图 | 典型问题 | 执行路径 |
|------|----------|----------|
| 找东西（默认） | 找、在哪、涉及、查、搜、设计、计划 | A + B + D |
| 看待处理事项 | 未解决、没修、bug、todo、进展、现状 | C + D |
| 看交付计划 | Milestone、M1、验收、需求、开发计划 | A + C + D |

边界含混时按“找东西”执行。

### 2. 并行检索

主对话先执行 fast path：精确路径继续按停止条件直接读取；文件名或 README 入口只命中
一个当前文档、没有活跃/归档冲突且用户不在询问 TODO/现状时，直接返回该结果，禁止启动
子 agent。其他情况才把下面的多路径搜索与候选摘要显式分派给 `light-explorer`，并在同一批
工具调用中运行所需路径：

- **Path A—文件名**：每个 token 执行 Glob `doc/**/*<token>*.md`。
- **Path B—README 入口**：在 `doc/**/README.md` 中大小写不敏感 Grep topic regex，返回内容，最多 30 条；命中后再判断是否读取目标文件。
- **Path C—Delivery + Bugs**：
  - 读取 `doc/README.md` 的 `delivery_layout`；
  - Glob `doc/1_delivery/**/<topic>*/**/*.md`，并匹配 `requirements_*.md`、`plan.md`、`acceptance.md`、`debates/*.md`；
  - Grep `doc/2_bugs/**/<topic>*.md`，只返回命中文件，最多 20 个。
- **Path D—多词共现**：仅多 token 时，在 `doc/**/*.md` 中按正反顺序做 multiline 共现 Grep，只返回文件，最多 15 个。

### 3. 查项目指令索引

读取 `.codex/find-doc.map.md` 的 `topic_to_sources` 表：命中主题或代码词时读取对应根/嵌套 `AGENTS.md` 位置；未命中时跳过，不全量扫描指令源。该文件由 BridgeForge 生成，禁止手工编辑。

### 4. 聚合与收尾

1. 聚合去重：A 作基线，D 作高优先级，B/C 调整次序，空段不显示。
2. 命中后读取 [references/output-format.md](references/output-format.md)，按其格式输出。

## 输出与验证

标明每项命中来自文件名、README、TODO 还是自动指令索引；区分进行中、待解决与已归档文档。需要正文结论时，只读用户选定或最高信号的目标文件。

## 停止条件

- 用户已给精确路径：直接读取该路径，不执行本 skill。
- 用户问的是源代码：转为限定源码目录的代码搜索。
- 本轮已检索同一主题：复用结果，除非用户明确要求刷新。
- fast path 唯一命中：主对话直接返回，不进入多路径 fallback。
- 所有路径无命中：说明检索范围和关键词，不编造文档。

## 禁止事项

- 禁止先读完整文件理解上下文；先检索，再按需读。
- 禁止扫描源代码、项目 `.codex/memory/`、原生 `~/.codex/memories/` 或全量 rules。
- 禁止要求用户创建、补充或修复自动生成的 Map。
- 禁止用 `todo`、`resume`、`summary` 的能力替代本 skill；新建文档、跨会话接续和对话总结应转交对应 skill。

输入：`$ARGUMENTS`
