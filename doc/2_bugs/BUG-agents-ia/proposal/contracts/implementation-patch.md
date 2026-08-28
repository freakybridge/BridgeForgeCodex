# 第十一版实施补丁清单

> 这是精确实施输入，不是已经发生的变更。真实骨架仍保持原状。

## 1. 指令 Hook

工厂镜像与 Template 的 `instruction_source_check.py` 同时把 `PUBLIC_REQUIRED_HEADINGS` 替换为：

```python
PUBLIC_REQUIRED_HEADINGS = (
    "## BridgeForge 公共区",
    "## 1 先找对位置",
    "## 2 交付与证据",
    "## 3 环境与安全",
    "## 4 任务控制与排障",
    "## 5 协作与项目资料",
    "## 6 版本与升级",
)
```

`PROJECT_REQUIRED_HEADINGS` 保持现行六项不变。实施时运行真实 Hook 入口，不只调用私有检查函数。

## 2. schema 3 managed contract

- 以真实 `templates/managed-skeleton.json` 为基底，使用 `contracts/instruction-contract.json` 的 `asset_replacements` 生成候选 manifest。
- `root.agents` 保持 `strategy: whole + agents_zones`；不得使用不存在的 `zones` strategy。
- 新增标准 region asset `root.readme.bridgeforge-public` 和标准 whole asset `codex.doc.hook-signals`。README 的 region 显式声明 `missing_marker: append`；其他 region 缺省 `fail-closed`。
- 候选 manifest 必须通过真实 `current_baseline.load_contract()`；三个候选 payload 必须通过 `verify_contract_payload()`。
- 候选 `current_baseline.py` 必须校验 `missing_marker` 只能是 `append` 或 `fail-closed`，并且只有资产 `root.readme.bridgeforge-public` 且目标为 `README.md` 才能声明 `append`；非法值或越权资产在读取合同阶段阻断。

## 3. README 首次安装

- 以唯一函数锚点把 `contracts/region_migration.py` 的行为合入完整候选同步器，并继续使用同步器原有的 `SyncBlocked`；候选完整文件必须匹配 `instruction-contract.json.candidate_implementation.project_sync_sha256`，禁止运行时 monkeypatch。
- 只有资产声明 `missing_marker: append` 时，无 marker 才在 EOF 后追加；`after[:len(before)]` 必须逐字等于原文件。其他 region 继续失败关闭，禁止 `rstrip` 或换行归一化项目内容。
- 单 marker、重复或逆序 marker 必须零写阻断；已有 region 只替换 marker 内部；二次执行必须 byte-identical no-op。

## 4. Skill 与旧指针

### 4.1 `$bridgeforge-codex` 与旧指针

- 按 `contracts/bridgeforge-codex-skill-patch.json` 的唯一锚点生成完整候选 Skill。现行 Skill 已经拥有模式判定、一次确认、plan/fingerprint/apply/rollback/最后写戳和用户结果合同；第十一版只补回 gap/degraded 终态、只读 release preflight、同一 ownership classifier 和动态逐文件 `G*`，不再重复合入一份文字 delta。
- 完全执行 `instruction-contract.json.pointer_migrations`：clarify、focus、fallback、test receipt、requirements、audit、文档索引、架构文档和根 Skill 测试都改用稳定文件/标题指针；Template 与 dogfood 同步。
- 删除 `doc/3_reference/codex-project-operating-guide.md` 及索引项；活跃运行面中的旧指南、`§1.3/§4.4/§5.2/§9.6` 和旧 clarify/focus 标题归零。
- 内部逐文件检查沿用动态 `G*`，不得固定成 `G0–G5`；用户结果继续按现行 Skill 输出白话摘要。

### 4.2 Summary Skill

- 修改 `skills/summary/SKILL.md` 验收模式第 6 步：删除“按路径加载的 Rule 必须带 `paths` frontmatter”，改为按全项目语义、目录语义、机器硬闸、命令权限和说明材料五类路由。
- 普通模式继续禁止修改 AGENTS、Hook、测试和文档；只有 `$summary 同意验收` 且满足长期稳定红线门槛时才允许进入该路由。
- 新建嵌套 `AGENTS.md` 时必须同步根目录索引；目标 owner、作用域或写入权限不唯一时停止并用一个问题请求裁决。
- 只有命令前缀的 allow / prompt / forbidden 可以写入 `.codex/rules/*.rules`；禁止创建 `.codex/rules/*.md`，也禁止把语义 Markdown 机械改名为 `.rules`。
- 同轮更新共享 Skill manifest、兼容入口与安装镜像，并增加普通模式零 AGENTS 写入、验收模式五类路由、歧义失败关闭和旧 `paths:` 文案归零测试。

## 5. 下游旧 Rule 迁移

- 同步器 plan 必须枚举下游 `.codex/rules/**`，区分官方 `.rules` 命令权限、Markdown 语义、显式引用说明和未知文件；禁止把整个目录视为同一 ownership。
- 每个 `.md` 迁移候选必须给出目标 owner：根项目区、最近目录的嵌套 `AGENTS.md`、Hook/测试或 Memory/文档。未知或多目标时进入 risk/gap，禁止 apply。
- apply 必须先安装新 owner、迁移根索引及所有活跃指针，再做语义等价与加载验证，最后才删除旧 Markdown；任一失败回滚本事务写入。
- 项目级 `.rules` 必须原样保留，除非其命令策略本身进入另一个明确、经用户授权的迁移项。
- fixture 至少覆盖：全项目 Rule、目录 Rule、可机判 Rule、说明性长文、合法 `.rules`、无引用 Markdown、混合 CRLF、项目自有修改、迁移失败回滚和二次 no-op。
- 真实下游验收至少覆盖一个低定制与一个高定制项目；StratusAgent 和 CausisRiskSuite 当前只完成只读盘点，未授权写入，BridgePersonalAssist 当前无旧 Rule 目录。

## 6. 发布与验证

- 以真实同步器对临时 Template/下游执行“不带 `--apply` 的 plan”、apply、失败回滚和二次 no-op；覆盖 README 无 marker 追加、pre-commit 无 marker 阻断、非法 marker 结构化 blocker、自定义项目区、CRLF、无结尾换行和多余尾部空行。
- 临时构建完整候选工厂；候选 instruction Hook 必须对工厂根项目区、`templates/AGENTS.md`、`scripts/AGENTS.md`、`skills/AGENTS.md` 和 `doc/2_bugs/AGENTS.md` 核对规范化全文 hash，并分别阻断工作树和 staged blob 的破坏。
- plan、阻断、no-op 和注入失败回滚必须比较受管可见树的目录集合与文件 bytes hash；该口径排除 `.git`、`.venv` 和 `__pycache__`，不得宣称覆盖这些边界。结构化 marker blocker 还必须通过真实子进程 CLI 验证退出码、唯一 JSON stdout，以及与 JSON error 严格一致的 `BLOCKED:` stderr 提示。
- 对完整候选 Skill 运行真实 root-skill 测试和正式分发 manifest `--check`。
- 所有 `--check` / `--dry-run` 零写行为与全部旧指针只在真实实施后的枚举矩阵中放行；proposal 只报告实际执行的 manifest `--check`、无 `--apply` plan 和合同列出的活跃面扫描。
- `contracts/semantic-contract.json` 必须锁定现行来源文件 hash，使每条红线恰好映射一次，并对高风险项验证主体、强度、触发和例外位于同一语义关系中。
- 真实实施必须更新 VERSION、CHANGELOG、manifest/hash、dogfood 镜像和相应测试；本 proposal 不执行这些动作。
