---
lifecycle: active
validation_status: awaiting_validation
---

# Git 同步编排开销优化

## 2026-10-20：R10、R11、R08 扩展实施

关联原需求 TODO-009，不重复新增 TODO。用户明确授权按 R10 → R11 → R08 实施；R07、R09 暂缓；不提交、不推送、不发布，保留前一轮 R04/R02/R01 的全部未提交改动。本节是本轮选择与验证事实源，后文保留此前交付证据。

用户确认本轮为 M 级、累计 210 分钟、约 30000 token（估算未实测，按既有系数约周额度 0.5%）、最多一名独立 review-auditor、最多两轮验证。范围含实现、定向检查、独立审计、七项工厂检查和隔离目标构建、合理修复余量。沿用已验证的 `D:\bfc-01a156` 专用临时目录；固定原 C 盘 Cargo target、禁用增量缓存、仅对验证子进程移除继承的 PSModulePath。检查前重新核对目录归属、Git 祖先隔离和原生 PowerShell 模块，保持全局配置不变。

验收安排已由用户明确调整：本轮完成工程验证和隔离 CLI 同方法对照；真实“Skill 指令 → 最终回复”的复测留到下一次明确授权的独立同步调用，不将 CLI 或当前开发回合时间当成端到端改善。当前工厂有未提交变更，禁止为测速执行真实同步。不得写真实下游或用户级安装。

| 条目 | 实施与保留边界 |
|---|---|
| R10 | 受管 Rust CLI 增加零写入聚合预检，Skill 用能力门控合并前置读取，配置已确认时不展开首次配置手册；审阅真实 diff、release 预览、fresh fetch、终态核验均保留 |
| R11 | 只在明确权限错误时提升权限原命令重试一次；TLS/网络、凭据、分叉、Hook、版本错误分别报告，不能因发生于 fetch 就进入权限重试；CLI 既有推送后复核不改 |
| R08 | 单调时钟记录发布分析、产物准备、基线及 Git 子过程耗时，成功与失败均保留已取得数据；commit 包含 Hook，父子耗时不可相加；保留原退出码、错误文本、已有收据语义及事务顺序 |

传播四问：通用产品层，源在 skills/git-sync 与 templates/hooks，传播到自身 dogfood、分发与受管清单；工厂专用预算、基准和临时路径仅留本卡，不下沉公共模板。需要产品版本与 CHANGELOG，但由后续明确 release 的受管事务处理，本轮根 VERSION 保持 2.6.0，不预写 CHANGELOG。

基准为 main@76643c9e，加已完成未提交的维护构建复用改动。变更前受管 CLI 字节 SHA256 为 `627a68304f64df08a24f6f5892c43f2868c3c5ee8b4dcd2e6640da692f8b953f`；副本及相关文件快照保存在忽略目录 `.runtime/git-sync-performance/`。真实历史样本：Holdem 无改动同步 65.304 秒（CLI 5.120 秒）、Risk 67.222 秒（CLI 6.072 秒）；Holdem 工具调用外间隔 43.598 秒。样本不承诺新版本固定时延。

实现、独立源码审计和完整工程验证已完成，工程交付已获本次用户验收；分发与真实端到端复测仍待后续授权。聚合预检不做 fetch、配置写入、锁/缓存初始化、构建或版本分析；旧 CLI 未声明新能力时禁止试跑未知参数。失败新增结构化计时收据属于已授权接口扩展，不将 unknown 伪造成 clean、0/0 或未推送。

### 本轮实现与定向证据

| 条目 | 实际结果与边界 |
|---|---|
| R10 聚合与兼容 | 新增 `git-sync-preflight-v1` 能力和 `git-sync --preflight`。Skill 先自检门控，再合并读取；旧工具走只读兼容路径，禁止用未知 flag 探测。已确认配置不重复 setup-status，不展开首次使用手册；真实 diff、release 预览与终态核验保留 |
| R10 零写入边界 | 只允许 Git 查询，禁用 fsmonitor 与 optional locks，diff 禁外部 diff/textconv；检查实际生效的 clean/process filter，未使用的全局 LFS 注册不阻断。活动 filter、Gitlink 子模块和不完整属性响应保守阻断聚合预检，交主对话专项核验，不禁用项目功能或改全局配置；实际 sync 原算法保留 |
| R11 重试分类 | 仅明确当前项目文件/目录不可写才允许相同命令提升权限重试一次。裸 Permission denied、SSH 鉴权、TLS/SSL、网络超时不能据此重试；推送失败后的 CLI 原有复核仍保留 |
| R08 阶段计时 | 新增 `git-sync-phase-timing-v1`、`timings_ms` 与 `timing_semantics`。覆盖输入校验、发布分析、产物准备、基线、自动写入/回滚和 Git 子过程；commit 含 Hook、push 含 pre-push、pull 可含 fetch，父子阶段不可相加。失败保留原退出码/错误和已取得数据；包装器转发产物缓存 traits，不改变事务顺序 |
| 定向测试 | 6 项 preflight、3 项 timing、1 项 CLI 能力及互斥测试通过；1 项行为夹具输入合同通过。断言含 clean/dirty 与配置分支零写入、helper 阳性对照、实际 push target、畸形属性/Gitlink 阻断、TLS/超时/启动失败仅一次 fetch、真实本地 commit/Hook/push、缓存转发。行为夹具合同不等于真实 Agent 行为已验收 |
| 独立源码审计 | 同一名 `review-auditor /root/review_build_reuse` 已复核本轮新增文件和相对本轮 before 快照的改动，最终计时增量也通过；已修正重复 setup-status 与 SSH 权限误分类。审计者未运行测试；测试执行由主对话负责 |
| 当前版本产物 | 受管 build-assets 成功；新 CLI SHA256 `ba1c9b85afe3bc3f85fd2f288f3630b94c69b296ff222de83ee81f7a9ebec34a`，自检两项新能力、baseline、manifest --check 通过；根 VERSION 仍为 2.6.0 |

### 本轮隔离 CLI 对照

运行 `cargo test --locked --offline --manifest-path scripts/tests/Cargo.toml git_sync_preflight_pipeline_performance -- --ignored --nocapture --test-threads=1`：1 项通过，旧/新各 5 次，交替先后顺序。两者在同一独立临时项目与本地 bare remote 上执行自检、前置读取、真实无改动 sync 和相同终态核验；断言 clean、0/0、无 push、无构建、无升版、HEAD 不变。测试仅初始化本地夹具，未同步真实仓库。

| 测量项 | 旧版中位数 | 新版中位数 | 解读 |
|---|---:|---:|---|
| 前置读取 | 1873 ms | 1645 ms | 本次样本减少 12.2% |
| sync 进程 | 1898 ms | 2075 ms | 本次样本增加 9.3%，不能声称 CLI 内部全面提速 |
| 整体 CLI 流水线 | 4220 ms | 4061 ms | 本次中位数减少 3.8%（159 ms） |
| 整体样本范围 | 3400–7284 ms | 3732–8515 ms | 样本少且波动大，不足以证明稳定幅度 |

原始数据：`.runtime/git-sync-performance/cli-comparison.json`，记录旧新 CLI SHA256 和各阶段值。该结果是 `isolated_cli_pipeline_not_chat_end_to_end`，不包含模型思考、工具往返、网络远端、真实 release/构建或错误重试；R10 的往返收敛、R11 的异常路径收益与用户实际端到端提速仍待下一次明确授权的独立同步调用。不得用本次 3.8% 外推历史 65–67 秒样本或承诺固定秒数。

本轮经验：零写入预检必须验证 Git 隐式 helper 副作用，不能只看命令名称；单独的 Permission denied 不能区分文件权限和 SSH 鉴权；性能数据需要分别保留前置、CLI、终态核验和真实对话口径。验证沿用外部短 TEMP 与子进程 PSModulePath 隔离，避免已知 Windows 环境问题；不清理他人缓存、不修改全局配置。仅在完整准备收据取得后声明工程验证通过，TODO-009 保留待验证。

首轮受管准备在 metadata 阶段发现 Skill 入口 127 行超过 120 行限制，workspace/fixture 尚未启动；删除已由主流程覆盖的输入清单与重复失败说明后为 119 行，保护规则未改，metadata、manifest 与 structure 已通过。分发清单已重新生成，独立审计和开发指纹重新绑定后恢复最终验证；不沿用首轮失败为成功证据。

### 本轮最终工程结果（2026-10-20）

第二轮受管准备已成功：`git-sync --prepare-release --message-file .runtime/git-sync-performance/message.txt --audit-file .runtime/git-sync-performance/audit.json`，返回 `prepared`、`checks_passed=7`、`checks_executed=7`、`checks_reused=0`，隔离目标版本 2.6.1。本轮工程结果覆盖现有工作区（保留上轮改动），不把上轮实现归为本轮成果。

| 工程证据 | 实际结果 |
|---|---|
| 短检查与镜像 | baseline、Skill metadata、manifest --check、project structure、模板/dogfood Rust 全树一致性全部通过；结构检查的归档建议不属于本轮范围 |
| 完整 workspace | `cargo test --locked --config scripts/tests/factory-cargo.toml --manifest-path .codex/hooks/Cargo.toml --workspace -- --test-threads=1`：435 通过、13 跳过、0 失败，受管阶段 2606.852 秒 |
| 完整 factory fixture | `cargo test --locked --manifest-path scripts/tests/Cargo.toml -- --test-threads=1`：93 通过、7 跳过、0 失败，受管阶段 1435.756 秒。跳过项为显式性能实验、需外部授权/模型/环境的场景及子进程 helper；本轮新 CLI 对照已另行显式通过 |
| 独立审计绑定 | `review-auditor /root/review_build_reuse`：passed，最终开发指纹 `sha256:832068750d6c45ae44ee9bd4deb136df3f4e3038ed29b0cc9d9f6347230a4d0b`；审计记录 `.runtime/git-sync-performance/audit.json` |
| 隔离目标构建 | 2.6.1 的 CLI/Hook 与构建收据均已取得内容哈希；根 VERSION 保持 2.6.0，HEAD 保持 `76643c9e8c2c405ac24709987684a74542d04e94`，暂存区为空 |

正式本机收据为 `.runtime/bridgeforge-codex/release-preparation/current.json`，各项完整日志按 `checks[].log_sha256` 存于同目录 `blobs/`。交付记录补写后已按记录类文档规则轻量复用绑定，返回 `checks_reused=7`、`checks_executed=0`，不组织第三轮完整验证。本轮在 210 分钟预算内完成，同一名审计者；token 仅估算、未实测。

未提交、未推送、未发布；未写真实下游或用户级安装。新共享 Skill 和能力尚未分发到用户级及其他项目，真实下游/runtime 使用与“Skill 指令 → 最终回复”性能仍未验证；R07/R09 继续暂缓，TODO-009 不结算。

### 用户验收与发布授权（2026-10-20）

用户明确调用 `$summary accept` 和 `$git-sync release`，确认 R10/R11/R08 的当前工程交付符合本轮约定，并授权将本会话两轮已验证改动合并受管发布、提交、推送。保留 R07/R09 暂缓，不新增实现或测试，不更新真实下游、用户级安装或全局 Git/TLS 配置；实际版本与同步状态以最终受管收据为准。

TODO-009 的工程部分已验收，但原需求要求的“Skill 指令 → 最终回复”同口径对照尚未取得，状态仍为待验证、归属仍为原需求，不改成后续事项绕过原验收条件。本次调用还包含 summary 验收写入，不能作为独立 git-sync 端到端性能样本；也不能用 3.8% 离线 CLI 中位数变化承诺真实提速。用户级分发与真实下游/runtime 边界不因发布授权自动变为通过。

前文经验已覆盖零写入 helper 副作用、错误分类、阶段计时及 Windows 验证环境。本次补入用户决定和 TODO 边界，不重复创建文档或新增规则；历史记忆关于受管产物与能力门控的边界只作对照，不替代当前收据。TODO-009 未完成，当前 topic 无归档候选；无新独立问题需要新增 TODO。

## 目标、授权与预算

用户指出本次 release 6 分多钟过长，并明确要求“需要优化减少”，授权实施减小发布前后编排与回读开销。保持受管入口、自检、真实 diff 审查、预览、锁/身份/漂移/回滚、轻量 pre-commit、最终独立 0/0 核验；不执行新的真实仓库 commit/push/release，不改 model/effort、Git/TLS 配置或其他项目，不手工安装用户级 Skill。正式需求关联 TODO-009；TODO-006/008 等不在本次收口范围。

M：45 分钟、约 20000 新增 token（估算，未实测）、最多 1 名 review-auditor、最多 2 轮验证。包含调查、共享 Skill 和内部计时、定向检查、工厂强制完整工程检查及合理重测；无适用周额度系数，不编造百分比。预算累计不因 Skill/Agent 切换重置；预计超额时停止新增动作并报告。

基准为 main / 3013776f045c65357c0b32753afd6534b2beed71 / VERSION 2.3.0，工作区干净。前期读取 develop/confirm/skill-creator 及直接调用点，已存在改动的 Git 保护算法不重设计。

## 实际基线

本次会话事件时间戳（本机 Singapore UTC+8）：

| 阶段 | 起止 | 时间 |
|---|---|---|
| 用户输入到最终回复 | 23:34:23.960 → 23:40:40.976 | 377.016 秒 |
| 输入到 release 调用 | 23:34:23.960 → 23:35:56.895 | 92.935 秒 |
| release 调用到 Git 推送目标更新 | 23:35:56.895 → 23:37:14 | 约 77.105 秒 |
| 推送目标更新到读回成功收据 | 23:37:14 → 23:38:33.158 | 约 79.158 秒 |
| 成功收据到最终回复 | 23:38:33.158 → 23:40:40.976 | 127.818 秒 |

77 秒是调用到推送完成的边界，不是整个 CLI 精确耗时；推送后命令何时结束未有独立逐阶段计时，不能把剩余时间全算 Agent 纯等待。此前确实构建 2 个 2.3.0 产物，2.2.3 候选不符合目标合同，未执行全量测试。优化聚焦重复读取、过大预览上下文、回读间隔与成功后的继续处理，而非删除必要构建或保护。

## 方案与传播四问

1. Skill 将正常路径收敛为一批初始核验、一次精简 release 预览、连续等待、一批终态核验后立即返回。新证据/异常才扩读，保留真正首次 diff 审查与最终实际推送目标核验。
2. core sync 包装器用单调时钟添加 sync_elapsed_ms，原事务主体、参数、错误和调用顺序不变；没有新 IO、持久日志或性能轮询。CLI 能力标记新增 git-sync-timing-v1，旧工具缺字段兼容并明确未测。
3. 属于通用产品层：shared skills 和 templates/hooks，镜像/manifest 同步；版本/CHANGELOG 在另获 release 授权后由受管事务处理，本轮不预写。当前用户级安装不修改，源码交付不等于跨会话已安装生效。
4. 效果目标为减少原约五分钟的前后处理；20–30 秒回读与四阶段是可检查的调度约束，不承诺模型/网络固定时延。

## 验证与交付边界

使用相同用户输入/工具事件/最终回复口径留存基线；真实端到端改善需要下一次用户实际 git-sync 对照，不通过未经授权的真实提交/推送造样本，也不把当前开发任务耗时当作优化后同步时长。

定向检查收据仍保留成功/失败状态与原字段，新增毫秒值为非负整数且不超过外层同步调用计时；真实本地 Git 夹具验证计时来源、成功语义和失败保护。全量工厂 workspace/fixture、镜像、metadata/manifest/baseline/structure 与独立审计遵守已有清单。已提供同一会话指令，后续可直接按新最短路径执行；正式分发/真实端到端结果未取得前明确未验证。

## 记录

工程实现和检查已完成，用户验收、用户级 Skill 分发与真实同步总耗时复测尚未完成；TODO-009 保留进行中，不代替 summary accept 收口。

| 验证 | 实际结果与边界 |
|---|---|
| 定向 core 测试 | clean_parity_has_stable_receipt：1 项通过；原状态字段保留，新增内部耗时为非负整数 |
| 既有隔离性能夹具 | factory_ordinary_sync_performance：1 项通过；外层 14617 ms，内部 sync_elapsed_ms 14464 ms；缓存复用 2、构建 0，临时本地 Git 推送后 clean、0/0；不是正式 release 或对话总耗时比较 |
| workspace | 270 项通过、8 项按既有设置跳过、0 失败 |
| 工厂夹具 | 90 项通过、6 项按既有设置跳过、0 失败；包含普通同步/release 内部计时不超过外层调用计时的断言 |
| 受管开发准备 | git-sync --prepare-release：7 项检查通过，隔离目标版本 2.3.1；根 VERSION 保持 2.3.0，未提交/推送/正式发布 |
| 独立审计 | review-auditor:/root/r07_audit：passed；原事务主体保持不变，镜像/manifest 一致，预览、身份、锁、漂移、回滚和最终实际推送目标核验保留；失败出口计时未单独实测 |

验证指纹：sha256:52d312db66f7d78c626f1a65881f9e1f2811c78f8dc18c52a1e1425431eb57fb。受管准备收据位于 .runtime/bridgeforge-codex/release-preparation/current.json，审计记录位于 .runtime/sync-orchestration-audit.json，均为本机工程证据。

当前项目受管 CLI 已构建安装，self-test 返回 git-sync-timing-v1；共享 Skill 仅修改工厂源码 skills/git-sync/SKILL.md，未手工改写用户级安装。下一次用户授权的真实 git-sync 应按四阶段路径执行，并同时记录用户输入到最终回复及 sync_elapsed_ms；取得同口径证据前不宣称已从 6 分 17 秒降到某个时长。
