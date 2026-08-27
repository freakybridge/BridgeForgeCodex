# AGENTS 第八版独立复评

> 状态：未通过，已进入第九版
> 需求卡：`../requirements_2026-08-27_agents-iterative-validation.md`

## 议题

1. 足够精简，且每类决定只有一个权威 owner。
2. 人类友好，每条规则只有一个主要判断，首次读者可快速找到入口。
3. 无语义损失，包括主体、强度、触发条件、顺序和例外。
4. 自动验证可信，所有结论不超过实际覆盖范围。

## 本轮候选

- 工厂根 126 行、59 个项目符号；Template 120 行、43 个项目符号。
- 补回性能测量、失败升级、换机配置、focus、版本域、Memory、项目占位和 Spike 的精确语义。
- 高风险语义分别绑定主体、强度、触发和顺序；最终 Skill 文本直接接受关系检查。
- marker blocker 通过真实子进程 CLI；五个工厂受保护面各自执行 worktree/staged 破坏测试，并另测嵌套文件删除。
- 快照明确称“受管可见树”，排除 `.git/.venv/__pycache__`；hash 明确称“规范化全文 hash”。
- 全命令零写矩阵、全部历史指针、真实下游、release suite、runtime 和用户试读仍明确列为实施后验证。

## 自动收据

```text
proposal-contract: PASS
assertions: hashed candidate baseline/syncer loaded from candidate paths, README-only append policy, subprocess CLI structured blocker, managed-visible-tree plan/no-op/rollback proof excluding .git/.venv/__pycache__, deterministic complete Skill plus explicit tests/manifest, five-surface normalized-content worktree/staged gate, source-hashed exact-once root/Template mapping with bound relations, enumerated active-surface pointer scan derived from migrations, unique owners, links, UTF-8 no BOM, size gates
```

## 第一轮

### 评审 A

- 构造了语义验证反例：把正确 venv 句藏进 HTML 注释、正文改成相反意思后，语义检查仍可 PASS。
- 确认 README 与根重复文档、环境和 Spike 命令红线；Memory 读取顺序可被理解为跳过主索引；双平台 pip 精确入口丢失。
- 判定过密复句仍使唯一 owner、精简和人类友好不能放行。

### 评审 B

- 构造了更高优先级实现反例：候选同步器处理非 README region 时会把 marker 外 CRLF 归一化为 LF，prefix/suffix 都不再逐字相等，而验证器仍 PASS。
- 另确认执行类任务丢“读上下文、判断风险”，后台入口丢“可验证”，新增嵌套指令登记义务和“填好删注释”丢失。

## 第二轮收敛

- 双方一致：第八版不能放行。
- 第九版必须改用 raw span 保留非 README marker 外字节，并以 CRLF/LF/混合换行/无尾换行反例验证。
- 普通规则检查必须剥离 HTML 注释和代码块；项目占位注释只能显式例外。
- README 退回解释层；命令强度留在 AGENTS/Skill。Memory 严格顺序和双平台 pip 入口必须恢复。
- 根文档规则与工厂事务规则必须拆开；新增嵌套登记、执行五步、可验证后台入口和占位注释删除义务必须恢复。

## 四项结论

- 精简与唯一 owner：不通过。
- 人类友好：不通过。
- 无语义损失：不通过。
- 自动验证可信：不通过。
