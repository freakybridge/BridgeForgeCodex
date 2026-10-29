---
lifecycle: completed
validation_status: verified
---

# 四项目 7.0.x 到 9.x 临时升级工具

关联 TODO-047。

## 需求与授权

用户选择一次性升级工具及独立临时命令，并于 2026-10-29 明确授权：落盘并开始实施，在四个项目的隔离测试 worktree 中验证；暂不升级正式工作目录，不提交、推送或发布。

支持 StratusAgent、BridgePersonalAssist、CausisRiskSuite（本机目录 causis_risk_suite）、HoldemTrainer。现场版本分别为 7.0.5、7.0.4、7.0.4、7.0.5，受管资产合同均为 schema 4 current-only。目标是工具对应的 9.x 骨架，完成后临时入口可退役。不是继续提供 7.x 开发或发布运行时，也不恢复任意旧项目升级能力。

M 级，沿用 confirm 默认预算与最多两轮验证；主对话实施，迁移安全由一名独立 Agent 审查。token 未实测。若出现需重写事务或迁移业务数据的范围变化，重新评估预算与授权。

## 行为与完成标准

- 工厂独立命令逐项目操作，只接受四个明确仓库身份；工作目录名称本身不作为身份依据，允许同仓库的隔离 worktree。
- 普通 project-sync 的 8.0.0 支持下限不变。临时例外限 7.0.4/7.0.5，目标限 9.x；已升级项目重复调用不重复修改。
- 预览零写入；apply 必须消费当前预览指纹，执行前再次核验身份、来源与漂移。复用原同步事务、风险确认、备份回滚和版本戳最后写入。
- 保留项目业务代码、自有配置、AGENTS 专区、项目 Hook 及版本历史；升级受管公共资产，未知归属或冲突阻断，不伪造旧版本身份。
- 四个真实仓库的隔离 worktree 分别完成预览、升级及重复执行检查；核验正式目录未变化。非目标仓库、错误来源版本、漂移和失败保护有针对性回归。
- 不将升级成功等同于业务回归、业务发布或自然 Codex Hook 全链路验证。

## 实现边界与传播四问

1. 产品实现：工厂专属命令位于 source/factory；现有同步事务需要的显式、受约束来源接口位于 source/workflow/source/skeleton，具体以调查结果为准。源身份和项目名单不得进入公共模板。
2. 工厂专属四项目策略与通用事务分离，不使用全局环境变量开关，不改普通入口默认策略。
3. 受影响子包通过 develop 必要检查与受管收口升版；根 VERSION 和正式 CHANGELOG 留给另行授权的 release。
4. 通用源码变化由受管 manifest 同步公共投影与 dogfood；工厂命令不分发为下游公共命令。不修改 AGENTS/Hook 规则或用户级安装。

## 初始证据与验证安排

修改前工作区干净。`bridgeforge-factory develop --preview --packages factory,workflow,skeleton` 返回 task-development-current、affected_packages=[]；factory=0.5.0、workflow=0.3.0、skeleton=0.2.0，主程序=9.0.0。

| 改动风险 | 检查 |
|---|---|
| 例外泄漏到普通入口、错误仓库接管 | 工厂参数/身份负例，普通入口拒绝 7.x |
| 来源或计划漂移、失败后部分写入 | 原事务相关回归与临时路径漂移/回滚用例 |
| 项目定制丢失、旧资产残留 | 四个真实隔离 worktree 的升级收据、保护内容与 baseline 核验 |
| 产物及投影失配 | 受管准备、manifest --check、最终子包开发收口 |

## 进展

- 需求及实施授权已确认；应用内 create_worktree 不支持当前聊天之外的仓库，使用已授权的 Git detached worktree，正式目录仅增加 Git worktree 登记，不修改工作文件。
- 用户级 TODO reference 缺失，已读取仓库 templates/skills/todo/references/contract.md 与机器合同；沿用已登记的 TODO-033，不扩展本轮修复。
- 已实现 `bridgeforge-factory upgrade-legacy-seven`，策略仅在 factory；workflow 新增显式临时来源规划与身份快照，不修改 skeleton 的支持下限。
- 独立审查发现并修复发布配置链的重复旧版本阻断；四项目既有发布配置仅进行唯一键 JSON 解析及 schema_version/release_model 身份核验，原样保留，不宣称完整业务配置语义验证。Git symbolic HEAD 对应 loose ref/packed-refs 也纳入身份指纹。
- `cargo test --quiet --locked --offline --config scripts/tests/factory-cargo.toml --manifest-path Cargo.toml -p bridgeforge-workflow --lib --target-dir .runtime/source-workspace-refactor/cargo legacy_upgrade -- --test-threads=1`：提升权限后 3 passed，覆盖成功升级/重复规划、身份漂移零写入、故障回滚/错误版本；首次沙箱运行因 os error 5 失败，未计作通过。
- TODO 机器检查通过；独立审查修正复核未发现新的阻断问题。

## 工程验证与受管收口

真实测试目录为本仓库 `.runtime/legacy-seven-upgrade/{StratusAgent,BridgePersonalAssist,CausisRiskSuite,HoldemTrainer}`，均是对应正式仓库 HEAD 的 detached worktree。

| 隔离项目 | 来源 | 首次升级动作数（含公共产物） | 重复执行动作数 | 保护文件不变 | 安装 CLI baseline |
|---|---|---:|---:|---:|---|
| StratusAgent | 7.0.5 | 35 | 0 | 15 | passed |
| BridgePersonalAssist | 7.0.4 | 36 | 0 | 7 | passed |
| CausisRiskSuite | 7.0.4 | 35 | 0 | 6 | passed |
| HoldemTrainer | 7.0.5 | 35 | 0 | 7 | passed |

四项目均成功升级至 9.0.0、版本戳最后写入。保护项包括 AGENTS、业务 VERSION/版本配置/收口历史、Hook 配置、pre-commit 及已有项目 Hook 文件；不存在的文件不计入。核验脚本首次误把公共 `source/**/project_*.rs` 算入项目自有 Hook，Stratus 已成功升级后出现误报；修正筛选后 35 个实际保护文件均哈希一致，没有回退升级或放宽公共资产检查。

负例：用 HoldemTrainer 身份操作 Stratus worktree 被 origin 检查拒绝；普通 project-sync 仍在写入前拒绝 7.0.5。新迁移单测另覆盖来源版本、身份漂移和注入失败回滚。

受管收口命令：`bridgeforge-factory develop --apply --packages factory,workflow --analysis-file .runtime/legacy-seven-upgrade/development-analysis.json`。实际返回 `development-closed`、`task-selected-final-candidate`、`executed=2`、`reused=0`、`git_written=false`。检查为 factory 的 `legacy_upgrade` 参数边界测试，以及 workflow 的 `project_sync::tests` 迁移与既有事务回归集合。

- factory：0.5.0 → 0.6.0，新增四项目临时升级命令。
- workflow：0.3.0 → 0.4.0，新增受约束旧版本规划与身份快照，普通下限和原事务不变。
- 均为兼容新增；未配置稳定验收合同，保持 0.x。根 VERSION 仍为 9.0.0。

实际收据位于 `.runtime/legacy-seven-upgrade/`：`*-preview.json`、`*-applied.json`、`*-repeat-applied.json`、`*-baseline.json`、`*-protected.json`、`development-result.json` 和 `formal-state.json`。源码、公共投影、dogfood 清单由受管 manifest 生成；正式版本记录由 develop 生成，没有手改验证记录。

四个正式目录最终检查均为 Git clean、HEAD 与初始值相同，骨架仍为原 7.0.4/7.0.5。未提交、推送、发布，未升级正式目录，未安装用户级工具。自然 Codex Hook 调度与业务全量回归不在本轮验证范围。

最终版本产物已刷新：受管准备返回 `prepared=3`、`written_files=6`、`self_test_executed=true`。四个隔离目录各应用 7 项最终版本/清单/公共产物更新后，安装 CLI baseline 与 35 个保护文件哈希再次通过。随后使用实际 factory 0.6.0 对四项目逐一重新预览并 apply，均为 `applied=[]`、`stamp_written_last=false`，见 `*-final-repeat.json`。

最终 `build-assets --factory --project-root . --check` 返回 `current`、`read_only=true`、`reused=3`、`built=0`；`manifest --check` 为 `changed=false`，`check factory-version` 为 `healthy=true`，`git diff --check` 与 TODO 机器检查通过。原临时目录权限错误通过原受管命令提升权限重试解决，未修改 ACL 或构建策略。全仓 rustfmt 误触的无关格式变化已核验、备份并恢复，没有纳入交付。

工程实现、独立复核、隔离下游验证和受管子包收口已完成。使用路径见 [INSTALL.md 的四项目临时升级命令](../../../INSTALL.md#四项目-70x-临时升级命令)。隔离 worktree 及忽略目录中的实际收据保留供复查；正式目录升级、下游业务发布和本工具退役仍需另行授权。

## 用户验收与发布授权（2026-10-29）

用户明确调用 `$summary accept`，确认 TODO-047 本次交付符合需求；同时独立调用 `$git-sync release`，授权发布本仓库本次改动。TODO-047 记为已完成，验收范围包括四项目限定入口、隔离升级与重复执行、保护内容、独立复核、受管子包收口及最终工具一致性。沿用上文实际证据，不因验收重复构建、业务回归或 smoke。

验收不表示已经升级四个正式目录，也不表示自然 Codex Hook 调度或下游业务全量回归已验证。这些边界继续保留。

发布预检已执行：main，HEAD `31d1451ca5f32a6c4ed129e439e9fb5f9f210070`，push target 为 origin/main，主程序 9.0.0，工厂工具 0.6.0。首次沙箱预检输出缺少 `version-coverage-v1`；只读源码核对发现已有声明，随后在实际执行宿主重跑同一预检，完整返回 `git-sync-remote-context-v1`、`git-sync-development-closure-v1` 和 `version-coverage-v1`，所需能力核验通过。两次宿主输出差异的根因未核实，未因此改代码、补造能力或重建工具。

发布从通过核验的实际宿主继续 S2–S5。此验收记录保存的是用户决定与发布授权，不预先声称提交、推送或目标版本已发布；实际结果以受管发布收据为准。发布不会升级四个正式下游目录。

## 9.1.0 撤回与 9.0.0 恢复（2026-10-29）

用户明确选择撤回 9.1.0：四个下游最终使用 9.0.0，工厂恢复原始 9.0.0 产品内容，并授权按已确认计划执行。CausisRiskSuite 由用户自行操作，用户已报告完成；本轮不操作或核验该项目。授权包含本仓库撤回提交、受管推送及用户级安装恢复；另外三个下游不提交、不推送。历史验收记录保留，不改写为从未发布。

恢复依据为 `31d1451ca5f32a6c4ed129e439e9fb5f9f210070`，被撤回的 9.1.0 提交为 `35b5ad7ad1829ceae8805d78abdb759065c6b645`。采用现有临时工具将三个 7.x 项目直接迁入原始 9.0.0，不先安装 9.1.0，不手改下游版本戳。先隔离验证，再逐个正式应用，然后恢复工厂源码、合同、子包与收口记录、dogfood 及最终产物；新增撤回提交，保留 Git 历史。

本次沿用 M 级预算：45 分钟、估算 20k 新增 token、主对话执行、最多两轮验证；token 未实测。保护项目业务代码、版本、配置、Hook 及未提交文件；BridgePersonalAssist 的三个未跟踪日记文件只核验哈希，不修改或发布正文。检查仅覆盖骨架迁移、内容保护、重复执行、原始产品一致性、受管构建和同步，不启动业务应用或连接业务账户。

执行收据保存于 `.runtime/restore-nine-zero/`。9.0.0 公共产物准备返回 `status=prepared`，执行两次锁定 Cargo 构建；缺少隔离缓存根目录的首次准备在写入项目之前失败，创建目录后原命令成功。应用内 worktree 工具不支持当前聊天之外的仓库，三个下游使用已授权的 Git detached worktree，正式工作文件未因此改变。

三个隔离项目全部完成升级、保护文件 SHA-256 核验、安装 CLI baseline 和重复 apply 零改动。随后逐个正式应用的结果如下：

| 正式项目 | 来源 | 目标 | 实际受管动作数 | 保护文件数 | 基线 | 重复动作数 |
|---|---|---|---:|---:|---|---:|
| HoldemTrainer | 7.0.5 | 9.0.0 | 35 | 802 | clean | 0 |
| StratusAgent | 7.0.5 | 9.0.0 | 35 | 2895 | clean | 0 |
| BridgePersonalAssist | 7.0.4 | 9.0.0 | 35 | 666 | clean | 0 |

正式升级均返回 `status=applied`、`project_readiness=ready`、`stamp_written_last=true`；保护集合覆盖所有非计划变更的 Git 文件、未跟踪文件及已有项目 Hook 可执行文件，哈希全部一致。三个下游不提交、不推送。CausisRiskSuite 仅采用用户完成反馈，不作为本轮 Agent 实测结果。

工厂恢复原始提交中的 19 个产品文件（含删除临时命令）；`git diff --exit-code 31d1451 -- source templates .codex Cargo.toml Cargo.lock scripts INSTALL.md VERSION` 通过，证明产品、配置、测试及安装说明与原始 9.0.0 一致。保留历史任务文档，并在 CHANGELOG 标明 9.1.0 已撤回。

工厂受管准备已返回 `status=prepared`、`prepared=3`、`written_files=6`、`self_test_executed=true`。最终 `manifest --check` 返回 `changed=false`，`check factory-version` 返回 9.0.0 / `healthy=true`，`build-assets --check` 返回 `current`、`reused=3`、`built=0`，`git diff --check` 通过。本次为恢复既有产品，不新增子包版本或新 release；factory / workflow 恢复为原始 0.5.0 / 0.3.0。

上述为撤回提交前的工程事实。后续由普通受管 git-sync 保存并推送撤回提交，再由官方 updater 从远端恢复用户级安装；不预先将待执行步骤记为成功。实际提交、远端一致性与用户级安装以本轮交付及 `.runtime/restore-nine-zero/` 的同步、更新收据为准。三个下游隔离 worktree 已清理，收据保留；原始 9.0.0 来源 worktree 因应用将其作为本聊天主工作目录而无法归档，保持原样。工厂正式恢复、验证和 Git 同步均在 `D:\Quant\BridgeForgeCodex` 执行。

### 首次同步：网络阻断

受管 `git-sync --preflight` 已返回 `ready`，main、目标 `origin/main`、工厂工具 0.5.0 / 产品 9.0.0。随后 `git-sync --message-file .runtime/restore-nine-zero/message.txt` 在首次 `git fetch origin` 阶段返回 `status=blocked`：`OpenSSL SSL_connect: SSL_ERROR_SYSCALL in connection to github.com:443`。收据为 `.runtime/restore-nine-zero/git-sync.json`，错误日志为 `git-sync.stderr.log`。

失败发生在暂存、提交、推送之前；已核对 HEAD 仍为 `35b5ad7`、原有 stash 未变化、23 个本轮文件改动保留，产品内容仍与原始 9.0.0 一致。用户级产品只读核验仍为 9.1.0，尚未调用 updater，避免再次安装尚未撤回的远端版本。不得将本地恢复称为远端撤回完成。

网络恢复且用户明确重试后，从 `D:\Quant\BridgeForgeCodex` 继续同一受管 git-sync；复用本轮已验证内容和提交消息，真实输入发生变化才补核。成功后独立核验实际 push 目标、工作区、HEAD 与远端一致性，再执行官方用户级 updater 并核验 9.0.0。不要重复升级三个已完成的下游，也不操作用户自行负责的 CausisRiskSuite。

用户随后明确授权重试同步、完成上游撤回及用户级 9.0.0 恢复，并再次要求三个已完成下游保持不动。重试沿用原仓库、已验证产物和提交消息；重试及用户级安装收据分别保存为 `git-sync-retry.json`、`user-update.log`，以实际终态为准。
