# bridgeforge-codex 项目同步事务

> 状态：implemented（最低清洁基线 1.4.31）
> 入口：`scripts/bridgeforge_codex_project_sync.py`

bridgeforge-codex 只维护 Codex 当前产品面。公共资产 ownership 的唯一产品来源是
`templates/managed-skeleton.json` schema 3；合同只保存当前版本的稳定 asset id、显式
source/target、ownership strategy 和当前 hash/projection，禁止历史版本集合、retirement、
adaptation proof 与 glob ownership。

## 版本分流

```text
空白骨架身份 + init
  -> 安装当前 Template

恰好一个合法版本戳，版本 < 1.4.31
  -> 独立只读审计
  -> 用户逐项确认项目 rules/hooks/AGENTS 项目区的保留或删除
  -> 一次破坏性重建风险确认
  -> fresh canonical Template + PreservationManifest 确认项 + Skills + legacy Memory 原样保留收据

恰好一个合法版本戳，版本 >= 1.4.31
  -> current-baseline 常规 update
  -> 若为旧文件名，同一事务删除旧戳并最终只保留当前戳

缺戳 / 双戳 / 非法戳 / 身份不一致
  -> 零写阻断
```

破坏性重建不读取旧 `.codex/managed-skeleton.json`，也不会复用常规 merge。它先生成 fresh canonical，再只放回确认的 AGENTS 项目区、
pre-commit 项目扩展、项目 rules 与 `.codex/hooks/project_XXXX/` 自包含 Python Hook 目录，
以及自动保留的 `.codex/skills/**`、
`.codex/find-doc.map.md` 和 `.codex/sync-docs.map.md`。两个项目映射只按精确路径识别并作为
required-preserve 原样保留；其他未知 `.codex/**` 仍 fail-closed。每个可选资产必须显式选择
保留或删除；临时 `PreservationManifest` 只存在于本次事务内，在写最终戳前清空，不生成持久
before 包或迁移账本。

下游既有 `.codex/memory/**` 不进入 fresh canonical，也不执行兼容性、索引或语义检查；同步器
必须在事务外原样保留并报告 `legacy project memory pending migration`。删除只能发生在对应项目
完成逐文件扫描、用户确认和独立清理授权之后，不属于 adopt/update 的直接写入面。

## Current-only 事务

```text
verify real baseline + trusted Git HEAD anchor
  -> build deterministic actions + aggregate fingerprint
  -> immediate replan/fingerprint check
  -> temporary transaction snapshot
  -> apply current assets / selected preserved assets / obsolete-stamp deletion
  -> verify legacy project memory was not targeted; on concurrent drift roll back managed changes without restoring legacy memory
  -> verify actions + preserved knowledge
  -> config health + text hygiene validators
  -> verify prospective current baseline on real disk
  -> write .codex/.bridgeforge_codex_version last
```

任一可捕获失败必须恢复本事务写入及 memory 派生产物。Planner、Apply、`$git-sync` 与
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

- 根 `AGENTS.md` 公共区由产品管理；项目区只有在 `PreservationManifest` 中明确保留才逐字回灌。
- `.codex/hooks.json` 只允许 canonical managed handler 与已确认的项目 Hook 注册；项目注册
  必须与一个 `.codex/hooks/project_XXXX/entrypoint.py` 目录成对，未知 managed ID 阻断。
- 散落 Hook、非 canonical 命令和未知 `.codex/**` 结构都必须零写阻断；独立 Agent 只能先在
  临时副本或受控前置步骤中把 Hook 整理为闭合的自包含目录，再重新生成清单。
- schema 3 merge/Markdown/region/AGENTS 都携带当前可验证 projection；真实下游不存在
  `templates/**` 时也不得跳过。
- 项目 Skills 正文不得语义改写，仍执行 current metadata 校验；legacy 项目 Memory 只保留并报告，禁止派生索引重建或 current metadata 校验。
- 项目 `find-doc` / `sync-docs` 映射是精确登记的 required-preserve 数据，重建前后必须字节不变。
- Claude、switch、project finalizer 与 harness parity 不属于当前产品面，也不保留识别谱系。
