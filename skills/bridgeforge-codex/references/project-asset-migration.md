# Legacy Rule / Memory 逐文件迁移

仅当 planner 返回 `asset_migration.source_count > 0` 时读取。所有选择必须由主对话向用户逐项确认；`review-auditor` 不得代替用户决定。

## 连续确认

1. 按 planner 给出的稳定顺序处理每个 `.codex/rules/*.md` 与 `.codex/memory/**` 源文件。
2. `MEMORY.md`、`MEMORY_COLD.md`、`_stats.json` 逐个展示“固定退役”并等待用户明确接受，其 `retirement_reason` 必须是 `fixed-derived-retirement`，禁止语义读取或生成目标；未确认不得删除。
3. 其他源文件必须读取正文并提出一个完整迁移包：源摘要、每段内容、目标资产、完整目标正文、废弃内容和理由。一个源可以拆到多个目标。
4. 目标职责必须严格匹配：项目红线进根或最近目录的 `AGENTS.md`；命令执行策略进 `.codex/rules/*.rules`；用户调用流程进 Skill；机械硬闸进 Hook / test；当前工作进 Delivery / Bug / TODO；设计、历史和案例进 `doc/`；项目自有知识话题与资料进 `doc/5_project_knowledgebase/`。知识话题不得仅因日期或完成状态强制归档，也不得变成 AGENTS 规则；目标正文、历史副本与索引仍须逐源确认。
5. 迁入 `AGENTS.md`、`doc/README.md` 或 `.codex/hooks.json` 时只能更新项目自有或非受管区；必须同时满足最新公共基线，禁止用迁移正文覆盖公共受管区。Hook 包必须包含入口与注册，文档新增必须进入索引。
6. 每个源（包括固定退役的派生文件）只问一次“接受当前完整迁移包 / 修改 / 停止”。用户未明确接受时不得标记 `confirmed`；判断不清时说明未知内容并请用户指定，禁止默认保留、迁移或删除。
7. 只有当前源确认后才能进入下一个。对话中断、用户停止或任一源未完成时，丢弃本轮全部内存决定；禁止写项目文件、`.runtime`、临时 manifest 或其他恢复点。下次从第一个源重新开始。

可以让 `review-auditor` 只读复核候选归属和受管区边界，但它不得与用户对话、替用户确认或保存决定。

## 规划与 Apply

全部源确认后，在当前对话内构造 schema-v1 manifest。每个对象必须严格使用以下字段，禁止添加临时状态或省略空数组：

```json
{
  "schema_version": 1,
  "sources": [{
    "asset_id": "<inventory 原值>",
    "source_path": "<inventory 原值>",
    "source_sha256": "<inventory 原值>",
    "kind": "<inventory 原值>",
    "confirmed": true,
    "retire_source": true,
    "summary": "<源摘要>",
    "retirement_reason": "<理由或 fixed-derived-retirement>",
    "decisions": [{
      "target": "<POSIX 相对路径>",
      "asset_type": "<agents|command-rule|skill|hook|hook-registration|test|delivery|todo|bug|documentation>",
      "reason": "<迁移理由>",
      "target_before_sha256": "<现有目标 sha256，目标不存在时为 null>",
      "content_utf8": "<完整目标 UTF-8 正文>"
    }],
    "discarded": [{"summary": "<废弃内容>", "reason": "<废弃理由>"}]
  }]
}
```

同一源产生 Hook 时，`decisions` 必须同时包含 `hook` 的 `project_*/entrypoint.rs` 和 `hook-registration` 的 `.codex/hooks.json`；新增文档必须同时包含 `doc/README.md` 的 `documentation` 决定。多个源可以引用同一最终目标，但各源必须给出逐字相同的完整 `content_utf8`、目标类型和 before hash；机器只物化一次，不同内容必须阻断并重新合并确认。已有复合目标的 `content_utf8` 表示用户确认后的完整目标，机器随后用 latest 公共区替换其中受管部分。

启动同步器时使用 `--asset-migration-manifest -`，通过进程 stdin 发送 UTF-8 JSON；禁止把 JSON 放入 shell 字符串、重定向或持久文件。

先用该 manifest 重新生成 combined plan。任一源 hash、目标 before hash、覆盖范围、注册、索引、路径或 latest 基线组合校验失败时，零写停止。通过后用完全相同的 manifest、plan fingerprint 与 `--confirmed-asset-migration` 执行 Apply；逐文件确认已经授权对应旧源删除，禁止再次询问清理。

成功终态必须同时证明：最新完整基线生效、所有目标资产可用、所有已确认旧源删除、版本戳最后写入。任一失败必须逐字恢复迁移前项目。
