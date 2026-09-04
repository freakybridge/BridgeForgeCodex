# bridgeforge-codex 项目同步事务

> 状态：implemented（固定升级分界线 + latest current-only 目标）
> 入口：`bridgeforge project-sync`

bridgeforge-codex 只维护 Codex 当前产品面。公共资产 ownership 的唯一产品来源是
`templates/managed-skeleton.json` schema 4；普通资产保存当前版本的稳定 asset id、显式
source/target、ownership strategy 和当前 hash/projection；生成资产另保存源码树、锁文件、
构建配方、自检合同、目标平台二进制路径与收据路径。合同禁止历史版本集合、retirement、
adaptation proof 与 glob ownership。

## 版本分流

```text
空白骨架身份 + init
  -> 安装当前 Template

恰好一个合法版本戳
  -> 低于固定 compatibility_baseline 时直接 latest rebuild
  -> 等于或高于基线时兼容更新；同版本验证 current baseline；高于产品版本拒绝降级

无戳但存在骨架资产
  -> adopt + latest rebuild

Rule / Memory 源存在
  -> 逐文件确认完整迁移包
  -> latest Template + 迁移目标 + 已确认源删除同一事务

双戳 / 非法戳 / 身份不一致
  -> 零写阻断
```

固定分界线在 `templates/managed-skeleton.json` 的 `compatibility_baseline` 声明，首版为
1.8.6；manifest 重建只更新 `release_version`，不自动抬高分界线。两条路径均只安装最新目标，
不积累逐版本 adapter。基线内兼容路径不枚举整个 `.codex` 做破坏性重建，按受管边界更新并
保留项目文件、区域、Hook 和表格行；已有 whole 资产替换仍需确认。旧于目标版本时允许
按照新模板顺序补齐受管 Markdown 章节、更新同列数表头；同版本缺失/漂移、无戳接入、
重复标题/歧义表格和列数变化仍阻断。

latest rebuild 不读取旧 `.codex/managed-skeleton.json` 的语义，也不按版本选择历史 adapter。它先盘点项目资产，再只放回确认的 AGENTS 项目区、pre-commit 项目扩展、项目 Hook 与自动保留的 `.codex/skills/**`。旧 `.codex/find-doc.map.md` 和 `.codex/sync-docs.map.md` 是已退役的骨架生成物，两条升级路径都在事务内按精确路径删除；新 Hook 随后的 `Stop` / `SessionStart` 会在 `.runtime/bridgeforge-codex/` 按当前项目事实重建自动索引。未被当前合同覆盖的普通文件以 `P:project-file:<path>` 列为决策项；链接和危险 Hook 结构仍阻断。每个可选资产必须显式选择
保留或删除；临时 `PreservationManifest` 只存在于本次事务内，在写最终戳前清空，不生成持久
before 包或迁移账本。

## 项目 Map 自动索引

`.runtime/bridgeforge-codex/find-doc.map.md` 与 `.runtime/bridgeforge-codex/sync-docs.map.md` 是骨架内生的本地运行时索引，禁止手工维护或加入 Git。`find-doc` 索引只从实际生效的根/嵌套 `AGENTS.md` 标题、作用目录和明确代码词建立主题到指令源的关系；`sync-docs` 索引只接受设计文档中明确引用且磁盘真实存在的源码路径。目录同名不是语义证据，无法证明的关系不进入 Map，由 Skill 继续搜索 fallback。

`PostToolUse` 对相关输入只在 `.runtime/bridgeforge-codex/` 写脏标记；`Stop` 合并重建，`SessionStart` 兜底校验，两个 Skill 在读取前调用 `bridgeforge-hook project-map ensure-current`。生成文件携带 schema 与输入 SHA-256；相同输入逐字生成相同内容，目标字节未变化时不得重写。生命周期 Hook 的维护成功与 no-op 都不输出；严格 Skill 入口失败且 fallback 也不能完成任务时才向用户报告真正阻断。

`.codex/rules/*.md` 与 `.codex/memory/**` 在两条升级路径中都由 Rust `project-sync` 盘点和验证。Agent 逐源文件提出语义迁移包；机器验证完整覆盖、source/target hash、目标职责、公共受管区、Hook 注册、文档索引和事务。`MEMORY.md`、`MEMORY_COLD.md`、`_stats.json` 逐个确认固定退役。全部确认前零写入，禁止落盘 manifest 或创建项目锁；中断后从第一个源重来。Apply 前重新盘点，新增、消失或改动的源均使旧计划失效。

## Current-only 事务

```text
refresh product home + identify project
  -> build deterministic actions + aggregate fingerprint
  -> confirm every Rule / Memory source in one continuous session when required
  -> immediate replan/fingerprint check
  -> 在仓库外临时目录构建并自检 Rust Hook 生成资产
  -> temporary transaction snapshot
  -> combine latest assets / generated binary + receipt / migration targets / selected project assets
  -> remove confirmed Rule / Memory sources
  -> verify actions + preserved knowledge
  -> config health + text hygiene validators
  -> verify prospective current baseline on real disk
  -> write .codex/.bridgeforge_codex_version last
```

任一可捕获失败必须逐字恢复迁移前项目，包括已删除的 Rule / Memory。Planner、Apply、`$git-sync` 与
pre-commit 直接复用 `bridgeforge check baseline`。编码门禁通过 NUL 文件列表与 Git 原始 blob 检查实际 index 内容，不能以工作区内容代替暂存字节；项目 Skill 检查覆盖 `.codex/skills`，工厂额外检查共享 `skills`。pre-commit 只读检查 worktree 与 Git index，
不得生成文件或执行 `git add`；`$git-sync` 在写入前生成完整 `SyncWritePlan`，并在提交前失败时
恢复自动写入和完整 index。工厂 `git-sync` 的计划先将版本变更投影到仓库外快照，渲染 manifest，按锁定 Cargo 配方构建 Hook / CLI 并生成实测收据；完整计划包含版本、Cargo manifests / locks、CHANGELOG、三份清单、二进制和收据。构建后复核原始输入、源码文件集合、目标原值、仓库身份及 index，任何漂移均阻断写入。安装后先验证含生成产物的完整 baseline，再暂存和提交。构建与安装持有 project-sync 锁，整个同步仍持有 Git common-dir 锁；两者均由系统句柄持有，进程退出释放。

Git 拉取上游与实际推送目标分别判定：上游负责快进更新；推送是否必要和最终 ahead/behind 使用实际 push 目标。两者不同时，先刷新实际推送 remote，再验证目标；仅上游 `0/0` 不能报告 synced。批量暂存前的敏感文件保护使用不折叠目录的 NUL 状态列表，覆盖新目录内的文件。

Windows 上当前 CLI 的运行映像不能直接覆盖：新程序先写入同目录临时文件，旧映像移入已被 Git 忽略的 `.runtime/bridgeforge-codex/git-sync-images/` 后安装。旧进程继续执行，不启动额外 shell 或清理进程；后续工厂同步只清理该目录中名称格式正确且内容哈希匹配的旧映像，仍被占用者留到下次。安装失败立即移回原映像；提交前失败按原字节恢复二进制及其余自动资产。该缓存不进入提交。

同版本修复也从可信产品目录重新生成目标，不信任项目内合同自证 ownership；已有 whole 资产替换须确认。版本戳内容未变化时仍可应用其他已确认动作，最后验证当前唯一版本戳与完整基线。

Rust Hook 在 `init`、`adopt`、`update` 的计划物化及 apply 前构建；构建只写仓库外临时目录。Cargo 缺失、平台不受支持、源码或
锁文件漂移、构建失败、自检不匹配均在产品写入前阻断；成功产物与包含真实 binary hash 的收据
由同一事务写入。日常 Hook 直接执行 `.codex/bin/bridgeforge-hook[.exe]`，禁止现场调用 Cargo、
Python 或 PowerShell 包装器。

生成阶段先从实际受管 workspace 读取文件并建立仓库外独立快照，按 manifest 相同的规范化算法
重新计算源码树与 Cargo.lock 哈希；构建配方必须与执行的锁定 Cargo 命令一致，自检合同也需重算。
所有值与声明匹配后才从快照运行 Cargo；每个资产自检后和整个生成批次结束前，均逐字复核
原始输入与快照，且自检不得改写二进制。任何漂移或失败都发生在安装资产写入之前。
收据记录实测哈希，不再直接复制 manifest 的声明。此证据覆盖受管源码、锁文件、执行配方和
自检合同，不表示已实现工具链、全局 Cargo 配置及环境变量的完全可复现构建。

共享进程执行器从启动前开始计时，输入写入和两路输出读取并行运行，子进程退出并且标准流
全部结束才算完成。Windows 使用挂起启动、加入禁止脱离的 Job Object、恢复执行的顺序，
并保留 CREATE_NO_WINDOW；超时结束整个 Job，Unix 使用独立进程组。超时后清理最多等待
两秒，清理未完成则返回显式错误，禁止无限等待 reader join。该进程组不是针对恶意进程、
外部服务代为启动或 Unix 主动脱组的安全沙箱；相应外部行为不属于本次验证范围。

## 输出合同

同步器把自动化收据与用户结果分成两个事实层：

- `machine`：当前版本 JSON，包含 plan、fingerprint、资产动作、版本与回滚字段，不提供旧 JSON 兼容，供测试、fixture、Hook 和其他程序读取。
- `human`：由同步器确定性生成的“结论、待处理事项、下一步”，不暴露 fingerprint、asset ID、内部枚举或 traceback。
- `combined`：同一 JSON 中同时返回 `machine` 与 `human`；`bridgeforge-codex` Skill 必须使用该模式，按 `machine` 推进流程并原样展示 `human`。

三种模式只改变结果表示，不得改变 plan、确认、Apply、回滚、版本戳与退出码语义。失败时 `machine` 默认模式继续保留既有 `BLOCKED` stderr；`human` 与 `combined` 不混入第二套临场错误文本。

## 项目资产边界

- 根 `AGENTS.md` 公共区由产品管理；项目区允许由 `PreservationManifest` 保留并由已确认迁移包追加，二者必须与 latest 公共区确定性组合。
- `.codex/hooks.json` 只允许 canonical managed handler 与已确认的项目 Hook 注册；项目注册
  必须与一个 `.codex/hooks/project_XXXX/` 自包含业务 Hook 目录成对；既有业务 Hook 不限制语言，整体逐项确认保留或删除，未知 managed ID 阻断。迁移包新建 Hook 的目标约束仍由资产迁移合同规定。
- 普通未知文件必须按精确路径确认保留或删除，禁止读取旧合同决定所有权。`project_` 文件名前缀不会把普通文件变成 Hook 目录；此类脚本确认删除后，已有 `hooks.json` 中匹配其完整路径的注册一并退役，文件与注册同一事务回滚，相似文件名注册保留。链接、非普通目录的项目 Hook 包、未知 Rule 格式仍阻断。
- schema 4 merge/Markdown/region/AGENTS 都携带当前可验证 projection；真实下游不存在
  `templates/**` 时也不得跳过。
- 项目 Skills 正文只有在对应源迁移包中逐项确认后才允许语义改写；legacy Rule / Memory 禁止派生索引、自动分类或未确认保留。
- 项目 `find-doc` / `sync-docs` 映射在 project-sync 事务内按精确路径登记为 required-preserve；事务完成后的受管 Hook 可按自动索引合同重建，禁止扩大为 glob ownership。
- Claude、switch、project finalizer 与 harness parity 不属于当前产品面，也不保留识别谱系。
