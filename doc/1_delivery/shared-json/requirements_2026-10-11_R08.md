---
lifecycle: completed
validation_status: verified
---

# R08：严格 JSON 解析器归入公共模块

## 目标与授权

用户在 R08 白话解释后回复“同意。开始吧”，明确授权实施保持行为的职责迁移：把 baseline 中 UniqueJson / parse_unique_json 放到中性的 core 内部 JSON 模块，各真实调用方直接使用；不扩大公开 API 或修改 JSON 入口的校验范围，不改变重复键拒绝、解析结果、错误文本、退出码或外部副作用。

正式需求关联 TODO-007。当前已有 R07 等未提交改动，保留并与本轮成果分开；不结算 TODO-006 或既有 TODO，不修改 R07 的规则与状态。未授权提交、推送、release、用户级安装、真实下游或实际文档归档。

## 规模、预算与基准

M：45 分钟、约 20000 新增 token（估算，未实测）、1 名 review-auditor、最多 2 轮验证；包含调用关系核验、迁移、定向等价性检查、工厂强制完整检查与构建、独立审计及合理重测。无适用同模型周额度校准系数，不编造百分比。预算从本次 R08 开工累计，切换 Skill 不重置；预计超额或范围变化时停止受影响动作并报告。

基准 HEAD 4520df3ab153860201d628fb84440f7a69c86473，main，VERSION 2.2.2。工作区已有已完成工程验证但未验收/提交的 R07；其 prepared 指纹 babf54d9…61bdf 只能证明 R07 当时现场，R08 源码变化后须重新核验累计现场。本轮迁移前源码快照保存于忽略目录 .runtime/r08-before/，不是第二份维护实现。

## 核实的调用关系与保留合同

解析器为 pub(crate)，UniqueJson 为私有类型，属于 serde Value 解码通用能力，未依赖 BaselineReport 或骨架状态。真实消费者为 baseline、artifact_cache、asset_migration、git_sync、manifest、project_hooks/package、project_sync、release、release_history、release_preparation。没有 crate 外消费者；baseline 私有测试的现有断言保留。

必须保持：顶层/嵌套对象与数组中对象的重复键拒绝；不同对象可用相同键；UTF-8、标量、整数/浮点、null、数组、对象顺序与转义行为；非法输入、尾部垃圾和重复键的完整 label/serde 错误文本。业务调用前后顺序和成功/失败分支原样保留，不将其他 serde_json 入口改为严格解析。

## 实施与传播四问

1. 通用产品层结构调整，新增 templates/hooks/crates/bridgeforge-core/src/json.rs，保留原算法与 crate 内可见性。
2. 修改基线进口、core 模块登记及真实直接调用方；测试实体位于 scripts/tests/unit/core_json.rs，保留已有消费者测试。
3. 同步明确文件的 .codex/hooks 镜像，通过受管 manifest 生成稳定资产 ID/target/hash；不对工厂执行 project-sync adopt/apply。
4. 正式发布需按产品策略处理 VERSION/CHANGELOG [product]；本轮不预写版本，不发布。共享 Skill 未修改，不手动安装用户级副本。

## 验收与风险

定向覆盖不同 JSON 形状、对象顺序与数字、跨对象同名键、嵌套/数组重复键、转义后同名键、坏 UTF-8、尾部垃圾与错误文本。审核迁移前后函数主体及所有调用标签/实参，证明是职责移动。新模块无文件写入或 IO，保留内部边界，避免 baseline 委托包装留下第二条常驻路径。

工厂按 .codex/development-checks.json 完成 7 项检查及受管 prepare；独立审计覆盖本轮增量、模板/dogfood 和累计现场边界。不重复对镜像执行相同全量测试。忽略项如实列明，不以 fixture 代替真实下游或原生 Hook 生命周期。

## 记录

2026-10-11：需求已确认并授权实施，TODO-007 新增，实现与验证进行中。lifecycle 保持 active，用户验收另由 summary accept 收口。

迁移前实际安装的 R07 CLI 在隔离 .runtime/r08-parser-baseline 中捕获四类重复键错误：顶层/嵌套/数组/转义后同名字段，列号分别为 17/27/18/18；新的严格解析单测使用这些实测旧错误文本，不猜测位置。解析器主体从迁移前快照机械提取，所有外部调用仅更换命名空间，未新增 IO 或校验入口。

定向 3 项回归通过，迁移后安装的 CLI 对同一四类输入返回与迁移前完全一致的错误。受管 build-assets 已安装当前 2.2.2 CLI/Hook，收据源树 sha256:495cdb710c7b46d8ea3d94017d71993525c3b0682c1a07dcc546fc80cd9acef3。当前 7 条 TODO 校验无错误。

独立审计复用 review-auditor:/root/r07_audit，针对 R08 增量 passed，无必修问题；函数主体与迁移前快照一致，10 个外部消费者仅换命名空间，13 个源码镜像与登记 hash 匹配。审计记录 .runtime/r08-audit.json 绑定当前开发指纹 sha256:b19983067960a6b0a2b7e2d70af018bff3ee32993a45ede767293dda29c2055c。既有 R07 prepared 因本轮源码变化而失效，原记录已只读复制到 .runtime/r08-before/r07-prepared.json 保留阶段证据；尚未宣称 R08 工程准备通过。

## 最终工程结果

2026-10-11：R08 实现、工程验证和独立审计完成，等待用户验收；前文保留各阶段事实，以本节为当前结果。

| 项目 | 实际结果 | 覆盖与局限 |
|---|---|---|
| 原主体与调用方对照 | 主体精确相同；10 个外部消费者只换命名空间 | 对照本轮迁移前快照，未把 R07 成果归为 R08 |
| 严格解析定向回归 | 3 passed / 0 failed | 形状、顺序、数值、跨对象同名字段、嵌套/转义重复键、UTF-8 与尾部垃圾 |
| 原/新 CLI 同输入对照 | 四类错误完整文本一致 | 隔离错误输入；不等于所有真实下游场景 |
| baseline / metadata / manifest / structure | 4 项退出码为 0 | 当前累计工作区、公共资产、源与索引/TODO |
| 全树镜像一致性 | 1 passed / 0 failed | 模板/dogfood 全树字节一致 |
| workspace | CLI 15、core 231、Hook 24，共 270 passed / 0 failed；8 ignored | 本次解析器/实际消费者及累计 R07；条件忽略项未声称执行 |
| factory fixtures | 90 passed / 0 failed；6 ignored | 隔离 Git、进程、同步事务和分发；不是实际业务下游 |
| 独立增量审计 | passed | 唯一 reviewer 复用 review-auditor:/root/r07_audit；无剩余必修问题 |

最终命令：`.codex/bin/bridgeforge.exe git-sync --prepare-release --message-file .runtime/r08-prepare-message.txt --audit-file .runtime/r08-audit.json`。

工程收据 `.runtime/bridgeforge-codex/release-preparation/current.json`：status=prepared、checks_passed=7、target_version=2.2.3；验证指纹与审计一致为 sha256:b19983067960a6b0a2b7e2d70af018bff3ee32993a45ede767293dda29c2055c。workspace 日志为 blobs/a1d65e49521ca0e43a1b960d02ca1273ca0d95c2871dc983a1dd0e64ca8ba194，fixture 日志为 blobs/e5e3426d4404ebf20ae5de264ff5b66405d523f27aac0edfbd81542508182965；其余日志从收据追溯。

本机当前 CLI/Hook 仍为 2.2.2，已通过受管构建及自检，并匹配当前源树 sha256:495cdb710c7b46d8ea3d94017d71993525c3b0682c1a07dcc546fc80cd9acef3。2.2.3 仅为隔离准备候选；根 VERSION=2.2.2、HEAD=4520df3，未暂存/提交/推送/发布，没有用户级安装或真实下游写入。旧 R07 阶段收据保留在 .runtime/r08-before/r07-prepared.json，当前 current.json 指向累计 R07+R08 现场，不混用阶段指纹。

TODO-007 已新增，未由 Agent 自行结算；TODO-001–006 未改状态。用户验收由 summary accept 更新本卡与 TODO-007。真实下游、自然 Hook 生命周期和用户级安装未验证。本次是保持行为的职责迁移，不承诺性能收益。

预算收据：1 名审计 Agent（复用），1 轮完整工程验证，含事前定向测试和准备收据核对；token 未实测，无新增范围。正式准备后仅更新明确允许的记录文档并只读确认有效性，不重复完整回归。

## 经验沉淀

1. 公共工具归位前先核对可见性与真实消费者。pub(crate) 的迁移不需虚构 crate 外兼容层；baseline 使用私有导入即可保持自身调用与私有测试，不保留无用途的委托包装。
2. 用迁移前现场快照区分累计改动与本轮增量。函数主体相同、十个消费者仅换命名空间是源码证据，再结合实际旧/新 CLI 的完整错误对照及消费者回归，不把“移动代码”本身当作行为验证。
3. 错误列号和转义键语义应以实际旧程序取证。转义后同名键的列号为 18，不能根据表面字符串长度猜测；同时覆盖对象顺序、数值和跨对象同名键合法性，避免把不同语义误当重复。
4. 工程准备是本机当前现场证据，源码改变后原阶段 prepared 会失效。保留旧阶段收据并明确新指纹；配置允许的验收记录变化只做轻量核对，不为 summary accept 重跑回归、审计或构建。Git release 与工程准备各自负责，不将准备候选版本写成已发布版本。

上述风险已由现行 AGENTS、受管检查和测试覆盖；没有新的 Rule/Hook/AGENTS 建议，不修改 Memory。本文可作为归档候选，但本轮不执行归档。

## 用户验收与发布授权

2026-10-11 用户明确调用 `$summary accept` 与 `$git-sync release`。按当前交付 R08 记录验收：严格解析器职责迁移、实际消费者切换、模板/dogfood/登记一致性、定向等价性与完整工程检查符合需求；TODO-007 标记已完成，并关联本节。R07 和 TODO-006 的用户验收状态不推定为完成，TODO-001–006 均保留。

验收依据为上述实际工程收据与独立审计；真实下游、自然 Hook 生命周期与用户级安装仍未验证。未发现新增未解决的 R08 事项，不自动记推测改进。正式 release 本次已授权覆盖当前累计 R07+R08 改动，发布包含 R07 代码不等于取得 R07 用户验收。

发布结果尚待受管同步收据确认；不预写版本、commit 或 push 成功。

本次受管零写入 release-preview 返回 release-planned，计划 2.2.2 → 2.3.0；提交消息为“feat: 统一待办收口与归档判定并归位严格 JSON 解析器”。minor 依据是累计 R07 新增 TODO 管理/收口与只读检查能力，R08 为保持行为的结构迁移，一并发布。此前工程准备的 2.2.3 是以 refactor 消息构建的隔离候选，不是实际发布版本；最终版本以 fetch 后受管事务重新计算的收据为准。

## 正式发布结果

2026-10-11：正式发布未完成。受管 `.codex/bin/bridgeforge.exe git-sync --release --message-file .runtime/r07-r08-release-message.txt` 首次因受保护锁返回 os error 5；按完全相同命令窄范围提升权限后，进入 fetch 并失败：

```text
git fetch origin failed: fatal: unable to access 'https://github.com/freakybridge/BridgeForgeCodex.git/': OpenSSL SSL_connect: SSL_ERROR_SYSCALL in connection to github.com:443
```

按 git-sync 的失败停止规则保留现场，没有改走手工 Git、调整 TLS/ACL、强推或循环重试。具体网络根因未确认，不把 TLS 失败归为代码问题或权限恢复成功。

失败后实际核对：main 跟踪 origin/main，HEAD 仍为 4520df3ab153860201d628fb84440f7a69c86473；根 VERSION 与 CLI 仍为 2.2.2，CHANGELOG 未写 2.3.0；无新暂存、commit 或 push，本轮代码和验收文档仍为未提交工作区。缓存 origin/main 的 ahead/behind 为 0/0，但 fetch 失败，不能当作最新远端同步成功的证明。

未完成的累计 R07+R08 发布以独立后续 TODO-008 跟踪，来源为本节；网络恢复后从同一受管入口继续。R08 工程验收与 TODO-007 已完成不因发布失败撤销；R07 用户验收仍未结算，所有未验证边界保留。
