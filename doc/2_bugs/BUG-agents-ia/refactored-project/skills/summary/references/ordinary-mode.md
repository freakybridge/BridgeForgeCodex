# summary 普通模式

> `$ARGUMENTS` 为空时必读。普通模式只汇总阶段进展，不结算交付，零写入。

1. 依据当前上下文、完整可见对话和已有事实源，按“目标、决策、已完成、验证、未验证、
   blocker、下一步”整理；禁止重新运行测试、build、审计或 smoke。
2. 对事实与推断分栏；冲突信息保留冲突，不替用户选择隐藏结论。
3. 普通模式禁止修改 TODO、AGENTS、Rule、Hook、需求、设计、计划、Bug、其他文档、项目
   `.codex/memory/` 或原生 `~/.codex/memories/`。
4. 发现 Rule / Hook 候选时只按 `deep-steps.md` 输出建议；没有稳定证据就不建议。
5. 发现已完成 delivery 或 Bug 时只列归档候选；禁止自动调用 `$archive-scan`。

输出必须明确：`阶段总结；零写入；候选均未执行`。
