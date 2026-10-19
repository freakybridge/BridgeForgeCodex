# 首次配置与发布策略

## 触发与持久化

1. 每个项目升级到有引导能力的版本后，首次普通同步或 release 都先提示；已有 schema 1/schema 2 配置也展示当前规则供用户确认。纯只读状态、发布预览、独立开发准备不要求先确认，不为这些命令写入记录。
2. 运行 `git-sync --setup-status`。未确认时返回 current_configuration、current_strategy 和适用 options；已确认时直接返回 configured，不再扫描项目或重新问。用户主动换策略时，改用 `git-sync --setup-status --reconfigure` 重新只读识别并展示。只展示业务项目实际支持的选项，不根据骨架自带 Cargo.toml 或文件扩展名猜语言。current 表示保留现有规则；rust_independent 只供 Rust 业务项目选择。已有未满足的项目适配要求原样说明。
3. 在对话中一次展示并等待用户选择；未回答、拒绝、配置非法或状态不可靠时停止依赖该选择的动作，不能默认确认、提前 fetch/提交或启动可见控制台。用户明确要求换策略时才重新进入本流程。
4. 用户选择后先只读生成计划：`git-sync --setup-plan --strategy <选择>`。展示配置写入范围；使用该计划的 fingerprint 执行 `git-sync --setup-apply --strategy <选择> --setup-fingerprint <fingerprint> --confirmed-choice`。禁止主 Agent 手写 JSON。发现漂移重新只读规划；业务选择仍有效时不重复询问，但写入范围或兼容性实质变化必须重核。
5. Rust 方案若返回 project adaptation required，先说明 Hook、产品版本来源或共享输入的适配范围；不宣称已启用、不自动改业务代码，不擅自退回旧规则。需要项目改造时另行取得授权。current 可保留有效旧规则，无配置项目保留原自动识别语义。
6. 受管保存成功后继续用户原来的普通同步或 release，重新核对真实 diff；首次确认记录会进入项目 `.codex/bridgeforge-version.json` 并由本次同步保存到仓库。后续调用、换机、clone、普通产品升级复用有效记录，不因版本号变化重问。未提交或未同步的本地记录不能声称已跨电脑保存。

## 策略与收据

- schema 1 和缺省项目保留原有规则；首次引导只在明确选择后保存配置，不静默切换策略。`manifest_policy: auto` 表示保持原无配置时对根 Cargo.toml/package.json 的自动识别，不代表新增 Python 包版本管理。
- schema 2、strategy=rust_independent 以根 VERSION 为产品版本、各包 Cargo.toml 为包版本事实源。release 按累计相关提交计算改动包升级级别，并沿声明的本地反向依赖传播 patch；同次每包一次，无关包版本不变。
- 首次继承版本物化保留有效版本，机械迁移不计为包自身业务改动。Cargo 隐式 path 成员按实际 workspace 归属递归发现；exclude、独立 workspace 和跨根边界仍须遵守。
- 新建包首次纳入 release 时保留当前有效 manifest 版本，收据 reason=first_release（前后版本相同）；其反向依赖仍跟升，后续发布再按普通业务改动计算。仅追加新成员的 workspace.members 登记不全升已有成员；其他共享输入仍按显式映射归属。
- 已登记 workspace 内的新包按 Cargo 归属发现；新建独立 workspace 需在项目配置 rust.workspaces 显式追加实际根，并为有改动的 Cargo.lock 配置 shared_inputs 或 product_inputs。保留 git_sync_setup，复用已有策略确认；这属于项目发布归属维护，不重新 setup，也不手改包版本或锁条目。只登记自上次发布以来新建、且在当前 Git 清单中的 manifest（已跟踪或未忽略的待跟踪文件）；被忽略的磁盘根必须先纳入 Git。基线中已有的未受管包、删除、改名、workspace 边界变化仍阻断，当前不提供这些身份迁移功能。
- 预览与最终收据分别显示产品版本及 package_changes 的包路径、前后版本、first_release/direct_change/dependency_change；普通 git-sync 不升版、不完成首次纳入。先运行 --release-status 与 --release-preview，确认新增清单和依赖传播，再按既有授权执行 release。写版本成功不代表兼容性已验证。
- 共享输入、锁文件改动归属、未支持的 Cargo 配置或身份迁移不能可靠判定时，保留原始阻断原因。适配、约束和锁文件精确更新由受管工具负责，Skill 不手改 manifest/lock，不加业务测试为每次同步前置，不静默降级。
