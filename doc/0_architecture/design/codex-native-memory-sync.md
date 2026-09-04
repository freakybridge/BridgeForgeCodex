# Codex 原生 Memory 同步架构

## 结论

Codex 原生 `~/.codex/memories/` 由 Codex 官方机制生成和注入。BridgeForge 同步器不解释其
内部语义、不创建或编辑正文，只在用户明确启用后把整棵目录作为不透明字节快照同步到固定
私有 GitHub 仓库，用于多台受信电脑之间的最终一致恢复。

Agent 的只读分析与同步器职责分开：`$summary` 检索相关记忆后阅读命中正文，必要时追溯
历史记录，再结合当前上下文和现行约束提出 Rule / Hook / AGENTS.md 建议。流程以
[`summary` 的记忆阅读与建议流程](../../../skills/summary/references/deep-steps.md)为准；
它不修改原生 Memory，也不因提出建议而自动修改项目约束。

项目 `.codex/memory/` 不属于本架构。新骨架不得创建、注入、检索或写入项目 Memory；下游
已有目录由 `$bridgeforge-codex` 逐文件迁往其他项目资产；确认后的迁移与删除属于同一事务。

## 组件与数据流

```text
Codex 官方生成/读取 ~/.codex/memories/
  -> 用户级生命周期 Hook 只登记 pending
  -> 用户级受管 bridgeforge 二进制启动或复用单一隐藏 worker
  -> Rust memory-sync 读取 opaque bytes 与 SHA-256 manifest
  -> 以 last-synced commit 做逐路径三方合并
  -> 以远端 HEAD 为父创建普通 commit 并 fast-forward 推送
  -> 其他电脑在下一次 worker 中合并或恢复
```

- `bridgeforge memory-sync` 是唯一同步实现；用户级 Hook 只负责触发，不解释 Memory 正文。
- Windows `commandWindows` 直接调用用户级受管 `bridgeforge.exe memory-sync hook-run`；隐藏 worker
  使用 `CREATE_NO_WINDOW` 与 detached process flags，配置中禁止持久化项目路径或语言运行时。
  旧 wrapper 和脚本命令只用于识别并迁移历史 handler，不是当前正式入口。
- 用户级 Hook merge 必须保留第三方 handler；BridgeForge 只能替换内容完全匹配的受管旧
  handler，遇到人工漂移时 fail-closed。

## Git 与合并合同

- Memory 文件按 opaque bytes 计算逐文件 hash 和整树 digest；禁止依赖内部 schema。
- 恢复只使用已核验 manifest 声明的文件字节，暂存后再次核验再替换原目录；未声明的缓存、锁和临时文件不进入恢复目录。声明文件损坏时保留原目录并停止。
- 冲突决议先核对 captured local；自动合并和恢复在发布前、替换前及移走原目录后复核本地指纹。期间新增或变化的本地内容导致阻断，必须重新取证。被移走的原目录保留为 memories 同级 `.memories.before-sync.<pid>.<counter>.<timestamp>.tmp`，不自动删除，供恢复或排查晚到写入使用；这不属于新的 Memory 内容源。
- 临时读取仓库和发布仓库都必须关闭 `core.autocrlf`，并禁止 attributes、clean/smudge 或
  其他换行转换改变 LF/CRLF。通过 `hash-object --no-filters` / index plumbing 写入原始 blob，通过 `cat-file` 读取原始字节，并在发布前核对 Git 存储字节与 manifest。
- 内容无变化必须 no-op；内容变化必须形成以远端 HEAD 为父的普通 commit，禁止 parentless commit 或 force-push 覆盖历史。
- `last-synced.commit` 是三方基线：不同路径双机修改自动合并；同路径只有单边修改采用修改版；同路径双边修改停止并保存 local / remote 两份。
- 旧 parentless 历史首次没有可信基线且两侧均变化时形成 bootstrap conflict，禁止猜测整树新旧。
- 冲突形成后若远端 HEAD 发生变化，只有新远端内容逐字节等于冲突包中的 captured local 时，才允许把已确认决议重放到新 HEAD；任何其他变化必须停止并重新取证。
- 任一正常 `push`、`restore`、`merge` 或 `noop` 完成后必须清除过期 active conflict；冲突证据包继续保留用于审计。
- 本地目录不存在且远端是合法空 manifest 时返回 `noop`、清除对应 pending、更新收据，
  但不创建空的 `~/.codex/memories/` 目录。
- 本地有文件而远端为空快照时不得静默删除本地内容；损坏 manifest、digest 不一致、symlink、
  junction 或 reparse point 必须 fail-closed。

## 授权、健康与失败语义

- 远端地址按完整协议、主机、owner 和固定仓库名解析；只接受 github.com 的 HTTPS 或 Git SSH 地址，不用字符串包含/后缀判断主机。非 GitHub、本地路径、URL 内嵌凭证或额外路径均阻断。
- 授权登记与运行时使用同一身份判定；同步和冲突恢复前核验 Git 实际读取地址及 GitHub private 状态，推送前逐个核验实际 push URL。Git URL 改写不能改变已授权仓库。
- GitHub CLI 查询使用完整 github.com URL，不依赖 GH_HOST；身份、私有性或查询结果不确定时停止同步，不提供跳过私有性检查的生产开关。

- 未获用户明确同意时，禁止启用开关、创建仓库、安装同步 Hook 或写入拒绝之外的配置。配置编辑使用 TOML 结构解析，保留注释并在写入前验证，禁止重复创建带注释或引号的已有表。
- `gh` 登录失效或查询失败时，以非交互 `git credential fill` 读取现有 Git 凭证，仅通过校验子进程的 `GH_TOKEN` 环境变量重试固定 github.com 仓库的 private 查询。禁止写入凭证、修改登录配置或回显凭证/原始诊断；没有可用凭证、`gh` 缺失、重试失败或结果非 private 时停止。
- `hookInstalled=true` 只作为历史兼容字段并等同 `hookConfigured`；`hookDispatchObserved` 证明 Codex 至少进入过当前 handler，只有迁移、授权、pending 登记和 worker 启动完成后才写 `hookRuntimeVerified` 所需收据；`busy` 不等于同步成功。
- SessionStart、Stop 与 SessionEnd 只登记需求并启动或复用隐藏 worker；单 worker 消费同步期间到达的后续需求，退出释放后再次检查 pending，避免遗漏尾部事件。死亡 PID 与受管临时目录可自动验证并自愈；五分钟未完成必须进入 `degraded` / `failed`。
- 同步、pending 队列和用户 Hook 配置使用操作系统持有的排他文件句柄；锁文件保留，进程退出即释放所有权。禁止按文件年龄删除锁，避免并发回收删除新持有者的锁。运行授权必须同时验证 ledger 与状态目录下的 `remote.txt`。显式 `--remote` 也必须与授权一致；`--memories` 和 `--state-dir` 必须解析到所选 Codex home 下的固定目录。
- 从历史 `.bridgeforge-codex/memory-sync` 升级时，只允许一次性迁移与 ledger 相符的 remote、通过 manifest/digest 验证的完整基线包、合法 pending 和其引用的当前冲突包；禁止覆盖有效新状态、复制 worker/锁/临时目录/健康状态或整段冲突历史。完成标记必须保留源指纹，避免旧冲突在新目录解决后被再次导入。
- `memory-sync status` 必须纯只读，不得因查询创建目录、更新健康状态或消费告警；活动告警持续报告，直到显式确认。同一失败或冲突的主动通知由显式运行路径负责。同步失败
  不得阻止 Codex 官方 Memory 的生成、注入或正常会话。
- 合法空快照、非空快照、损坏远端、换行保持、并发 lease、wrapper 入口和第三方 Hook 保留
  都必须由隔离测试覆盖；真实安装或 GitHub 状态只能由真实 runtime 收据证明。

## 不支持范围

- 同步器不总结、分类或整理原生 Memory 正文；Agent 对已有正文的只读阅读和分析不属于同步器操作。
- BridgeForge 不创建、编辑或删除原生 Memory 正文；快照恢复继续遵守上述字节核验与事务合同。
- 禁止把原生 Memory 与项目 `.codex/memory/` 合并、junction 或逐文件拼接。
- 加密与安全擦除不属于当前模型；需要时必须另开设计。

实现历史和现场收据见
`doc/1_delivery/memory-rule-organization/requirements_2026-08-14_memory-governance-native-sync.md`、
`doc/2_bugs/BUG-codex-native-memory-empty-snapshot-reconcile.md` 与
`doc/2_bugs/BUG-codex-desktop-native-memory-powershell-hook-not-entering-python.md`。
