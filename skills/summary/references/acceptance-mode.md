# summary 验收模式

> `$ARGUMENTS` 精确等于 `同意验收` 时必读。该参数只授权收口当前交付，不授权扩大范围。

1. 核对当前交付的验收条件、blocker 和已有收据。任一必要条件未满足或收据冲突，都必须
   保持未完成状态并说明原因。
2. 只更新当前交付已经存在且能唯一关联的需求、设计、计划、Bug 或 TODO 中的验收状态与
   收据；交付或 Bug 的全部必要条件满足时，按 `doc/README.md` 合同写
   `lifecycle: completed`、`validation_status: verified`。缺少必要证据或存在 blocker 时保持
   `lifecycle: active`，并写 `validation_status: awaiting_validation`；禁止为了收口新建文档或猜测路径。
3. 只结算当前交付 TODO；其他 topic、Bug 和项目级 TODO 保持不变。非阻塞后续事项只列为
   候选，等待用户另行决定。
4. `同意验收` 只批准交付收口，不自动采纳 Rule / Hook 建议。建议仍必须标记
   `等待用户采纳；未写入；未实现`，以后由其他开发方法落地。
5. `lifecycle: completed` 的 delivery 或 Bug 只列归档候选并提示“请另行调用 `$archive-scan`”；禁止执行
   `git mv`、移动文档或更新归档索引。

最终写入面只限当前交付既有记录；项目 `.codex/memory/`、原生 `~/.codex/memories/`、
Rule、AGENTS、Hook、配置和测试始终零写入。
