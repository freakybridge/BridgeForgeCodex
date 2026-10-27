# 子包开发收口与主程序发布合同

> 核验 self-test --json：开发需要 package-development-v1、package-stability-v1、task-selected-development-checks-v1 和 version-coverage-v1；发布需要 git-sync-development-closure-v1 和 version-coverage-v1。旧工具缺能力时停止相关版本写入，不手工改号或伪造记录。

本文是 develop / git-sync 共用的版本判级、稳定条件和机器入口事实源。轻量便捷是总目标：开发完成本次必要验证和版本事务即可结束；发布核验累计变化和版本后完成 Git 同步，不补跑 develop。普通同步不主动升版。

## 1. 版本对象与两个基线

根 `VERSION` 管主程序；`product_manifests` 管整 workspace 产品映射，`product_packages` 登记混合 Rust workspace 中跟随产品版本的载体。独立子包登记稳定 id、原生名称、语言、输入归属和唯一版本来源，允许版本不同。普通目录、第三方依赖和骨架工具不自动登记为业务子包；不新增全仓漏登记扫描，保留已有归属检查。

| 阶段 | 比较范围 | 写入责任 |
|---|---|---|
| develop | 每包上次版本记录与当前完整内容；首次以可证明的已发布基线衔接 | 受影响子包版本、必要原生依赖/锁、追加版本记录 |
| release | 上次正式发布至当前最终状态，含已提交、暂存、未暂存和未忽略新文件 | 主程序 VERSION、原生产品载体、必要锁及正式发布记录 |

两个基线分别管理，不能吞掉累计未发布变化。同一内容不重复升版，无实际变化不因达到稳定条件单独升版。包改名、移动、移除、拆分、合并保留显式身份和历史，不复用退役 id。既有版本不统一重置，新包可从 0.1.0 开始。

语义版本用 MAJOR.MINOR.PATCH，高位递增时低位归零；已发布版本不得覆盖、复用或倒退。Python 保留 PEP 440 语义，不支持的格式报告适配缺口，不猜测转换。Rust/Python 之外的语言不能仅凭文档宣称支持。

## 2. 判级与自动 1.0.0

| 实际变化 | 0.x 子包 | 稳定子包 |
|---|---|---|
| 兼容修复或必要内部调整 | patch | patch |
| 兼容新增或公开接口弃用 | minor | minor |
| 不兼容接口、数据、配置或使用要求变化 | minor | major |
| 无实际变化 | 不升版 | 不升版 |

按公开合同逐包分析，不从提交标题推导。依赖升版不自动带升调用方；源码、有效依赖要求或公开合同确有变化时才纳入。未知兼容性停止相关收口。数字级别与兼容性分别记录，breaking 须有绑定当前 fingerprint、scope、reason、source 的明确接受依据；已有适用授权直接复用。

有实际改动的 0.x 子包只有同时满足以下条件才自动选择 1.0.0：公开合同基线明确；非空稳定清单覆盖 core 和实际调用方/场景 integration；完整稳定清单在最终候选上通过；本次无不兼容变化。不逐包追加批准，缺失、失败、跳过或未知不算通过。

稳定检查失败只让对应包回到 0.x 候选，重新验证最终依赖图并报告缺口。普通任务检查失败则回滚，不完成收口。有限检查不证明没有 Bug，不以开发时长或版本已到 0.9 代替稳定条件。

## 3. 必要检查与短期保护

每个实际待处理包在分析的 `validation` 中选择非空的已登记普通检查，依据本次风险选择并保留项目明确必需的检查。配置清单不自动删减；自动 1.0.0 的稳定清单独立完整执行，不能以普通子集替代。

所选检查保留原 id、program、inputs、artifacts、kind 和参数。唯一参数扩展是给 Cargo test 在原参数末尾、原 -- 之前追加一个非选项测试名过滤条件，不替换 flags、manifest、package、runner 或原有后缀。超时可缩短不可放宽，不能自行开启 reusable。过滤命令须有可解析且执行数大于零的 Rust 测试摘要；仅列出、仅编译、零测试或无法证明执行均不通过。既有未过滤自定义 harness 保留原退出码/产物合同，不声称自动识别其数量。

命令从项目根执行，真实退出码 0 且未超时才通过，缺失输入或产物不通过。release 的 analysis.checks 声明不能代替 develop 执行。检查输入、稳定合同、必要环境、原始字节和目录成员仍绑定当前严格计划，漂移时停止或回滚。

版本、原生依赖/锁、生成投影和记录在同一本地事务处理，不写主程序 VERSION 或 Git。检查在最终候选执行，交付物对应最终版本，不把升版前二进制标为最终交付。失败恢复保留外部修改，不留半套已登记状态。

## 4. 版本覆盖与历史

`version-coverage-v1` 追加 `evidence_schema=6`，长期身份仅含 content、dependencies、build 三项；检查命令/输出摘要、产物摘要、理由及接受依据保留作历史事实。记录负责去重、版本连续性及累计公告，不是跨环境长期测试证书。

换会话、机器、工具安装或修改测试策略、检查输入/稳定合同本身，不自动重新打开版本记录；本次改动仍按真实影响验证。源码、原生依赖闭包或实际构建输入变化继续要求处理。持久影响构建的工具/配置应按事实登记为包输入或 build_inputs，不能只用检查环境声明表达源码构建归属。

只使用 Git 证明的 CRLF/LF 等价；二进制、-text、特殊编码/ident 及声明产物保留原字节语义。活动 clean/process filter 阻断，不执行项目 helper。build_inputs 包含忽略文件存在状态、目录全部成员及空目录，事务中增删改仍受保护。隐式 Cargo.lock 按实际依赖闭包比较，显式全局输入仍完整核验。

旧 schema 2–5 保持原字节和哈希链，不补字段或重绑旧证明。已有分项只比较可证明的版本覆盖项；旧聚合记录无法分离证明时，经实际必要检查追加不升版覆盖记录。仅已发布且原记录未变的旧聚合记录可在内容、原生依赖及构建身份完全可证明时只读衔接；显式/未跟踪输入不明则停止。未知 schema、链断裂、覆盖缺口、版本倒退仍拒绝。

旧 portable-development-evidence-v1、git-eol-development-evidence-v1、cross-machine-development-evidence-v1 标签退役，不再广告对应长期证书写入能力；历史读取、Git 内容等价及当前显式环境执行保护继续保留，新消费者以 version-coverage-v1 为准。

## 5. release 的最小职责

release 核验累计范围内的版本覆盖、历史、归属和明确交付物，不执行业务测试或补做 develop。缺口时列包和原因并停止，不降级普通同步。仅环境或检查策略变化不强制整包重新收口；项目显式 required_checks/check_scopes 仍按当前验证指纹及真实记录核验，SDK 等显式环境变化使必要发布检查失效。

release_artifacts 只能引用已登记检查产物，表示发布实际依赖的交付物；当前声明新增、缺失或摘要改变均阻断，develop 可实际检查后无升版刷新。普通 checks.artifacts 在执行时核验，不因换机缺少它们否定版本覆盖。新二进制不继承旧二进制运行结论。

主程序按正式发布以来累计影响判级，只升级主程序及原生产品载体，不再次升级子包。低于建议或复用已发布版本拒绝，无发布变化 no-op。依赖/锁同步不得引入未覆盖独立包变化。成功公告完整列主程序及全部累计子包旧新版本和原因，新增/退役单列，不受效果摘要三条上限限制。

普通 git-sync 不以子包收口或 prepared 为前置，保留 Git、编码、配置、Hook、漂移及最终提交保护。存在 develop journal 时先受管 recover，不同步半完成版本。--release-status 的 readiness_scope=baseline-only 不代表完整发布门禁通过。

## 6. 配置与执行入口

schema 4 保留 release_model=product-packages、原生适配和正式发布 baseline。配置只经既有 --migration-plan/--migration-apply --setup-config-file，提案不填 baseline；已有 schema 4 只更新实际合同，不重切历史或认证未发布内容。混合 workspace 独立包进 packages，产品载体进 product_packages 且明确源码归属；不用整个 product_manifests 吞掉独立包，独立包不得依赖产品载体。

首次纳管旧包用 package_adoptions 声明当前新 id、不可变来源 revision、原生版本和 source 身份；程序从紧邻首次登记的 Git revision 重建真实 workspace/锁核验。不能替代 split/move、冒领退役身份、挑方便历史或改写 baseline。

development 按包 id 登记非空 checks，每项有 id、program、args，可选 timeout_seconds（默认 300，1–14400）、inputs、artifacts。验证脚本/数据须为包输入或 checks.inputs，跟踪产物须为 build_inputs。可选 stability 含非空 contracts 和至少 core/integration 各一项检查。下游 Python 项目保留 python_environment 的解释器、运行时及依赖目录要求，Forge 自身工具仍用受管 Rust。

1. develop --preview --packages <id1,id2> 读取 affected、effective scope 和 fingerprint；无待处理项不写记录。
2. 在忽略的 .runtime/develop-analysis.json 写 `{schema:1,fingerprint,packages:{id:{level,reason,compatibility,acceptance?}},validation:{id:[完整检查对象]},targets?,interface_impacts?}`。不含 product 或 release checks，必要关联包不得删除。
3. 相同包选择运行带分析预览，核对候选及稳定缺口，再用计划指纹 apply。实际终态须为 development-closed；预览成功不是执行完成。
4. 中断用 develop --recover，绑定 checkout 和 HEAD，外部冲突保留。预览/执行的选择、宿主和输入须一致，变化则重新分析，不重绑旧 JSON。

task-scoped-development-v1 的 --packages 接收唯一非空 id，不与 recover 合用。工具展开依赖、调用方和生命周期端点，当前保守纳入依赖，不保证都无需检查。task-development-current 与 outside_task_evidence_checked=false 只说明本次范围；无关历史不自动纳入，不据此宣称 release 就绪。

release 用无分析预览生成上下文，分析只含 product、可选 targets.product 和显式必要 checks。实际 --release 内部重建并验证带分析计划再写入，通常不另跑一次外部带分析预览；需要只读审阅时仍可选用。release-external-analysis-preview-v1 只允许预览读取外部普通无 link/reparse 文件，实际 release/develop 仍须项目 .runtime 路径；预览不证明提交、Hook、推送或网络成功。

## 7. 必要环境与可选复用

项目可显式声明 environment 的 tools（入口/编译器/linker/wrapper）、非秘密 variables、稳定角色到文件/目录的 inputs、cargo_configs。按实际填写，不自动猜合同。Rust 工具绑定文件和版本；目录绑定成员，变量路径按所指内容核验并保留本机运行值。执行仅继承合同参数及必要定位变量，不登记秘密，不以相同 Rust 版本代替 SDK 等价。

保留现有受限 Cargo 配置建模：build/target/term 和远程 source/registries/registry/net；本地 directory/local-registry、path patch、env 注入、credential-provider/alias、未建模 include/alias、linker/sysroot flags 继续拒绝，inline 表不例外。远程索引不得嵌凭证、查询或片段；Git 拉取绑定 Git 身份，相对工具按配置文件基准解析。不为轻量化删除项目配置，不能完整声明时保留原宿主模式。不运行 setup.py、安装脚本、SCM 后端或动态宏猜元数据。

resumable-development-checks-v1 仅为可选同机完整候选缓存：完整显式 environment、项目明确 reusable:true 的确定性检查才可用；在线、未登记状态或必需副作用检查不开启。输入输出不重叠，产物须忽略。核验命令、工具、候选、原字节/目录、严格宿主、Git 身份、日志及产物；失败、超时、漂移不存成功。不重绑、不默认跨机复用、不扩建缓存框架。收据 checks.scope=final-candidate 的 executed/reused 仅统计最终轮，不含失败或稳定回退成本。

旧 development-checks.json / prepared 仅为项目明确要求的可选发布准备，不成为普通开发或 release 前置。本合同不授权安装、下游写入或正式发布。
