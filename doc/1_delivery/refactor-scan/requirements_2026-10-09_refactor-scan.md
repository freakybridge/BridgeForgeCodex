---
lifecycle: completed
validation_status: verified
---

# 通用 refactor-scan Skill

## 原始需求与已确认决定

用户希望参考 GitHub 上面向 Codex 的 Skill/仓库，扫描自己的已有项目，
以高内聚低耦合、按业务合并散落代码、健壮性可读性和性能为目标形成重构清单，
由用户确认后交给 develop 开发。

1. 用户选择 B：扫描时允许现有定向测试和离线性能基准，区分实测与待验证。
2. 用户选择 A：纳入 BridgeForge 通用 Skill 与受管分发。
3. 完整确认卡提出下述范围和预算后，用户明确“确认并开始实施”。
   本轮允许创建 Skill、必要支持资产、工程检查与独立审计，不授权 Git 发布、
   用户级安装或其他项目写入。需求确认与实施授权在本卡统一记录。

## 目标、非目标与自动化边界

默认扫描当前项目，支持指定目录/模块；先整体定位再沿实际调用关系深入。
输出稳定 R 编号、证据、建议动作、收益取舍、影响面、依赖、风险、验收及用户决定。
保持行为的结构重构、行为修复和性能优化分别标明；业务相似度先核查语义。
允许现有定向测试/离线基准，不以扫描授权执行外部服务、生产数据、依赖安装、
源码自动修复或非必要完整回归。性能瓶颈和收益必须由相应证据支持。

首版使用 Codex 理解代码和项目已有工具，不新增跨语言静态分析程序或测试框架。
默认对话报告；用户要求保存时遵守项目布局并同步索引。
选择条目后仅移交选中范围；实施已有授权直接继承，新增独立问题另列。
confirm/develop 不可用时交付材料，不能虚构已执行。

不直接重构业务项目，不修改 confirm/develop，不安装第三方 Skill，
不手动覆盖用户级受管目录，不提交、推送、发布或预写 VERSION。

## 传播四问

1. 产品层：新增共享 skills/refactor-scan；本需求记录为元文档。
2. 通用能力：不把 Python/FastAPI 或本工厂规则下沉为通用项目架构。
3. 未来 release 需要产品版本及 [product] CHANGELOG；本轮不预写版本。
4. 登记 bridgeforge-codex-manifest.json，执行工厂一致性与完整检查。
   共享 Skill 通过用户级受管分发供工厂使用，不伪造 .codex/skills 镜像；
   正式用户级安装本轮未授权，状态须标为未验证。

## 规模、预算与停止点

M：边界明确的新 Skill，涉及分析、定向验证和既有高级 Skill 交接。
初始预算 45 分钟、约 20,000 新增 token、最多 1 个 review-auditor、最多 2 轮验证。
完整 fixture 检查接近原上限时，用户明确选择“增加 20 分钟，完成现有验证”，
总时间上限调整为 65 分钟，token、Agent 与验证轮次上限不变。
token 未实测。确认卡已说明本机缺少当前默认模型的周额度校准，
本次按 token 预算推进，不跨模型套用旧系数，不报告无依据的周占比。
调研、实现、独立审计、工程检查和合理重测累计计入预算；预计超限停止新增动作并保存进度。
不因 Skill 或 Agent 交接重置。

## 已核实事实与拟修改

doc/README.md 为 flat 布局；共享 Skill 位于 skills/，由根分发 manifest 显式登记文件。
受管 manifest 命令填充 hash，--check 验证模板/dogfood 与分发清单一致性。
当前 CLI 2.0.3 提供 git-sync-direct-release-v1，项目明确要求完整工厂检查及独立审计。
开始时已有 develop 四正文、根 manifest、doc/README.md 和另一需求卡的改动；保留并单独归因。

拟新增 skills/refactor-scan/SKILL.md、references/report-contract.md 及本卡；
更新根分发 manifest、doc/README.md 及 README 最短使用入口。
本轮不改 Rust 运行时、公共 AGENTS 或已修改的 develop。

## 验收与验证计划

| 编号 | 验收目标 | 验证方式 |
|---|---|---|
| A1 | 扫描针对职责和业务，不以行数/相似度直接下结论 | 独立 review-auditor 读取真实 Skill 与代表性代码场景 |
| A2 | B 验证边界、性能基线和未验证状态明确 | 独立场景核验：离线基准、带外部副作用测试、缺基准入口 |
| A3 | 编号稳定、证据可追溯、只交接选中并授权范围 | 独立核验接受候选/授权实施/未授权依赖/不可用 Skill |
| A4 | 分发文件登记与 hash、元数据、工厂结构有效 | 受管 metadata/manifest/baseline/structure 与项目完整检查 |
| A5 | 保留既有工作、版本及授权边界 | 比对本轮差异与此前文件 hash；不执行 commit/push/release/install |

独立规则核验不冒充真实 Codex 自动触发、真实下游或用户场景实测。
本轮仅在隔离临时材料核验工作流，真实业务重构与优化收益不在验收范围内。

## 实施与验证记录

已完成需求落盘、首版正文与分发文件登记。受管 manifest 生成 hash 时首次遇到
目录写权限错误，通过同一入口的窄范围沙箱外重试完成，未手写 hash 或修改权限。
skill-metadata 返回 issues/warnings 为空；manifest --check 返回 changed=false；
project-structure 返回 errors=[]，仅既有归档建议；git diff --check 通过。
baseline 首次因临时 .git/config 写权限失败，同一入口重试后返回 state=clean。
独立 review-auditor /root/refactor_review 审计实际需求、Skill、README、分发 hash
和完整当前差异，结论 passed，无必修问题。收据为 .runtime/refactor-scan/audit.json，
绑定开发指纹 sha256:7885c507113ebe3e201b53c8998895dd82883592f7a68df9534884b4bee4546b。

前向练习只提供隔离的源码片段、命令登记和用户消息，不提供预期答案或执行环境：
billing 的 HALF_UP 收费重复归为 R01；preview 的 ROUND_DOWN 保留；逐账户
load_balance 列为待核实 R02，不断言数据库 N+1 或性能瓶颈。
现有纯计算测试因没有运行环境未执行，生产网关测试因缺单独授权未执行，
没有性能基准不报告实测收益。“选中 R01”仅记录接受候选；随后“实施 R01”
继承授权只交接 R01，R02 仍暂缓。独立练习通过是规则行为核验，
不是金额运行等价性、真实测试、性能或真实项目完整扫描的实测证据。

源码/配置与正式安装、真实下游/runtime、自动发现和用户试用分别判断；后四项仍未验证。

受管完整收口命令实际执行并返回 prepared：

```powershell
.codex\bin\bridgeforge.exe git-sync --prepare-release --message-file .runtime/refactor-scan/message.txt --audit-file .runtime/refactor-scan/audit.json
```

首次因临时锁写权限失败，通过完全相同入口的窄范围沙箱外重试执行。
返回 checks_passed=7，验证指纹与独立审计一致；记录在
.runtime/bridgeforge-codex/release-preparation/current.json。

| 编号 | 完整检查 | 实际结果 |
|---|---|---|
| V01 | baseline、metadata、manifest、structure | 四项退出码 0 |
| V02 | template/dogfood Rust 源码一致性 | 1 项通过，退出码 0 |
| V03 | dogfood 完整 workspace | 223 通过、5 ignored、0 失败；命令耗时约 1014 秒 |
| V04 | 完整 factory fixture | 89 通过、4 ignored、0 失败；命令耗时约 960 秒 |
| V05 | 隔离目标构建与准备记录 | prepared；目标 2.1.0 仅为隔离产物，未发布、未改写当前 VERSION |
| V06 | 原有工作保留 | 四份 develop 正文、VERSION、AGENTS.md 的实际 hash 与开工前一致；当前版本仍 2.0.3 |

测试框架标为 ignored 的入口不能计入独立通过：workspace 包括 file_lock 的
lock_holder_process、runtime 的 loaded_image_child、Hook 的 adapter_stream_child /
adapter_timeout_child 及 high_cost_live_logs_read_only；fixture 包括 child_helper、
factory_ordinary_sync_performance、high_cost_native_stop_reaches_tenth_round_without_extra_request
和 assist_registry_migration_in_isolated_fixture。其中若干是由父测试调用的子进程入口，
本轮未额外执行对应的独立/显式实验；不把它们当作真实安装、下游、性能或 live 验证。

实现、独立审计及约定工程验证已完成；用户于 2026-10-09 明确同意本次交付验收。
累计约 49 分钟、1 个 review-auditor、1 轮完整验证；token 未实测。
权限重试及等待未单独增加验证轮次。本轮未暂存、提交、推送、release 或安装。

## 经验沉淀

1. 重构合并的判断必须落到业务规则和生命周期。相似实现可以具有不同舍入、
   单位或错误语义；行数与相似度只能帮助定位，不能代替消费者和契约核查。
2. 性能报告应拆开重复工作线索、路径测量和瓶颈结论。扫描取得基线与
   develop 同法复测各有职责，不把待测收益写成性能验收通过。
3. 稳定候选编号、接受候选与实施授权分开记录，可以复用已有决定并限制交接范围。
   首版不复制 confirm 的规模阈值或 develop 的实现流程；缺失能力如实交付材料。
4. BridgeForge 的共享 Skill 登记在用户级分发 manifest，工厂 .codex/skills
   不是其手工镜像。受管 hash 一致性与用户级安装/runtime 是不同验收面。
5. 工厂完整检查耗时应纳入启动预算。本轮 workspace 与 fixture 分别约 16.9 和
   16.0 分钟，完整流程共约 49 分钟；临近原 45 分钟上限时取得增加 20 分钟的明确授权。
   等待、权限重试和隔离构建不能从任务成本中扣除；此耗时只代表本轮环境，
   不作为其他项目的固定耗时或周额度校准。

## 用户验收与后续边界

1. 验收依据：用户本轮调用 summary 并明确表示“同意验收”。
   按用户明确验收意图执行 accept 语义；不把这条消息写成用户输入了其他命令，
   也不修改 summary Skill 的参数规则。
2. 验收范围：通用 Skill、报告合同、受管分发登记、最短使用入口、独立前向练习及
   已有 prepared 工程收据。A1–A5 均有本卡记录的完成证据。
3. 正式用户级安装、真实业务项目扫描、实际自动发现及完整 confirm/develop 连续运行
   仍未验证，不因验收标为通过；发布后真实项目试用记录为 TODO-004。
4. 用户验收、Git 发布和受管安装为不同授权。本次仅更新经验、验收及 TODO，
   未提交、推送、发布、安装或归档，不重跑测试、构建或独立审计。
5. 经验与验收事实继续留在本卡；已有约束及当前 Skill 已承载相关规则，
   不新增 Rule、Hook、AGENTS 或 Memory 建议。
