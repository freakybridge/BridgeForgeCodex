---
lifecycle: active
validation_status: awaiting_user_acceptance
---

# 旧项目发布接入与版本基线修复

> 本文保留已发布 2.0.2 的历史需求和证据。用户后续确认 Git 同步/升版不依赖开发清单和收据，当前规则以[独立 Git 同步](requirements_2026-10-09_independent-git-sync.md)为准。

## 授权、范围与预算

来源：当前对话，用户确认六步 fix 计划后说“开始吧”。实施范围为 BridgeForge 上游诊断、升级提示、共享 Skill、测试、dogfood、分发合同，以及 StratusAgent 隔离测试 worktree 的发布接入和基线恢复。仅在隔离仓库使用本地提交与本地 bare remote 验证完整发布，不提交、推送、发布工厂或真实 StratusAgent，不修改真实项目的现有 14 个文件。

规模 M：两个已有流程的确定缺陷，复用版本保护及事务。预算 45 分钟、20k 新增 token（估算，未实测）、最多 1 个 review-auditor、最多 2 轮验证。当前模型无已核实周额度系数，不伪造百分比；预计超额前停止超额动作并报告。确认卡直接整理已授权计划，不再次索取实施决定。

## 事实与决定

1. StratusAgent 的 2.0.1 工具已安装，发布状态检查缺 `.codex/development-checks.json`。普通同步另受平台审批拒绝，此修复不能绕过平台审批。
2. 实查 HEAD VERSION 为 1.47.13，本地为 1.47.12；当前 planner 拒绝未提交手工版本变化和已提交降版。保留已推送的 1.47.13 基线，不删除已发布记录或重写历史。
3. 发布状态一次报告缺配置/非法配置及版本基线阻断；仅 prepared 允许发布。缺配置不是测试失败，更不能用空检查兜底。
4. 骨架升级独立显示 release_setup，项目无 VERSION 不提示。发布未接入不阻断普通同步和安全骨架升级；已有项目清单逐字保留。
5. 清单由项目开发流程读取真实入口后建立；StratusAgent 已有 Rust 项目 Hook 的只读 version-sync 与隔离 runtime 回归入口，不能抄工厂 checks。

## 传播与计划

通用产品修复进入 templates/、skills/，同步 dogfood 与 manifest。后续需要产品版本及 [product] CHANGELOG，由显式发布事务统一生成，不预写 VERSION。

| 顺序 | 任务 | 验证 |
|---|---|---|
| 1 | 发布接入及基线只读诊断 | 缺配置和版本差异同时报告；HEAD/index/文件及 .runtime 零写入 |
| 2 | 升级 plan/apply 独立发布接入状态 | 骨架 ready 与发布未接入分开；清单缺失不创建，已有清单字节不变 |
| 3 | 共享 Skill 接入步骤 | 缺口由开发处理；发布复用准备，不自动大回归，不绕过配置/版本保护 |
| 4 | Stratus 隔离 worktree | 复制实际骨架改动、保留 1.47.13 版本基线；运行项目检查并准备 |
| 5 | 真实 CLI 回归 | 无预置清单的旧项目 → 诊断 → 接入 → 普通同步 → 准备 → 显式及重复发布；仅本地测试 remote |
| 6 | 独立审计与工厂收口 | 完整 workspace、fixture、baseline、metadata、manifest、structure、准备产物 |

## 边界、风险与证据

项目清单不能覆盖业务源码、测试生产账户或写数据库。真实项目状态先按字节哈希记录并在结束时复核。隔离测试使用私有本地 clone 的 worktree，不改真实仓库 Git 配置或远端；需要环境只读复用已有运行时，不安装新依赖。

源码、产品传播、dogfood、fixture、真实下游、runtime 六类证据在执行后分别记录；隔离真实项目通过不能表述为原工作区已经修复或已发布。当前实施中，尚未完成验证。

以下阶段记录保留开发过程；最终状态与证据见文末收口收据。

## 实施和阶段证据

- 发布只读状态已在真实 StratusAgent 上一次报告缺清单及 VERSION 手工降回的冲突。未写真实项目。
- 新增诊断回归 3 passed，覆盖缺清单+VERSION、manifest、Cargo.lock、非法 manifest 路径及不支持 JS 锁；HEAD/index/文件/.runtime 零写入。升级配置保留定向 1 passed。
- 真实 CLI 四策略流程定向 1 passed：不预置检查清单；先诊断、无清单普通同步、开发接入、准备和显式/重复发布。随后原生 validator 的改动将由最终完整验收再次覆盖。
- 独立审计发现 P2：最初只检查 VERSION，当前原生元数据及不支持 JS 锁仍会晚报。已将原 planner 拒绝条件完整提取为共用只读 validator，保留原错误语义；最终复审 passed。审计未代替测试。
- 私有真实 StratusAgent worktree 已经受管升级、baseline clean 和普通同步成功；版本 1.47.13 未变，clean/0/0，只推送本地 bare remote。项目清单针对本次骨架与版本元数据接入，不代表业务交易/回测的完整验证。
- Fixture 快照首次把 bin 复制成嵌套 bin，已限定私有 worktree 绝对路径清理并按单文件重新复制；随后受管升级重新构建并验证 project Hook 收据。未改产品代码来绕过该 fixture 错误。

## 经验沉淀

发布接入需要在旧项目缺清单时验证，不能只测已配置夹具。诊断与 planner 必须复用完整原生元数据校验，而不是只复用 VERSION 比较；完整对应表由独立审计确认。骨架更新成功不等同于业务发布就绪，已推送误升版本不宜用本地降版补救。

## 最终开发收口收据

| 证据 | 实际结果 |
|---|---|
| 源码与独立审计 | 共用原生 validator 覆盖原 planner 拒绝条件；review-auditor 最终 passed；隔离项目测试断言另行只读复核通过 |
| dogfood | 全树镜像一致、当前版本受管构建和 baseline 通过；CLI 声明 git-sync-release-readiness-v1 |
| workspace | 14 CLI + 187 core + 22 Hook，共 223 passed / 5 ignored / 0 failed |
| 工厂 fixture | 89 passed / 4 ignored / 0 failed；与 workspace 共 312 passed，不重复计算定向集 |
| 产品传播 | 模板、共享 Skill、manifest 已更新；根 VERSION 仍 2.0.1，HEAD 仍 2f36deb，未提交/推送/发布/用户级安装 |
| 真实下游隔离验证 | 私有真实 Stratus worktree 普通同步不升版、显式发布 1.47.13 → 1.47.14、重复发布不再升版且同一提交；仅本地 bare remote，clean/0/0 |
| 项目 runtime 回归 | 三项真实检查均 exit=0；Python Hook suite 44 passed，含内嵌 Rust 单元测试；未验证交易/回测业务功能或 GUI |
| 原真实目录 | 原14文件 SHA-256 复核：changed=0；真实目录仍有原接入缺口，尚未落地本轮配置/版本恢复/断言修正 |

工厂准备：status=prepared，7 checks passed，target_version=2.0.2，validation_fingerprint=`sha256:c86e81b8000fc8e1a47e1eba1286b2193768ba389195c4bf9e9b1ba20eb8634d`；记录 `.runtime/bridgeforge-codex/release-preparation/current.json`。Workspace 日志 `c8b3bc34f9d886a1e56f6e34da4d1a44bf8a5eb96b60b61fd60eaf97fd614d6d`，fixture 日志 `a995f03230a648746a1014965fb0f7c984c3029ed752ebaf9fbd332165e39743`，同目录 blobs 可复核。

忽略项如实保留：workspace 四个子进程 helper 和需额外现场日志的只读检查；fixture 的显式性能实验、需额外原生环境的高成本用例、子进程 helper 和需授权 Assist 源的迁移。不声明它们通过。

私有 Stratus worktree：`.runtime/release-onboarding/stratus`；其 origin 严格固定为 `.runtime/release-onboarding/origin.git`。发布提交 `0319ff1a48416865f707e2828620dd11fc1d25cc`，重复发布 commit 不变、push_performed=false；原生 version-sync=OK 1.47.14。项目准备指纹 `sha256:00a3dd2c897701294a993dee7416027f41f65ce13e9da54d4d9ff7552ba9e461`，三项日志及收据保存在该 worktree 的 .runtime。

真实项目 suite 首次 43 passed / 1 failed：包装器硬写8 passed，而现有Rust实际9 passed/0 failed。仅在私有 worktree 将断言改为成功摘要、0 failed、至少8项，仍要求cargo exit=0；44项重测通过。原目录脚本没有修改，该修正是接入材料之一。

接入材料位于 `.runtime/release-onboarding/handoff/`：项目检查清单、修正测试文件和 `published-version-baseline.zip`。ZIP 从私有提交0a0062提取六个已发布1.47.13版本文件，不包含测试发布的1.47.14。后续原目录应用应先复核14文件原哈希与测试脚本差异，保留回滚副本，通过项目受管流程更新工具和准备；不能把测试分支或测试升版直接推送到真实GitHub。

源码与隔离验收已完成，待用户验收及真实目录落地授权；原平台审批拒绝仍需用真实授权与范围证据处理，不能由代码绕过。使用一个独立审计Agent并复用其复审；token未实测。完整检查及等待按实际成本记录，不把等待视为免费或宣称精确额度消耗。
