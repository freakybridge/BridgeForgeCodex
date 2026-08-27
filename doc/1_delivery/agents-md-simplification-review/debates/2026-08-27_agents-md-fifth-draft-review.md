# AGENTS 第五版独立复评

> 状态：已收敛；第五版不通过，已形成第六版输入
> 需求卡：`../requirements_2026-08-27_agents-iterative-validation.md`

## 议题

1. 改动后是否足够精简，且同步规则只有一个权威 owner。
2. 文本是否足够人类友好，首次读者能否在五分钟内复述入口。
3. 相比现行骨架是否存在语义损失。
4. proposal 是否已用真实骨架链路证明“自动静态候选通过”。

## 已有收据

- `contracts/validate_proposal.py`：真实 schema 3 parser/payload、真实 instruction Hook 入口、真实同步器 plan/apply/no-op、注入失败回滚、README 字节保留与非法 marker 失败关闭、指针迁移、factory-only hash、现行红线账本覆盖、唯一 owner、语义 sentinel、链接、BOM 和长度门均通过。
- `git diff --check`：通过。
- `project_structure_check.py --json`：`errors=[]`；仅有任务外既存归档提示。

## 第一轮

- 评审 A 发现 region 迁移器被全局替换后，会让无 marker 的 pre-commit 从失败关闭变成自动追加；验证器只测 README，存在确定假阳性。A 同时指出 Skill 只有文字 delta、工厂镜像没有真实 consumer。
- 评审 B 发现 clarify 漏了“可选路线”，并完全漏掉 `$develop` / 评估咨询分流；迁移账本只检查来源行被范围覆盖，不检查目标语义，因此“无语义损失”结论不成立。
- 两方都认为人类友好仍有复句、模糊指代、术语首次未解释和 README 重复红线。

## 第二轮

交叉质询后，两方一致把以下内容列为第六版硬门：

1. 只有 README 资产允许缺 marker 追加，其他 region 必须保持失败关闭；非法 marker 必须经真实同步器入口给出结构化阻断并保持零写。
2. 用确定性 patch 生成完整候选 Skill，校验完整 hash，并运行真实 root-skill 测试和正式分发 manifest 检查。
3. 临时构建完整候选工厂；三个嵌套 `AGENTS.md` 必须被真实工厂 gate 消费，并覆盖工作树与 staged blob。
4. 用“来源语义 ID -> 目标文件 -> 必需语义”代替只圈行号的迁移证明。
5. 恢复可选路线、`$develop` 分流、Template 必填、项目资产逐字保护、工厂 `.codex/**` 职责和动态 `G*` 等漏项。
6. 删除同步器不存在的 `--check` / `--dry-run` 宣称，只验证“不带 `--apply` 的 plan”零写。

## 四项判定

| 议题 | 结论 |
|---|---|
| 足够精简 | FAIL：行数通过，但 owner 与候选 Skill 尚未闭环 |
| 人类友好 | FAIL |
| 无语义损失 | FAIL |
| 自动静态验证可信 | FAIL：机器脚本返回 PASS，但存在覆盖漏洞 |

第五版因此淘汰，不得作为实施输入。
