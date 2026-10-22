# artifact-layout：布局检查与提交工作流

本合同描述 Forge 的通用布局检查。任务选择、授权、预算和本轮验证证据见 [TODO-020 需求与计划](../../1_delivery/artifact-layout/requirements_2026-10-23_artifact-layout-merge.md)。

## 入口与职责

| 入口 | 数据来源与行为 |
|---|---|
| `bridgeforge check project-structure --root .` | 原有文档结构检查，加工作区布局审计；发现违规或必需读取失败返回非零 |
| `bridgeforge check project-structure --root . --pre-commit` | 原有文档检查，加本次暂存改动及其影响范围的布局检查；不搬文件、不改索引 |
| 既有 `bridgeforge-hook post-edit` | 对归一化后的编辑路径提供布局警告；操作已经完成，警告不表示撤销 |
| release preparation 记录文档核验 | 只调用文档检查，不扫描布局、缓存或构建树 |

普通 git-sync 与 git-sync release 均通过 Git commit 的 pre-commit 调用一次统一提交入口。外层不重复扫描，SessionStart 默认不扫描。无新提交的同步、release preview 和发布分析不主动运行布局审计。baseline、编码、版本、构建身份和 Git tree/推送保护仍各司其职。

## 配置所有权

项目在 `.codex/artifact-layout.json` 声明自己的规则。Forge 管理检查实现，不接管、覆盖、删除或自动生成该配置正文；无配置时只运行既有根 test/tests 禁令。

| 字段 | 含义及约束 |
|---|---|
| `schema` | 必填，当前为 1；未知版本和未知/重复字段报错 |
| `source_roots` | 生产源码根列表，默认空；不与测试、工具或缓存根重叠 |
| `tests_root` | 独立测试根，默认 `scripts/tests/`，必须遵守公共测试目录边界 |
| `tools_root` | 独立工具根，默认 `scripts/`；允许包含 tests_root |
| `python_artifacts.enabled` | 是否检查 Python/pytest/coverage 过程产物，默认 false |
| `python_artifacts.allowed_roots` | 启用检查时必需，允许的仓库根相对缓存目录 |
| `forbidden_source_components` | 生产根之下禁止出现的目录组件；不根据普通文件名直接阻断 |
| `cargo_registration.manifest_roots` | 检查 Cargo.toml 的生产目录根 |
| `cargo_registration.forbidden_target_roots` | 这些目录中的工具/测试不能被生产 manifest 注册为 workspace member 或显式 target |

路径只能是仓库相对路径，不允许绝对路径、冒号、越界、glob、换行或空值。配置中的目录为明确前缀，匹配按组件边界，不将 `scripts-other/` 当作 `scripts/`。Windows 按平台大小写和分隔符语义匹配。无配置与非法配置不同：非法配置或必需读取失败不静默关闭检查。

示例（通用项目，无历史专名）：

```json
{
  "schema": 1,
  "source_roots": ["src/"],
  "tests_root": "scripts/tests/",
  "tools_root": "scripts/",
  "python_artifacts": {"enabled": true, "allowed_roots": [".runtime/"]},
  "forbidden_source_components": ["tests", "scripts", "benches", "examples", ".runtime"],
  "cargo_registration": {"manifest_roots": ["src/"], "forbidden_target_roots": ["scripts/"]}
}
```

## 规则与覆盖限制

1. 根 `test/`、`tests/` 禁止；独立测试使用 `scripts/tests/`。启用项目规则时，目录中的 `tests` 组件不得越过声明的测试根。受管测试文件按 managed-skeleton 的明确 target 身份识别，不整体豁免 `.codex/` 或旧宿主目录；缓存位置检查仍有效。
2. Python 过程产物包括 `__pycache__`、`.pytest_cache`、`htmlcov`、`.coverage` 及分片、`coverage.xml`、`.pyc`、`.pyo`。启用后它们必须位于声明的缓存根，单凭 Git 忽略不代表位置正确。
3. 生产源码根内的禁止目录由项目声明。不按 `test_*`、`*_test` 等名字推定文件职责，不将 Rust 内联 cfg(test) 判为独立测试违规。
4. Cargo 检查覆盖 workspace.members（包括通配成员）及显式 test/bench/example/bin 的 path，兼容表数组和内联表数组，按 manifest 所在目录解析目标。成员通配在提交时按暂存树目录展开，手动审计按实际遍历树展开；编辑提示不为展开通配遍历全仓。TOML 无效、目标绝对路径或越界均报告。普通生产依赖不在此规则内。
5. 当前不覆盖源码 include/path 引用、Cargo 自动发现入口、依赖方向、DLL 身份，也不判断历史诊断程序的业务职责。
6. 工作区审计先检查目录自身，再跳过 `.git`、依赖环境、node_modules、target 和允许的缓存树内容；不跟随链接，报告排除范围。遍历或必需读取失败属于不完整检查，不能据无输出宣称全面通过。

## A 模式：提交只检查本次变化及影响范围

- 新增、修改及重命名目标按最终暂存路径检查；删除违规文件允许通过。
- 未涉及的历史违规、工作区未暂存内容及被忽略的缓存不阻止提交。需要另行手动工作区审计与清理。
- 规则变动比较有效配置；仅格式变化不扩大检查。语义变化时检查新规则影响的暂存树，未改变规则下的无关历史问题不阻断。
- 修改 Cargo manifest 时比较新旧显式注册；更改被禁止的目标目录时检查相关注册。既有违规但本次未影响的引用不当作新违规。
- Cargo 通配成员按展开后的目标身份比较；仅改 manifest 注释不会把既有违规重新算成新增。政策与 manifest 同时变化时结合旧新禁止范围判断，不按整份 manifest 是否修改粗略放大影响范围。
- managed-skeleton 移除受管测试身份时，只检查因此失去豁免而仍保留在暂存树中的路径；历史比较使用旧身份，无关兄弟文件和历史问题不扩大检查。
- 检查暂存版本的配置与 Cargo blob，不能读取工作区修复替代暂存违规。配置可以显式停用或删除，按新的有效规则处理；布局检查不是阻止项目修改自身政策的权限系统。
- 不保存一份历史违规基线来判断“新旧”；使用本次 Git 差异、配置和引用关系。相关输入不能可靠读取时阻断并报告。

## 性能与验收

提交模式使用一份暂存差异、一份索引和按需批量 blob 读取，不逐文件启动 Git，不在本机工作区全量遍历。Cargo 无关变更不解析全部生产 manifest。布局检查不触发构建、不新增 Git 同步步骤，也不使文档轻量核验变成全仓产物审计。

Git 事务在每个既有身份保护点仍重新读取现场；git_dir、common_dir、index_path 和 HEAD 使用一次多值 rev-parse 查询，严格校验完整输出，不跨写入缓存身份。split-index、配置漂移、锁和提交后 tree/推送保护保持原位置。多值协议对无法可靠解析的输出明确拒绝，不使用部分结果；含换行的仓库路径不在该协议支持范围。

git-sync 与 release 性能不退化属于本次交付硬条件：相同隔离 fixture 和缓存状态，前后各场景至少 5 次有效样本，逐场景判断。可重复、可归因退化不得通过；静态去重不能代替实测，离线 CLI 不能冒充真实对话端到端性能。触发限定范围 refactor-scan 的条件和预算边界以需求卡为准。
