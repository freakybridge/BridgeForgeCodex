# 工厂重构扫描换机交接

> 本文为扫描阶段历史交接。后续用户已授权 R01 开发，接续前先读取
> [R01 开发确认卡](requirements_2026-10-09_r01-cleanup.md)及已授权实施的
> [R02 确认卡](requirements_2026-10-09_r02-persistence.md)，不沿用本文对 R01/R02 的旧授权状态。
> R03 亦已授权实施，接续时同时读取 [Memory CLI 确认卡](requirements_2026-10-09_r03-memory-cli.md)；R04已获预算/实施授权，最新累计证据见 [独立缓存确认卡](requirements_2026-10-10_r04-artifact-cache.md)。
> 2026-10-10 用户已验收累计 R01–R04 工程范围并授权正式 release；验收记录见 R04 卡，发布结果以受管同步收据为准。下文授权状态与接续提示均为扫描阶段历史，不用于当前执行。

## 当前主线与事实源

本次目标是使用 refactor-scan 分析 BridgeForge 工厂，解释 R01–R04，保存候选清单并交接。
候选、源码位置、取舍、验证和白话解释的唯一事实源为
[扫描报告](scan_2026-10-09_factory-refactor.md)。
Skill 本身的开发、验收与发布是已完成的前一事项，见
[原需求卡](requirements_2026-10-09_refactor-scan.md)；不得把它的验收扩大为候选重构完成。

## 保存时的现场

| 编号 | 项目 | 保存时事实 |
|---|---|---|
| H01 | 仓库 | freakybridge/BridgeForgeCodex，origin=https://github.com/freakybridge/BridgeForgeCodex.git |
| H02 | 分支/版本 | main / 2.1.0 |
| H03 | 基准 HEAD | 0425b2da9ed5bf285ff6268ccd55b68e0d846c86；这是保存交接前的基准，不是随后 git-sync 的最终提交 |
| H04 | 原工作区 | 写交接前干净，无源码改动 |
| H05 | 本次文档 | 新增扫描报告、本交接文档，更新 doc/README.md；后续用户已授权普通 git-sync，最终成败以同步收据为准 |
| H06 | 历史 stash | 1 条 codex_git_sync_autostash，OID cc744c352c2012cf5af600c36a397bd5286c8594；保留，不是本轮待恢复工作 |

另一台机器先同步 origin/main，再按实际 HEAD 核对文档，不将上面的保存基准当作最新提交。

## 已有决定与授权

1. 默认扫描当前工厂；允许现有定向测试及离线基准，报告范围不是全仓穷尽。
2. R01 清理不用的旧路径；R02 归位通用持久化；R03 集中 Memory 命令；R04 评估独立产物缓存。
   用户已理解解释，但四项均未选中、未授权实施。优先建议不是用户决定。
3. 本轮 summary switch 授权保存扫描经验和交接；另行同时调用 git-sync，授权普通提交/推送本轮文档。
   未授权 release、重构代码、生产访问、其他项目写入、自动归档或 Memory 写入。
4. 后续明确“选中”只记录候选决定；明确“实施”才形成对应范围授权。
   不自动纳入未授权依赖，R04 与 R01 的接口处理需协调。

## 已完成、未完成与预算

扫描、白话解释和既有定向验证已完成。13 项定向测试/基准通过，完整结果与命令见扫描报告。
普通同步 CLI 基准为单次 29.185 秒、复用 2/构建 0；R04 的实际收益未测。
尚未完成的是用户选择、实施范围/预算确认，以及任何候选的代码实现和新方案验证。

扫描按 M 级 45 分钟、约 20,000 token 估算、0 子 Agent推进，实际约 15 分钟、token 未实测。
本次是人工交接，不是 autopilot；没有原生 Goal、在途测试或需要恢复的子 Agent。
后续实施预算按 confirm 对选中范围核算，不伪报扫描余量或把换机当作旧任务额度重置。

## 不随 Git 携带的环境和证据

1. 需要 Windows、项目规定的 Rust/Cargo 与锁定依赖。换机按 INSTALL.md 和根 AGENTS 的快速命令恢复，
   不猜安装命令、不回退 Python，不对工厂执行下游 project-sync adopt/apply。
2. .codex/bin 的 CLI/Hook、Cargo target、测试临时目录、.runtime 的测试与工程准备缓存被忽略。
   本机旧 prepared 收据、审计 JSON 和候选二进制不能假设在另一台机器存在，也不能用文档伪造它们。
   本次只读核对 development-status 实际为 stale，原因文本为 development inputs changed；
   当前指纹 sha256:25bab316d336ed80b09996cf8fbb0dfcdee6aa28bb96fb5c15c6bbaf1bf00103。
   不声称旧工程准备仍有效，不为 summary 重跑测试/构建；普通 Git 同步独立于开发准备状态。
3. 用户级 Skills 和分发账本是本机状态；当前会话已能读取安装后的 refactor-scan。
   换机需要时走受管分发，不能手工覆盖目录或据此声称新机器菜单已可见。
4. 本文不复制凭证、个人配置、原生 Memory、完整日志或二进制；已有测试结论不代表新环境实测通过。

## 接续步骤

1. 同步仓库后读取本交接、扫描报告、根 AGENTS 与 doc/README.md，核对实际分支、HEAD 和工作区差异。
2. 对源码/配置变化只复核受影响候选和证据；不因为换机机械重复整仓扫描或完整回归。
3. 保留 R01–R04 编号，等待用户选择；有明确选中/实施消息时继承它，只补会改变结果的关键决定。
4. 授权范围及预算齐备后交给 confirm/develop，按模板源、dogfood、登记、真实消费者和验证边界实施。
   原公开接口、错误、输出、只读查询和 Git/文件安全语义须保持，行为变化另行明确。
5. 真实安装、下游、runtime 和性能结果各自记录。Git 发布、版本升级和用户验收分别处理。

本交接保存时文档尚未提交/推送；用户同时授权的 git-sync 将随后执行，勿预写成功或最终 commit。
归档和规则修改未执行；不要调用已退役的 snapshot/resume，也不自动新建对话。

可复制的接续提示：

> 读取 doc/1_delivery/refactor-scan/handoff_2026-10-09_factory-refactor.md，先核对当前分支、代码与文档差异，再在已有授权范围内从下一步继续；缺失证据如实标明。R01–R04 尚未选中、未授权实施，不要直接改源码。
