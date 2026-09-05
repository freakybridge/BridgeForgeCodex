# bridgeforge-codex 上游更新 Playbook

> **定位**：把 bridgeforge-codex 的 Codex 骨架安全更新到当前项目。唯一产品入口是 `$bridgeforge-codex`；禁止手工复制模板或串联旧脚本。

## 1. 更新对象

- 用户级产品仓库：`~/.bridgeforge-codex`。
- 用户级薄入口：`~/.codex/skills/bridgeforge-codex`。
- 项目骨架：`AGENTS.md`、`.codex/`、`.githooks/pre-commit` 与 `doc/README.md`。
- 项目版本戳：`.codex/.bridgeforge_codex_version`。

Claude 骨架已经退役。遗留 `.claude/` 只报告存在，不读取正文、不迁移、不删除；旧用户级 Claude 资产、旧 ledger 与旧产品 home 不再由当前产品接管或清理。

## 2. 唯一流程

1. 在目标项目运行 `$bridgeforge-codex`。
2. 入口先识别项目版本戳；双戳、非法戳停止，无戳空项目 init，无戳已有资产 adopt。运行 `doctor --product-root <产品目录> --json` 检查产品 runtime，不要求目标项目已经安装 Rust 骨架。
3. 任意合法旧版本直接进入 latest current-only rebuild；版本只证明身份，禁止运行旧 schema、旧 manifest 或逐版本兼容链。
4. `.codex/rules/*.md` 与 `.codex/memory/**` 按 [迁移手册](../../../skills/bridgeforge-codex/references/project-asset-migration.md)逐源确认；全部确认前零写入、不新增持久恢复状态。中断恢复时按手册核验决定与现场，仅重核失效部分；确认同时授权在同一事务删除对应源，不再追加清理确认。
5. apply 先取得项目级事务锁，再重算聚合 fingerprint。锁覆盖验证、写入、版本戳与回滚；独立 build-assets 使用同一把锁。任何漂移、验证失败或运行错误都必须零写入或事务回滚。
6. validators 与真实磁盘 current baseline 全部通过后，最后写新版本戳；旧项目成功时同事务删除旧戳。

项目事务锁只协调 BridgeForge 写入，不阻止用户从其他工具修改文件，因此仍需核对指纹、目标原值与仓库身份。项目、Git 和批次修改锁使用操作系统持有的句柄；进程退出释放所有权，保留的锁文件不代表仍有持有者，禁止按年龄删除或重建锁文件。

## 3. 所有权边界

- `whole`、`merge`、`managed_blocks`、`region` 均只接受 schema 4 当前 hash/projection；`generated` 还必须通过源码、构建配方、自检与二进制收据核验。
- 旧项目不解析旧 managed contract，也不复用常规 merge；每个可选项目资产必须明确保留或删除，只把已确认保留项注入 fresh canonical。
- 项目 Hook 以 `.codex/hooks/project_XXXX/` 自包含 Rust 目录为原子所有权单位，目录与 `.codex/hooks.json` 注册必须成对。删除注册按路径边界匹配 `command` / `commandWindows`，不能误删相似前缀目录；已确认退役的空目录随文件一并删除，失败时恢复目录与文件。
- 散落 Hook 先阻断；独立 Agent 只能在临时副本或受控前置步骤中完成目录正规化，随后重新生成 `PreservationManifest`。
- `seed` 只在缺失时创建，既有内容归项目。
- `.codex/skills/` 自动保留正文并执行当前兼容检查；legacy Rule / Memory 只能按已确认 manifest 迁移，机器不得猜语义；其他未登记普通文件逐路径确认保留或删除，不得静默删除。

## 4. 验收收据

完成更新至少核对：

- plan 的 `status/readiness` 与 apply 的 `execution_status/project_readiness` 分开核对；
- actions/`PreservationManifest`/blocker 清单与实际执行逐项对账；
- `stamp_written_last=true` 只在完整验证后出现；
- 再次 plan 为 no-op；
- `git diff --check`、manifest/schema、memory 与 hook 验证通过；
- 遗留 Claude 内容未被读取或改写。

有 gap 时，结果不是“完美更新”。收据必须给出用户还需处理的文件、原因和可执行选择，不得用项目成功掩盖用户级 Memory 未就绪。

## 5. 禁止事项

- 禁止手工 `copy`/`cp` 覆盖下游骨架。
- 禁止用旧同步脚本或多入口串联写戳。
- 禁止覆盖未确认的项目 rules、AGENTS 项目区、memory 或 doc；确认迁移目标必须与 latest 公共受管区确定性组合。
- 禁止在同一轮重复索要确认。
- 禁止把 Claude 遗留目录当作 Codex 模板来源。

反方向的通用经验回灌见 [reverse-sync-playbook.md](reverse-sync-playbook.md)。
