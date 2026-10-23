---
lifecycle: completed
validation_status: verified
---

# Forge 开发收口分层与发布准备解耦

## 本轮授权与范围

用户明确要求按分层流程精简 Forge 开发收口，解除开发完成与发布准备的强制绑定，按改动范围选择验证并复用证据；本次只做必要验证，不跑全量回归、不修改下游、不提交或推送。

这是工厂自身配置层与元文档调整，不是通用运行时能力变更。修改根 AGENTS 项目专区及工厂流程文档，不改公共模板、共享 Skills、Rust 实现或下游。工厂 root 规则本身即生效，无需同步产品 dogfood 代码或冷构建；本轮不改版本和 CHANGELOG，后续保存时按 repo/meta 分类。

保留上一批 artifact-layout 默认规则的未提交改动，不将其实现、测试或性能成果记成本轮新增成果。

## 实施决定

1. 开发完成以选定的必要验证、适用审计与明确限制为准，不要求 prepared。
2. 文档/规则、局部实现、广泛高风险变更分别选择验证；完整回归必须有具体风险依据。
3. 先稳定实现与定向检查，再集中构建和集成验证；按实际影响复用证据。
4. 发布候选准备保持现有工具和完整机器合同，仅显式选择时运行。development-checks.json 不改名、不删检查，文件存在不触发普通开发重路径。
5. 工厂分层合同为主要事实源；根约束、测试 README 与反向回灌手册只保留必要入口及职责说明。

## 本轮必要验证

| 项目 | 方式 | 状态 |
|---|---|---|
| 规则衔接 | 审查 AGENTS、收口合同、测试入口及回灌手册，检查残留强制绑定 | 通过；普通开发不再要求完整清单或 prepared |
| 结构与受管边界 | git diff --check、manifest --check、project-structure | 均退出 0；manifest changed=false，结构 errors=[] |
| 独立复核 | 限本轮流程差异的只读 review-auditor | 通过；未发现分层、复用、准备解耦及 Git 边界的明显冲突 |
| 运行时与完整测试 | 本轮不改运行时代码，按用户要求不构建、不跑 Cargo/性能/全量回归 | 不执行 |

场景审查覆盖：仅文档变更无需构建；局部 Hook 改动选择相关回归与 smoke；广泛迁移有风险依据才全量；记录文档变化可复用代码验证；未 prepared 不阻止开发交付；已选必要检查失败不能算通过。

未实现自动按文件匹配测试或新的机器级验证选择器：当前由 Agent 按工厂合同选择现有单项命令。可选准备入口仍按原配置完整运行，其机器缓存身份规则保持不变。

## 交付记录

本轮只修改六个工厂规则/文档文件。`.codex/development-checks.json`、共享 Skills、运行时源码、VERSION 和 CHANGELOG 均未因本轮改变；之前已验证的运行时无需重建或重测。可选准备仍保留七项机器检查和 audit_required=true，不通过删除检查伪装流程变轻。

实际必要检查为 `git diff --check`、`.codex/bin/bridgeforge.exe manifest --root . --check`、`.codex/bin/bridgeforge.exe check project-structure --root .`。29 项既有归档/排除范围建议不属于本轮，不做清理。独立 review-auditor 核对六个文件和直接衔接的 develop reference，未发现重新强制全量或 prepared 的冲突；没有把历史文档逐篇重审。

未运行 Cargo、冷构建、性能、全量回归、development-status 或 prepare-release；未修改下游、未暂存、提交或推送。后续按本合同选择验证的实际任务效果尚待使用观察，不将规则审查等同于已证明未来耗时上限。

## 用户验收2026-10-23

用户明确调用 `$summary accept` 和 `$git-sync release`，接受本次分层开发收口与发布准备解耦，授权将当前交付通过受管 release 保存。补登记独立问题 TODO-021：工厂开发收口原先强制全量与 prepared，现已按改动范围选择必要验证、复用有效证据，发布候选准备独立按需触发。

验收依据是本卡记录的差异/manifest/结构检查和独立流程复核；运行时、共享 Skills 与可选准备机器配置均未改变，不追加构建、测试或审计。TODO-021 标记已完成并回链本段。本次验收不证明未来任务必定满足某个耗时上限，不更改无关 TODO，不写下游或 Memory；正式发布结果以受管工具终态收据为准。
