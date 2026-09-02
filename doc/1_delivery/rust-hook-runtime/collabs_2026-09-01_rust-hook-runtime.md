# Rust Hook Runtime 协作记录

## 需求与预算

- 确认卡：`requirements_2026-09-01_rust-hook-runtime.md`
- 目标：项目骨架 6 个 Hook 的完整运行链迁移到 Rust；Hook 运行时零 Python，保持跨平台、
  无可见控制台窗口及失败语义等价。
- 当前预算：4 小时、100k 新增 token 估算（未实测）、最多 3 个子 agent、最多 3 轮验证。

## 只读研读收据

- `light-explorer` 已核对同步事务、manifest/current baseline 合同、Hook 测试向量、Cargo 产物
  忽略策略和 dirty 重叠文件；未修改任何文件。
- 最安全的构建插入点位于同步器完成 fingerprint、用户确认和 stamp 漂移检查之后，任何
  `transaction.write()` 之前。Cargo 使用仓库外临时 `--target-dir`，成功产物再经现有
  `_Transaction.write()` 纳入同一回滚事务。
- 当前 schema 3 只支持静态 `whole|merge|region|seed` 资产，不能表示平台相关生成二进制。
  生成产物必须使用明确合同，禁止伪装成普通静态资产。
- 当前行为事实源主要是 `scripts/tests/test_codex_hook_single_source.py`；事务、rollback、
  fingerprint 向量主要在 `test_current_baseline_project_sync.py` 与
  `test_project_asset_migration.py`。

## 固定接口合同

| 项目 | 合同 |
|---|---|
| Runtime CLI | `bridgeforge-hook <pre-tool|post-edit|post-shell|post-compact|stop|session-start>` |
| 自检 | `bridgeforge-hook self-test --json` |
| 下游源码 | `.codex/hooks/**` |
| 下游二进制 | Windows `.codex/bin/bridgeforge-hook.exe`；非 Windows `.codex/bin/bridgeforge-hook` |
| 构建目录 | 仓库外临时 Cargo `--target-dir`，禁止在项目内生成 `target/` |
| 生成收据 | `.codex/bin/build-receipt.json`，记录 platform、source tree、Cargo.lock、recipe、binary hash 与 Cargo version |
| Plan fingerprint | 锁定平台键、源码树 hash、Cargo.lock hash、构建配方和自检合同；不锁定本机二进制 hash |
| Apply | 构建/自检在零产品写入阶段完成；二进制和收据经 `_Transaction.write()` 写入并参与 rollback |
| Runtime I/O | stdin UTF-8 JSON；stdout 最多一个 Hook JSON；stderr 诊断；返回码与现行合同等价 |

## 拟拆分

| 任务 | 负责人 | 唯一文件边界 | 依赖 |
|---|---|---|---|
| A. Rust runtime 与行为测试 | 主 agent | Rust crate、Template/dogfood Rust 源码、Hook 配置、Rust 定向测试及旧 Hook 资产清理 | 使用本记录固定 CLI、路径和收据合同 |
| B. generated asset 与同步事务 | `implementation-worker` | `scripts/bridgeforge_codex_project_sync.py`、`scripts/rebuild_shared_skill_manifest.py`、`templates/scripts/current_baseline.py`、`scripts/tests/test_generated_hook_build_transaction.py` | 只依赖固定 generated spec 与 self-test CLI；不得修改任务 A 文件 |
| C. 独立交付审计 | `review-auditor` | 只读真实需求卡、协作记录、源码、测试与 `git diff` | A/B 串联完成后执行，不参与实现 |

主 agent 负责 `templates/managed-skeleton.json`、dogfood 镜像、manifest 重建、版本、CHANGELOG、
文档串联和最终测试，避免与任务 B 的 dirty 同步器重叠修改冲突。

## 当前状态

- 只读研读：完成。
- 产品实现：完成，等待用户在真实 Codex Desktop 观察无弹窗体验。
- 预算：共使用 3 个子 agent，未超过上限；实现由主对话整合，未提交、未推送、未写真实下游。
- 独立审计：完成；blocker/high/medium 均已逐项修复或明确列为平台实测边界。
- 下一步：用户试用一次；若确认无弹窗，再由 `$summary 同意验收` 结算。

## 验证记录

- 三轮边界内完成：首轮完整 factory 暴露问题；第二轮实现聚焦与 downstream fixture；第三轮
  审计修复聚焦。逐命令收据与未验证边界见确认卡“验证记录”。
