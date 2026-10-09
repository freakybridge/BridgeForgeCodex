---
lifecycle: completed
validation_status: verified
---

# develop 验证与收口重构

## 目标、范围与授权

用户要求整体规划 develop 重构，Q1–Q5 妥善处理；已审阅四文件预览后明确“开始对 develop skill 进行重构优化”。实施阶段允许实现及测试，不授权提交、推送、发布、修改其他项目或用户级安装目录。2026-10-09 用户随后明确“同意验收”并调用 $git-sync，新增本轮 develop 验收与普通 Git 同步授权；不包含 release、用户级安装或无关任务提交。

正文范围为 skills/develop/SKILL.md 及 references 下的 ml-delivery.md、agent-execution.md、completion-release.md。需求卡、文档索引、受管分发登记及验证收据为必要支持资产，不新增 Agent、Skill 或测试框架。

## 传播四问

1. 产品层：通用 develop 行为规则；需求记录为本仓库元文档。
2. 通用能力：进入共享 skills/；Stratus 的具体 P3 条款不进入公共模板。
3. 发布时需要产品版本与 [product] CHANGELOG；本轮不提前改 VERSION，不执行 release。
4. 更新受管 manifest 并验证 dogfood；不对工厂运行下游 project-sync，不写骨架版本戳。用户级已安装 Skill 不手工覆盖，本轮未授权分发。

## 规模、预算与停止点

M 级：一个已确认流程内的四文件重构，初始沿用 confirm 的 45 分钟、约 20,000 新增 token、最多一个 review-auditor、最多两轮验证口径；用户明确批准将验证时间上限延长为 75 分钟，不扩大修改范围，累计不重置。token 未实测。当前模型为 gpt-6.1-sol/high，本机只有 gpt-6-astra 的临时周额度系数，不能跨模型套用，因此周额度占比未估算。预算包含工厂强制检查、权限重试及合理修复余量；达到上限时停止本轮等待，保存已有结果。验收仅只读复核既有收据并补文档，没有重跑测试、审计或构建。

## Q1–Q5 验收

| 编号 | 目标行为 | 主要解决位置 |
|---|---|---|
| Q1 | 纯显示微调走轻量路径；布局测量和告警语义变化不能机械免验 | 主入口“实现与验证”第 3、4 项 |
| Q2 | 按项目入口构建和核验产物；项目步骤不污染通用 Skill，构建通过不等于效果通过 | 主入口第 6 项；项目特殊收口 |
| Q3 | 微调默认不启动审计，用户或项目要求及关键风险仍有效 | 主入口路径及验证规则；Agent 路由 |
| Q4 | 复用有效证据，变化只重验受影响部分，原位补记录，不搭建临时指纹系统 | 主入口第 1、7、8 项；Agent 收据 |
| Q5 | 从用户动作和预期结果选择最小验证集，实际入口与替代环境的结论分开 | 主入口第 2、5、9 项 |

M/L 不再无条件要求 prepared；仅项目明确要求时读取特殊收口并执行。现有 confirm 预算、autopilot 整包继承、L 级审计、失败升级、用户验收和 Git 授权边界保留。

## 验证计划与记录

1. 执行受管 metadata、manifest、baseline、structure 和 diff 检查。
2. 独立 review-auditor 读取真实文件、需求和 diff，并核验代表性情景：颜色微调、列宽逻辑、环境差异、证据复用、项目强制检查、L 级审计。只读情景核验不冒充真实下游/runtime 或性能 A/B。
3. 按 .codex/development-checks.json 完成工厂 workspace、fixture、独立审计及准备记录；不重复执行模板与 dogfood 的同一套测试。
4. 已完成四正文实现。受管 skill-metadata 返回 issues/warnings 为空；manifest 重建后 --check 返回 changed=false；project-structure 无 errors（仅既有归档建议）；factory-version healthy=true，当前版本仍为 2.0.3；git diff --check 通过。manifest 写入及 baseline 临时 Git config 首次权限阻断后，均通过同一受管入口的窄范围沙箱外重试解决。
5. 独立 review-auditor /root/develop_review 读取真实需求、文件及 diff，结论 passed、无必修问题；六情景决策核验确认纯显示微调不补审计、列宽逻辑不能机械免验、替代环境不扩大结论、颜色变化可复用映射检查而函数变化须重验、项目强制 prepared 仍有效、L 级依赖任务顺序实施并独立审计。收据 .runtime/develop-refactor/audit.json 绑定本轮独审时的指纹 sha256:d490aa6fcf17c0a29d2c96cdf415692a62a771d07210fa9acd6a7094928c22e5；四正文后续未改变，与共享准备收据中的文件 SHA256 一致。此项为只读规则核验，不是模型行为实测。
6. 本轮 workspace 223 passed / 5 ignored / 0 failed，fixture 89 passed / 4 ignored / 0 failed，镜像一致性 1 passed。workspace 日志为 .runtime/bridgeforge-codex/release-preparation/blobs/14de0d2a5b0a35f66c5a05a10c9b450dff1074433c19d9fc878a6ef5e8284b94；fixture 日志为同目录下 4c31fa098464a83093fc35be4ffe539e45882809c6aa0fab005f550892df0a31。首次准备在七项检查后因同目录并行任务新增文件导致输入变化而拒绝认证，未生成候选产物；不是测试失败。
7. 验收时只读复核现有 .runtime/bridgeforge-codex/release-preparation/current.json：status=prepared，指纹 sha256:7885c507113ebe3e201b53c8998895dd82883592f7a68df9534884b4bee4546b，七项检查 exit_code 均为 0，候选产物已生成。该共享工程收据来自另一任务完成稳定现场后的检查；它不替代本轮独立 develop 审计，不将其他任务成果计入本轮。四个 develop 文件及分发 manifest 的实际 SHA256 均与收据输入一致，复用既有工程结果，没有重新验证。
8. 收据中的 target_version=2.1.0 只是共享隔离候选版本，不是本轮发布决定，当前 VERSION 仍为 2.0.3。本轮不执行 release，也不把普通 Git 同步称为版本发布。

## 用户验收与收口

2026-10-09 用户明确“同意验收”：Q1–Q5 的规则与结构重构验收完成，四正文、分发登记、本轮独立审计及有效工程准备证据齐全。只有项目记录正文和验收元数据发生变化，属于 .codex/development-checks.json 声明的记录文档，不改变 Skill 或构建输入。未执行、未验收实际模型行为、用户级安装、真实下游、runtime 或性能改善。

验收记录更新后，development-status 返回 prepared、record_documents_changed=true、record_documents_check=passed，验证指纹保持 sha256:7885c507113ebe3e201b53c8998895dd82883592f7a68df9534884b4bee4546b，已复用现有产物。后续真实安装与开发试用见 TODO-005；验收和 Git 同步分别判断。受管 Git 同步采用整仓暂存，当前工作区包含其他任务未提交内容，因此本轮 Git 范围需另行明确，不自动提交无关改动。

## 经验沉淀

1. 轻量路径必须在 Skill 主入口闭合，否则小任务仍可能因统一读取复杂收口流程而产生额外开销。项目强制闸须单独保持有效，不能以任务规模绕过。
2. 通用验证规则只保留一个事实源；需求状态、Agent 路由及项目特殊收口分别引用它。prepared 属于项目明确选择，不能在 M/L 流程中写成普遍前置。
3. 验证必须声明真实覆盖入口；产物新鲜度、工程构建、视觉效果和用户验收分别报告。元数据和 fixture 输入检查通过不证明模型行为、真实运行或实际提速。
4. 同目录并行写入会使整仓准备指纹漂移，即使测试均通过，也不能伪造 prepared。保留实际日志和有效的局部审计，待稳定现场形成有效工程收据再关联；不同任务的成果、审计范围和验收分别记录。
5. 用户明确中文验收意图与实施、发布和安装授权分开处理，不因参数格式重问已明确的业务决定；普通 git-sync 不升版，也不负责安装新版 Skill。

## 未验证与非目标

不迁移 Stratus 根 AGENTS 的 P3 详细条款，不修改公共 AGENTS/Agent 模板。用户级安装、真实下游、真实用户入口/runtime smoke 和实际提速未验证；规则和工程检查通过不能证明运行速度提升。
