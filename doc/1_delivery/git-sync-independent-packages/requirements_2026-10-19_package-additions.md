---
lifecycle: completed
validation_status: verified
---

# Rust 新增包首次纳入发布

## 授权与范围

2026-10-19 用户明确要求修复 rust_independent 在正常新增包后不能 release 的问题。授权上游产品、文档、测试及受管构建和工程检查；StratusAgent 仅复现与只读验证，不写入下游、不提交、推送或发布。本卡关联 TODO-010，补充原独立包发布需求，不新建重复 TODO。修复规模 M：既有适配器内的新增包规则与历史校验，主对话实现，独立 review-auditor 收口；沿用 M 默认 45 分钟、20k 新增 token 估算、最多一个子 Agent、最多两轮验证，token 未实测。必要工程检查遵循工厂清单；超额动作停止并报告。

## 根因与调用链

1. CLI 的 release-status / release-preview 与实际 git-sync --release 均进入 release_history 的基线检查；普通 git-sync 不调用升版规划。
2. release_boundary 从 VERSION 和新增 CHANGELOG 发布节确认最近边界；历史原生 manifest / lock 必须有效。
3. Rust 发现从配置 workspaces 展开显式、glob、隐式 path 成员，再校验真实根、依赖、映射和锁闭包。
4. 原 validate_baseline 要求新旧 manifest 键完全相等；只有报错，没有 Rust 包身份迁移 CLI。setup 只负责策略确认和适配，不执行新增包身份迁移。
5. 原历史加载使用当前配置读所有历史，新增独立根尚未存在时必然失败；最终图与各历史图单向比较也不能安全覆盖删除后恢复。
6. 事件归属和 Conventional Commits 决定已有包业务版本，声明的本地反向依赖传播 patch；产品 VERSION 独立取累计最高级别。计划合并进 git_sync 的 WritePlan，执行前核对 HEAD/index/内容/inventory/status；失败由既有守护回滚，预览不构建或写入。

## 最终规则

| 编号 | 场景 | 规则 |
|---|---|---|
| 1 | 新成员 / 隐式成员 | manifest 必须在发布基线文件清单中不存在，且在当前 Git inventory 中（已跟踪或未忽略的待跟踪文件）；被忽略的磁盘文件不能首次纳入。按真实 Cargo workspace 归属纳入 |
| 2 | 新独立 workspace | 项目明确在 rust.workspaces 追加根，根和包必须自基线以来新建；可在创建提交后、同次首次 release 前补登记 |
| 3 | 首次版本 | 保留首次识别后保持稳定的当前合法 major.minor.patch 有效版本，不套产品版本、不再因同批 feat/fix 或依赖升级加一；继承版本仍物化为原值。首次识别至发布之间手工改版仍由历史守卫阻断 |
| 4 | 收据 | first_release；version_before 和 version_after 同为初始版本。已有业务升级仍为 direct_change / dependency_change |
| 5 | 依赖 | 新包作为传播起点，已有依赖者至少 patch；同批直接业务升级取更高等级。新包依赖已有升级包时只更新必要约束，初始包版本不变 |
| 6 | 共享输入 | 仅追加全新成员的成员清单登记为机械变化；其他 workspace 修改和显式 shared_inputs 仍保守归属。Cargo.lock 改动继续要求明确归属，不猜测其业务影响 |
| 7 | 重复 release | 成功发布提交成为新基线，无后续改动不重复首次纳入、不重复升版；后续业务改动按既有包处理 |
| 8 | 保护 | 删除、改名、同名身份冲突、已有包改有效版本/根、基线已有但此前未纳入的 manifest、无法解析的历史或锁均阻断。不提供这些场景的通用迁移；错误说明实际不支持，禁止让用户寻找不存在的迁移命令 |
| 9 | 历史 | schema 2 使用该历史提交的根与映射，逐提交向前校验；schema 1 保留既有有限锁迁移兼容。schema 1 与无配置历史仅过滤当时尚不存在的新根/映射目标；无配置历史锁仍全部严格校验 |
| 10 | 同步与事务 | 复用配置确认，普通同步不升版。预览零写入；执行使用原受管事务，锁精确更新、失败回滚、预览及执行同源收据 |

“首次发布”指 BridgeForge 的版本与 CHANGELOG 发布事务，不执行 cargo publish，不表示包已发布到 registry。

## 现场核验与恢复路径

只读核验 StratusAgent：HEAD 0b8af8a3，工作区干净；schema 2、rust_independent、setup 已确认，配置有八个根。stratus/apps/normandy/Cargo.toml 在 stratus/Cargo.toml 的 members 中，初始版本 1.52.0；strategies/template/deribit-dual-ma/Cargo.toml 自带 [workspace]，版本 0.1.0，未在配置根中，不能仅因 Git 新建就称其已纳入发布器。

更新工具后的具体步骤（均须在下游已有授权范围内执行，本轮未写入）：

1. 通过 bridgeforge-codex 的正式受管升级流程取得包含本修复的工具，保留现有项目配置及 git_sync_setup；`self-test --json` 应包含 `git-sync-rust-package-additions-v1`。当前上游未发布，不把本机修改标称为新正式版本。
2. 若要让 deribit-dual-ma 一并进入独立包发布，在项目 .codex/bridgeforge-version.json 的 rust.workspaces 追加 strategies/template/deribit-dual-ma/Cargo.toml；该根必须在上次发布边界不存在。其 Cargo.lock 改动需明确 shared_inputs 业务包映射，例如 path=该锁文件、packages=[该包 manifest]；仅作为解析结果时也可明确选择 product_inputs。新增外置测试同样按真实所属包补齐 shared_inputs，不能只登记两个 manifest 就保证 632 项历史文件全部有归属。保持已有映射且不重叠。若选择不管理此包，应另行明确产品输入归属；不静默排除。
3. 对当前已实测的首个归属阻断 `.gitignore` 补齐配置：仅属仓库规则时可明确追加到 rust.product_inputs；若其改变确实影响业务包，则映射到实际业务包。再执行 `.codex/bin/bridgeforge.exe git-sync --setup-status`，应复用 configured；运行 `git-sync --release-status` 与 `git-sync --release-preview`，按具体错误补齐尚未声明的外置输入。检查 Normandy 保持初始 1.52.0；登记 Deribit 后应显示初始 0.1.0。已有包按实际历史与共享映射计算，不能承诺全部不动。
4. 经检查预览且已有下游 release 授权后，执行 `.codex/bin/bridgeforge.exe git-sync --release --message "fix: complete managed package release"`。消息需反映真实提交内容；历史已有 feat 时产品升 minor，不能仅用当前消息推定产品结果。成功收据核对产品版本、first_release 清单、已有包传播、clean 和 ahead/behind=0。

## 传播四问与修改范围

通用产品能力：templates/hooks 的 release/rust.rs、release/independent.rs、CLI main.rs capability 和共享 git-sync reference；同步 .codex Rust 镜像与机器生成的分发清单。测试位于 scripts/tests，说明在本 topic。没有工厂专属事实下沉，没有下游业务改动。正式 release 时由受管事务升级 VERSION 与 [product] CHANGELOG，本轮不预写。

## 验证与经验

最终实现与工程验证已完成，等待用户验收，尚未提交、推送或正式发布。以下保留过程记录；当前证据以末尾“最终工程收据”为准。首轮定向回归 72 passed、2 failed，均指向初始版本值未变时仍格式重写 manifest 的问题，已限制为版本实际改变时写入；继承版本物化和依赖约束更新仍保留。独立 review-auditor 发现无历史配置基线仍读取尚不存在的新根，已补齐“过滤不存在的根＋严格锁校验”分支及真实 Git fixture；复核结论通过，未执行测试。按 summary 流程在本卡沉淀经验，不修改 Memory、AGENTS 或 TODO 验收状态。

新上游 CLI 直接用于下游只读探测（没有安装到下游）：release-status 返回 release-ready、version_before=1.52.0；完整 release-preview 退出码 2，原始包键集合错误消失，首个后续阻断为 `unattributed Rust release input: .gitignore; configure shared_inputs or product_inputs`。同一探测前后 HEAD 与 index SHA-256 均相同，最终 git status --short 为空。该结果证明基线兼容已解除及未知输入保护保留，不证明当前下游整个 release 计划或执行已通过；登记新根及补齐归属后的预览、实际 release、业务运行仍未验证。

修正 P1 前的完整 dogfood workspace 已通过：CLI 16、core 286、Hook 24，共 326 passed、0 failed、8 ignored。新包首次版本、反向依赖、已有成员不跟升、无历史配置、重复 release、删除/改名/身份冲突/边界保护和 Hook 拒绝回滚均包含在该日志，`log_sha256=4f48a9692daa070358b532d06c9dee5bd4b7585193173f12973a4c7055e40fb1`。8 ignored 是既有辅助入口，不作为通过。

第一次 prepare 在完整 workspace 检查通过后返回 `development checks changed inputs; no preparation was certified`。原因是主 Agent 在检查期间更新了本卡叙述，虽 development fingerprint 仍为 `sha256:df7f996a409ee5704e860154f929e5d10dff36befd9896c4ce6683e1cda213b4`，prepare 的检查中快照要求全输入不变。记录类文档可在成功 prepared 后按轻量记录校验复用，不代表允许首次准备进行中修改。此操作时机已纠正：全部输入冻结后原样重试，不改源码、不放宽配置、不伪造收据。完整 factory fixture 与目标隔离构建仍待该流程实际完成。

第二次准备运行中，独立复核确认 P1：显式根可以从磁盘读取而不在 Git inventory，现有事务的 tree guard 只验证已暂存树没有漂移，不证明被忽略的新根进入提交树。已仅终止本轮准备进程所属的 core 测试（按父进程链和精确 exe 路径确认，不结束用户应用）；该轮记为主动中止，不能当通过。补齐新增根/包的当前 inventory 条件及真实 Git 忽略根 fixture，断言 release-status / preview 阻断且 HEAD、index、配置、版本、锁文件均不变。此变更是新增身份准入保护，不取消未知输入保护；新源码需重新审计、构建和验证，前一轮 326 passed 仅保留为修正前证据。

独立 review-auditor 已复核 P1 修正并通过：当前 inventory 条件在计划前生效，fixture 的状态/预览及零写入断言覆盖该缺口；rust.rs 模板与 dogfood 哈希一致。针对性回归、当前产物构建及最终完整检查结果仍以实际收据为准。已更新共享操作参考中的 Git 纳入要求；完整检查前再次冻结全部输入，检查期间不补写说明。

P1 针对性命令 `cargo test --locked --config scripts/tests/factory-cargo.toml --manifest-path .codex/hooks/Cargo.toml -p bridgeforge-core independent_ignored_new_workspace -- --test-threads=1` 已返回 1 passed、0 failed。最终开发输入指纹为 `sha256:c4b773033d99b1a8f4c8e471a43cf9823befa5740b6b2715708900eb875b6c0b`，后续完整结果须对应此输入。已知完整检查和构建等待超出最初 45 分钟估算；使用一个 review-auditor，没有扩大实现范围或宣称 token 已实测。

| 编号 | 证据类别 | 当前状态 |
|---|---|---|
| 1 | 源码 | 已实施，独立审计及修正复核通过，七项完整工程检查通过 |
| 2 | 产品传播 | 模板和共享 Skill 已改，分发清单已受管重建；未正式发布 |
| 3 | dogfood | 三个源码镜像一致性测试通过；受管 CLI/Hook 构建通过，新 CLI capability 已核验 |
| 4 | fixture | 完整 workspace 327 passed；完整 factory 90 passed；无失败，分别 8/6 ignored，不重复计入已包含的定向测试与镜像检查 |
| 5 | 真实下游 | 旧 2.5.1 原始错误已复现；新 CLI release-status 通过，preview 正确保留 .gitignore 未归属阻断；HEAD/index 不变、工作区干净，无下游写入和实际 release |
| 6 | runtime | 上游真实 CLI 隔离同步/预览/发布/重复发布及保护流程通过；下游自然运行、实际发布与业务兼容未验证 |

## 最终工程收据

最终命令 `.codex/bin/bridgeforge.exe git-sync --prepare-release --message-file .runtime/package-additions-message.txt --audit-file .runtime/package-additions-audit.json` 返回 `status=prepared`、`checks_passed=7`、`target_version=2.5.2`，输入指纹 `sha256:c4b773033d99b1a8f4c8e471a43cf9823befa5740b6b2715708900eb875b6c0b`。实际独立审计 passed，无未关闭发现。目标 CLI/Hook 及收据已在隔离快照构建，不改写当前 VERSION；当前仍为 2.5.1。机器收据位于本机忽略目录 `.runtime/bridgeforge-codex/release-preparation/current.json`，不是发布授权。

| 编号 | 检查 | 最终结果 |
|---|---|---|
| 1 | baseline | exit 0，包含当前受管产物关联校验 |
| 2 | skill metadata | exit 0 |
| 3 | manifest --check | exit 0 |
| 4 | project structure | exit 0 |
| 5 | mirror drift | 1 passed，模板与 dogfood Rust 全树一致 |
| 6 | 完整 workspace | CLI 16、core 287、Hook 24，共 327 passed、0 failed、8 ignored |
| 7 | 完整 factory fixture | 90 passed、0 failed、6 ignored，包含真实 CLI 新包与后续发布、普通同步不升版及失败回滚 |

两套完整集共 417 passed、14 ignored；镜像与定向回归均包含于完整集，不重复累加。忽略项是既有辅助、性能/live/需额外环境或授权入口，不视为已验证。workspace 日志 `sha256:618dde6e130ac75cf03f8b8238acd0ede9ebbe6cb7e758ef9eabf74b31df917d`，factory 日志 `sha256:9960491bbf2b86a5503ee0ac7630bec084bb8032bbbd876555478edda4a971d4`，均保存在准备目录的 blobs/ 下。

版本断言包括：产品 1.0.0 → 1.1.0，新包初始 0.3.7 保持不变，重复发布不再升版、后续 fix 才到 0.3.8；真实 CLI 新包初始 0.1.7、已有包 0.4.0 → 0.5.0，后续新包 fix 到 0.1.8、已有反向依赖到 0.5.1、产品到 1.1.1。无关包不升版；精确依赖约束和 local lock 更新保留，第三方同名条目不改。Cargo locked/offline 接受生成 manifest/lock；Hook 拒绝后文件与 index 恢复。

最终构建 CLI 在 StratusAgent 再次只读核验：setup-status=configured、strategy=rust_independent；release-status=release-ready、version_before=1.52.0；release-preview 以 .gitignore 未归属退出 2。HEAD/index 不变，工作区干净。未安装到真实下游、未维护其配置、未执行实际 release、未验证业务 runtime，未提交/推送/发布上游。源码、模板、dogfood 与 fixture 的工程完成不代替这些验收。

最终收据成功后才补写本节。此处仅为配置已声明的 record_documents 变化，使用轻量记录校验复用工程检查与产物，不再启动完整回归；开发输入指纹保持不变。

## TODO-010 当前验收

2026-10-19 用户通过 `$summary accept` 验收扩展后的 Rust / Python 通用发布交付，当前完整回归覆盖本卡的新增包规则。TODO-010 的正式验收、最新工程证据和发布授权统一见[当前验收记录](requirements_2026-10-19_polyglot-lifecycle.md#用户验收与发布授权2026-10-19)。以上内容保留当时现场与收据，不替代当前状态；真实下游补归属配置、实际发布与业务 runtime 仍未验证。
