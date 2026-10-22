---
lifecycle: completed
validation_status: verified
---

# schema 4 发布接入合同

关联 [TODO-015 确认卡及工程结果](requirements_2026-10-21_schema4.md)。本接口已完成本地实现和工程验证，2026-10-22 用户已验收，见[正式记录](requirements_2026-10-21_schema4.md#用户验收2026-10-22)；尚未发布、用户级分发或接入真实下游。

## 入口与职责

`bridgeforge git-sync` 只同步，不升版。`--release-preview` 为零写入预览；`--release` 在现有 Git 事务中应用相同发布规则。Skill 参数 `release` 对应 CLI `--release`。新能力标识为 `git-sync-schema4-v1`；旧工具不得读取新配置后猜测执行。

schema 4 只提供 `synchronized`、`independent` 两种策略，两者共享 Rust/Python 适配器。配置位于 `.codex/bridgeforge-version.json`，登记稳定包身份、原生声明、输入归属及迁移基线。已有 schema 1–3 可普通同步，但 release 必须迁移；不运行旧升版算法 fallback。

普通同步不要求先选择发布策略或完成迁移；首次正式 release 才配置/确认两种策略。现有配置格式和 Git/产物安全检查仍保留。`--setup-plan/--setup-apply` 的生产入口只创建或更新 schema 4，不再新建 schema 1–3。

## 一次迁移

`--migration-plan` 只读生成配置提案及聚合 fingerprint；`--migration-apply --migration-fingerprint <值> --confirmed-choice` 重新计算并核对后只原子写入配置，不升版、不暂存、不提交、不推送。用户有效的策略决定可复用。可使用 `--setup-config-file <JSON>` 提供明确的新策略/包登记提案。

迁移核对最后正式发布、当前 HEAD、版本、原生包/锁及身份，保存语言无关的发布基线和退役身份，不以当前 HEAD 代替最后发布来跳过未发布改动。旧格式解析只服务迁移及历史证据转换。无法核实、目标漂移、未知/冲突归属或旧显式目标未处理时停止，不部分写入。

## 发布分析与目标

`--release-preview` 在缺少分析时返回 `analysis-required` 及输入 fingerprint、最终变化、逐包归属、程序/包现版本和证据要求。Agent 检查实际差异后，以 `--release-analysis-file <JSON>` 提交逐对象 `level`（patch/minor/major）、`reason`、兼容性说明和适用证据，再次预览和执行使用同一文件。分析文件应放在项目忽略的 `.runtime/` 中；内容会绑定计划，变动时拒绝复用。

分析顶层包括 `schema: 1`、`fingerprint`、`product`、`packages`、`targets`、`checks`；`packages` 以配置稳定 id 为键。每个有实际发布变化的对象必须有分析，不能把一个 feat 提交涉及的所有包判为 minor。Rust 校验完整覆盖、合法级别、最低目标、归属、证据及漂移，不声称可从任意源码自动证明兼容性。unknown 兼容性阻断；breaking 必须 major 且明确记录本次 fingerprint 与影响范围的接受信息。

文件和依赖要求未改但转发公开合同发生变化时，可用 `interface_impacts` 按调用方 id 声明 dependencies 与具体 reason，并提供调用方自己的判级。CLI 核验当前/基线的真实依赖和以本次实际变化为起点的影响链；不进行无条件图传播，不以纯版本目标伪装接口变化。

`targets.product` 是程序显式版本，其他目标按稳定包 id 指定。低于建议、重复使用旧版本或目标未知时直接拒绝。无内容变化的显式目标须有 `pure_version_reason`；只提升指定包，程序仍按整体发布规则升版。没有发布变化且没有目标时 no-op。

## 输入和版本规则

比较最后正式发布与当前最终内容，包括已提交、暂存、未暂存及未忽略新文件。归属采用显式角色和包注册；未知/歧义输入阻断。包发布文件任何变化按包判断，完整撤销无变化；所有 Cargo.lock 及声明的 Python lock 为程序/构建输入，不直接升包。显式 `build_inputs` 改变构建输入而不直接升包；`record_inputs` 只用于明确自动生成记录，不能覆盖源码、原生版本声明或业务生成内容。

依赖目标升版不自动带升调用方。若为了新目标必须改变调用方有效依赖要求，该实际变化须进入调用方分析，不静默修改其要求后仍声称调用方无变化。统一策略的已登记包跟随程序版本；独立策略逐包升版。不管理第三方 SDK 版本，不执行任意 setup.py、SCM 动态版本或安装脚本。

统一产品的 `synchronized_manifests` 用于已有 Cargo 产品映射，包含其 workspace 成员和锁文件；有效成员版本必须与根 VERSION 一致，保留 `version.workspace = true`。多份互不依赖的同名 workspace 分别校验；跨 workspace 的本地依赖按实际闭包校验和更新，不按同名或全局文本替换第三方包。

包移动/改名/移除/拆分/合并沿用显式生命周期身份和已发布历史，退役身份不能复用，新包保留合法首次发布版本。项目自有 Hook 和构建系统需按该合同适配；Stratus 的 BT 构建缓存继续归 Stratus。

## 证据、事务与验证边界

配置显式声明必要检查，检查证据的 fingerprint 使用预览返回的 validation_fingerprint，绑定最终源码、依赖、构建输入及环境，并声明检查 id 和实际影响对象；VERSION、CHANGELOG 和声明的自动记录不参与验证内容指纹。暂存或普通提交本身不会使相同内容的有效检查失效。发布分析及不兼容 acceptance 使用完整 fingerprint，仍严格绑定 Git 状态。通过、失败及有依据的无关历史失败分别记录；新失败、关联不明、缺失或失效必要证据阻断。业务验证由项目在发布前完成，git-sync 不组织全量测试、不依赖工厂 prepared 或审计 JSON；缺少未声明的开发记录不阻断同步。

预览零写入；迁移/应用前重算 fingerprint；fetch 后核对 HEAD、清单、配置、内容和分析，漂移时停止或重新分析。发布候选按实际内容/模式差异计算，排除只因 Git 索引 stat 缓存刷新而变化的假脏候选；相对 HEAD 的真实净还原输入仍绑定，不削弱内容、环境、暂存状态及并发漂移保护。沿用 Git 锁、索引及提交 tree 保护、外部修改保留、受保护回滚和 push 终态核验。

Python 验证证据要求配置 `python_environment.interpreter`、`runtime_roots`、`dependency_roots`，明确实际解释器、运行时/标准库和完整依赖目录，包括外部 editable/local 依赖。只读内容哈希，不执行安装或后端，不因 PATH/VIRTUAL_ENV 字符串未变就认定环境未变；应用前再次核验。没有该身份时不认证 Python 检查证据，普通同步和不依赖检查证据的发布不被强加运行 Python 的要求。

`check_scopes` 可将 required_checks 分别绑定包或 product；不填则保持全局检查。只要求各必需检查覆盖其本次适用范围，无受影响对象的局部检查可复用/不重跑；通过证据合计覆盖本次影响范围，单包检查不必虚报成全项目检查。

迁移配置保存初始策略合同及内容摘要。首次 schema 4 正式发布前，迁移完整性校验从不可变 Git 原生元数据重建包版本、依赖、退役身份与历史身份快照，并与配置核对；首次正式新格式发布后以已提交合同为边界。baseline 不可任意漂移，不能通过同步修改当前版本与序列化基线隐藏手工升版。唯一受管补正为首次正式发布前，从同一历史提交证明并添加 Cargo 外置入口及静态可达文件图的精确路径与内容 hash；已有 hash 的键和值不得删改。已普通提交的迁移也必须通过同一原生证明复核，已正式发布的基线禁止改写。`--repair-baseline-plan/apply` 复用指纹、锁和原子配置写入，不能接受任意 baseline 或改变策略。此校验属于迁移证据核对，不执行旧提交判级或依赖升版算法。

本任务在工厂和临时仓库验证统一/独立 × Rust/Python/混合、历史迁移、净差异、输入例外、显式版本、生命周期和失败保护。真实下游升级、真实用户安装、业务 runtime、真实发布分别保持未验证，须另获授权。
