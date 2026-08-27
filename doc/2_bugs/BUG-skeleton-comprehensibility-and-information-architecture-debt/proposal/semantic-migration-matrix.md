# 第十一版语义迁移账本

> “映射完成”只表示已有唯一 owner；“静态验证通过”也不表示真实骨架已安装。现行来源由 hash 锁定并保留行号，候选 owner 只用稳定文件与章节名，避免改稿后漂移。

| 现行来源 | 现行语义 | 第十一版稳定 owner（文件 / 章节） | 加载时点 / 验证门 | 映射 | 静态验证 | 安装 |
|---|---|---|---|---|---|---|
| `AGENTS.md:4` | 公共区不可直接改，项目规则进项目区或嵌套指令 | `template/AGENTS.md`「BridgeForge 公共区」 | 根常驻；candidate instruction Hook | 完成 | 通过（自动合同） | 未安装 |
| `AGENTS.md:10-11` | 等价迁移和完整调用链 | `template/AGENTS.md`「交付与证据」 | 根常驻；语义 sentinel | 完成 | 通过（自动合同） | 未安装 |
| `AGENTS.md:15-20` | 明确结论、禁空泛、审查/排障/架构、执行和交付格式 | `template/AGENTS.md`「交付与证据」 | 根常驻；独立 cold-reader | 完成 | 自动合同通过；独立复评中 | 未安装 |
| `AGENTS.md:24-33` | 工具边界、二次验真、危险处证据、当次核验、自错、venv、编码、控制台 | `template/AGENTS.md`「交付与证据 / 环境与安全」 | 根常驻；Hook + sentinel | 完成 | 通过（自动合同） | 未安装 |
| `AGENTS.md:39-50` | 指令承载、rules 限制、业务/骨架版本 | `template/AGENTS.md`「先找对位置 / 版本与升级」；操作顺序归 `$bridgeforge-codex` | 根常驻；真实 schema/Hook | 完成 | 通过（自动合同） | 未安装 |
| `skills/summary/SKILL.md:139-140` | 验收模式仍会生成带 `paths:` 的 Markdown Rule | `architecture/codex-native-instruction-architecture.md`「`$summary` 的长期红线路由」；`contracts/implementation-patch.md`「Summary Skill」 | 调用 `$summary 同意验收`；Skill 测试、manifest 与安装镜像 | 完成 | 通过（自动合同） | 未安装 |
| `AGENTS.md:54-58` | 双 Memory 隔离、检索、topic 门槛和验收 | 隔离/索引禁改归 `template/AGENTS.md`「协作与项目资料」；流程沿用现行 `$find-memory`/`$summary` | 根常驻；调用 Skill | 完成 | 通过（现行 Skill 未改） | 未安装根文本 |
| `AGENTS.md:64-68` | 五层、delivery_layout、禁散落/改层、索引同步、测试位置 | 红线归 `template/AGENTS.md`「协作与项目资料」；说明归 README「文档怎么放」 | 文档任务读 README；structure gate | 完成 | 通过（自动合同） | 未安装 |
| `AGENTS.md:74-76` | 项目与用户级 Skill 位置、常用入口和发现方式 | `template/AGENTS.md`「先找对位置」；README「指令和说明分别放在哪里」 | 根常驻；当前会话 Skill 列表 | 完成 | 自动合同通过；独立复评中 | 未安装 |
| `AGENTS.md:80-86` | 换电脑、新机 clone 或重装时进入换机流程，并按主语言恢复依赖 | 触发义务归 `template/AGENTS.md`「BridgeForge 公共区」；步骤归 README「第一次 clone、换机或重建环境」 | 根常驻触发；执行时读 README | 完成 | 通过（自动合同） | 未安装 |
| `AGENTS.md:90-95` | 模型继承、主对话、子 agent 与三个主对话 Skill | `template/AGENTS.md`「协作与项目资料」 | 根常驻 | 完成 | 自动合同通过；独立复评中 | 未安装 |
| `AGENTS.md:97-115` | clarify/focus 触发、可选/推荐路线、develop 分流、单题/收敛、spinoff/todo 和修改披露 | 根触发归 `template/AGENTS.md`；细则归 `shared-docs/codex-hook-signals.md` | 收到信号时读 reference；真实 Hook；semantic ID | 完成 | 通过（自动合同） | 未安装 |
| `AGENTS.md:123` | 主观体验先采证据再量化 | `template/AGENTS.md`「任务控制与排障」 | 根常驻 | 完成 | 自动合同通过；独立复评中 | 未安装 |
| `AGENTS.md:127-137` | 独立验证、陌生模块、失败升级、Bug 假说/影响面、性能、自改审计 | `template/AGENTS.md`「任务控制与排障」 | 根常驻；独立审阅 | 完成 | 自动合同通过；独立复评中 | 未安装 |
| `AGENTS.md:144` | 工厂项目区逐字所有权 | `factory/AGENTS.md`「项目级专区」 | 工厂根常驻；项目区 byte fixture | 完成 | 通过（自动合同） | 未安装 |
| `AGENTS.md:148-163` | 工厂产品源、传播四问、playbook、dogfood、地图、版本、资产、事务、Skill 分发、Bug 证据、发布门 | 跨目录结果归 `factory/AGENTS.md`；操作归现行完整 `$bridgeforge-codex`；同步器实现、Skill 分发和 Bug 证据归三个嵌套 AGENTS | 工厂根路由；调用 Skill；候选 factory gate；真实 schema/fixture | 完成 | 通过（自动合同） | 未安装 |
| `AGENTS.md:167` | 禁业务污染与真实下游授权 | `factory/AGENTS.md`「项目业务与安全红线」 | 工厂根常驻 | 完成 | 自动合同通过；独立复评中 | 未安装 |
| `AGENTS.md:171-175` | 工厂目录职责 | `factory/AGENTS.md`「项目目录地图」 | 工厂根常驻；structure gate | 完成 | 通过（自动合同） | 未安装 |
| `AGENTS.md:177-183` | 工厂三条真实快速命令 | `factory/AGENTS.md`「项目快速命令」 | 工厂根常驻；raw section contract | 完成 | 通过（自动合同） | 未安装 |
| `AGENTS.md:187` | 嵌套指令索引 | `factory/AGENTS.md`「目录级 AGENTS 索引」及公共区目录读取路由 | 工厂根常驻；factory-only contract | 完成 | 通过（自动合同） | 未安装 |
| `templates/AGENTS.md:148,152,156,160` | Template 风控/合规、入口/配置/测试/文档和五类快速命令 | `template/AGENTS.md`「项目级专区」 | 下游根常驻；heading/sentinel | 完成 | 通过（自动合同） | 未安装 |
| `templates/AGENTS.md:162-164` | 下游目录级 AGENTS 索引的发现说明 | `template/AGENTS.md`「目录级 AGENTS 索引」 | 下游根常驻；精确占位注释合同 | 完成 | 通过（自动合同） | 未安装 |
| `codex-project-operating-guide.md:5-7` | 公共/项目区所有权、工作树与 staged 防绕过 | 红线归 `template/AGENTS.md`「BridgeForge 公共区」；机制说明归 README「指令和说明分别放在哪里」；真实 Hook/pre-commit | 根常驻；编辑/提交事件 | 完成 | 通过（候选入口模拟） | 未安装 |
| `codex-project-operating-guide.md:11` | 五层与 `$archive-scan` | README「文档怎么放」；根红线 `template/AGENTS.md`「协作与项目资料」 | 文档任务；structure gate | 完成 | 通过（自动合同） | 未安装 |
| `codex-project-operating-guide.md:15-17` | Python/Node/Rust、venv、配置、绝对路径和凭据 | 根 `template/AGENTS.md`「环境与安全」；README「第一次 clone、换机或重建环境」 | 根常驻；换机/依赖任务 | 完成 | 通过（自动合同） | 未安装 |
| `codex-project-operating-guide.md:21-26` | 版本隔离、同 classifier preflight、动态 G*、回滚和版本戳顺序 | 根只管版本域；操作唯一归 `contracts/bridgeforge-codex-skill-patch.json` 生成的完整 Skill；README 只解释入口 | 根常驻；真实 Skill 测试与 manifest；真实同步 fixture | 完成 | 通过（自动合同） | 未安装 |
| `codex-project-operating-guide.md:30-32` | 2–4h 外部实验、禁先改主项目/lockfile、禁只看 CHANGELOG、用户确认 | 根 `template/AGENTS.md`「版本与升级」；README「跨大版本升级先做小实验」 | 根常驻；升级任务读 README | 完成 | 自动合同通过；独立复评中 | 未安装 |

## 下游遗留 Rule 只读抽查

| 下游 | 磁盘事实 | 当前加载状态 | 迁移状态 |
|---|---|---|---|
| StratusAgent | 21 个 `.codex/rules/*.md`，19 个带 `paths:`，0 个 `.rules` | 不会原生按路径加载；根 `AGENTS.md` 有显式读取索引 | 未迁移，禁止宣称有效 Rule 已闭环 |
| CausisRiskSuite | 6 个 `.codex/rules/*.md`，全部带 `paths:`，0 个 `.rules` | 不会原生按路径加载；根 `AGENTS.md` 有显式读取索引 | 未迁移，禁止宣称有效 Rule 已闭环 |
| BridgePersonalAssist | 不存在 `.codex/rules/` | 项目约束已明确由根 `AGENTS.md`、Hook 或嵌套指令承载 | 当前无旧 Rule 迁移对象 |

该表是 2026-08-28 本机只读快照，不是批量下游验收。实施时仍必须重新盘点目标工作树、显式引用和项目自有差异。

## 机器合同

- overlay：`contracts/instruction-contract.json`；它以真实 schema 3 manifest 为基底，只声明精确下游资产替换和指针迁移。
- 可执行语义迁移：`contracts/semantic-contract.json`，逐项绑定来源行、目标文件和必需语义。
- `$summary` 旧 Rule 分支：来源 `skills/summary/SKILL.md:139-140` 由 hash 锁定，目标路由和实施动作由语义合同强制。
- 完整 Skill 候选：`contracts/bridgeforge-codex-skill-patch.json`；确定性 patch 后校验完整文件 hash。
- README 专属 append、其他 region 失败关闭：`contracts/region_migration.py`。
- 实施输入：`contracts/implementation-patch.md`。
- 自动验证：`contracts/validate_proposal.py`。
- 尚未验证：真实 Codex 嵌套加载、真实下游 update/rollback、正式发布、runtime smoke 和用户本人试读。proposal 的临时工厂/下游验证不能替代这些实施后证据。
