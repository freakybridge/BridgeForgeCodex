# AGENTS 第六版独立复评

> 状态：已收敛；第六版不通过，已形成第七版输入
> 需求卡：`../requirements_2026-08-27_agents-iterative-validation.md`

## 议题

1. 改动后是否足够精简，且每类决定只有一个权威 owner。
2. 文本是否足够人类友好，首次读者能否快速找到规则、流程、文档、Memory 和同步入口。
3. 相比现行骨架是否存在语义损失，包括强度、触发条件和例外。
4. proposal 的自动静态验证是否能可信证明候选闭合。

## 本轮候选

- 工厂根 120 行、53 个项目符号；Template 115 行、38 个项目符号。
- `semantic-contract.json` 逐项绑定现行来源行、候选目标文件和必需语义。
- region 合同默认 `fail-closed`，只有 README 显式 `missing_marker: append`。
- 完整 Skill 由唯一锚点 patch 生成并校验 hash；临时工厂运行真实 root-skill 测试和分发 manifest rebuild/check。
- 候选工厂 instruction Hook 消费三个嵌套 `AGENTS.md`，并验证工作树与 staged blob。

## 自动收据

```text
proposal-contract: PASS
assertions: schema-3 parser with asset-level region policy, README append and non-README fail-closed, structured blockers, real sync plan/apply/no-op/rollback, deterministic complete Skill plus real tests/manifest, full factory worktree/staged instruction gate, executable semantic IDs, pointer migrations, unique owners, links, UTF-8 no BOM, size gates
```

## 第一轮

- 评审 A 判定精简 PASS，其余三项 FAIL。确定漏掉 Spike 的 2–4 小时/新版最小代码/同现状对比/用户确认改善、主观体验的“禁止猜修/能否保存现场”、每项目必须自建 `.venv`、自改审计精确触发与轻查例外。
- 评审 B 同样只判精简 PASS。另发现 gap/degraded、`--check` / `--dry-run` 零写丢失；append 权限未锁死到 README；staged Template 可绕过；嵌套文件只保留标题与 scope token 就能过；零写与回滚未比较完整项目树；旧指针扫描只扫声明清单。

## 第二轮

两方交叉质询后完全收敛：

1. 根文本补回四类精确语义，并拆开多判断复句；Template 长度门放宽到 125，禁止为守数字再次合句。
2. gap/degraded 和 release preflight 归完整 `$bridgeforge-codex` Skill；`--check` / `--dry-run` 零写归 `scripts/AGENTS.md`。
3. parser 只能允许 README 资产声明 append；非 README 必须在读合同时失败。
4. 生成并从候选路径加载完整 baseline 与同步器，禁止 runtime monkeypatch。
5. 工厂根项目区、Template、三个嵌套文件必须在工作树和 staged 中校验全文 hash，并覆盖“保留 token 删除规则”和“stage 后恢复工作树”旁路。
6. plan、阻断、no-op 和回滚必须比较完整项目树。
7. 旧指针扫描完整候选活跃面；新增 CHANGELOG 当前头部、antifabrication 设计和未归档交付指针迁移。
8. 语义合同锁定来源文件 hash，每条红线恰好映射一次，高风险项用关系断言；root-skill 测试必须明确断言新增 Skill 语义。

## 四项判定

| 议题 | 结论 |
|---|---|
| 足够精简 | PASS |
| 人类友好 | FAIL |
| 无语义损失 | FAIL |
| 自动静态验证可信 | FAIL |

第六版淘汰，不得作为实施输入。
