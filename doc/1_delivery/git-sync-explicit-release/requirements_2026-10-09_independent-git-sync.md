---
lifecycle: active
validation_status: awaiting_user_acceptance
---

# 独立 Git 同步与用户可选升版

## 确认卡与授权

用户明确需要“自动同步＋用户选择的打版本号”，拒绝 JSON / develop / 验收收据作为使用前置。随后确认：必要 CLI/Hook 构建自动执行（A）；轻量提交检查保留（B）；最后选择“确认并记录需求，开始实施；不提交、不推送”（B）。当前授权包含上游源码、共享 Skill、相关 Hook/文档、dogfood、分发合同、隔离夹具和独立审计；不修改真实下游，不提交、推送、发布或安装用户级资产。

规模 M：同步、升版、必要构建及轻量提交检查四个已有职责解耦，不改变业务数据。预算 75 分钟、约 20k 新增 token（估算、未实测）、一个 review-auditor、最多两轮完整验证。无可靠周额度换算，不编造百分比；预计超额前报告并停止超额动作。

## 完整功能

| 编号 | 功能 | 决定 |
|---|---|---|
| 1 | 普通 git-sync | 自动 fetch、必要快进、提交和推送；不自动升版 |
| 2 | git-sync release | 显式升版并同步，不依赖是否完成 develop |
| 3 | 版本规则 | 累计未发布业务提交取最高级别；breaking major、feat minor、其余合法类型 patch；纯骨架不升业务版本 |
| 4 | 版本传播 | 同一事务更新 VERSION、原生 manifest/lock、CHANGELOG；失败回滚，不覆盖外部改动 |
| 5 | 无前置验收 | 不读取 development-checks.json 或 prepared/audit 收据来决定是否同步、升版；缺失、非法、过期或失败记录不成为使用门槛 |
| 6 | 必要构建 | 自动刷新需要更新的受管 CLI/Hook，尽量复用现有构建缓存；工具链和实际构建失败仍准确停止 |
| 7 | 轻量检查 | 保留编码、文档索引、版本和产物一致性等既有提交 Hook；不组织业务测试、全回归、独立审计或用户验收 |
| 8 | 幂等与状态 | 无业务发布内容不升版、不空提交，仍同步；真实 push 目标 clean/0/0 才报告完成 |
| 9 | Git 恢复边界 | 不强推/rebase/丢改动或 stash；冲突、权限、网络、分叉及真实失败说明阶段原因，保留现场 |

开发质量检查仍可独立执行：保留可选开发状态和准备工具，已有配置与收据不删除、不覆盖。--release-status 改为只读版本可发布性，不再报缺开发 JSON；--development-status 供独立开发检查读取旧收据。骨架升级不再提示缺清单会影响业务发布。

## 传播四问与实施

通用产品能力进入 templates/ 与 skills/，同步工厂 dogfood 和 manifest；产品版本及 [product] CHANGELOG 只由后续明确发布生成，当前 VERSION 不预改。工厂专属开发验收规则与同步使用分开，调整本项目收口文档及 AGENTS 项目专区，公共模板不吸收工厂要求。

| 步骤 | 内容 | 验证 |
|---|---|---|
| 1 | 去除 sync 的 preparation.load/validate/artifacts 依赖，直接使用受管 write plan | 无、非法、过期、失败开发配置/记录均可正常发布 |
| 2 | 版本状态与开发状态分离，新能力标记与 Skill 用法 | release-status 不读清单；旧工具能力不足准确提示升级 |
| 3 | 保留直接事务的 Git/index/提交 tree guard 与构建有效性 | Hook 改内容不推送；构建、提交失败回滚；冲突保留 |
| 4 | 升级提示、develop 与工厂收口解耦 | 升级不生成项目清单或报使用门槛，独立检查工具仍可用 |
| 5 | 真实 CLI、完整 workspace、工厂 fixture 与独立审计 | 普通不升、累计升版、重复不升、必要构建、轻量 Hook 生效，无 develop 前置 |

## 边界和风险

取消的是业务开发验收依赖，不取消 Git、版本元数据、工具构建和本次提交范围的真实一致性。之前因 Hook 改内容留下的 guard 仍保护本地提交，不能借新版本绕过。已有 JSON 留作可选开发工具的数据，不参与同步和版本授权。兼容旧开发命令仅用于独立验证，不声称它们是 release 的必要条件。

当前已开始实现，源码修改不等同于验证通过；收口后分别记录源码、传播、dogfood、fixture、真实下游与 runtime 六类证据。

## 阶段实施与经验

- 核心 sync 的 preparation.load/validate/artifacts 前置已移除；直接调用 WritePlan.prepare，必要生成资产自动构建或缓存复用。普通同步不升版，显式升版不因开发状态降级。
- --release-status 返回只读版本资格 release-ready/blocked；--development-status 才读取可选工程收据。升级提示不再将缺 JSON 归为业务发布缺口，已有数据保留。
- 原来仅 prepared 保护提交 tree，现改为全部实际提交建立 guard，避免普通提交被 Hook 改内容后直接推送；保留旧 guard 兼容，不清除有问题提交。
- 首次同步定向集 40 passed；增加两模式 Hook 改写覆盖后定向1 passed。真实 CLI 四策略流程1 passed：缺失/非法配置、失败或外仓库记录不阻断升版，零写入预览、普通不升、累计/重复发布及文件字节保留。
- 独立审计指出能力门槛需要明确及普通 Hook 改写测试缺口；已补齐，最终复审 passed。删除旧 Prepared 消费/发布绑定方法与字段；可选工程状态仍验证输入和产物哈希。编译警告已消除。
- 最终工具安装曾遇 Windows os error 5 并受管回滚；等待 CLI 测试结束后同入口重试成功。未修改权限、绕过受管入口或把错误当源码修复。
- 当前版本构建、完整 baseline、Skill metadata、manifest --check、文档结构及 diff 检查通过。完整工厂验收尚在执行；VERSION 仍为2.0.2，未提交/推送/发布，也未写真实下游。

经验：版本和 Git 操作的必要输入，不应与项目开发验收机制绑定。可选开发工具的数据存在、缺失、失效必须与直接同步独立；能力检测、必要编译和轻量提交检查仍保留。源码与产物构建不能以机器已有开发收据作为唯一刷新通道。

第一轮完整 workspace：CLI 14 passed，core 186 passed / 1 failed / 3 ignored；唯一失败是旧记录文档夹具仍用 requirements.md，未进入生产要求的 requirements_ 文档扫描，原发布绑定移除后旧 stale 断言失效。首次改无效生命周期仍被同一命名问题跳过，定向失败；停止继续盲改并由独立审计确认发现规则及最小修正。最终将文件改为 requirements_2026-10-09_sample.md 并同步索引，保留 invalid-lifecycle → stale 断言，定向1 passed，终审 passed。此前判断合法状态组合被允许不准确，实际生产规则禁止 completed + 非verified；命名修正才使现有规则真正被测试。没有增加或放松生产文档检查。

第二轮完整工程验证通过，最终指纹 e30584edb21e708c48b9460edc02eca67f818c318e23e2130e59040af3fa0fc7；7 项检查 exit 0，独立隔离构建完成，可选工程工具返回 prepared、候选版本2.0.3。此次运行是工厂验证本次修复，不是 git-sync 使用前置，也没有改写根 VERSION 或执行提交、推送。

## 最终交付证据

| 类别 | 结果与证据 |
|---|---|
| 源码 | 通用同步、状态接口、升级提示及共享 Skill 已解耦；独立 review-auditor 最终复审 passed |
| 产品传播 | templates、共享 skills 和分发 manifest 已更新；manifest --check 通过；尚未发布或安装用户级工具 |
| dogfood | 源码镜像一致性、baseline、metadata、structure 通过；CLI 14、core 187、Hook 22，共223 passed / 0 failed |
| fixture | scripts/tests 完整89 passed / 0 failed；直接发布覆盖缺失、非法、失败和过期开发数据，普通与 release 的 Hook 改写均阻止推送 |
| 真实下游 | 本轮未修改真实下游目录，未验证原项目更新后运行；不将隔离夹具结果当作真实项目验收 |
| runtime | 真实 CLI 四策略隔离流程及 factory 普通/显式同步流程通过；必要构建、缓存复用、零写入预览、累计/重复升版和轻量 Hook 已覆盖；用户桌面试用未验证 |

完整测试合计312 passed / 0 failed。workspace 的5个 ignored 包括4个供父测试调用的子进程入口和1个需显式日志输入的现场检查；工厂的4个 ignored 分别为显式性能实验、需要运行环境的高成本测试、子进程入口及需额外授权的数据迁移夹具，不宣称这些场景已完整验收。

工程收据：.runtime/bridgeforge-codex/release-preparation/current.json；workspace 日志 hash 724a176bac604168844afd565909f5eece932128abafea955f17c897ef5cb8bd；factory 日志 hash 9c09d77991b80b3f49178d96c53ce83cb5518824832bb8a697fc47d2293582ce。根 VERSION 仍2.0.2，HEAD 仍06af45b3675ec7f2996d64b5e5686178619a2c13；本次改动仅保留在工作区，未提交、推送、发布或部署到其他项目。
