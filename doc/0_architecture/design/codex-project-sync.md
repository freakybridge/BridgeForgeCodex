# bridgeforge-codex 项目同步事务

> 状态：implemented（dynamic latest current-only）
> 入口：`scripts/bridgeforge_codex_project_sync.py`

bridgeforge-codex 只维护 Codex 当前产品面。公共资产 ownership 的唯一产品来源是
`templates/managed-skeleton.json` schema 3；合同只保存当前版本的稳定 asset id、显式
source/target、ownership strategy 和当前 hash/projection，禁止历史版本集合、retirement、
adaptation proof 与 glob ownership。

## 版本分流

```text
空白骨架身份 + init
  -> 安装当前 Template

恰好一个合法版本戳
  -> 版本只证明骨架身份
  -> 旧于产品 home 时直接 latest rebuild；同版本时验证 current baseline

无戳但存在骨架资产
  -> adopt + latest rebuild

Rule / Memory 源存在
  -> 逐文件确认完整迁移包
  -> latest Template + 迁移目标 + 已确认源删除同一事务

双戳 / 非法戳 / 身份不一致
  -> 零写阻断
```

latest rebuild 不读取旧 `.codex/managed-skeleton.json`，也不按版本选择历史 adapter。它先盘点项目资产，再只放回确认的 AGENTS 项目区、pre-commit 项目扩展、项目 Hook 与自动保留的 `.codex/skills/**`、
`.codex/find-doc.map.md` 和 `.codex/sync-docs.map.md`。两个项目映射只按精确路径识别并作为
required-preserve 原样保留；其他未知 `.codex/**` 仍 fail-closed。每个可选资产必须显式选择
保留或删除；临时 `PreservationManifest` 只存在于本次事务内，在写最终戳前清空，不生成持久
before 包或迁移账本。

`.codex/rules/*.md` 与 `.codex/memory/**` 由 `project_asset_migration.py` 盘点和验证。Agent 逐源文件提出语义迁移包；机器只验证完整覆盖、source/target hash、目标职责、公共受管区、Hook 注册、文档索引和事务。`MEMORY.md`、`MEMORY_COLD.md`、`_stats.json` 固定退役。确认期间不得落盘 manifest；中断后从第一个源重来。

## Current-only 事务

```text
refresh product home + identify project
  -> build deterministic actions + aggregate fingerprint
  -> confirm every Rule / Memory source in one continuous session when required
  -> immediate replan/fingerprint check
  -> temporary transaction snapshot
  -> combine latest assets / migration targets / selected project assets
  -> remove confirmed Rule / Memory sources
  -> verify actions + preserved knowledge
  -> config health + text hygiene validators
  -> verify prospective current baseline on real disk
  -> write .codex/.bridgeforge_codex_version last
```

任一可捕获失败必须逐字恢复迁移前项目，包括已删除的 Rule / Memory。Planner、Apply、`$git-sync` 与
pre-commit 直接复用 `current_baseline.py`。pre-commit 只读检查 worktree 与 Git index，
不得生成文件或执行 `git add`；`$git-sync` 在写入前生成完整 `SyncWritePlan`，并在提交前失败时
恢复自动写入和完整 index。公共资产漂移、合同损坏或同版本合同自证修改不能通过风险确认覆盖。

## 输出合同

同步器把自动化收据与用户结果分成两个事实层：

- `machine`：默认且向后兼容的 JSON，保留 plan、fingerprint、资产动作、版本与回滚字段，供测试、fixture、Hook 和其他程序读取。
- `human`：由同步器确定性生成的“结论、待处理事项、下一步”，不暴露 fingerprint、asset ID、内部枚举或 traceback。
- `combined`：同一 JSON 中同时返回 `machine` 与 `human`；`bridgeforge-codex` Skill 必须使用该模式，按 `machine` 推进流程并原样展示 `human`。

三种模式只改变结果表示，不得改变 plan、确认、Apply、回滚、版本戳与退出码语义。失败时 `machine` 默认模式继续保留既有 `BLOCKED` stderr；`human` 与 `combined` 不混入第二套临场错误文本。

## 项目资产边界

- 根 `AGENTS.md` 公共区由产品管理；项目区允许由 `PreservationManifest` 保留并由已确认迁移包追加，二者必须与 latest 公共区确定性组合。
- `.codex/hooks.json` 只允许 canonical managed handler 与已确认的项目 Hook 注册；项目注册
  必须与一个 `.codex/hooks/project_XXXX/entrypoint.py` 目录成对，未知 managed ID 阻断。
- 散落 Hook、非 canonical 命令和未知 `.codex/**` 结构都必须零写阻断；独立 Agent 只能先在
  临时副本或受控前置步骤中把 Hook 整理为闭合的自包含目录，再重新生成清单。
- schema 3 merge/Markdown/region/AGENTS 都携带当前可验证 projection；真实下游不存在
  `templates/**` 时也不得跳过。
- 项目 Skills 正文只有在对应源迁移包中逐项确认后才允许语义改写；legacy Rule / Memory 禁止派生索引、自动分类或未确认保留。
- 项目 `find-doc` / `sync-docs` 映射是精确登记的 required-preserve 数据，重建前后必须字节不变。
- Claude、switch、project finalizer 与 harness parity 不属于当前产品面，也不保留识别谱系。
