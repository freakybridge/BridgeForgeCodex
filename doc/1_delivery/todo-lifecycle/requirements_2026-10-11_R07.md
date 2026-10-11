---
lifecycle: active
validation_status: awaiting_user_acceptance
---

# R07：TODO 管理与归档收口一致性

## 目标、授权与预算

用户选择确认卡 B 并回复“开始吧”，授权记录需求、实施 R07、适配当前 TODO、完成工厂工程检查和独立审计。未授权提交、推送、release、实际文档归档、用户级安装或真实下游写入。R08 不在范围内。

L 级：跨 Skill、Rust 多入口和已有记录适配。整体上限 3 小时、约 60000 新增 token（估算，未实测）、1 名 review-auditor、最多 2 轮验证；包括完整工程检查、构建、独立审计与合理修复重测。无适用同模型周额度系数，不编造百分比。预算从本次确认累计，切换 Skill 不重置；预计超额时停止新增工作，保存证据并报告。

基准 main / 4520df3ab153860201d628fb84440f7a69c86473 / 2.2.2，实施前工作区干净。

## 已确认决定

1. 增：todo 与 confirm。confirm 在正式需求确认并获准落盘时新增关联 TODO；已有同一事项则引用，讨论中的想法不自动记入。
2. 删：不支持删除条目、复用编号或覆盖历史；取消保留记录和明确决定。
3. 改：summary accept 按本次证据更新相关状态、进展和收口；发现新的独立事项时使用 todo 的新增规则，不另维护规则。
4. 查：find-doc 等只读入口核对 TODO 与关联正式文档，不能从查询自动开始开发。
5. 收口：R07 统一 TODO、正式文档与归档判断。原需求未完成不能通过转入 TODO 绕过验收；已验收后新增且明确承接的独立事项不阻止原交付归档。
6. 固定状态为待确认、待执行、进行中、待验证、待验收、已完成、已取消；当前已有条目按来源证据适配。
7. 新增归属列，值为原需求或后续事项；保留编号、日期、内容、状态和关联文档。

用户依次选择 A（confirm 正式落盘时记账）、A（区分原需求/后续事项）、A（固定状态）、A（增加归属列）。

## 事实、数据与传播

目前结构检查基于需求卡，archive-scan 基于多种 Markdown，SessionStart 另有旧标记规则，产生 13/12 候选差异。archive-scan Skill 消费字段与 CLI 不一致。TODO 已有五条，都是交付记录留下的后续事项，未取得完成证据；TODO-005 含组合状态，安装未执行时按待执行适配，不抬高验证阶段。

固定路径 doc/0_architecture/TODO-INDEX.md；表头为编号、日期、内容、状态、归属、关联文档。总表保存摘要与来源，详细需求/验收留在原文档。已完成必须有验收依据；后续事项保留来源和独立承接记录。

传播四问：通用产品层；进入 templates/hooks 与共享 skills；正式发布按产品版本策略处理 VERSION/CHANGELOG [product]，本次不预写；同步本仓库 dogfood 和受管 manifest，不执行下游 project-sync adopt/apply。

## 拟修改与验收

- 核心：共享 TODO 合同与只读检查；归档扫描、项目结构检查、Hook 共用归档判定。
- Skill：todo、confirm、summary accept、find-doc、archive-scan 使用同一合同，明确权限、来源和失败边界。
- 文档：规范、索引、当前 TODO 适配并新增本需求；不改写历史验收事实。
- 验证：新增去重、非法/重复编号和状态、损坏表格、链接与证据缺失、原需求未完成阻断、后续事项不阻断、活动需求卡、历史报告、单文件/目录 Bug、flat/milestone、旧完成标记、只读零写入。
- 工厂必需：metadata、manifest、baseline、project structure、mirror drift、完整 workspace/fixture、受管构建和独立审计。只用锁定 Rust workspace；不引入 Python 或安装依赖。

## 保留契约与风险

候选判断不等于移动授权；只读检查不修复/新增 TODO。工程通过不替代用户验收，lifecycle 保持 active；真实下游与真实 Hook 生命周期未运行则明确未验证。旧总表不自动批量迁移；本轮仅适配本仓库明确授权的六列格式，其他项目检查应说明迁移需求并零写入。来源、状态或归属有歧义时保留原行并报告，不猜测标为完成。

## 实施与验证记录

2026-10-11：需求记录已建立并关联 TODO-006；实施进行中，工程验证尚未完成。

首轮 core 定向 9 项通过。独立审计指出两个 P2：无首管道的 TODO 行被忽略、收口证据编号采用子串匹配；已补支持无首管道行/截断报错、完整编号 token 匹配及两项回归。Hook 首次测试编译因私有辅助函数访问失败，改为验证实际 show_state 输出，未扩大生产接口可见性。当前完整工程检查尚未完成，不预写通过。

前向复核另补齐无来源待确认项的取消证据路径：在本次明确记录授权内按当前 topic 建最小决定记录，明确编号/用户决定/原因并登记索引；无授权或证据保留状态。confirm 将 TODO 编号回链到卡，summary accept 将完整编号写入收口正文。属于既定收口职责补全，不增加自动写入入口或伪造需求/验收。

首轮完整 workspace 发现既有 prepared-record fixture 写入旧复选框式 TODO，现被六列合同拒绝。已将该夹具改为有本事项验收证据的六列表，并增加旧格式使准备失效的断言；保留“记录文档变化可复用产物、非法文档仍失效”的原验收语义。最终全量将在定版审计与产物核验后通过正式 prepare 入口执行。

独立 review-auditor:/root/r07_audit 已完成真实差异/来源与只读前向推演，最终 passed，无剩余必修问题；修正的 prepared-record 夹具另经定向复核。当前开发输入指纹为 sha256:babf54d90cb96f56f7283572e82761806ac4604ed74da557ea9f263e6bf61bdf（由源码计算当前 key，不是旧 prepared 的成功证明）。审计记录 .runtime/r07-audit.json 绑定该值。第二轮 TODO 8 项及 prepared-record 1 项已实际通过。

当前版本 2.2.2 的 CLI/Hook 已经受管 build-assets 安装并各自自检通过，构建收据源树 sha256:2a9a80df0e318f7e0791eeeb2777b36111fdc5caf3ef2b5dac78e6204d91d5ff 与现行合同匹配，baseline 为 clean。当前 TODO 六条零错误，结构/归档候选均为 20，包含 refactor-scan；metadata 和 manifest 检查通过。正式准备尚在执行，完整 fixture/workspace 与用户验收未预写通过。

## 最终工程结果

阶段证据说明：后续 [R08](../shared-json/requirements_2026-10-11_R08.md) 已授权并完成工程验证；R07 当时的原始 prepared 收据保存在本机 `.runtime/r08-before/r07-prepared.json`。受管 current.json 已更新为累计 R07+R08 现场，不能把其新指纹当作下文旧阶段指纹；R07 需求与用户验收状态未改变。

2026-10-11：实现、工程验证与独立审计完成，等待用户验收。前文保留各阶段事实，以本节为当前结果。

| 检查 | 实际结果 | 覆盖与限制 |
|---|---|---|
| baseline / metadata / manifest / structure | 4 项退出码均为 0 | 当前版本与受管资产、Skill、六列 TODO 和正式文档；不替代业务验收正文复核 |
| mirror-drift | 1 passed / 0 failed | 模板/dogfood 全树一致 |
| dogfood workspace | CLI 15、core 228、Hook 24，共 267 passed / 0 failed；8 ignored | 包含新增 8 项 TODO、3 项归档、CLI/Hook 行为与修正 prepared-record；忽略项是已有条件性能/子进程/live 用例，未声称执行 |
| factory fixtures | 90 passed / 0 failed；6 ignored | 包含真实本地 Git、隔离进程/同步事务、分发与结构一致；不是实际业务下游或原生 App 生命周期 |
| 独立审计 | passed；唯一 review-auditor:/root/r07_audit | 三处审计缺口均修正，源码/规则/夹具和前向推演；不是实际模型执行 Skill 的实测 |

正式命令：`.codex/bin/bridgeforge.exe git-sync --prepare-release --message-file .runtime/r07-prepare-message.txt --audit-file .runtime/r07-audit.json`。

收据为 `.runtime/bridgeforge-codex/release-preparation/current.json`，`status=prepared`、`checks_passed=7`，验证指纹与审计均为 `sha256:babf54d90cb96f56f7283572e82761806ac4604ed74da557ea9f263e6bf61bdf`。workspace/fixture 日志分别为 blobs/eb2c347fd74f4e4097e8fd35853841237de0f893792680d034aa8a3f9939a9fe 及收据中 factory-fixtures 的 log_sha256，可从当前收据追溯，不复制完整日志。

隔离候选 target_version 为 2.2.3，根 VERSION 与当前安装程序仍为 2.2.2；未暂存、提交、推送、发布、正式分发用户级 Skill 或移动归档。TODO-001–005 未取得新增完成证据，保持未完成；TODO-006 保持原需求、未结算，后续由 summary accept 按实际用户验收收口。lifecycle 保持 active。

试用入口：`check todo --root . --contract` 读取机器合同；`check todo --root .` 只读校验；`archive-scan --root .` 和 `check project-structure --root .` 共用候选规则。真实下游、另一台机器、实际模型行为、用户级安装和自然 Hook 生命周期均未验证，不把 fixture 或源码审计扩大为这些验收。

预算收据：1 名审计 Agent，2 轮工程验证（含首轮既有旧夹具失败及同一范围修复重测）；token 未实测。源码/Skill/登记定版后没有继续扩大范围，文档记录变化按项目 record_documents 轻量核对，不重跑完整回归。
