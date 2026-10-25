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

Cargo.lock、已声明 Python lock、明确全局构建输入和自动记录不因单独变化直接升包；业务生成源码与资源不属于自动记录。不得执行任意 setup.py、SCM 后端、安装脚本或动态宏来猜测元数据。未知格式、歧义归属或不支持的证据应报告缺口。

配置迁移只写配置，不改变业务版本或锁。无变化的既有包无需补造收口；存在未发布内容、未知版本变化或不可证明历史时必须先处理缺口。本合同不授权真实下游写入。

发布准备 `development-checks.json` / `prepared` 仍是原有可选重路径，不自动成为普通开发或 release 的前置；新子包收口是独立职责，不能复活全量测试门槛。
