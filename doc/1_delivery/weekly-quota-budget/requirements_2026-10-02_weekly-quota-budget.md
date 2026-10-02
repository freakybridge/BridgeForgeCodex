---
lifecycle: completed
validation_status: verified
---

# 任务预算同时预估周额度

## 已确认目标与授权

用户要求每次报告任务 token 预算时同步估算周额度占比。接受经验约数，拒绝把缺少官方精确公式
当成默认不报告数字的理由。最终只显示：`预计消耗约 N token，占周额度约 y%。`
不显示范围、当前余额、预计剩余量或重置时间；y 为周总额度比例。模型、速度、类别不同需分组，
混合 Agent 分项求和，纯编译/等待不虚构成模型 token。

用户在上述方案后明确“开始开发吧”，授权当前功能实现和必要验证，不含提交、推送或发布。
已有 autopilot 改动属于前一任务，本轮保留并仅衔接其预算显示，不把原有成果归为本轮新增。

## 范围、预算与传播

- M：已有 Skill 预算规则的已知公式增强；不新增计费服务、后台采样器、原生数据库写入或 Rust 算法模块。
- 预算沿用用户开工前看到的约 40000 token / 周额度约 0.6%，为包含验证与修复余量的估算。
  开场 60000 的口径已明确更正，未作为追加授权或新 Goal；时间估计约 45 分钟，最多一个独立审计
  Agent（可复用既有角色），最多两轮验证。计量不可用时明确估算，不能伪报实际消耗。
- 通用产品层：计算规则置于 confirm 的唯一 reference，调用方只引用，不复制系数/公式。
  更新分发登记、版本/CHANGELOG 和自身运行产物；个人临时系数仅存本机 .runtime，不下发到模板。
- 本轮产品版本推进到 1.21.0；未发布，不改变现有 canonical main 正式安装约束。

## 规则与数据映射

1. 每次新报或调整任务预算，读取账户/套餐、bucket、实际模型、速度、类别、token 口径与可用校准。
2. 系数 k 的单位为每万 token 消耗的周额度百分点；各组件 p=N/10000*k，最后合计并四舍五入一位。
   正值不足 0.1% 显示低于 0.1%，超过 100% 不截断，已经包含子任务的整包预算不再重复加子预算。
3. 有同配置临时系数就先给初估，内部标 provisional 和来源。新模型不能静默套旧模型系数，速度
   换算只用已核实的订阅额度倍率，不拿 API/credits 价格冒充周额度规则。
4. 后续利用实际任务前后观察；同账户、同窗口、同配置、无重置且能归属全部消耗才校准。
   零变化不等于免费，受并发污染的样本不能自动覆盖有效校准。至少三个有效样本取中位数。
5. 本机数据 `.runtime/task-budget/weekly-quota.json` 使用匿名账户标识与配置键，不保存对话正文。
   只读咨询不为校准写盘；开发授权内更新必要记录。用户固定值未经允许不覆盖。

## 拟修改与验收

| 编号 | 位置 | 验收 |
|---|---|---|
| 1 | confirm 主入口与 weekly-quota reference | 唯一公式、两数输出、临时系数优先、校准条件明确 |
| 2 | develop/autopilot/collab/debate/escalate | 新报预算时复用中央规则，继承已有预算不重复申请 |
| 3 | 本机运行记录 | 当前临时系数能给出约定示例，污染样本不进入有效校准 |
| 4 | 行为夹具与独立复核 | 单配置/混合/小数/污染/重置/缺配置边界，不伪报模型行为验证 |
| 5 | 分发与版本 | reference 完整登记，metadata/manifest/版本/结构及镜像检查通过 |

验证采用算例核对、既有 Rust 场景结构/分发检查、锁定构建和基线验证。无运行源码变化时不为规则
文案重复所有重型 Git/迁移回归；沿用先前有效证据，版本/分发受影响部分重新验证。
正式用户安装与真实长期校准仍在发布及用户试用后验证，本轮不冒充计费准确度已验证。

## 实施与验证记录

- 已核对预算入口及原生周窗口接口；该接口仅报告使用比例/窗口/重置，没有周 token 总量。
- 当前临时系数按用户已接受的观察保存在忽略目录，不将该账户经验值传播给其他用户。
- 独立 reviewer 完成三条虚构算例，分别正确输出 40000/0.6%、30000/0.6%、100/低于0.1%。
  指出一项 P2：不增加 token 但切换配置时也应更新周额度。已分离“重算显示”与“补充授权”的条件，
  添加同总 token 切换系数的用例；不把静态算例冒充真实账户扣减验证。
- 修复后同一 reviewer 给出 30000/1.2% 的新配置结果，最终静态审计无剩余发现。
- 已通过 skill-metadata（无 issues/warnings）、project-structure（无 errors）、manifest --check、
  git diff --check；本机记录仍为 provisional、污染样本 eligible_for_calibration=false，实际算例
  40000 token 按该记录舍入为 0.6%。git check-ignore 确认本机系数被忽略，公共产品路径无个人系数/账户标识。
- Release 构建完成；受管 build-assets 返回两份 schema_version=2 收据，source_tree_sha256 为
  55a7c21be64b8c3f47f6bfbdddfc19cd8f1861795ffc23fb37cb61f76b98e885。
- 预算复核：本轮开始于 2026-10-02T14:27:57.709Z，主对话非缓存输入＋输出从 646653 增至 737016，
  独立审计从 56750 增至 75413，合计新增 109026，已超过先前 40000 估算。复核不及时，不能把估算写成实测。
  已停止新增工作，只收集在途构建结果；追加最多 20000 token（周额度约 0.3%）的收尾请求等待用户回答。
- 尚待：定向 Rust 场景结构/分发安装/版本/镜像测试及最终基线核对。未运行的测试不计为通过。
  未提交、未推送、未正式安装用户级 Skill；保留原有 autopilot 未提交改动。

## 收尾验证与交付（2026-10-02）

- 用户明确“确认”，批准追加最多 20000 token（周额度约 0.3%），仅完成既有改动的验证和交付。
- 已执行 `cargo test --locked --manifest-path scripts/tests/Cargo.toml <filter> -- --test-threads=1`，
  五项过滤器分别为 gpt6_behavior_cases_have_unique_ids_and_resolvable_inputs、
  shared_bundle_commit_keeps_components_consistent_when_old_image_is_running、
  factory_release_updates_all_three_cargo_manifests_and_locks、managed_manifests_are_current_and_python_free、
  rust_source_is_identical_in_template_and_dogfood。五项各 1 passed、0 failed，整组退出 0；
  日志 `.runtime/weekly-quota-final-tests.log`。其中隔离事务实际安装 confirm 的 weekly-quota reference，
  逐文件核对哈希并验证重复安装 no-op；不代表真实用户目录已经安装。
- 最终受管检查：baseline clean、project_version=1.21.0，fingerprint 为
  84461c31b485f53a059517c6ef91edb712a0a22e34f8b61520f7c0359692054a；factory-version healthy=true；
  skill-metadata 无 issues/warnings；manifest --check changed=false；project-structure 无 errors；diff --check 通过。
- 四个独立算例和修复后的静态审计完成。本轮新增为 Skill 规则与分发资产，未新增运行代码，
  因此只重测受影响的检查，不把上一轮 1.20.0 全量回归冒充本轮 1.21.0 全量回归。
- 当前源码与本地验证已完成，等待用户验收。经验系数仍为 provisional；真实长期校准效果、跨配置
  预测误差及用户级新会话生效需后续实际使用验证，不能宣称为官方或精确额度计算。
- 暂存为空，HEAD 保持 3b540944522d6f72db19a7d8697b5b5180c949e5，未提交、未推送、未正式分发。

## 用户验收（2026-10-02）

用户明确调用 `$summary 同意验收`，接受本轮规则实现、本机临时记录和已经列明的本地验证结果；
另行调用 `$git-sync` 授权保存本次累计改动。上述字段结算此规则交付，长期校准误差和安装后新会话
行为仍为已披露的后续验证边界，不将用户接受估算规则写成官方或精确扣费已经验证。
保留各阶段历史收据；正式同步与安装以随后实际返回的受管收据为准，不提前声明成功。
本次不归档、不新增 Rule/Hook/AGENTS，不写原生 Memory。

## 本次 Git 发布前验证（2026-10-02）

按发布门禁另行执行当前 1.21.0 候选的完整验证，不属于 summary 为收口重复运行测试：
templates/hooks 与 .codex/hooks 的 workspace 分别 193 passed、0 failed、5 ignored；
scripts/tests 完整集 87 passed、0 failed、3 ignored（796.33 秒）。三组命令整体退出 0，日志分别为
`.runtime/release-1.21-template-tests.log`、`release-1.21-dogfood-tests.log`、`release-1.21-factory-tests.log`。
显式 ignored 项不计为已运行；现有独立审计与修复复核继续有效。受管 CLI 自检 1.21.0、基线 clean、
metadata/manifest/版本检查通过；个人系数仍被 .runtime 忽略，不纳入提交。
实际发布版本、提交和推送结果以 git-sync 最终收据为准，用户级安装状态以发布后受管安装收据为准。
