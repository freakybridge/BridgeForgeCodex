# 骨架事务与回滚

仅当根入口已取得有效 plan、fingerprint 和所需用户选择，准备执行 Apply 时读取。本文是骨架同步 preflight、事务、回滚和版本戳写入顺序的唯一操作 owner；根 `AGENTS.md` 只保留版本域红线，其他 Skill 或说明文档不得复制本流程。

Apply 必须传 `--apply --plan-fingerprint <fingerprint>` 和唯一用户选择；destructive rebuild 还必须传已确认的 `PreservationManifest`。禁止人工 copy、merge、删除或写戳。

同步器必须：

- 只修改 schema v3 current-only 合同逐资产登记的 Codex 目标。
- 常规更新保留 project-owned、未知文件和人工定制；破坏性重建对未知 `.codex/**` 结构零写阻断，并严格执行用户确认的 `PreservationManifest`。
- 破坏性重建把精确路径 `.codex/find-doc.map.md` 与 `.codex/sync-docs.map.md` 作为 required-preserve 项目映射原样保留，禁止用 glob 扩大所有权边界。
- Planner、Apply、`$git-sync` 与 pre-commit 调用同一 `current_baseline.py` 检查器。
- memory 只做只读兼容检查和派生索引重建，禁止 organize 或移动正文。
- Skill 只允许确定性修复 frontmatter；缺少 description 或 routing 语义时必须阻断。
- 先应用并验证资产，最后写 `.codex/.bridgeforge_codex_version`。
- 任一失败回滚本事务全部写入；成功后不得保留 before 包。
- Claude 项目遗留只提示，不读取、不修改。

Apply 完成后必须重新验证目标资产、合同、版本戳和工作区收据；任一终态断言失败都不得报告成功。
