---
lifecycle: completed
validation_status: verified
---

# LF/CRLF 开发证据误判修复

用户已授权精简实施；原目标 60–90 分钟，固定受管检查超出估算后，用户明确选择继续完成收口。保留 StratusAgent、HoldemTrainer 修改前后的零写入 develop/release 预览、定向回归、独立审查和受管收口。不提交、推送、发布、安装或修改真实下游，不实现自动格式转换 Hook。

本次是通用产品修复，维护源为 source/lifecycle；公共投影与 dogfood 由工厂生成。受影响子包通过受管流程收口，根 VERSION 和正式 CHANGELOG 留待 release。

## 验收合同

- develop 和 release 共用内容摘要实现；仅 Git 证明等价的 CRLF/LF 表示允许归一化，保留真实内容、二进制、特殊属性和产物的严格判断。
- 新证据标记算法，旧记录只做可证明的兼容匹配；不改写历史哈希链，不因算法变更重复升版，缺证据时明确补验。
- 预览到执行的原始字节、文件成员及有效规范化规则变化仍阻断。
- 两个真实项目以修改前基线比较，消除已确认换行误判，不把未知环境或依赖失配自动放行。

## 当前证据

基线收据在 `.runtime/eol-fix/`：Stratus 10 包 pending，HoldemTrainer 8 包 pending；两项目原本 release 均阻断。预览前后分别核对 2924、833 个路径（含索引和关键账本），Git 状态和原始字节摘要未变。BridgeForge 自身在修改前已有 core、factory、lifecycle、skeleton、workflow 的环境或检查输入证据失配。

## 实现与验证

公共内容实现使用 Git 属性与 filtered/unfiltered blob 标识证明 EOL 等价，批量查询避免每个 CRLF 文件启动两次进程。新记录使用 evidence_schema=4，记录规范化规则；旧记录保留原哈希链，按原始或经证明的 LF 表示完整匹配。无实际 Git 源码/mode 差异、且无法证明旧 CRLF 表示的同一已发布包停止升版。事务仍比较原始字节；属性规则变动也会使执行分析失效。

- 最终换行专项 `eol_`：6 passed，涵盖双向转换、源码/检查/ignored 构建输入、旧 schema3、旧 CRLF 防误升版、属性漂移、二进制、裸 CR 及特殊 filter 名称。日志 `.runtime/eol-fix/eol-final2.log`。
- 较宽 `development_tests` 在早期候选上为 56 passed / 1 failed；唯一失败为删除源码提交后遗留规范化路径，已修正。最终完整名称补测 1 passed，日志 `.runtime/eol-fix/deleted-source.log`。不将早期失败运行改写为全绿。
- 独立 Agent `/root/eol_review` 最终静态复核无新阻断；此前 release 分析未绑定属性、旧证据保护误拦退役、特殊 filter 名称绕过三项均修正并补覆盖。
- 两项目候选 dry-run 对比见 `.runtime/eol-fix/comparison.json`。从基线到最终候选，HEAD、Git 状态及原始文件摘要一致；每次预览前后也均未变。Stratus 已确认的六个源码换行误判消除，其他内容/依赖/检查输入/环境差异仍阻断；HoldemTrainer 的内容和检查输入差异消除，八包剩余原因均为 environment。未绕过验证，也未在下游补跑 develop。
- 首个逐文件候选耗时 Stratus 384.74 秒、HoldemTrainer 95.17 秒；改为批量后为 131.51 秒、50.74 秒。原基线为 58.79 秒、25.66 秒，最终仍有额外开销；测试与构建并发，不能当作严格独占性能基准。候选工具 SHA-256 及输入快照已记录。
- 最终源码的锁定 release 构建成功；公共投影和 dogfood 已由受管 manifest 入口生成。
- 首次受管 apply 的 `lifecycle-unit` 失败，候选版本由工具回滚，未遗留事务；日志 `.runtime/eol-fix/closure-apply.err`。入口只保留 stderr，未输出完整失败断言。单项复现确认旧测试 helper 用底层 capture 构造 release 指纹，遗漏新增规则绑定；另经独立复核发现外部分析测试同类遗漏，已补齐公开 gate 的只读绑定，保留原断言。现有 lifecycle 检查增加 `--nocapture` 以保留失败详情，未删减检查。后续以重跑收据为准，不把中断的诊断运行计为通过。

## 受管收口结果

第二次 `develop --apply` 成功返回 `development-closed`，收据 `.runtime/eol-fix/closure-apply2.json`。七包既有受管检查全部通过，工具追加七条 schema 4 记录，事务已清理；未手工修改版本或认证账本。首次失败后的两项补测分别通过，日志为 `fingerprint-fixed.log`、`external-analysis-fixed.log`；独立复核确认测试断言未削弱。

| 包 | 原版本 | 最终版本 | 原因 |
|---|---|---|---|
| lifecycle | 0.1.3 | 0.1.4 | 修复开发与发布的换行证据误判 |
| factory | 0.1.2 | 0.1.3 | 声明与公共 CLI 一致的换行证据能力 |
| checks / core / memory / skeleton / workflow | 0.1.0 / 7.0.1 / 0.1.0 / 0.1.0 / 0.1.1 | 不变 | 仅刷新适用验证证据 |

主程序 VERSION 保持 7.0.4；未配置稳定合同的 0.x 包保持开发阶段，未自动宣称达到 1.0.0。

最终声明入口 `cargo run --locked --release --manifest-path Cargo.toml --bin bridgeforge-factory -- build-assets --factory --project-root .` 返回 prepared，三个产物均完成自检，日志 `.runtime/eol-fix/runtime-prepare-final.log`。随后 `manifest --root . --check` 返回 ok / changed=false，产物只读检查返回 current / reused=3 / built=0。

最终工具 `develop --root . --preview` 返回 `nothing-to-close`，`affected_packages=[]`、`writes=[]`、`pending_reasons={}`，收据 `.runtime/eol-fix/closure-final-preview.json`。源码、公共传播、dogfood、fixture、真实下游只读预览证据已取得；runtime 仅覆盖受管产物自检，未执行真实下游 apply/release 或自然运行。最后仅更新本记录与文档索引，不改变包的源码、检查输入或产物，复用该有效预览证据。

本轮实际约 3 小时 45 分钟，明显超过精简估算。原因包括固定受管检查成本，以及漏改两处测试指纹辅助入口导致首次收口失败并重跑；原估算不准确。

上述为开发交付时状态：当时等待用户验收，尚未提交、推送、正式发布或安装到真实下游。两个下游剩余证据阻断继续保留，不宣称已经可发布。

## 用户验收与发布授权（2026-10-27）

用户在本对话明确调用 `$summary accept`，验收 LF/CRLF 摘要修复、保守旧证据兼容、两项目零写入 dry-run、必要检查及完整受管开发收口；同时调用 `$git-sync release`，单独授权当前 BridgeForge 改动的受管提交、推送和主程序发布。实际发布版本与同步结果以随后生成的受管收据为准，不以授权代替成功记录。

本次验收复用既有七包 passed 记录、最终 `nothing-to-close` 预览、产物自检与一致性收据、专项回归和独立复核；不因总结重新测试。下游实际 apply/release、安装和自然 runtime 仍未验证。原需求无对应既有 TODO，不补造已完成编号；TODO-035 保留待确认，新增独立后续 TODO-041，详见下节。当前主题仍有待确认的后续事项，整 topic 无归档候选。

## 可复用经验与后续事项

- 内容身份和事务防漂移分别判断：Git 证明的 LF/CRLF 等价可复用内容证据，执行计划仍绑定原始字节、属性、配置与目录成员。不能用文本归一化证明所有历史摘要都等价。
- 修改指纹绑定时同时核对公开入口和测试辅助入口。本轮两个旧 helper 直接使用底层 capture，遗漏 release gate 的规则绑定，导致完整受管检查失败；修正辅助入口后原断言保留并通过。
- 运行时准备、开发收口、用户验收和发布保持各自证据。预计时长必须纳入实际受管检查成本；本轮精简估算未兑现，不将检查成本隐藏为“收尾”。
- TODO-035：本轮再次遇到失败摘要丢失 stdout。`--nocapture` 仅改善当前生命周期测试的错误输出，通用 `run_check` 诊断未修复，不结算该事项。
- TODO-041：换行规范化增加了真实项目预览耗时。Stratus 基线 58.79 秒、最终候选 131.51 秒，HoldemTrainer 25.66 秒、50.74 秒；并发负载不同，不能据此量化独占性能回退。后续先做同条件对照，再决定优化，当前待确认，未实施。
- TODO-033：验收时用户级 TODO Skill 所引 `references/contract.md` 缺失，实际读取工厂 `templates/skills/todo/references/contract.md` 与 `source/checks/src/todo_contract.json` 完成核对；该模板仍有已登记的旧机器合同路径。沿用 TODO-033 跟踪导航完整性，不在 summary 修复 Skill 或用户级安装。

复用本轮已读的源码归属与公共投影历史，结合当前 AGENTS、实现和受管收据核对；上述边界已有合同和测试承载，不新增 Rule、Hook 或 AGENTS 候选，不修改 Memory。
