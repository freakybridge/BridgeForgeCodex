# Codex 原生指令架构与 Rule 退役映射

## 决策

信息放置合同以根 `AGENTS.md` 的“信息放置”和“文档与资产边界”为唯一事实源；本文只记录 Codex 原生加载原理和历史迁移映射，不复制运行时规则。

Codex 启动时发现全局指令，并从项目根目录沿路径读取到当前工作目录；每层优先使用 `AGENTS.override.md`，否则使用 `AGENTS.md` 或配置的后备文件名。根目录启动的任务不会因为后来处理某个子目录就保证动态加载该目录的指令；根索引只能提供发现线索。Skill 先通过元数据被发现，选中后读取正文，reference 按入口条件读取。`.codex/rules/*.rules` 是命令执行策略，Markdown frontmatter `paths:` 不是指令加载机制。bridgeforge-codex 不实现自研加载器。依据：[官方 AGENTS 说明](https://learn.chatgpt.com/docs/agent-configuration/agents-md)、[官方定制与 Skill 说明](https://learn.chatgpt.com/docs/customization/overview)。

根 `AGENTS.md` 使用两个精确 marker 区域：BridgeForge 公共区由产品维护并按发布 hash 校验，项目级专区由下游完全所有并在更新中逐字保留。下游只能在项目专区、项目自有嵌套 `AGENTS.md` 与项目自有 hook 中增加约束；公共区修改会在编辑后提示、pre-commit 和同步计划中 fail-closed。旧无 marker 项目不再由同步器自动分类迁移；缺少或损坏 canonical marker 时，必须保留原文件、旧 rule 与旧版本戳并零写阻断，等待项目明确适配。

## Agent 路由责任链

Agent 路由不使用中央映射文件。根 `AGENTS.md` 是默认执行与委派红线的唯一 owner；没有显式委派的阶段由主对话执行。每个 `SKILL.md` 只负责本流程的阶段划分，并在需要委派时点名已存在的 Agent 角色；`.codex/agents/*.toml` 只定义角色职责、工具与安全边界。Codex 原生运行时负责创建 Agent、等待结果、续接指令和汇总，bridgeforge-codex 不实现第二套调度器。

工厂测试核对共享 Skill 的已用角色与分发登记。同步器只检查退役角色在项目 Skill、根 AGENTS、配置及其他角色文件中的引用，发现后生成 gap、零写保留；尚无通用角色引用检查器。角色职责不代表 model / effort 档位。

## 逐节语义映射

| 旧位置 | 新位置 | 处理理由 |
|---|---|---|
| `architecture.md` §1-§3 | `AGENTS.md` 项目专区“项目架构红线 / 项目目录地图” | 职责/依赖/数据流是项目事实；通用模板只要求项目填写，不泛化业务示例 |
| `architecture.md` §4 | `AGENTS.md` 公共区 | 重写等价性是常驻红线 |
| `architecture.md` §5、`modules.md` | `AGENTS.md` 项目专区；本文档 | 保留职责/依赖方向；Python 目录只作参考 |
| `anti_fabrication.md` R1-R5 | `AGENTS.md`“修改与证据”；`antifabrication-framework.md` | 核验/禁编造/立即更正为证据红线；案例留文档 |
| `debugging.md` §1-§5 | `AGENTS.md`“修改与证据”“排障与验证” | 根因、影响面、外部副作用必须常驻 |
| `debugging.md` §6-§11 | `AGENTS.md`“排障与验证”；`$escalate` / `$debate` | 反循环和量化保留；调试 SOP 由 skill 执行 |
| `workflow.md` §1-§4 | `$develop` / `$summary` / `$sync-docs` / `$archive-scan` | 任务流程不常驻占用上下文 |
| `workflow.md` §5-§8 | `AGENTS.md`“文档与资产边界”；`codex-project-operating-guide.md` | 文档红线常驻，目录/SOP 按需查阅 |
| `workflow.md` §9 | `AGENTS.md`“文档与资产边界”；`codex-project-operating-guide.md`「版本域隔离」 | 业务 `VERSION` 与骨架 stamp 是独立生命周期，禁止互相代替 |
| `workflow.md` §10 | `codex-project-operating-guide.md`「大版本依赖升级 Spike」 | 体验驱动的大版本升级先在主项目外验证，用户确认有效后再升级 |
| `portability.md` §1-§4 | `AGENTS.md`“授权与协作”“工具与运行环境”；`codex-project-operating-guide.md` | 环境和依赖遵守项目规范，安装细节按需 |
| `portability.md` §5 | hook / pre-commit | 编码、注册、dogfood 是机器可判契约 |
| `anti_drift_hooks.md` | `AGENTS.md`“授权与协作”；`codex-hook-signals.md`；`$focus` | Clarify 与 Focus 自动 Hook 均已退役；澄清由 Agent 依据常驻红线语义判断，手动任务锚由 `$focus` Skill 承担 |
| `meta_rule_design.md` | `AGENTS.md`“信息放置”；本文档 | 保留单一事实源/可机判交 hook/说明进 doc；删除伪 path 加载模型 |
| `bridgeforgecodex-product-change.md` | 根 `AGENTS.md` 项目专区“项目架构红线” | 工厂发布红线真正常驻，且不下沉普通项目 |

## 下游退役

所有 legacy Markdown Rule 必须逐源文件读取并确认完整迁移包；禁止依据历史 hash 自动删除或改扩展名冒充命令策略。全部确认前零写入，确认后的目标与最新基线、源删除在同一可回滚事务完成；有未解决 gap 时禁止写新版本戳。
