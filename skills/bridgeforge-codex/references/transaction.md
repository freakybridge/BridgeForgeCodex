# 骨架事务与回滚

仅当根入口已取得有效 plan、fingerprint 和所需用户选择，准备执行 Apply 时读取。本文是骨架同步 preflight、事务、回滚和版本戳写入顺序的唯一操作 owner；根 `AGENTS.md` 只保留版本域红线，其他 Skill 或说明文档不得复制本流程。

Apply 必须传 `--apply --plan-fingerprint <fingerprint>` 和已取得的用户选择；plan 有 risk 时传 `--confirmed-risk`，该标志只证明前面逐项决定已齐全，不得触发额外确认。有 legacy Rule / Memory 时，还必须把同一份内存 manifest 通过 `--asset-migration-manifest -` 的进程 stdin 传入并带 `--confirmed-asset-migration`。destructive rebuild 还必须传已确认的 `PreservationManifest`。禁止人工 copy、merge、删除或写戳。

同步器必须：

- 只以刷新后产品 home 的 latest current-only 合同生成目标；禁止解释旧 schema 或逐版本执行历史迁移。
- 常规更新保留 project-owned、未知文件和人工定制；latest rebuild 对未知 `.codex/**` 结构零写阻断，并严格执行用户确认的 `PreservationManifest` 与资产迁移 manifest。
- 破坏性重建把精确路径 `.codex/find-doc.map.md` 与 `.codex/sync-docs.map.md` 作为 required-preserve 项目映射原样保留，禁止用 glob 扩大所有权边界。
- Planner、Apply、`$git-sync` 与 pre-commit 调用同一 `current_baseline.py` 检查器。
- legacy Rule / Memory 只能按逐文件确认的 manifest 迁移；`MEMORY.md`、`MEMORY_COLD.md`、`_stats.json` 只做固定退役。确认期间禁止落盘 manifest、写目标或删除源文件。
- Skill 只允许确定性修复 frontmatter；缺少 description 或 routing 语义时必须阻断。
- 同一事务组合并验证最新基线、迁移目标和项目自有区，随后删除已确认源文件，最后写 `.codex/.bridgeforge_codex_version`。
- 任一失败回滚本事务全部写入；成功后不得保留 before 包。
- Claude 项目遗留只提示，不读取、不修改。

Apply 完成后必须重新验证目标资产、合同、版本戳和工作区收据；任一终态断言失败都不得报告成功。
