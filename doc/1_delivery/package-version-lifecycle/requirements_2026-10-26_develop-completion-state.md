---
lifecycle: completed
validation_status: verified
---

# develop 完成状态与受管收口

关联 TODO-037。用户于 2026-10-26 授权在 Forge 调查、修复并针对性验证，禁止修改 StratusAgent、执行下游安装、提交、推送或发布。当前目标是阻止“代码和构建完成但收口被省略”的交付状态误导，不预判 apply 有程序 Bug。

## 范围与预算

按 M 级处理：涉及 Skill 主入口、收口编排和文档状态表达，默认 45 分钟、20,000 新增 token 估算、0 个子 Agent、最多两轮验证；token 未实测，当前没有适用的周额度换算系数，不虚报比例。复用现有证据并运行与三类场景直接相关的既有 fixture，不跑无关 workspace 全量。若出现程序能力缺陷或需要扩大实现，先报告实际证据与影响。

传播四问：通用产品层 Skill 修复，维护源为 `templates/skills/develop/`；非工厂专属规则。由受管 manifest 生成器同步用户级分发登记；当前 dogfood 合同只投影 git-sync，不新增重复的本地 develop Skill，不修改用户级安装。主程序版本和 CHANGELOG 留待另行 release；若受管开发预览出现仅验证输入变化，按实际合同完成重验，不手动升包。

## 已核实的根因与程序边界

原始对话：[对比blackswan](codex://threads/01a17073-ffeb-7c63-ab2b-ef6c09038186)，实施 turn `01a17075-7620-71f1-b018-2657b08fda86`。读取实际用户消息、命令与结果确认：

- Agent 明确采用 develop 小修路径，并读取主 Skill 和 completion-release reference；6.0.0 自检声明具备 package-development 能力。
- `develop --preview` 返回 `development-analysis-required`，包含十个包。该 turn 没有执行 `develop --apply`。
- Agent 表示“版本收口预览还列出了其他包，因此本次不执行批量升版”；最终仅报告文案修改、构建和 DLL 标识核验通过、未升版，未说明开发未完成及 release 阻断。

因此已证实的是流程执行和交付状态缺陷：原指令已要求收口，Agent 实际停在 preview 且未报告开发未完成。S 路径“通常无需 reference”存在误读空间，但本案确实读过 reference，不能把漏读认定为本案原因。可修复的指令薄弱点是额外包缺少分流步骤、缺少以受管终态核对整体完成的交付检查，以及 M/L 文档状态没有明确把收口未完成留在待验证。不能据未调用 apply 的案例认定 apply 程序出错。

当前 `development.rs` 明确区分 analysis-required、planned、closed 和 nothing-to-close；仅 execute 成功追加记录并清除 journal 后才返回 closed。apply 要求分析覆盖全部待处理包，当前没有任务级局部 apply；这是范围处理限制，不在本次新增选择性事务。待处理包包括证据或产物失效，仅需重验并不等于必须升版。7.0.2 的历史证据/换行兼容修复已有独立证据，本次不重写该算法，不放宽 release。

原生 Memory 对本案例的 success 摘要仅覆盖代码和构建，没有记载收口缺口；本次以原对话和程序返回为依据，不据历史 success 推定完整开发成功，也不改写 Memory。

## 实施与验收标准

1. S 级轻量验证仍保留适用的受管收口与项目必需检查；入口明确引导 completion-release。
2. 额外包分为本次直接/连带影响、可证明的既有事项、疑似误判及未知；不统一升版，不擅造局部 apply，不把数量多当成工具错误。
3. 未取得有效收口结果时明确“开发未完成”、包范围、原始阻断和继续条件、已确认或尚未确认的 release 影响。
4. 完成结论依据实际 closed/有效 nothing-to-close、追加记录、最终输入和工程验证；恢复成功不是开发完成，预览退出码 0 不是通过认证。
5. 不新增伪造完成收据、全局 Hook 或 release 放行路径；不改变业务验证合同。

## 验证记录

### 三类场景

| 场景 | 程序证据 | 新 Skill 的静态走查结果 |
|---|---|---|
| 小修正常收口 | 正常 fixture 经 preview、apply 完成版本/记录事务，再次 preview 为 nothing-to-close；release 仅升级产品。未收口或收口后再改源码仍阻断 release | 轻量验证后继续受管收口，核对最终输入、包范围和记录；只报告实际终态，不以构建或 preview 退出码代替 |
| 预览出现额外包 | 修改 core 时调用方 b 重新验证但版本不变、level=0，五个无关包不新增记录；检查策略变化只重验、不重复升版 | 本次直接/连带影响、可证明的既有事项、疑似误判及未知分别处理；未获授权或未查明的额外包明确阻断事务，不批量强升或擅造局部 apply |
| 收口失败或中断 | 必需检查失败回滚 manifest/lock 且无成功记录；模拟中断保留 journal 并阻断预览；recover 恢复版本但不生成认证，外部编辑冲突保留；过期分析及空必需检查被拒绝 | 代码/构建结果与开发整体状态分开，报告开发未完成、失败阶段、阻断包及继续条件；recovered 不得当作 closed |

实际执行的受管 Cargo 入口为 `cargo test --locked --offline --config scripts/tests/factory-cargo.toml --manifest-path Cargo.toml -p bridgeforge-lifecycle --lib --target-dir .runtime/source-workspace-refactor/cargo -- <以下七个过滤器> --test-threads=4`：

- `package_development_closes_once_and_release_only_writes_product`
- `package_development_blocks_release_until_validated_and_after_new_source`
- `package_development_scoped_rust_build_revalidates_actual_core_consumers`
- `package_development_configuration_change_revalidates_without_bump`
- `package_development_failed_normal_check_rolls_back_without_closure`
- `package_development_recovers_interrupted_candidate_and_preserves_external_conflict`
- `package_development_stale_analysis_and_empty_checks_fail_closed`

实际结果：7 passed、0 failed、239 filtered out，耗时 186.41 秒。复用既有测试，没有新增仅匹配 Skill 文案的测试，也未跑 lifecycle 全套或 workspace 全量。这些是受控程序 fixture，部分必需检查由测试 runner 模拟成功、失败或 panic；它们验证真实收口状态机与事务保护，不等于 Stratus 业务构建或真实 Agent 行为通过。

### Forge 自身的受管结果

本次修改三个 develop Skill 文件；受管 manifest 生成器同步 `bridgeforge-codex-manifest.json` 中三条分发摘要。既有 dogfood 合同仅包含 git-sync，未新增本地 develop 镜像或修改用户级副本。Rust 源码、测试源码、release 门槛及项目检查配置均未改动。

修改后当前预览仅列出 factory：Skill 是其显式登记的检查输入。其他六包复用已发布认证。按当前受管入口在同一宿主生成完整分析、核对 `development-planned` 后执行 apply，实际执行 `factory-unit` 必需检查，不以七个定向测试替代它。

结果为 `development-closed`、`git_written=false`；factory 0.1.1→0.1.1，仅重验，不升版。账本 24→25 条，旧 24 条逐条保持原样；VERSION 7.0.2、CHANGELOG、Cargo.lock、配置、Git 索引和 HEAD 均保持不变。实际收口耗时 866.80 秒。收据：`.runtime/develop-completion-state/result.json`、`verification.json`；计划与分析在同目录。

当前源码工具准备返回 `current`、`reused=3`、`prepared=0`、`self_test_executed=true`，没有改写受管二进制。Skill metadata（目录集合 `templates/skills`）、manifest 零写入检查、TODO 合同与 diff 空白检查通过。首次单 Skill 路径参数被校验器视作目录集合根而拒绝，改用其实际要求的父目录后通过，未修改校验规则。

最终 `develop --preview` 返回 `nothing-to-close`、`affected_packages=[]`、`writes=[]`，确认当前收口有效。manifest 检查 changed=false，project-structure errors=[]，TODO errors=[]，产物检查为 current、reused=3、built=0，diff 检查通过。Git HEAD 仍为 `7ebfd42aa50357ea4f3bdb9b9ea8750771274893`，索引未暂存；diff 确认 source、测试源码、版本、锁和配置未变。本段仅补充验证记录，复用上述仍有效结果，不再重复完整检查。

### 限制与状态

程序 fixture 已通过，新增指令的三类场景已逐步静态核对；尚未在安装候选 Skill 后开启真实 Agent 对话验证稳定执行，也未模拟完整 Codex 调用链。Skill 指令不构成程序对最终回复的强制拦截，不能保证 Agent 永不违背。当前 CLI 仍要求分析覆盖全部待处理包，没有新增选择性 apply 能力；未获授权的既有问题仍可能阻断，但必须在开发交付时明示，不能延后到 release 才告知。

开发交付时未修改 StratusAgent、未执行下游或用户级安装、未提交推送发布。自然 Agent 对话、真实下游写入和业务 runtime smoke 均未验证。交付范围为 Forge 分发源修复、程序边界核验及自身受管重验。源码/程序能力无新增变更，产品传播仅更新分发登记，dogfood 无对应 develop 镜像，六类证据边界不互相替代。

本轮使用 0 个子 Agent、一次预定验证集；token 未实测。记录的测试与收口耗时不作为全任务端到端耗时或真实 Agent 效果指标。

## 用户验收与发布授权（2026-10-26）

用户明确调用 `$summary accept`，确认上述工程交付符合本次范围，TODO-037 验收完成并更新为已完成。验收复用七个定向 fixture、factory 受管重验和最终 nothing-to-close 的真实结果，不重跑测试或构建来补造验收。自然 Agent 对话效果仍未验证，验收不将 Skill 指令提升为程序强制保证，也不补造下游安装或运行证据。

用户同时调用 `$git-sync release`，授权接续当前 Forge 的受管发布、提交和推送。实际版本及发布结果以该事务收据为准；不包含 StratusAgent 写入或用户级安装。TODO-035 等既有独立待办不在本次结算，没有新增已证实的独立问题。

经验已沉淀在本记录的根因、状态边界和三类场景中：预览成功不代表认证成功，待处理包不等于批量升版，恢复成功不代表开发完成。复用已读原生 Memory 与原对话核对的结果，原 Memory 的 success 只覆盖代码和构建，不能作为收口证明；未修改 Memory。现有 Skill 与程序合同已承载本次修复，不另建议重复 Rule、Hook 或 AGENTS 约束。当前记录已具备验收依据，归档仅为候选；如需整理请另行调用 `$archive-scan`，本次未移动文档。
