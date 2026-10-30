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

旧 Rule / 项目 Memory 文件存在
  -> 不再执行语义迁移
  -> 兼容更新保留；重建按普通项目资产保留或删除决策处理

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

## 内容合并边界

`project_sync/content.rs` 负责 AGENTS 区域、受管 Markdown、Hook JSON、普通 JSON、区域替换和 seed/whole/merge 策略的内容转换，继续复用 `managed_markdown` 基础解析。输入是已经读取的字节、资产规则及项目路径信息，输出为新字节或原有错误；该模块不读写文件，也不调用计划或事务。

`project_sync::render_asset` 保留路径校验和文件读取，将当前磁盘内容交给转换器，并单独返回磁盘原始字节用于计划与漂移判定。计划、确认、指纹、锁、落盘、回滚和版本戳顺序仍归 `project_sync`。旧语义迁移输入、`build_plan_with_migration`、plan 的 `asset_migration` 和 receipt 的 `asset_migration_manifest_sha256` 已移除；CLI 明确拒绝旧迁移参数，不静默忽略后执行 Apply。

## 共享构建边界

公共产物由用户级 updater 准备，缓存位于当前用户 .codex/.cache/bridgeforge/public-build，身份绑定源码、锁、配方、自检与编译环境，不绑定下游项目。沿用 artifact_cache 的 ownership、锁、内容校验和容量限制；public-ready 存放有界引用收据。项目 Apply 只读捕获字节，不编译、自检或更新 recency，缺失或损坏时需重新准备。

项目 Hook 独立维护边界固定在代码中，不依赖策略文件。init/adopt/update 和公共构建、基线检查均不编译或验证项目 Hook 业务源码；更新公共注册时保留项目 handler 和扩展字段。旧策略及迁移状态原样保留并忽略，不再生成提示或记录。项目自行构建、安装和维护注册；公共布局迁移不搬移项目程序或改写其路径，归属不明时保留并报告阻断。

core 的 build_inputs 负责输入快照及漂移验证，generated_assets 负责隔离构建、自测和收据，public_artifacts 负责用户级准备和只读交付。工厂自身构建及 Git 同步仍使用受管构建能力；普通下游 project-sync 不直接编译。初次 CLI 引导结果按相同身份进入缓存，并与公共 Hook 共用依赖输出。

project_sync 拥有只读项目计划、内部准备、固定公共产物挂接、项目锁、安装、版本戳与回滚，不重做公共版本选择。git_sync_plan 保留工厂构建职责。共享构建不反向调用项目同步。用户级事务成功后，项目失败不撤销共享安装。

`source_root` 与 `target_source_root`、Cargo 参数/超时、输入验证、自测、清理错误和收据 schema 均保持原合同。开发证据见[R02 需求卡](../../1_delivery/shared-generated-build/requirements_2026-10-11_R02.md)。

## 受管 Markdown 解析边界

`bridgeforge-core::managed_markdown` 是 core 内部的标题、表格和行标识解析实现。`project_sync` 负责合并、章节插入与升级判断，`baseline` 负责投影、哈希和漂移校验；`manifest` 继续使用 baseline 的投影生成合同。

解析模块显式保留历史兼容策略：合并按下一个精确 `## ` 行结束章节，保留 key 大小写并接受原有链接形式；投影先归一化换行，按同级或更高层标题结束章节，对 key 转小写并沿用较窄的反引号链接语法。多表唯一候选选择仍属于合并策略，投影仍收集原有范围的行。注释和代码围栏不新增过滤规则。

两侧职责集中不代表统一历史语义。改变上述规则、错误文本或投影哈希必须作为行为变更单独评估；R01 的旧实现样本与跨链路回归见[需求与证据](../../1_delivery/managed-markdown-parser/requirements_2026-10-10_R01.md)。

## 项目 Map 自动索引

引用路径解析中的行号后缀与 Windows 盘符正则按 Hook 进程首次使用时编译，并在该进程内复用；不缓存路径、文档正文或索引输入。源码事件仍触发原有严格刷新，以便同时发现外部 AGENTS/文档修改。显式离线性能入口为 `scripts/tests/src/project_map_performance.rs`，只测隔离项目的 Stop 索引路径及进程开销，不代表完整 Codex 回复耗时。R04 的基线、输出等价性与复测结果见[交付卡](../../1_delivery/project-map-refresh/requirements_2026-10-11_R04.md)。

`.runtime/bridgeforge-codex/find-doc.map.md` 与 `.runtime/bridgeforge-codex/sync-docs.map.md` 是骨架内生的本地运行时索引，禁止手工维护或加入 Git。`find-doc` 索引只从实际生效的根/嵌套 `AGENTS.md` 标题、作用目录和明确代码词建立主题到指令源的关系；`sync-docs` 索引只接受设计文档中明确引用且磁盘真实存在的源码路径。目录同名不是语义证据，无法证明的关系不进入 Map，由 Skill 继续搜索 fallback。

`PostToolUse` 对相关输入只在 `.runtime/bridgeforge-codex/` 写脏标记；`Stop` 合并重建，`SessionStart` 兜底校验，两个 Skill 在读取前调用 `bridgeforge-hook project-map ensure-current`。生成文件携带 schema 与输入 SHA-256；相同输入逐字生成相同内容，目标字节未变化时不得重写。生命周期 Hook 的维护成功与 no-op 都不输出；严格 Skill 入口失败且 fallback 也不能完成任务时才向用户报告真正阻断。

`.codex/rules/*.md` 与 `.codex/memory/**` 的语义迁移已退役，不再专门扫描、解析或生成目标。兼容更新保留，重建通过普通项目文件盘点和 `PreservationManifest` 明确保留或删除；不存在固定退役的 Memory 派生文件。现有资产哈希、漂移检查、确认和回滚合同继续适用，不把功能退役视为项目文件删除授权。

## Current-only 事务

```text
probe canonical commit + reuse/stage product source
  -> prepare public artifacts + commit verified user-level installation
  -> identify project
  -> build deterministic actions + aggregate fingerprint
  -> confirm project preservation/deletion decisions when required
  -> Apply owns the necessary replan/fingerprint check (no extra Skill replan)
  -> internal payload preparation + capture prepared public bytes
  -> project lock + input verification and rollback snapshot from the same read
  -> combine latest assets / generated binary + receipt / selected project assets
  -> remove explicitly approved or contract-owned retired assets
  -> verify actions + preserved knowledge
  -> config health + text hygiene validators
  -> verify prospective current baseline on real disk
  -> write .codex/.bridgeforge_codex_version last
  -> verify unchanged validated contents + final stamp
```

可捕获失败恢复本事务能够证明归属的改动；外部编辑保留并报告回滚未完成。Apply、`$git-sync` 与 pre-commit 复用 baseline 规则，规划不额外执行完整健康检查。编码门禁通过 NUL 文件列表与 Git 原始 blob 检查实际 index 内容，不能以工作区内容代替暂存字节；项目 Skill 检查覆盖 `.codex/skills`，工厂额外检查共享 `skills`。pre-commit 只读检查 worktree 与 Git index，
不得生成文件或执行 `git add`；`$git-sync` 在写入前生成完整 `SyncWritePlan`，并在提交前失败时
恢复自动写入和完整 index。工厂 `git-sync` 的计划先将版本变更投影到仓库外快照，渲染 manifest，按锁定 Cargo 配方构建 Hook / CLI 并生成实测收据；完整计划包含版本、Cargo manifests / locks、CHANGELOG、三份清单、二进制和收据。构建后复核原始输入、源码文件集合、目标原值、仓库身份及 index，任何漂移均阻断写入。安装后先验证含生成产物的完整 baseline，再暂存和提交。构建与安装持有 project-sync 锁，整个同步仍持有 Git common-dir 锁；两者均由系统句柄持有，进程退出释放。

Git 拉取上游与实际推送目标分别判定：上游负责快进更新；推送是否必要和最终 ahead/behind 使用实际 push 目标。两者不同时，先刷新实际推送 remote，再验证目标；仅上游 `0/0` 不能报告 synced。批量暂存前的敏感文件保护使用不折叠目录的 NUL 状态列表，覆盖新目录内的文件。

Windows 上当前 CLI 的运行映像不能直接覆盖：新程序先写入同目录临时文件，旧映像移入已被 Git 忽略的 `.runtime/bridgeforge-codex/git-sync-images/` 后安装。旧进程继续执行，不启动额外 shell 或清理进程；后续工厂同步只清理该目录中名称格式正确且内容哈希匹配的旧映像，仍被占用者留到下次。安装失败立即移回原映像；提交前失败按原字节恢复二进制及其余自动资产。该缓存不进入提交。

同版本修复也从可信产品目录重新生成目标，不信任项目内合同自证 ownership；已有 whole 资产替换须确认。版本戳内容未变化时仍可应用其他已确认动作，最后验证当前唯一版本戳与完整基线。

公共 Rust Hook 在用户级准备时构建或复用；项目预览零构建，Apply 只安装已验证产物。构建、环境或自检失败在用户级提交前阻断；公共二进制与收据随项目配置同事务安装。日常公共 Hook 使用项目程序，不依赖产品源 home 或现场编译；项目业务 Hook 使用自己的独立工程。

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

- 根 `AGENTS.md` 公共区由产品管理；项目区由项目所有并按既有合同保留，与 latest 公共区确定性组合。
- `.codex/hooks.json` 只允许 canonical managed handler 与已确认的项目 Hook 注册；项目注册
  必须与一个 `.codex/hooks/project_XXXX/` 自包含业务 Hook 目录成对；既有业务 Hook 不限制语言，整体逐项确认保留或删除，未知 managed ID 阻断。旧 Rule／Memory 不再生成 Hook 包。
- 普通未知文件必须按精确路径确认保留或删除，禁止读取旧合同决定所有权。`project_` 文件名前缀不会把普通文件变成 Hook 目录；此类脚本确认删除后，已有 `hooks.json` 中匹配其完整路径的注册一并退役，文件与注册同一事务回滚，相似文件名注册保留。链接、非普通目录的项目 Hook 包、未知 Rule 格式仍阻断。
- schema 4 merge/Markdown/region/AGENTS 都携带当前可验证 projection；真实下游不存在
  `templates/**` 时也不得跳过。
- 普通骨架升级不语义改写项目 Skills 或旧 Rule／Memory；项目资料仍按所属资产保留或经明确决定处理。
- 项目 `find-doc` / `sync-docs` 映射在 project-sync 事务内按精确路径登记为 required-preserve；事务完成后的受管 Hook 可按自动索引合同重建，禁止扩大为 glob ownership。
- Claude、switch、project finalizer 与 harness parity 不属于当前产品面，也不保留识别谱系。
