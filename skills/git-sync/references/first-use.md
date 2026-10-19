# 首次配置与发布策略

## 触发与持久化

1. 每个项目升级到有引导能力的版本后，首次普通同步或 release 都先提示；已有 schema 1/schema 2 配置也展示当前规则供用户确认。纯只读状态、发布预览、独立开发准备不要求先确认，不为这些命令写入记录。
2. 运行 `git-sync --setup-status`。未确认时返回 current_configuration、current_strategy 和适用 options；已确认时直接返回 configured，不再扫描项目或重新问。用户主动换策略时，改用 `git-sync --setup-status --reconfigure` 重新只读识别并展示。只展示业务项目实际支持的选项，不根据骨架自带 Cargo.toml 或文件扩展名猜语言。current 表示保留现有规则；rust_independent 只供 Rust 业务项目选择。已有未满足的项目适配要求原样说明。
3. 在对话中一次展示并等待用户选择；未回答、拒绝、配置非法或状态不可靠时停止依赖该选择的动作，不能默认确认、提前 fetch/提交或启动可见控制台。用户明确要求换策略时才重新进入本流程。
4. 用户选择后先只读生成计划：`git-sync --setup-plan --strategy <选择>`。展示配置写入范围；使用该计划的 fingerprint 执行 `git-sync --setup-apply --strategy <选择> --setup-fingerprint <fingerprint> --confirmed-choice`。禁止直接手写目标配置替代受管 apply；schema 3 可准备提案文件，再经下述受管 plan/apply 保存。发现漂移重新只读规划；业务选择仍有效时不重复询问，但写入范围或兼容性实质变化必须重核。
5. Rust 方案若返回 project adaptation required，先说明 Hook、产品版本来源或共享输入的适配范围；不宣称已启用、不自动改业务代码，不擅自退回旧规则。需要项目改造时另行取得授权。current 可保留有效旧规则，无配置项目保留原自动识别语义。
6. 受管保存成功后继续用户原来的普通同步或 release，重新核对真实 diff；首次确认记录会进入项目 `.codex/bridgeforge-version.json` 并由本次同步保存到仓库。后续调用、换机、clone、普通产品升级复用有效记录，不因版本号变化重问。未提交或未同步的本地记录不能声称已跨电脑保存。

## 策略与收据

- schema 1 和缺省项目保留原有规则；首次引导只在明确选择后保存配置，不静默切换策略。`manifest_policy: auto` 表示保持原无配置时对根 Cargo.toml/package.json 的自动识别，不代表新增 Python 包版本管理。
- schema 2、strategy=rust_independent 以根 VERSION 为产品版本、各包 Cargo.toml 为包版本事实源。release 按累计相关提交计算改动包升级级别，并沿声明的本地反向依赖传播 patch；同次每包一次，无关包版本不变。
- 首次继承版本物化保留有效版本，机械迁移不计为包自身业务改动。Cargo 隐式 path 成员按实际 workspace 归属递归发现；exclude、独立 workspace 和跨根边界仍须遵守。
- 新建包首次纳入 release 时保留当前有效 manifest 版本，收据 reason=first_release（前后版本相同）；其反向依赖仍跟升，后续发布再按普通业务改动计算。仅追加新成员的 workspace.members 登记不全升已有成员；其他共享输入仍按显式映射归属。
- schema 2 已登记 workspace 内的新包按 Cargo 归属发现；新建独立 workspace 需在项目配置 rust.workspaces 显式追加实际根，并为有改动的 Cargo.lock 配置 shared_inputs 或 product_inputs。保留 git_sync_setup，复用已有策略确认；这属于项目发布归属维护，不重新 setup，也不手改包版本或锁条目。只登记自上次发布以来新建、且在当前 Git 清单中的 manifest（已跟踪或未忽略的待跟踪文件）；被忽略的磁盘根必须先纳入 Git。schema 2 的删除、改名、workspace 边界变化仍阻断；完整生命周期需显式选择下述 schema 3。
- 预览与最终收据分别显示产品版本及 package_changes 的包路径、前后版本、first_release/direct_change/dependency_change；普通 git-sync 不升版、不完成首次纳入。先运行 --release-status 与 --release-preview，确认新增清单和依赖传播，再按既有授权执行 release。写版本成功不代表兼容性已验证。
- 共享输入、锁文件改动归属、未支持的 Cargo 配置或身份迁移不能可靠判定时，保留原始阻断原因。适配、约束和锁文件精确更新由受管工具负责，Skill 不手改 manifest/lock，不加业务测试为每次同步前置，不静默降级。

## schema 3：Rust / Python 通用独立包发布

需要能力 `git-sync-polyglot-release-v1`、`git-sync-package-lifecycle-v1`；Python 另需 `git-sync-python-independent-v1`。schema 1/2 保持原行为，不因升级自动切换。启用之前展示真实项目根、包身份、版本来源、输入归属和锁范围；不得把骨架自身的 Rust 工程当作业务包。

项目声明的核心结构如下；实际值必须读取项目，例子不直接 apply：

```json
{
  "schema_version": 3,
  "release_policy": "explicit_release",
  "strategy": "independent",
  "adapters": {
    "rust": {"workspaces": ["rust/Cargo.toml"]},
    "python": {
      "projects": ["python/pyproject.toml"],
      "locks": [{"path": "python/uv.lock", "format": "uv", "members": ["python/pyproject.toml"]}]
    }
  },
  "packages": [
    {"id": "core", "language": "rust", "manifest": "rust/core/Cargo.toml", "name": "example-core"},
    {"id": "app", "language": "python", "manifest": "python/pyproject.toml", "name": "example-app"}
  ],
  "dependencies": [{"owner": "app", "target": "core", "kind": "build"}],
  "product_inputs": ["README.md", "doc/", "rust/Cargo.toml", "rust/Cargo.lock", "python/uv.lock"],
  "lifecycle": []
}
```

准备位于项目忽略目录内的 schema 3 配置提案后，只通过受管入口保存：

```text
bridgeforge git-sync --setup-plan --strategy independent --setup-config-file .runtime/release-proposal.json
bridgeforge git-sync --setup-apply --strategy independent --setup-config-file .runtime/release-proposal.json --setup-fingerprint <返回指纹> --confirmed-choice
```

只读 plan 不写配置、版本或锁；apply 只保存配置。提案、HEAD、完整文件清单、元数据或版本源变化会使指纹失效。复用仍有效的用户决定，不重复索取开工授权。前 schema 3 的 Python 历史有不同源映射或锁边界时，用 `bootstrap_adapters` 明确原合同，不能用当前映射猜过去。

生命周期事件包含唯一 `id`、上次发布提交的完整 `base`、`operation`、`compatibility` 及 `from` / `to` 身份数组。身份由稳定 ID、language、manifest、name 组成，必要时另列 `workspace`。操作支持 rename、move、remove、split、merge；普通修改和新增从真实快照识别。名称/路径改变需声明；已有提交可在发布前补声明，不改写 Git 历史。

| 规则 | 行为 |
|---|---|
| 版本继承 | rename/move 保留 ID；split 至多一个输出继承旧 ID；merge 至多承接一个旧 ID，其余退役；全新 ID 保留初始版本 |
| 退役 | 历史和最后版本保留，ID 不再复用；已发布 lifecycle 不删除或改写 |
| 机械移动 | 必须证明源码及原生元数据等价、依赖身份不变；业务入口改变不能标为机械移动 |
| 兼容性 | mechanical 仅用于纯 move；存续旧版本线的 compatible 至少 patch，breaking 至少 major 并提高产品级别；新增包保留初始版本，退役包不生成新版本；兼容性本身未由发布器证明 |
| 历史 | 逐真实提交消费迁移，按实际顺序把旧路径变化传到存续包；未被真实快照证明的中间身份不虚构 |
| 依赖 | 结合旧边删除和当前反向图传播，保守覆盖声明条件；无关包保持版本，单次每包只升一次 |
| 显式目标 | `target_versions` 按包 ID 声明；必须高于原版本且满足影响下限。非标准自动升级版本另声明 `target_levels`（1/2/3），不丢弃 epoch/pre/post/dev/local |
| 收据 | `package_changes` 为存续包版本变化；`package_events` 为身份前后/退役；`lifecycle_events` 含原始有方向关系和兼容性 |

Python 以 distribution 为发布单位，安装名规范化而 import 名、源码目录不代替身份。支持静态 PEP 621、setup.cfg、传统 Poetry 元数据；`version_sources` 可声明 text、TOML 键或专用静态 Python 字符串赋值文件。任意 setup.py/SCM/动态后端不执行；多版本事实源、跨包/产品源冲突阻断。Poetry path/git/url 源表不能伪造同时存在的 version，标准版本约束和工具源补充分别处理。`local_sources` 显式区分本地包与同名 registry 包；虚拟 workspace 的继承映射可通过此项明确列出。

Python lock 通过 `adapters.python.locks` 显式登记，不自动找一个同名文件就改；Cargo.lock 按登记的 workspace 根发现。支持的 wire format 为 Cargo 3/4、uv 1/revision 0–5、Poetry 2.1、PDM legacy 4.5.0/4.5.1、pylock 1.0；requirements-export 仅支持可证明的 editable 本地源，手写 requirements 不是完整 lock。工具版本与原生验证结果以项目证据为准，不将 wire format 支持等同于任意工具版本验证。

结构调整先提供正确拓扑；release 精确更新本地版本、引用及摘要，不重新选择第三方版本、不伪造 wheel/sdist hash。校验本地成员闭包、失效旧边、条件及特性；第三方 source/checksum/版本保持。PDM canonical/workspace 摘要、未知格式、无法证明条件或制品的情况明确阻断。缺锁的策略不能被描述为锁已验证。原生工具检查属于工程验证，不把 install、构建、业务测试加入每次 git-sync。
