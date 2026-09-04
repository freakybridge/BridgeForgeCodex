---
name: sync-docs
description: 根据当前 Git 代码变更同步对应设计文档，并核对项目源码到文档的映射；用户调用 /sync-docs、$sync-docs，或要求让设计文档与本轮实现保持一致时使用。
user_invocable: true
argument: 可选的改动重点或额外上下文
---

# sync-docs — 同步设计文档

## 定位与边界

依据真实代码 diff 更新既有设计文档。源码到文档的可证明关系由 BridgeForge 自动生成到 `.runtime/bridgeforge-codex/sync-docs.map.md`；skill 只定义通用同步流程与未命中 fallback。

## 输入

- `git diff --stat HEAD`、`git status` 与必要的具体 diff。
- BridgeForge 自动生成的 `.runtime/bridgeforge-codex/sync-docs.map.md`（可用时；禁止加入 Git）。
- `$ARGUMENTS`：用户指定的重点。

## 核心流程

1. 先运行当前平台的受管入口：Windows 使用 `.codex\bin\bridgeforge-hook.exe project-map ensure-current`，其他平台使用 `.codex/bin/bridgeforge-hook project-map ensure-current`。成功路径无输出；入口缺失或失败时跳过 Map，继续文档搜索 fallback，禁止要求用户维护 Map。
2. 读取 Git 状态与 diff，确定本轮实际修改文件和行为变化。
3. 把 diff-to-document location 显式分派给 `light-explorer`，由它只读映射文件和候选文档：
   - 命中映射时，按表定位设计文档。
   - 文件不存在或路径未命中时，依据路径和变更内容寻找最相关的既有文档。
4. 主 agent 读取候选收据和目标文档原文，只更新与代码实质变化对应的部分：新增或删除的对象、字段、接口、行为和可由代码证实的设计决策。
5. 再核对一次文档陈述与实际 diff，保留无关内容不动。

## 输出与收据

- 列出每个已更新文档及对应代码变化。
- 列出未找到目标文档的源码路径。
- Map 未命中时说明本次使用了文档搜索 fallback，不向用户提出 Map 维护动作。

## 停止条件

- 找不到对应既有文档时，报告源码路径并停止该项，不自行新建设计文档。
- 无法从代码证实行为或设计决策时，标为未验证，不写入文档。

## 禁止事项

- 禁止添加代码中不存在的行为、接口或结论。
- 禁止改动与本轮 diff 无关的文档内容。
- 禁止因零散、无规律的变更提醒建立映射。
- 禁止要求用户创建、填写或修复自动生成的 Map。

$ARGUMENTS
