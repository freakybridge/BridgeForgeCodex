---
lifecycle: completed
validation_status: verified
---

# git-sync 执行环境一致性与单次确认

## 目标、授权和边界

2026-10-22 用户明确要求修复 release 预览与实际执行跨沙箱产生的指纹失配，并检查整个 git-sync 是否实现最多一次用户确认。当前事实：受限与沙箱外执行的 TEMP、TMP、LOCALAPPDATA 不同；Rust 将其计入发布环境身份。前次带分析预览成功，提权后的实际同步因 analysis stale 停止，未 fetch、未提交推送，VERSION 仍为 3.0.0。

本次按 develop 实施产品层通用 Skill 编排修复，同步共享源与工厂 dogfood；不删除环境身份或放宽 Rust 漂移保护，不修改 Git 事务，不安装用户级或写真实下游，不执行真实 release、不提交推送。关联既有后续 TODO-018（发布中断）及本次原需求 TODO-019（全流程确认上限）。TODO-016、TODO-017 的已验收实现保持不变；本次新改动需独立验收。

传播四问：产品层通用行为，事实源为 skills/git-sync/ 及 templates/hooks/；同步 .codex/skills/git-sync/、.codex/hooks/ 与分发 manifest。需要产品版本及 CHANGELOG，但只在后续用户显式 release 的受管事务中生成，本次不预写版本。保留环境与 Git 漂移保护，原生修复仅规范化发布候选的内容判定。

## 规模与预算

M：一个 Skill 入口及三个 reference 合同、一个 Rust 发布输入模块和对应回归，跨执行环境及确认路径，复用现有 CLI 和事务，无新增持久协议。1 个既有 review-auditor 负责只读独立审计，主对话实施，token 未实测。用户初批 2 小时后，已明确改为修复和必要验证完成后再交付，旧时间和单轮失败停止限制不再适用；仍不提交推送或执行真实 release。最近完整 workspace 约 59 分钟、fixture 约 18 分钟，保留为耗时参考。

## 调查结论

1. Rust CLI 不询问用户，只消费 --confirmed-choice 和逐对象 acceptance；没有调用级计数器。现有 Skill 仅局部规定策略问一次，策略/迁移、基线补正、逐包不兼容接受及权限恢复之间没有统一上限，因此原实现不满足用户要求。
2. 已确认跨沙箱会改变 TEMP、TMP、LOCALAPPDATA，并使旧分析失效；但后续也出现过同称沙箱外的准备失配，尚不能将全部历史 stale 都归因于权限切换。保留完整环境身份，最终预览到执行须在同一宿主连续编排，不排除环境变量来掩盖变化。
3. CLI 尚不支持针对未落盘迁移配置直接完整预览发布；无法提前查明所有风险的异常项目应停止，不以“一次确认”为由猜测同意或承诺任意项目必定闭环。

## 实现合同

1. 首次有效预览前根据平台明确限制及当前项目可核对的具体权限拒绝证据确定执行环境；有明确 .git 只读限制时从一开始就在获准环境执行，不创建试写探针。
2. 初始化模块和序列化后，最终无分析预览、完整分析生成、带分析预览和实际 release 必须在同一宿主进程内连续执行；核对当前实际变化与已审阅事实及决定一致。宿主中断或切换时整段重走，不只替换 fingerprint、不伪装环境、不搬用旧 passed 证据。
3. 整轮最多主动发起一次用户确认：策略、迁移/补正、所有 breaking 对象和新增范围统一汇总；既有有效决定复用，普通已配置路径零新增业务确认。确认后新实质风险、用户修改/停止均保留现场并停止，不追加第二问。
4. 主动触发且可能需人工批准的平台权限请求纳入同一预算；既有权限或平台自动批准不算新增用户确认，无法观察则保守预留。平台批准不能替代缺失业务决定；不能在一次交互内齐备则停止。平台自身强制 UI 不由仓库控制，不保证其次数、不绕过或扩大权限。
5. 普通同步仍原命令恢复一次；release 在新执行环境完整重新分析和预览后接续同一受管入口，最多一轮权限恢复。TLS、鉴权、分叉、冲突、Hook 改写、版本和实际构建错误仍停止。

## 验收与证据

| 场景 | 预期 |
|---|---|
| 已配置普通同步/release | 复用已有授权，不另问提交或推送确认 |
| 首次策略、迁移、多个 breaking 对象 | 能提前查明的决定合并一次；不能查明则停止，不先写配置后逐轮问 |
| 基线补正 | 复用适用授权；新缺口纳入唯一确认，用过机会则停止 |
| 已知 .git 只读限制 | 首次有效预览前确定实际权限上下文 |
| 提权后恢复 release | 旧环境分析被拒绝；新环境完整重建分析并预览通过 |
| 确认后新范围/环境中的验证证据失效 | 不自动接受、不追加第二问，不伪造新环境 passed |

验证使用同版本受管 CLI，只读重现跨环境旧分析失效及同环境新分析通过；检查 HEAD、VERSION 和索引未变，不触发真实同步。Skill 自身确认次数由编排合同及独立审计验证，不能把静态审计或 CLI 无交互误写为真实对话/平台 UI 次数已经实测。

项目强制 baseline、metadata、manifest、structure、镜像、完整 workspace、完整 factory fixture、独立审计及隔离工程准备仍执行；当前结果待补。既有 456/96 项结果保留为历史，不冒充本轮新结果。

## 本轮实施与定向结果

四份共享 Skill/合同与 dogfood 镜像已同步；为遵守入口 120 行硬闸，将权限细节移入 runtime-context.md 并显式登记 source/target/hash，受管分发 manifest 刷新。独立 review-auditor 审计通过：确认上限没有缩减为仅业务询问，人工权限请求计入预算，授权不足时停止；权限恢复仍要求真实输入重新分析，不修改 Rust 环境身份或 Git 保护。

真实工厂 CLI 只读回归通过：相同源码差异下，受限环境旧分析在沙箱外退出 2 并明确报 stale；沙箱外连续两次无分析预览指纹相同，按实际输入重新生成完整分析后，带分析预览返回 release-planned、目标 3.0.1。HEAD、Git index 内容 SHA-256 和工作区 VERSION 均未变。结果位于 `.runtime/git-sync-context/context-regression.json`，不是实际发布或用户对话次数验证。

前次审计指纹 `ff80a8d42daba008636314cffda7c3e79b9b81eb08a090a06b637d768c326271` 对应拆分 reference 前状态；最终工程准备须绑定登记后的当前指纹。临时 debug 哈希诊断因 LLVM OOM 构建失败，未运行且已全部撤销，两份 analysis.rs 与原源码无差异；本机当时约剩 1.0 GiB 物理可用、1.4 GiB 可提交内存，后续 PowerShell 初始化也报 OOM。未使用替代产品实现；轻量同步/检查改由原生 cmd 宿主直接调用同一受管 Rust CLI。无真实提交、推送、用户级安装或下游写入。

## 前一轮中断记录（已恢复）

入口 132 行超过 120 行的 metadata 失败已修正：权限细节移入 runtime-context.md，四文件源/镜像一致并显式登记分发；元数据检查返回 issues=[]、warnings=[]，受管 manifest 返回 changed=false。最终增量独立审计通过，绑定的工程输入指纹为 `4a2e590966cbbfc45b255885438f0411890b8c7d4d28a40f791431ae8bbaf4f7`；临时 Rust 诊断不在最终源码中。

完整工程准备仍遇到独立缺口：同一 elevated PowerShell 进程中的两次无分析预览相同、带分析预览 release-planned，且裸 git diff 前后 index hash 不变；该次准备通过分析后停在上述 metadata 硬闸。修正承载并切为沙箱外 cmd 宿主后，最终无分析预览返回 fingerprint=`637aa6b583b92ddb09f47191e5e3a5ad1d4e53cc95766629a09038c0e0aee9cf`，完整分析据此生成；最终 prepare 退出 2，再次报 `release analysis is stale or has an unsupported schema; inspect final inputs again`，未启动完整 workspace/fixture。实际变化的指纹组成尚未取得，因此不能把所有 stale 都归因于沙箱切换，也不能宣称整个发布问题已修复。

用户在实际 OOM 和硬闸消耗验证缓冲后，明确允许“当前这一轮跑到成功或明确失败后收口，不再追加修复或新一轮测试”。最终这一轮已明确失败，按此决定停止，不继续重试或扩大实现。已有源修改、独立审计和跨环境正反回归保留；此前 456/96 项与旧 prepared 收据不能替代本轮完整工程验证。

该次收口没有完成工程验证，也不能宣称全部发布问题已经修复。

## 继续实施授权与当前状态

用户随后明确要求“彻底修好再交付”，并回答“允许，修复和验证完成后再交付”，覆盖原 2 小时及单轮失败收口限制。范围仍限执行环境一致性和全流程最多一次主动确认；不提交、不推送、不发布。

继续定位时，临时 debug 诊断构建成功；同一宿主连续预览/准备的逐项输入和环境哈希一致，debug 与受管正式 CLI 捕获也一致，均通过分析硬闸后到达当时的 metadata 检查。跨宿主历史失败的具体差异未复现，不能据此推断所有历史失败原因。全部临时 Rust 诊断源码已撤销，两份 analysis.rs 无本轮差异。

最终合同进一步收紧为同一宿主内连续最终预览、实际输入核对、完整分析生成、带分析预览及执行；宿主中断必须重走最终段。共享源、dogfood 和显式分发登记已同步，metadata issues=[]、warnings=[]。当前继续独立增量审计及完整工程准备，完成后补充实际结果；TODO-018/TODO-019 尚未结算，真实自然对话和平台 UI 次数未实测。

## 同宿主 stale 的复现与源码修复

正式 CLI 在同宿主最终预览通过后，prepare 仍报 stale；前后完整指纹从 e79cf2fe… 变为 805f104d…，但 validation_fingerprint 与 44 条实际 changes 完全相同，Git index 字节变化而暂存内容未变。随后正式与诊断 CLI 捕获一致，排除了此例中的构建模式差异。

隔离 Git fixture 证明：内容相同但 stat 过期的文件，在禁止刷新索引的 --name-only 查询中作为候选出现；普通 diff 刷新索引后候选消失。schema4 inputs 原先把候选无条件插入完整指纹，即使 old==new 已从 changes 跳过，仍保留候选哈希。新增自动回归在旧源码真实失败：changes 和验证指纹相同，完整指纹 ad41b2b0… 与 8be6f408… 不同。这确认了一条同宿主失败机制，并不推断所有历史 stale 的具体路径。

两处相对正式边界/HEAD 的候选查询改为 --numstat --no-renames -z，按实际内容差异排除 stat 假脏，使用第三个 tab 字段及 NUL 记录保留路径边界。环境变量、Git 状态、raw mode、真实输入读取和并发终态校验不变，净还原到正式版本但仍相对 HEAD 脏的输入继续绑定。模板和 dogfood 同步，受管当前版本资产重建后进入最终工程准备。

定向绿测：stat 刷新稳定、预览索引零写入、真实内容/暂存/模式漂移拒绝通过（1 passed，56.66 秒）；二进制修改、空文件新增及删除、纯模式修改和空格路径实际出现在 changes（1 passed，15.37 秒）。tab/换行文件名仅 Unix 分支，当前 Windows 未执行该分支。旧净还原保护由既有回归及后续完整 workspace 覆盖，不伪称定向已执行。

工厂自身验证配置将 workspace 单项上限从 3600 秒放宽为 5400 秒：既有实测约 3543 秒已接近原硬上限，本轮增加回归且此前存在资源压力。保留完整检查、4 线程及原断言，仅为本项目本地验证留出有界缓冲，不向下游分发，也不改变 Git 同步/发布检查范围。

当前版本受管资产重建成功：source_tree_sha256=019a8e2b710f66237e1849f21b8a58b05f02c028de64352979cfdde4b927214e，CLI/Hook 收据一致，manifest changed=false，工作区仍 3.0.0。新正式 CLI 跨环境正反回归通过，旧环境分析拒绝、当前实际输入与已审阅事实相同、完整新分析预览 release-planned，未改 HEAD 或暂存条目；结果见 `.runtime/git-sync-context/final-context-regression.json`。

首次进入完整检查后，主 Agent 发现自己新增的 Unix 字面反斜杠路径用例违反既有 product_inputs 约束，独立审计进一步指出 Unix mode-only 还应改变真实工作树权限；这属于新增测试的跨平台错误。已删除非法路径，并在 Unix 条件分支设置真实执行权限，合法 tab/换行分支保留。精确中断约十几分钟的旧工程准备后重新绑定审计及全量验证，不手改成功缓存或把旧结果冒充新指纹通过。生产源码和已构建 3.0.0 资产未因此变化；Unix 分支仍未实测。独立审计已更正前轮遗漏。

## 切换前保存（2026-10-22 21:17，北京时间）

用户显式调用 `$git-sync switch`，本次改为普通提交与同步保存当前工作，覆盖此前“先不提交推送”的限制；未请求 release，不升版。此前工程准备已精确停止，避免提交改变 HEAD 时仍认证旧输入。全量 workspace 自 12:39:50 UTC 运行约 37 分钟后中断，未返回最终结果，不能记为通过；factory fixture 和隔离目标构建尚未执行。

最终生产源码及当前版本资产已完成：source_tree_sha256=019a8e2b710f66237e1849f21b8a58b05f02c028de64352979cfdde4b927214e。最终独立增量审计通过，工程指纹为 4ad92d44b02ba2f8882dd24bbcbf5c2861dab900dcd32e49e362f5e94109c7c7。新正式 CLI 跨环境旧分析拒绝、新环境完整分析预览通过；源码修复及确认合同已完成，真实自然对话、平台 UI 次数及 Unix 分支未实测。

已通过本轮 baseline、metadata、manifest、structure、mirror-drift 五项（退出码均为 0）；完整 workspace、factory fixture 和最终隔离准备未完成，validation_status 保持 awaiting_validation，不结算 TODO-018/TODO-019。恢复时先核对实际 HEAD、工作区和受管产物；提交会使此前绑定旧 Git 状态的分析与检查缓存失效，不能把本机 `.runtime/` 缓存或旧 prepared 收据当成换机后的成功证据。开发验证按开发收口清单恢复；普通 git-sync 不因此运行全量回归或要求开发验收。

## 恢复完整工程验证（2026-10-22）

用户明确要求继续完成剩余完整测试及隔离发布验证，通过后报告，先不发布。当前基准提交为 `aa9f0f76c786eda2e1e55d2e1f80307ef5c080e0`，恢复前工作区干净、VERSION 为 3.0.0。本轮只恢复验证并补充本记录，不提交、推送、正式发布、安装用户级或修改真实下游。

沿用 `.codex/development-checks.json` 的七项检查及受管隔离准备流程；旧本机准备记录与当前输入不匹配，不作为通过证据。按当前源码独立复核审计结论，再于同一执行宿主重新预览、生成完整分析及运行 prepare-release。历史约 59 分钟 workspace 与约 18 分钟 fixture 只用于估时，本轮实际耗时及结果待记录。

### 本次增量验收授权

用户随后要求缩短测试时间、尽快完成开发，并明确回复“同意开始吧”，同意本次不重复最新全量测试，改为针对最新改动的增量验收。该决定只替代本次完整 workspace/fixture 重跑要求，不永久修改工厂开发收口规则。

本次保留五项基础检查，核心库 `release::` 和 `git_sync::` 测试，以及 factory 的 `factory_version_config::`、`git_sync_runtime::`、`git_sync_polyglot_runtime::`、`build_provenance::` 场景；保留独立审计及受管隔离 3.0.1 目标构建。暂时缩小本轮检查配置，结束后按原字节恢复全量配置；单独保存增量准备收据，不将恢复原配置后的状态宣称为全量 prepared。上一轮 456/96 通过只引用为历史证据，本机不具备对应最终日志，不伪造本轮复用或通过记录。真实用户级/下游、自然对话确认次数及 Unix 分支继续标为未验证。

### 本次增量工程结果（2026-10-22）

以下为用户验收前的工程快照，最终验收决定见文末。用户授权的增量验证已全部通过；当时尚未结算 TODO-018/TODO-019，也未正式发布。生产源码及测试断言未改动。本次受管流程于 22:54:06 至 23:23:50（UTC+8）执行，耗时约 29 分 44 秒，不含前置审查、路径参数恢复和交付记录时间。

| 验证项 | 本次结果 |
|---|---|
| 基础检查 | baseline、metadata、manifest、structure、mirror-drift 五项全部退出 0 |
| 核心增量 | 234 passed / 0 failed / 4 ignored / 190 filtered out；检查耗时 1259.022 秒，测试主体 1235.86 秒 |
| 发布场景增量 | 18 passed / 0 failed / 3 ignored / 82 filtered out；检查耗时 310.659 秒 |
| 独立审计 | `/root/validation_audit` 对当前 HEAD 的实际修复、回归断言、确认合同及镜像传播只读复核 passed，未发现确定 P1/P2；认可本次增量范围 |
| 隔离目标构建 | 3.0.1 的 CLI、Hook 和两份构建收据生成成功；prepare 返回 prepared、checks_passed=7、checks_executed=7、checks_reused=0；同宿主终态再次返回 prepared |
| 原仓库保护 | HEAD 仍为 aa9f0f76c786eda2e1e55d2e1f80307ef5c080e0，暂存条目不变，VERSION 与实际 CLI 自检仍为 3.0.0；未提交、推送、正式发布或写真实下游 |
| 配置恢复 | 原 `.codex/development-checks.json` 按字节恢复，SHA-256 为 c2fbd0effbe5b214f765a1e1b3ad6cfb4f0995fd5ada56b0f86bca044d65c490；永久全量规则未修改 |

核心命令：`cargo test --locked --config scripts/tests/factory-cargo.toml --manifest-path .codex/hooks/Cargo.toml -p bridgeforge-core --lib -- --test-threads=4 release:: git_sync::`。

场景命令：`cargo test --locked --manifest-path scripts/tests/Cargo.toml -- --test-threads=4 factory_version_config:: git_sync_runtime:: git_sync_polyglot_runtime:: build_provenance::`。

同宿主受管入口：`git-sync --prepare-release --message "fix: validate sync context and ownership repairs" --audit-file .runtime/git-sync-validation-20261022/audit.json --release-analysis-file .runtime/git-sync-validation-20261022/analysis.json`。执行前先核对实际 changes，与已审阅内容及临时验证配置一致，再重新生成完整分析；没有复用其他宿主的旧 fingerprint。

本次明确覆盖 stat 假脏稳定、索引只读、真实内容/暂存/模式漂移拒绝、二进制与空文件差异、净还原绑定、Rust/Python 发布、三个 Cargo manifest/lock 同步、失败回滚、重复执行以及构建产物来源保护。四项核心 ignored 为指定 PDM/Poetry/uv 工具环境条件项；三项 fixture ignored 为原有显式性能实验。Unix 特殊路径分支、真实自然对话/平台 UI 次数、真实用户级与下游验证未执行；最新全量未重跑，不把增量结果等同全量。

独立保存的准备收据为 `.runtime/git-sync-validation-20261022/incremental-prepared.json`，工程指纹为 `sha256:9fb7a506754d29b8c196ff2d33356719ce9a5a5e618e2b29dbbdd265e03a1911`。同目录的 `final-status.json`、`finished.json`、`config-restored.json` 分别证明临时配置下终态、原仓库状态和原配置恢复。核心日志 hash 为 `7d786c8324ee9768a4d3f5d46cac58de9716d1ce3a695064468bfc5b62d39097`，fixture 日志 hash 为 `74c8448ecf13a2c3464097d6bf6e95e7427b25af7d6d1e3563a9fe7c9dcaa900`，均位于 `.runtime/bridgeforge-codex/release-preparation/blobs/`。

收据绑定本次临时增量配置及执行环境；恢复全量配置后，从当前工具环境读取 development-status 返回 stale（development check environment changed），不代表本次测试失败，也不能称当前全量配置 prepared。后续正式发布须在实际执行环境重新预览并生成分析，不携带本次旧分析直接发布。

编排经验：首次带分析预览拒绝绝对分析路径（release analysis must belong to this project），测试尚未启动且配置自动恢复；改用项目内相对路径后完成本次流程，未修改产品解析器或放宽路径校验。完整核心组日志按组返回，等待期间没有可靠百分比；此次核心增量实测约 21 分钟，不能把其他批次约 59 分钟当作固定耗时，也不能据此宣称同条件性能提升。

## 用户验收2026-10-22

用户明确调用“$summary accept，按这次增量验证结果验收，先不要发布。”，确认本次执行环境一致性修复及全流程单次确认开发符合需求，接受已约定的增量验证范围。该决定不授权提交、推送、正式发布、用户级安装或真实下游写入。

1. TODO-019 按本次增量结果验收为已完成：核心测试 234 passed、发布场景 18 passed、五项基础检查、独立审计及隔离 3.0.1 构建均通过；原全量检查配置已按字节恢复。需求卡状态更新为 completed / verified，该状态仅表示用户接受本次明确约定的验证范围。
2. TODO-018 保留为待执行的独立后续事项：相关修复、同宿主预览及隔离准备已有通过证据，但正式受管发布尚未执行。用户本次明确要求先不发布；后续须另获发布授权，在实际执行环境重新预览并生成分析，不复用失效分析或绕过漂移保护。
3. 保留全部未验证边界：最新全量未重跑，7 项既有条件测试/性能实验未执行，Unix 特殊路径、真实对话/平台 UI 确认次数、真实用户级及下游仍未实测；不得把增量验收或临时配置下 prepared 扩大为这些场景通过。

本次只更新既有验收记录、TODO-018/TODO-019 和文档索引，不新增待办、不修改源码/测试/配置/规则/Memory，不自动归档。VERSION 仍为 3.0.0；未提交、未推送、未正式发布。
