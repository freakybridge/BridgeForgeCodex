# 子包开发收口与主程序发布合同

> 运行前检查 CLI `self-test --json`：开发需要 `package-development-v1` / `package-stability-v1`，发布需要 `git-sync-development-closure-v1`。仅有旧 `git-sync-product-packages-v1` 不足以支持本合同。实现与验证状态见项目需求卡；禁止手写认证记录或用旧发布流程冒充。

本文是 develop / git-sync 共用的版本判级、稳定条件和证据规则事实源；Skill 只负责各自阶段的编排。普通同步始终不主动升版。

## 1. 版本对象与来源

- 根 `VERSION` 表达主程序整体发布；`product_manifests` 用于整 workspace 的原生产品映射，`product_packages` 显式登记混合 Rust workspace 内跟随主程序的载体，不作为独立子包。
- 所有项目自有子包须登记稳定 id、原生包名、语言、输入归属和唯一版本来源；普通目录不自动视为包，第三方依赖和骨架工具不登记为项目业务子包。不新增全仓库漏登记扫描硬闸，已有归属检查不放宽。
- 子包允许版本不同。已有 Rust/Python 适配边界保留，其他语言不能仅靠文档宣称自动支持。
- 语义版本采用 MAJOR.MINOR.PATCH，高位递增时低位归零；已发布版本不得覆盖、复用、倒退。Python 特殊版本保留 PEP 440 原生语义；不支持自动计算的格式必须由适配器明确处理，不猜测转换。
- 无既有历史的新包可从 `0.1.0` 开始；既有包保留自身合法版本，不统一重置。根 VERSION 的稳定阶段策略沿用项目主程序合同，本轮新增自动转稳定版只针对子包。

## 2. 两个比较基线

| 阶段 | 比较范围 | 写入责任 |
|---|---|---|
| develop 收口 | 每个包上次有效开发收口与当前最终内容；首次接入以可证明的已发布基线衔接 | 受影响子包版本、相关原生依赖/锁、受管收口记录 |
| release | 最后正式发布与当前最终状态，包含已提交、暂存、未暂存和未忽略新文件 | 主程序 VERSION 及原生载体、主程序相关锁、正式发布记录与基线 |

同一内容重复收口不重复升版；有新改动才重新判级。无包变化不为凑流程升版，达到稳定条件也不绕过重复收口保护。收口比较和正式发布比较不能共用一个不断前移的基线，以免吞掉尚未发布的累计变化。

例如行情包在两次 develop 中由 `0.3.0 → 0.3.1 → 0.4.0`，release 保持 `0.4.0`，公告累计 `0.3.0 → 0.4.0`；中间子包版本不等同于一次正式产品发布。移除包记录退役，不制造新版本；新增包明确“新增 → 当前版本”。

## 3. 子包判级与自动稳定

| 实际变化 | 0.x 子包 | 1.0.0 及以上子包 |
|---|---|---|
| 兼容修复，或确需收口的内部调整 | patch | patch |
| 兼容新增功能或公开接口弃用 | minor | minor |
| 不兼容的接口、数据、配置或使用要求变化 | minor | major |
| 无实际变化 | 不升版 | 不升版 |

按实际公开合同逐包判断，不从提交标题统一判级。依赖包升版不自动带升调用方；调用方有效依赖要求、源码或转发的公开合同确有变化时才纳入自身分析。兼容性未知时停止依赖该结论的收口，不伪造 compatible。数字级别与兼容性分别记录：0.x 的 minor 也可能 breaking，不得据此省略影响说明和适用的不兼容接受。

程序在有实际子包改动的 develop 收口中检查以下条件，全部成立才自动选择 `1.0.0`，不逐包索取转稳定版批准：

1. 对外接口、配置或数据合同有明确可核对的基线。
2. 已配置非空稳定验收清单，覆盖核心功能以及真实调用方或使用场景的集成检查。
3. 清单中的必需证据全部有效通过，并对应当前源码、依赖、配置及必要运行环境；缺失、失败、跳过、未知或过期均不算通过。
4. 本次相对该包上次有效收口没有不兼容变化。

条件不足则按 0.x 规则收口，并报告未满足的稳定条件；其中“稳定检查未满足”不能伪装成“普通开发必需验证已通过”。若普通开发必需检查失败，仍不得完成子包收口。程序执行明确的验收标准，不声称有限检查能证明软件没有 Bug，不用开发时长、发布次数或版本到了 0.9 代替证据。

## 4. develop 收口与证据

已授权的开发按影响范围完成必要验证后，由受管入口完成子包升版；不修改主程序 VERSION，不暂存、提交或推送。验证记录必须来自真实执行，字段、指纹、范围、产物路径和内容摘要可核对；不能用 Agent 的 passed 声明或旧 prepared 记录代替真实证据。

相关源码、依赖、配置或必要环境改变时，只失效受影响的证据。普通暂存、提交或与该包无关的文档变化不应使所有包无条件重测；如何证明仍适用由机器合同实现，不手改旧指纹。

版本、原生依赖/锁与收口记录须一致提交到本地事务；中断或失败不得留下半套已认证状态。版本写入若影响编译产物、依赖解析或测试结果，须按实际影响构建并核验最终候选；交付产物必须对应最终版本。不得把升版前的二进制直接标记为最终交付。

包改名、移动、移除、拆分、合并保留显式身份与历史，不复用退役 id。稳定基线、收口状态和版本生命周期须一起校验，不能通过改名重置约束。

## 5. release 核验与公告

release 不替代 develop。对本次受影响的注册包核验：当前版本和相关输入已被有效收口覆盖；收口后再改源码、依赖或相关配置，须回到 develop 补验和收口。缺口时停止发布，列出包、原因和完成验证/升版的动作，不在发布中自动补做开发，也不降级成普通同步掩盖失败。

release 依据正式发布以来的累计实际影响判定主程序目标；不能只看最后一次 develop 或只比较版本数字。主程序遵守自身版本合同，所选级别须反映累计兼容性影响。已明确接受且仍适用的不兼容决定直接复用，不反复逐包询问。

发布事务只升主程序和对应原生载体，不再次修改独立子包版本。依赖/锁同步不得引入未经开发收口覆盖的独立包变化；若发现这种变化，报告缺口并停止。无正式发布变化为 no-op。

成功后完整展示“对象 / 旧版本 / 新版本 / 原因”：主程序以及自上次正式发布以来所有发生版本变化的子包，不能受代码效果摘要的三条上限限制。未变包可只汇总数量；新增、退役分别说明。失败只能展示计划或实际中间状态，不能宣布计划版本已发布。

普通 git-sync 不因缺少子包收口记录而阻断，不主动升版；仍保留既有 Git、编码、配置等安全检查。若 develop 中断留下未完成写事务，必须先 `develop --recover`，以免同步半完成版本；普通同步不自动恢复事务。

## 6. 机器入口与配置

schema 4 保留 `release_model=product-packages`、原生适配和不可移动的发布基线。通过已有 `git-sync --migration-plan/--migration-apply --setup-config-file ...` 配置提案登记 `development`，不在提案中填写 `baseline`。已有 schema 4 只补项目自己的检查合同，不重新切分历史，不认证未发布内容。配置提案必须按项目实际入口填写；骨架升级不猜造业务检查。

同一个 Rust workspace 混合独立库和产品 CLI/Hook 时，在 `packages` 登记独立身份，在 `product_packages` 登记产品载体；载体源码必须有显式产品输入归属。独立包不得继承产品版本或依赖产品载体。不把混合 workspace 整体填入 `product_manifests`，以免同步独立包。产品载体可保留 `version.workspace=true`。

旧原生包首次纳入独立管理、此前没有登记身份时，可通过提案的 `package_adoptions` 声明当前首次分配的 id、不可变来源 revision、原生版本和 source 身份。程序从紧邻首次登记的 Git revision 重建真实 Cargo workspace 与锁，核验名称、语言、版本和历史连续性；此声明不表示历史已经登记，也不能替代 split/move 等已有身份生命周期。不可伪造旧版本、冒领产品/退役身份、挑更早的方便版本，或删除/改写已纳管的来源声明。原发布 baseline 保持不变。

`development` 以包的稳定 id 为键；每包必须有非空 `checks`。检查使用 `program`、`args`、可选 `timeout_seconds`（默认 300，范围 1–14400）、`inputs`（包输入之外的验证脚本、夹具或验收数据）和 `artifacts`（需核验的最终产物路径）。必要验证输入必须纳入包归属或 checks.inputs；受 Git 跟踪的产物须声明为 build_inputs。`stability` 可选，包含非空 `contracts` 文件列表与 `checks`；稳定检查至少分别声明 `kind=core` 和 `kind=integration`，后者必须是实际调用方或使用场景。命令从项目根执行，退出码 0 且未超时才通过，缺失输入或产物不算通过。Python 仍须配置 `python_environment` 的解释器、运行时及依赖目录。

```json
{
  "development": {
    "example": {
      "checks": [{"id":"unit","program":"cargo","args":["test","--locked","-p","example"]}],
      "stability": {
        "contracts": ["doc/0_architecture/example-api.md"],
        "checks": [
          {"id":"core","kind":"core","program":"cargo","args":["test","--locked","-p","example","--test","core"]},
          {"id":"caller","kind":"integration","program":"cargo","args":["test","--locked","-p","example-caller","--test","integration"]}
        ]
      }
    }
  }
}
```

1. 运行 `.codex/bin/bridgeforge develop --preview`，读取 `affected_packages` 与 `fingerprint`；`nothing-to-close` 不写入。
2. 核对实际 diff、接口与用户决定，在忽略的 `.runtime/develop-analysis.json` 保存 `{schema:1,fingerprint,packages:{id:{level,reason,compatibility,acceptance?}},targets?,interface_impacts?}`。开发分析不含 product。breaking 的 acceptance 绑定本次 fingerprint、scope、reason、source；复用已有适用授权，不重问。
3. 运行 `develop --preview --analysis-file .runtime/develop-analysis.json`，取得 `development-planned`。计划展示候选版本及稳定缺口；候选 1.0.0 尚须最终版本检查。
4. 运行 `develop --apply --analysis-file .runtime/develop-analysis.json --fingerprint <计划指纹>`。工具写入候选原生版本/依赖/锁并执行配置检查，再写受管 `.codex/package-closures.json`。稳定检查失败只让相应包回到 0.x 候选，并重新验证最终依赖图；普通必需检查失败回滚版本写入。实际结果以 `development-closed` 的 `package_changes` 为准。
5. 中断遗留 `.runtime/bridgeforge-codex/develop-transaction.json` 时使用 `develop --recover`。恢复绑定当前 checkout 和 HEAD，遇到外部修改保留冲突并停止；不覆盖外部编辑。开发与 Git 同步共用互斥锁。

收口记录是随代码保存的追加历史，含版本、内容/环境/依赖指纹、检查命令和输出摘要、最终产物摘要及接受依据。纯证据失效只重新验证，不重复升版。正式发布仍用 `--release-preview` / `--release-analysis-file` / `--release`；分析只含 product 和可选 `targets.product`，子包级别与理由从收口历史累计读取。已有 `required_checks` / `check_scopes` 继续校验主程序及受影响子包的必需证据，不替代 develop 的实际检查，也不在 release 自动执行 develop。

具备 `portable-development-evidence-v1` 的工具写入 `evidence_schema=3`，记录内容、依赖、构建、环境、检查配置、合同、检查输入及适配器的分项摘要。长期证据排除 `CODEX_THREAD_ID`、`CODEX_SESSION_ID`、`CODEX_TOOL_CALL_ID` 和 `CODEX_APP_TOOLS_PIPE_PATH` 这些会话关联信息；其余环境、工具链和 Python 运行时检查保留。预览/执行计划仍绑定严格宿主指纹，不能把另一宿主的分析直接交给 apply/release。`develop --preview` 的 `pending_reasons` 解释待处理包；schema 3 可报告变化类别，旧总摘要无法可靠拆解时明确报告缺失的可移植证明，不输出环境原值。

具备 `git-eol-development-evidence-v1` 的工具写入 `evidence_schema=4`，在上述分项中增加有效规范化规则。develop 与 release 共用摘要实现：只对 Git 证明等价的 CRLF/LF 表示计算 LF 内容摘要；二进制、`-text`、特殊编码/ident 输入和声明产物保留原始字节语义。外部 clean/process filter 不参与验证；预览不执行这些项目程序。检查输入、合同、显式构建文件和忽略输入遵循同一规则。计划同时绑定有效属性和相关 Git 配置，执行期间仍检查原始字节及目录成员，不用内容等价掩盖并发改写。

旧记录保持原哈希链：仍能精确匹配原算法的记录直接复用；完整原算法证据与 Git 证明的 LF 表示精确匹配时也可只读复用。已正式发布的历史证书仅在内容、版本、依赖、配置及声明输入证明未变时复用。隐式 Cargo.lock 按包的实际依赖闭包比较，显式全局构建/检查输入仍按完整输入核验。旧 CRLF 或混合表示无法由完整摘要证明时不猜测迁移；没有实际 Git 源码/mode 差异时禁止据此升版，需先解决历史证据。未正式发布且旧总摘要失配的记录不自动转换成新认证；无法证明时须经实际受管检查补验。新记录不能借历史复用跳过真实环境变化，也不能降级到旧算法。

显式 `build_inputs` 包含被 Git 忽略的文件及其存在状态；目录声明同时绑定所有成员和空目录，验证期间新增、删除或改动成员会阻断认证。只有计划内版本写入和声明的检查产物更新可进入最终证据。旧摘要未覆盖这些声明输入时不能直接复用，须受管补验，内容不变不重复升版。

### 跨电脑合同与最小发布检查

具备 `cross-machine-development-evidence-v1` 的工具允许在包的 `development` 策略中显式声明 `environment`。它表示项目已界定检查的工具和环境输入，不由骨架自动猜测业务合同。没有该声明的检查保留原保守环境规则，不能声称任意旧记录已可跨电脑复用。

```json
{
  "environment": {
    "tools": ["cargo", "rustc", "git"],
    "variables": [],
    "inputs": {"sdk": "${PROJECT_SDK_ROOT}"},
    "cargo_configs": ["scripts/tests/cargo.toml"]
  },
  "release_artifacts": []
}
```

以上是字段示例，SDK、配置文件和工具清单必须按项目实际填写，不能直接当作通用配置。`tools` 必须覆盖检查入口及其使用的编译器、linker、wrapper 和辅助程序；路径可由 PATH 解析、使用项目相对路径或 `${VARIABLE}/suffix`。证据比较文件身份，不比较安装目录。Rust 工具另外绑定实际版本输出；Cargo 的祖先配置、用户配置及 `cargo_configs` 文件一起核验。已知编译器/linker/wrapper 覆盖必须指向声明工具；无法建模的 alias、include、内联配置或 linker/sysroot 参数明确阻断，不能当作无关变量忽略。平台、SDK 和原生库等输入必须完整登记；不能只因 Rust 版本相同就保证原生构建条件相同。

当前 portable Cargo 配置支持普通 `build`、`target`、`term` 表，以及受限的远程镜像配置：source 仅 registry/replace-with，registries 仅 index/protocol，registry 仅 default，net 仅 retry/offline/git-fetch-with-cli。远程索引不允许内嵌凭证、查询参数或片段；启用 Git 拉取时同时绑定实际 Git 程序身份。本地 directory/local-registry、path patch、env 注入、自定义 credential-provider/credential-alias 和其他未建模字段拒绝；这些配置节的 inline 表也不绕过检查。配置文件里的 linker/sysroot flags 同样拒绝；相对工具路径按 Cargo 的配置文件基准解析后核验。此限制不修改旧 host-bound 检查的执行方式，不应为接入 portable 自动删除项目已有配置。

`variables` 仅用于非秘密的有效参数；不登记 token、密码等秘密。`inputs` 用稳定角色映射环境文件或目录，以文件内容和相对成员计算身份。`${VARIABLE}` 路径变量按所指输入身份核验，执行时保留其本机值，不将绝对路径作为跨机身份。检查进程只继承合同声明、明确构建参数和必要宿主定位变量；显示和语言默认值固定，其他应用变量不自动继承。检查不得隐式依赖未声明的机器身份或宿主文件；无法界定的检查应保留原环境模式。

首次启用合同须通过受管配置迁移和真实 develop 检查，生成 `evidence_schema=5`，保存可诊断分项并保留历史链。旧记录不能补填字段冒充新认证。纯证据刷新不重复升子包；环境不等价时说明变更字段，不输出环境原值。

schema 5 将源码验证记录与本机产物分开：普通 `checks.artifacts` 仍在实际检查及认证时核验，但换电脑后缺少这些产物不否定源码证据。`release_artifacts` 只能引用已声明检查产物，表示本次发布确实交付并依赖它们；缺失或改变时 release 阻断，develop 可按原检查无升版刷新。新生成的二进制不自动继承旧二进制的运行验证。

具备 `scoped-release-evidence-v1` 的工具先判断累计业务发布需求，纯骨架且无业务目标时不计算包验证证据。实际发布只核验变化包、相关检查输入/共享规则及真实依赖传播范围；范围从上次正式发布累计，包含已提交未发布内容。release 不运行 develop 或业务回归，同次计划不重复执行相同收口门禁；保留账本、未完成事务、归属、版本、输入漂移和最终写入前的外部环境保护。`--release-status` 的 `readiness_scope=baseline-only`、`development_evidence_checked=false` 表示尚未经过完整发布预览。

具备 `release-external-analysis-preview-v1` 时，`--release-preview --release-analysis-file <绝对路径>` 可读取目标项目以外的普通分析文件，仍核验唯一 JSON、当前项目 fingerprint、必需证据和输入漂移，用于不写目标项目的 dry-run。外部路径不得经过 link/reparse；项目内分析仍放在忽略的 `.runtime/`。此能力只适用于 release 预览，实际 release 和 develop 继续要求本项目 `.runtime` 分析文件；预览通过不证明后续提交、Hook、推送或网络成功。

Cargo.lock、已声明 Python lock、明确全局构建输入和自动记录不因单独变化直接升包；业务生成源码与资源不属于自动记录。不得执行任意 setup.py、SCM 后端、安装脚本或动态宏来猜测元数据。未知格式、歧义归属或不支持的证据应报告缺口。

配置迁移只写配置，不改变业务版本或锁。无变化的既有包无需补造收口；存在未发布内容、未知版本变化或不可证明历史时必须先处理缺口。本合同不授权真实下游写入。

发布准备 `development-checks.json` / `prepared` 仍是原有可选重路径，不自动成为普通开发或 release 的前置；新子包收口是独立职责，不能复活全量测试门槛。

### 本次任务范围

具备 `task-scoped-development-v1` 时，开发入口的 preview/apply 可携带 `--packages <id1,id2>`。值必须是唯一、非空的注册包 id；不支持等号形式，不与 recover 合用。无分析预览、分析预览和 apply 保持同一选择，选择及有效范围均绑定严格计划指纹。更换任务选择必须重新预览与分析。

工具自动展开注册依赖、调用方和相关生命周期端点，以 `effective_task_packages` 显示有效范围；整包输入参与核验，不能绕过必要关联或只认证部分源码。当前展开保守纳入依赖，不承诺其检查均可省略。`readiness_scope=task`、`outside_task_evidence_checked=false` 明确全仓其他证据未检查；本次范围没有待处理项返回 `task-development-current`，不复用全仓 `nothing-to-close` 的状态名称。

不携带 --packages 的旧入口保持全仓诊断与兼容语义。本次完成不要求无关历史待处理项清零；release 仍核验累计实际发布范围，任务成功不能替代发布门禁。任务范围能力本身不代表检查缓存能力。

### 同机完整候选的检查恢复复用

具备 `resumable-development-checks-v1` 时，项目可对具有完整显式 `environment` 的确定性检查声明 `reusable: true`。默认 false 且不写入序列化默认值，保留既有证据兼容。只有项目明确可复用的检查才能开启，不自动改写项目清单；在线、依赖未登记状态或需要实际执行副作用的检查保持关闭。检查的显式输入与自身输出不能重合。当前保守要求该开发事务中所有声明检查产物均被 Git 忽略；有 tracked 或未忽略产物时，在写候选前明确拒绝该缓存组合，应关闭 reuse 后按原流程验证，不能放松 Git 身份保护。

缓存位于忽略的 `.runtime/bridgeforge-codex/develop-check-cache/`，保存完整 stdout/stderr、成功证明、产物摘要与完整性校验。只绑定同一本机根目录、实际运行工具二进制、完整候选和计划、严格宿主环境、Git HEAD/分支/索引/配置/refs/status、已捕获输入和目录成员。命中前核验记录、日志和产物；实际执行前后核对输入、环境、Git 和目录状态，写正式账本前再次核验相关 Git 身份。失败、超时或检查期间输入变化不保存成功缓存。

后续检查失败并回滚版本后，恢复同一完整候选可复用前面已通过的检查；缓存不是部分收口账本，整组必需条件仍须满足。字段、内容、工具、环境或 Git 身份不匹配时实际重跑，不重绑旧证明。当前不承诺跨机器缓存或局部修正的最小失效范围，实际采用保守完整候选身份；跨机器长期证书继续遵守 schema 5，不能混淆两者。

成功收据的 `checks.scope=final-candidate` 统计最终候选轮的 `executed`/`reused`，不包含之前失败或稳定回退轮的全部成本；逐项证明以 `cache_reused`、`cache_key` 标注。预览不创建缓存，不把人工 passed 作为缓存来源。
