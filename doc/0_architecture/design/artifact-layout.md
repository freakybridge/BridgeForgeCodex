# artifact-layout：布局检查与提交工作流

本合同描述 Forge 的通用布局检查。当前三目录增量及验证状态见 [TODO-048](../../1_delivery/artifact-layout/requirements_2026-10-30_three-directories.md)；原公共默认规则与合并验收历史分别见 [默认规则增量](../../1_delivery/artifact-layout/requirements_2026-10-23_artifact-layout-defaults.md) 和 [TODO-020](../../1_delivery/artifact-layout/requirements_2026-10-23_artifact-layout-merge.md)。

## 三目录基础合同

路径及类别由 `source/core/src/artifact_paths.rs` 统一维护，合同标识为 `three-directory-layout-v1`。运行产物进入 `.runtime/`，依赖进入 `.deps/`，编译产物进入 `.build/`；工厂生成工具使用 `.build/bridgeforge/bin/`，消费项目公共工具使用 `.deps/bridgeforge/bin/`。本机配置、下载和测试结果分别固定在 `.runtime/config/`、`.runtime/download/` 和 `.runtime/tests/<任务>/`。受版本管理的配置、源码、锁文件和输入样本不因扩展名被视为运行产物。

三个根必须有项目 `.gitignore` 的根目录排除规则。检查按规则顺序识别使根目录重新可见的反向规则，不依赖用户全局配置；根目录已经被排除时，子项反向规则不能突破 Git 的目录排除语义。已跟踪产物另行检查。手动审计使用工作区规则；提交使用暂存 blob，删除历史违规文件不被额外阻断；编辑只警告。

可选 `outputs` 列表登记额外精确路径，每项含 `path`、`kind` 和可选 `directory`（默认 false）。类别为 `runtime/config/download/test/dependency/build`，必须符合固定根和测试任务层级；不能登记根外豁免。基础规则不因 `python_artifacts.enabled=false` 停用，旧 `allowed_roots` 只接受 `.runtime/` 内路径，不作为遍历剪枝例外。

build 输出还可登记可选 `cleanup: keep/cache/history`，供显式 cleanup 消费；缺省保留，keep 优先，不能将整个 `.build` 标为可删。cache 表示编译缓存，history 表示项目已确定的旧版本产物，keep 用于当前程序、配套运行文件和收据。不按文件时间猜测版本，不改变 runtime 固定保留范围。详细清理行为与验证见 [cleanup 需求](../../1_delivery/artifact-cleanup/requirements_2026-10-30_cleanup-skill.md)。

默认识别范围包含 Python 字节码、pytest/coverage 产物、`.venv`/`venv`/`node_modules`、`target`/`dist` 及受管二进制/收据；源码中的同名目录和合法下载资料按类型、上下文区分。任意业务写文件逻辑及普通 JSON/log 文件不能仅凭名称确定用途，须由项目登记，检查器不宣称证明所有程序的写入行为。

## 入口与职责

| 入口 | 数据来源与行为 |
|---|---|
| `bridgeforge check project-structure --root .` | 原有文档结构检查，加工作区布局审计；发现违规或必需读取失败返回非零 |
| `bridgeforge check project-structure --root . --pre-commit` | 原有文档检查，加本次暂存改动及其影响范围的布局检查；不搬文件、不改索引 |
| 既有 `bridgeforge-hook post-edit` | 对归一化后的编辑路径提供布局警告；操作已经完成，警告不表示撤销 |
| release preparation 记录文档核验 | 只调用文档检查，不扫描布局、缓存或构建树 |

普通 git-sync 与 git-sync release 均通过 Git commit 的 pre-commit 调用一次统一提交入口。外层不重复扫描，SessionStart 默认不扫描。无新提交的同步、release preview 和发布分析不主动运行布局审计。baseline、编码、版本、构建身份和 Git tree/推送保护仍各司其职。

## 配置所有权

Forge 提供公共默认规则，项目仅在可选的 `.codex/artifact-layout.json` 声明差异。无配置、仅有 `{"schema":1}` 和显式填写完整默认值具有相同的有效规则；文件不存在不关闭通用检查。Forge 不接管、覆盖、删除或自动生成项目配置正文。

未填写的字段继承默认值，包括嵌套对象字段；显式列表是该字段的完整声明，不与默认数组隐式拼接。配置删除后恢复公共默认规则，只移除项目扩展检查。保留 schema 1 和已有字段，不要求项目重写完整配置。默认检查增强属于产品行为变化，发布分析必须说明兼容性影响。

| 字段 | 含义及约束 |
|---|---|
| `schema` | 必填，当前为 1；未知版本和未知/重复字段报错 |
| `source_roots` | 生产源码根列表，默认空；不与测试、工具或缓存根重叠 |
| `tests_root` | 独立测试根，默认 `scripts/tests/`，必须遵守公共测试目录边界 |
| `tools_root` | 独立工具根，默认 `scripts/`；允许包含 tests_root |
| `python_artifacts.enabled` | 保留旧配置字段；Python/pytest/coverage 的严格三根基础检查始终启用 |
| `python_artifacts.allowed_roots` | 兼容旧声明，仅接受 `.runtime/` 内路径；不开放根外豁免，也不覆盖固定测试任务前缀 |
| `forbidden_source_components` | 生产根之下禁止出现的目录组件；不根据普通文件名直接阻断 |
| `cargo_registration.manifest_roots` | 检查 Cargo.toml 的生产目录根 |
| `cargo_registration.forbidden_target_roots` | 这些目录中的工具/测试不能被生产 manifest 注册为 workspace member 或显式 target |

路径只能是仓库相对路径，不允许绝对路径、冒号、越界、glob、换行或空值。配置中的目录为明确前缀，匹配按组件边界，不将 `scripts-other/` 当作 `scripts/`。Windows 按平台大小写和分隔符语义匹配。无配置与非法配置不同：非法配置或必需读取失败不静默关闭检查。

示例（通用项目，无历史专名）：

```json
{
  "schema": 1,
  "source_roots": ["src/"],
  "python_artifacts": {"enabled": true, "allowed_roots": [".runtime/"]},
  "forbidden_source_components": ["tests", "scripts", "benches", "examples", ".runtime"],
  "cargo_registration": {"manifest_roots": ["src/"], "forbidden_target_roots": ["scripts/"]}
}
```

## 规则与覆盖限制

1. 根 `test/`、`tests/` 禁止；独立测试、基准及专用辅助文件使用 `scripts/tests/`，独立工具归 `scripts/`。无配置也检查目录中的 `tests` 组件不得越过测试根。受管测试文件按 managed-skeleton 的明确 target 身份识别，不整体豁免 `.codex/` 或旧宿主目录；缓存位置检查仍有效。
2. Python 字节码进入运行根，依赖/构建/测试树中的附属字节码保持对应所有权；pytest、覆盖率与报告必须位于 `.runtime/tests/<任务>/`。基础规则默认启用，单凭 Git 忽略不代表位置正确。
3. 生产源码根内的禁止目录由项目声明。不按 `test_*`、`*_test` 等名字推定文件职责，不将 Rust 内联 cfg(test) 判为独立测试违规。
4. Cargo 检查覆盖 workspace.members（包括通配成员）及显式 test/bench/example/bin 的 path，兼容表数组和内联表数组，按 manifest 所在目录解析目标。成员通配在提交时按暂存树目录展开，手动审计按实际遍历树展开；编辑提示不为展开通配遍历全仓。TOML 无效、目标绝对路径或越界均报告。普通生产依赖不在此规则内。
5. 当前不覆盖源码 include/path 引用、Cargo 自动发现入口、依赖方向、DLL 身份，也不判断历史诊断程序的业务职责。
6. 工作区审计先检查目录自身及显式声明，再剪枝依赖、构建和合法运行派生树；错误位置的 `.venv`、node_modules、target 先报告再剪枝。`.runtime/tests/` 保留任务层级检查，不将其中隔离仓库当成项目源码。不跟随链接或 reparse 点，发现时报告错误；必要读取失败属于不完整检查。剪枝不豁免暂存路径。
7. 公共 AGENTS 承载布局和依赖红线，配置只补项目路径事实，项目 AGENTS 保留测试命令、构建组和业务副作用约束。Hook 不仅凭名称判断所有测试或工具，不证明完整依赖方向；迁移时同步测试发现、manifest、引用和执行入口，保留原覆盖。

测试根可细化到 scripts/tests 的子目录；手动审计允许该根的祖先目录本身，不豁免祖先下的文件或兄弟目录。真实、可读且确实为空的非根测试目录没有 Git 文件布局对象，仅不报告其 misplaced-test-directory；根 test/tests 禁令、项目显式禁止目录及缓存规则仍生效。隐藏文件也属于内容，读取失败仍阻断，链接和 reparse 点不按空目录豁免；不自动清理遗留空目录。

## A 模式：提交只检查本次变化及影响范围

- 新增、修改及重命名目标按最终暂存路径检查；删除违规文件允许通过。
- 未涉及的历史违规、工作区未暂存内容及被忽略的缓存不阻止提交。需要另行手动工作区审计与清理。
- 规则变动比较有效配置；格式变化、显式填写默认值、增删等价默认配置均不扩大检查。语义变化时检查新规则影响的暂存树，未改变规则下的无关历史问题不阻断。旧配置损坏属于未知规则，不能冒充原本没有配置；修复或删除时重新检查受新规则影响的范围。
- 修改 Cargo manifest 时比较新旧显式注册；更改被禁止的目标目录时检查相关注册。既有违规但本次未影响的引用不当作新违规。
- Cargo 通配成员按展开后的目标身份比较；仅改 manifest 注释不会把既有违规重新算成新增。政策与 manifest 同时变化时结合旧新禁止范围判断，不按整份 manifest 是否修改粗略放大影响范围。
- managed-skeleton 移除受管测试身份时，只检查因此失去豁免而仍保留在暂存树中的路径；历史比较使用旧身份，无关兄弟文件和历史问题不扩大检查。
- 检查暂存版本的配置与 Cargo blob，不能读取工作区修复替代暂存违规。项目扩展检查可以显式停用或随配置删除，公共默认检查始终保留；布局检查不是阻止项目修改自身政策的权限系统。
- 不保存一份历史违规基线来判断“新旧”；使用本次 Git 差异、配置和引用关系。相关输入不能可靠读取时阻断并报告。

## 性能与验收

提交模式使用一份暂存差异、一份索引和按需批量 blob 读取，不逐文件启动 Git，不在本机工作区全量遍历。Cargo 无关变更不解析全部生产 manifest。布局检查不触发构建、不新增 Git 同步步骤，也不使文档轻量核验变成全仓产物审计。

无项目配置时，仅在选定路径可能使用受管测试身份例外，或受管清单本身变化时读取身份 blob；选定路径包括政策变化影响的历史路径。项目配置存在时保留原读取路径。无需该数据与必需读取失败严格区分，后者不能降级为空身份集合。

Git 事务在每个既有身份保护点仍重新读取现场；git_dir、common_dir、index_path 和 HEAD 使用一次多值 rev-parse 查询，严格校验完整输出，不跨写入缓存身份。split-index、配置漂移、锁和提交后 tree/推送保护保持原位置。多值协议对无法可靠解析的输出明确拒绝，不使用部分结果；含换行的仓库路径不在该协议支持范围。

git-sync 与 release 性能不退化属于本次交付硬条件：相同隔离 fixture 和缓存状态，前后各场景至少 5 次有效样本，逐场景判断。可重复、可归因退化不得通过；静态去重不能代替实测，离线 CLI 不能冒充真实对话端到端性能。触发限定范围 refactor-scan 的条件和预算边界以需求卡为准。
