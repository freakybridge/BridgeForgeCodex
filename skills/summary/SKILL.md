---
name: summary
description: 将当前对话开发/debug 的经验教训沉淀到项目 doc；switch 额外保存换机交接，accept 额外记录验收并更新 TODO。用户调用 /summary、$summary 或要求沉淀本轮成果时使用。
user_invocable: true
argument: 无参数、switch 或 accept
---

# summary — 经验沉淀、换机交接与验收

## 模式与授权

| 调用 | 写入范围 |
|---|---|
| `$summary` | 当前主题的经验文档与必要索引；不验收、不结算 TODO |
| `$summary switch` | 同上，并保存可随仓库同步的工作交接文档；不验收、不结算 TODO |
| `$summary accept` | 同上，并记录当前交付验收、更新相关已有 TODO、补入明确发现但尚未解决的问题 |

只有精确参数 `accept` 表示用户确认开发符合需求；已知 blocker 或缺失必要证据仍不得记成完成。
旧参数 `同意验收` 和其他参数只提示以上三种用法，不写入。用户明确要求只读时遵从该限制。
总结、验收、采纳规则和 Git 发布分别判断；三模式都不暂存、提交、推送，不自动调用 git-sync。

## 执行路径

1. 以当前可见对话及相关既有文档和收据为依据，区分事实、推断、已完成和未验证。不为总结重新编译、运行完整测试、审计或 smoke；必要时只读核对 Git 与相关文件。
2. 三模式都读取[经验沉淀流程](references/ordinary-mode.md)，实际新增或更新 doc，必要时更新 doc/README.md。同一事项优先补充已有文档，重复调用只补新事实。
3. 按[记忆阅读、建议与归档流程](references/deep-steps.md)做有界检索和证据核对；Memory 不可用不阻止沉淀当前对话。规则和归档仅列有依据的候选，不自动实施。
4. `switch` 额外读取[换机交接](references/switch-mode.md)；`accept` 额外读取[验收与 TODO](references/acceptance-mode.md)。
5. 返回实际写入文档的链接、关键经验、未验证边界和必要后续动作。没有新增有效经验或已被文档覆盖时给出具体原因及已有链接，不能把聊天回复称为已落盘。

## 边界

- 不修改业务代码、Rule、AGENTS、Hook、配置、测试或 Memory；不自动归档、移动或删除历史文件。
- 经验记录不能补造需求确认、验证证据或开发授权；不把历史成功收据当本轮验证通过。
- 不创建或使用项目 .codex/memory/，不直接写原生 ~/.codex/memories/。
- 规则候选标记“等待用户采纳；未写入；未实现”；文档已沉淀不等于规则已采纳。
- 项目启用发布准备时，按其声明区分记录类文档与开发输入。仅允许的文档变化可通过轻量文档检查复用产物；其他变化如实报告失效，交回 develop，不在 summary 自动补跑长检查。

$ARGUMENTS
