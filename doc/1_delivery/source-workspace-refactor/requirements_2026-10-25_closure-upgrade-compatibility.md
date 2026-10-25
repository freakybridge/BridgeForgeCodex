---
lifecycle: completed
validation_status: verified
---

# 骨架升级后的开发收口记录兼容

关联 TODO-036。2026-10-25 用户要求修复 Stratus 升级 7.0.1 后再次无法 release 的兼容问题，并尽快完成。读取指定对话确认：旧记录使用 6.0.0 的 evidence 算法，7.x 新增 rust_build 并改用依赖范围内的锁摘要，但记录没有算法版本；九包 content 未变，Backtester 确有未收口源码变化。旧 evidence 是否在当前宿主仍完全匹配，须实测，不以 content 相同代替。

## 范围、预算与传播

规模 M，沿用原重构累计 8–12 小时、100,000–160,000 token 上限，不重置。先完成局部兼容修复和定向回归，再执行实际受影响包的受管收口；上一轮配置检查曾耗时 132 分钟，不能承诺完整收口在几分钟完成。新增实现与调查预算约 20,000 token，token 未实测，周额度换算系数仍未知。避免重复全量验证；如既定检查造成预计超额，先报告实际证据和未完成部分，不削减必需检查或伪造认证。最初计划一位只读独立审查；两次实际候选预览仍阻断十包后，按 escalate 停止修改并引入第二位独立审查定位盲点，再实施其确认的 Git 内容比较修正。未将失败计数重置或隐去。

本次属于通用产品层兼容修复，维护源为 source/lifecycle；通过工厂生成器传播公共清单与 dogfood。验证通过后受管收口 lifecycle patch；主程序 VERSION、正式 CHANGELOG 留待另行 release 授权。只修改 BridgeForge；允许读取 Stratus 的日志/配置或零写入预览，不写入真实下游、用户级安装，不提交、推送或发布。

## 实现与验收

1. 6.x、7.0.x 无算法标记的历史记录按对应算法严格比较；历史 hash 链保持原样，不把旧记录重绑定为新验证。新记录标记 evidence 算法版本，未知算法阻断，已标记的新记录不能回落旧算法。
2. 源码、依赖、锁、策略、环境和产物的真实变化继续阻断；报错区分内容变化、验证依据变化和缺少记录。旧算法不能证明新算法新增的检查范围，兼容读取保留旧合同，不宣称旧证据覆盖新增范围。
3. 隔离回归覆盖旧记录 release/develop 读取、累计已提交记录、零写入、真实源码变化、验证输入变化、链篡改、未知算法和禁止降级。保留上一轮迁移归属回归。
4. 按实际入口核验旧记录；如宿主环境另有差异，单列原因，不将算法修复等同于真实下游已发布。环境范围收窄、工作区配置语义化和重复检查去重不在本次未经验证地连带重设计。

实际只读预览确认：仅兼容旧算法后仍有十包 pending；其中 Backtester 为内容变化，其余为 evidence 不匹配。进一步修正影响范围：已正式发布的同一条认证，在当前内容、传递依赖身份/内容、配置、工作区清单、锁、显式全局构建输入、检查输入及稳定合同均可对照正式发布证明未变时，按“复用已发布认证”处理；不声称旧证据匹配当前环境、不改写账本。未发布记录仍需当前 evidence。此路径只覆盖无 Python 环境证明依赖的纯 Rust scope；目录型 build_inputs 或历史缺失的 ignored 精确输入保留原门槛。实际修改、产物缺失/漂移仍走既有检查。

## 执行记录

1. 实现旧算法兼容、新记录 `evidence_schema=2` 和 release 阻断原因分类。无标记旧记录序列化保持原样；未知算法拒绝读取，已标记的新记录不得降级匹配旧算法。
2. 首轮三项旧算法、无标记新算法和混合追加链回归通过；实际 Stratus 预览仍阻断十包，证明仅重算旧算法不足。补充受严格输入证明约束的正式发布认证复用，不改写既有认证，也不声称旧 evidence 匹配当前宿主。
3. 独立复核发现显式 ignored 全局输入遗漏及 Python 环境证明边界，已保守修正。第二次实际预览仍阻断十包，按 escalate 暂停修改并重新独立取证。
4. 第二位审查确认 Stratus 的 `*.ps1 text eol=crlf` 会使历史 Git blob 与工作区原始字节不同。原始字节比较误把正常 checkout 转换当成修改；改用已有 Git 安全捕获的完整内容差异和 mode 差异集合，且在骨架过滤前保存。没有手动替换换行，也没有绕过 Git filter 检查。新增回归证明 LF/CRLF 转换可复用、脚本真实修改仍阻断。最终静态复核无阻断项；未单独补充“被骨架过滤的路径只变 mode”场景，完整 mode 集合保护经代码复核。
5. 通过受管配置迁移，仅将工厂 `lifecycle-unit` 的 `--test-threads=1` 改为 `4`，其余参数、全部用例和超时不变。独立审查核对 fixture 隔离及无进程级环境/CWD 修改；实际完整回归通过。此项为工厂自身配置，不下沉至公共模板。

## 验证与版本收口结果

新增七项隔离回归，覆盖旧算法升级读取、证据真实变化与禁止降级、混合追加链、已发布与未发布记录、共享工作区输入、显式输入及 ignored 历史缺口、Git 换行转换与真实脚本修改。定向用例通过后，最终候选通过受管 `develop --apply` 的五组必需检查：`core-unit`、`factory-unit`、`lifecycle-unit`、`skeleton-unit`、`workflow-unit`。其中 lifecycle 全套 246 项、workflow 全套 97 项；未用定向通过替代完整合同检查。

本次受管收口返回 `development-closed`、`git_written=false`，实测 5373.28 秒（89.6 分钟）。账本由 19 条增至 24 条，旧记录前缀逐条保持原样，新五条均记录 `evidence_schema=2`。主程序 `VERSION`、`CHANGELOG.md`、配置、Git 索引及 HEAD 在事务前后保持不变。收据保存在 `.runtime/closure-upgrade-compatibility/development-result.json` 与 `closure-verification.json`。

| 子包 | 收口前 | 收口后 | 原因 |
|---|---|---|---|
| lifecycle | 0.1.1 | 0.1.2 | 兼容修复，patch |
| core | 7.0.1 | 7.0.1 | 证据重验，无自身内容变化 |
| factory | 0.1.1 | 0.1.1 | 证据重验，无自身内容变化 |
| skeleton | 0.1.0 | 0.1.0 | 证据重验，无自身内容变化 |
| workflow | 0.1.0 | 0.1.0 | 证据重验，无自身内容变化 |

主程序仍为 7.0.1；其余注册子包版本不变。未配置稳定验收合同的 0.x 包保持开发阶段，没有自动宣称达到 1.0.0。

最终按项目入口执行 `cargo run --locked --release --manifest-path Cargo.toml --bin bridgeforge-factory -- build-assets --factory --project-root .`，返回 `prepared=3`、`self_test_executed=true`。随后 `build-assets --factory --project-root . --check` 返回 `current`、`reused=3`、`built=0`；manifest、factory-version 与 baseline 检查通过。baseline 首次因沙箱临时 Git 仓库写权限失败，保留原失败结果后原命令提权重试通过，未更改检查规则或 ACL。最终工具的 `develop --preview` 返回 `nothing-to-close`、`affected_packages=[]`、`writes=[]`，没有重复升版。

耗时改进仅报告实测：本次完整收口 89.6 分钟，上轮记录为 132 分钟；两次输入与环境并非严格对照实验，不宣称固定加速比例。全任务 token 和累计墙钟未持续计量，不能将收口耗时当作本轮全部投入。

## 实际下游结果与交付边界

最终候选工具对 `D:\Quant\StratusAgent` 执行 `develop --preview` 与 `git-sync --release-preview`，均只读。release 从十包阻断缩小为两包：

- `stratus-app-backtester`：真实源码内容变化。
- `stratus-test-suite`：受变化依赖影响，验证依据需要重新确认。

其余八包可以复用正式发布认证。release 预览仍以退出码 2 正确阻断，不能宣称 Stratus 已具备发布条件。develop 预览另有六个已复用包仍因真实产物漂移待处理：data-manager、database、launcher、replayer、strategy-sdk、trader 共同登记的 `stratus/target/debug/StratusAgent.exe` 和 `stratus/target/debug/stratus_app_backtester.dll` 与历史摘要不同；两项均为 changed，不是 missing，也不是骨架算法误判。既有产物检查保持有效。

最终只读脚本返回 `develop_exit=0`、`release_preview_exit=2`、`downstream_unchanged=true`，核对下游 Git 状态、账本、配置、索引与版本均未变。诊断输出保存在 `.runtime/migration-release-classification/stratus-candidate-develop-preview.json` 和 `stratus-candidate-release-preview.txt`。

| 证据类别 | 结果 |
|---|---|
| 源码 | 修复与七项回归已落地，必要检查通过，独立复核完成 |
| 产品传播 | 公共清单和 source 投影已生成并校验；尚未发布 |
| dogfood | 镜像、manifest、baseline 与最终受管产物一致性通过 |
| fixture | 历史认证、发布边界、输入变化和 Git 换行场景通过；既有迁移回归保留并随完整套件执行 |
| 真实下游 | 仅零写入预览通过现场保护；未安装修复、未执行业务 develop 或 release |
| runtime | 本仓库最终 CLI/Hook/工厂工具已构建、自检并实际预览；未执行真实下游写入流程或业务运行 smoke |

开发交付时工程开发与子包收口完成、等待用户验收；当时未提交、推送、发布，未修改真实下游或用户级安装。TODO-035 的失败 stdout 诊断改进仍是独立待办，未在本次连带实施。

## 用户验收与发布授权（2026-10-25）

用户明确调用 `$summary accept`，确认本次骨架升级后的 release 兼容修复符合需求；TODO-036 验收完成，更新为已完成。复用本轮七项回归、五组必需检查、受管子包收口、最终产物自检及 Stratus 零写入预览证据，不为验收重新构建或运行完整回归。

同时明确调用 `$git-sync release`，授权随后执行当前仓库受管发布、提交与推送；具体版本和成功结果以该事务收据为准。该授权不包含真实下游升级、业务收口或用户级安装。Stratus 两包真实变更阻断及六包产物漂移继续保留，不将本次验收解释为 Stratus 已发布。TODO-035 保持待确认；本次没有新增独立未解决问题。

经验结论：证据算法升级必须保留历史解释能力，不能改写旧认证链；已发布认证复用必须证明正式发布边界和相关输入未变；Windows 文本身份应遵循 Git 属性语义，不能直接比较工作区 CRLF 与历史 LF 原始字节；兼容修复必须用实际下游入口的零写入预览确认，同时保留真实业务变化和产物漂移的阻断。

复用已阅读的原生 Memory 中 source/templates/dogfood 职责及真实下游验证边界，与当前 AGENTS 和实际收据一致。上述经验已由实现、回归和现有规则承载，不新增 Rule、Hook、AGENTS 或 Memory 建议。当前 topic 中其他未结事项仍保留，无整组归档候选，未移动文档。
