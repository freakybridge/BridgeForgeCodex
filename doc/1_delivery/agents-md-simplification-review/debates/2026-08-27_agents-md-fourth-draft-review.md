# AGENTS 第四版独立复评

> 状态：进行中
> 需求卡：`../requirements_2026-08-27_agents-iterative-validation.md`

## 议题

1. 改动后是否足够精简。
2. 文本是否足够人类友好。
3. 相比现行骨架是否存在语义损失。
4. proposal 的机器合同和临时模拟是否足以支持“静态候选通过”。

## 评审对象

- `doc/2_bugs/BUG-skeleton-comprehensibility-and-information-architecture-debt/proposal/**`
- 现行 `AGENTS.md`、`templates/AGENTS.md`、相关 Hook、Skill、manifest 和操作指南只作为对照。

## 第一轮

### Agent A（语义与机器合同）

- 四项均不通过。`strategy: zones` 不符合真实 manifest schema，候选合同不能直接被现行 baseline parser 读取。
- Hook/脚本中的旧章节和旧操作指南指针迁移不完整；验证器没有消费 pointer migrations。
- 现行动态 `G*` 收据被错误改成固定 `G0–G5`，属于语义改变。
- 公共区 staged 防绕过、跨版本升级禁先改 lockfile 等规则丢失。
- 版本戳最后写同时出现在根、README、Skill 和 scripts 嵌套，唯一 owner 仍未成立。
- README 无 marker 时 `rstrip` 会修改项目原有尾部空行，现有 fixture 是自证。

### Agent B（首次读者与可实施性）

- 精简通过，其他三项不通过。冷读可以复述整体路由，说明结构方向成立。
- 空泛安抚/模糊建议、危险处真实证据和“不知道”、文档禁散落/禁改五层、跨版本升级禁令、Skill 位置与发现方式存在语义遗漏。
- README 同步段仍像内部协议，`G0–G5` 未解释且与真实 Skill 的用户收据边界冲突。
- 验证器没有验证真实 manifest、同步器、Hook 注册、staged/worktree、旧指针和迁移账本。
- README region 在无 marker fixture 中没有逐字保留尾部空行。

### 第一轮结论

第四版不通过。保留四分法结构，进入第二轮交叉质询，收敛第五版的最小修复和可验证合同。

## 第二轮

### 交叉收敛

- 两名评审统一修正为：第四版四项均不通过。长度与章节已达到合理区间，但单一 owner 仍失败，不能把行数 PASS 当整体精简 PASS。
- 第五版不再继续砍根文件；只定点恢复语义、删除 README/嵌套重复，并把机器合同改成真实 schema 3。
- 根只保留同步入口和版本域所有权；`$bridgeforge-codex` 唯一规定 plan/apply、fingerprint、gap、回滚、收据和版本戳顺序；`scripts/AGENTS.md` 只要求实现与测试保持 Skill 合同。
- README 同步说明只保留“何时调用、会先说明改什么、需要时确认、失败恢复、白话报告”；删除 plan/apply、fingerprint、gap、ready/degraded、版本戳顺序、classifier 和技术收据。
- 动态 `G*` 不得改成固定 `G0–G5`。
- 候选 manifest 必须使用真实 `strategy: whole + agents_zones`、`region`、`whole` schema，并由真实 `current_baseline` 解析和核验。
- README 首次新增 region 必须是显式迁移：禁止 `rstrip`，原文件必须保持字节前缀；单 marker、重复和逆序零写，二次执行 byte-identical。
- 旧操作指南和旧章节指针必须覆盖 clarify、focus、fallback、test receipt、requirements、audit、索引和测试；验证器必须消费 pointer migrations、factory-only files 和语义账本。
- 恢复表达约束、危险处真实证据/不知道、文档禁散落/禁改层、Skill 发现路径、跨版本升级禁先改主项目/lockfile 与禁只看 CHANGELOG。

### 第四版最终结论

不通过。进入第五版自动修订，完成后换新评审者重新 debate。
