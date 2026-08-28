# BridgeForge Batch 双缺陷修复与真实批次恢复确认卡

- 状态：实施、完整回归与独立审计完成，待发布和真实恢复
- 日期：2026-08-27
- 调用来源：`$bridgeforge-codex-batch` 共性阻断后转入 `$develop -> $confirm`
- 后续交接目标：`$develop`

## 原始需求摘要

用户要求恢复 `$bridgeforge-codex-batch` 的正常工作能力，修复 BridgeForgeCodex 1.5.6
下发到真实下游的 `.gitattributes` 校验缺陷，以及工厂 Batch 在 pending 目标漂移后进入
不可恢复状态的缺陷。修复发布后，受控恢复 StratusAgent 共享 Git 仓库，保留 M2 当前
未提交骨架改动，并从头完成本次四项目批次。

## 目标

1. 临时 Git 校验不得继承真实 pre-commit 仓库环境或修改真实 common Git dir。
2. `$git-sync` 必须检测 commit / hook 前后的 repository identity 越界。
3. Batch 必须能把确认后漂移的 pending 目标安全延期，继续其余首次处理，再重新确认重试。
4. 发布修复版后恢复 StratusAgent 的普通 worktree 身份，并完成四项目真实批次。

## 不做

- 不修改任何下游业务代码、交易逻辑、配置语义或 Git 历史。
- 不跳过 Hook，不手工拆分 fetch、commit、push，不使用 reset、rebase、merge 或 force push。
- 不删除 active batch 状态，不手改批次 JSON，不静默吸收目标漂移。
- 不丢弃、覆盖或错误归因 M2 当前 8 个未提交骨架文件。
- 不把工厂专属 Batch Skill 下沉到 Template 或共享 Skills。
- 不把手工 `core.bare=false` 当作产品修复；只有修复版发布后才执行一次受控恢复。

## 规模与预算

- 规模：L。
- 判定依据：跨产品 Template、dogfood、Git 事务边界、工厂 Batch 状态机、真实共享 Git
  仓库恢复、产品发布和四项目端到端重跑。
- 时间上限：120 分钟。
- Token 上限：50k 新增 token（估算；平台无可靠计量器，未实测）。
- 子 agent：最多 3 个，用于只读调研、分区实现和独立审计。
- 验证预算：最多 3 轮。
- 超预算停止点：需要扩大到下游业务代码、Git 历史修复、数据丢失恢复，或预计超过任一
  预算时停止并重新确认。

## 已核实事实

- 工厂 HEAD 为 `734b9544bd2e27884a655a6808c32bf518ed065a`，产品版本为 `1.5.6`。
- 工厂当前仅有两份 Bug 报告和 `doc/README.md` 索引变更，源码尚未修改。
- `current_baseline.py::_gitattributes_default_state()` 在 pre-commit 中执行临时 `git init`
  时未清除 repository-local Git 环境，也未把隔离环境传给 `git init`。
- StratusAgent common Git config 当前为 `core.bare=true`；M2 的
  `git rev-parse --is-inside-work-tree` 返回 `false`，普通 `git status` 失败。
- 使用不写配置且禁用 optional locks 的临时只读覆盖后，M2 可见 8 个未提交文件，全部为
  BridgeForge 1.5.6 骨架文件；未发现业务代码改动。
- M2 与 master 共用 `D:\Quant\StratusAgent\.git`，因此 common config 恢复会同时影响两者。
- Batch `begin_target()`、`finish_target()`、`refresh_plan()` 与 `_next_target()` 的条件互锁，
  pending 目标漂移后不存在官方可达的 deferred -> reconfirm 路径。
- 当前批次中 BridgePersonalAssist 与 causis_risk_suite 已升级到 1.5.6、保存到 GitHub并达到
  clean / 0 0；两个 StratusAgent 目标尚未由当前批次成功处理。

## 未核实事实

- 修复后真实 pre-commit 是否保持 common Git config 字节不变，需由真实 linked-worktree
  验收证明。
- StratusAgent master 在恢复后是否还有独立工作树改动，需恢复前后分别只读核验。
- 当前远端凭据与推送权限是否仍有效，需各仓库官方 `$git-sync` 实际验证。
- 产品修复是否需要第三轮以外的修改重测；若需要则触发预算升级闸。

## 已确认规则

- 先修复、验证并发布 BridgeForgeCodex，再恢复 StratusAgent common config；禁止在仍安装
  1.5.6 缺陷副本时直接重试 `$git-sync`。
- 恢复前必须再次核对 common Git dir、原普通 worktree 证据、当前 `core.bare` 和两个
  worktree 身份，并在执行高风险配置修改前取得窄范围确认。
- 恢复只允许把已证实被本缺陷改坏的 `core.bare=true` 受控改回 `false`；禁止顺手重写其他
  Git config。
- M2 当前骨架改动必须原样保留，经修复版重新校验后只通过 repo-local `$git-sync` 保存。
- Batch 修复后必须从新的 factory HEAD 和可验证的新产品 fingerprint 重启原 active batch，
  四个目标全部从头核验。
- 工厂产品修复需要 bump 根 `VERSION` 并在 `CHANGELOG.md` 标记 `[product]`；Batch 工厂专属
  修复同时记录 `[repo]`，但不单独污染下游 Template。

## 数据与状态映射

| 当前事实 | 修复后目标状态 |
|---|---|
| pre-commit 继承真实 `GIT_DIR` 等变量 | 临时 Git 命令使用统一隔离环境，真实仓库零写入 |
| common config `core.bare=true` | 经确认后受控恢复为 `false`，其他配置保持不变 |
| M2 8 个未提交骨架文件 | 保留并由修复版 `$git-sync` 正常提交、推送 |
| Batch pending 目标漂移 | 原子转为 deferred，继续后续目标，再重新确认重试 |
| active batch `common_blocked` | 新工厂版本发布后 restart，全目标重新核验 |

## 拟修改

- `templates/scripts/current_baseline.py` 及 dogfood 镜像：统一清除 repository-local Git 环境，
  同一隔离环境传给临时 `git init` 与 `git check-attr`。
- `templates/scripts/codex_git_sync.py` 及 dogfood镜像：增加 commit / hook 前后 repository
  identity 只读快照与越界阻断收据。
- 对应 managed contracts、manifest、Template / dogfood hash、根 `VERSION` 与
  `CHANGELOG.md`。
- `.codex/skills/bridgeforge-codex-batch/scripts/batch_control.py`：补齐 pending drift 的原子
  deferred 转换，并使 factory-only 修复后的 restart 证明与真实修复资产一致。
- `scripts/tests/**`：增加真实 pre-commit、linked worktree、Git 环境注入、common config
  sentinel、Git 事务身份和 Batch 首/中/末目标漂移回归。
- 两份 Bug 报告、当前确认卡及 `doc/README.md` 的实施与验证状态。

## 验收

### 产品与 Git 安全

- 普通仓库和 linked worktree 通过真实 `git commit` 触发 current-baseline 两次，common Git
  config 前后字节一致，`core.bare=false`，所有 worktree 的 `git status` 正常。
- 显式注入 `GIT_DIR`、`GIT_WORK_TREE`、`GIT_INDEX_FILE`、`GIT_COMMON_DIR`、object、alternate
  与 quarantine 变量时，临时验证仍不触碰 sentinel repository。
- `$git-sync` 能区分工作树/index 回滚与 repository identity 漂移，禁止产生误导性“已恢复”收据。

### Batch 状态机

- 首个、中间、最后一个 pending 目标漂移均能零目标写入地转为 deferred，并继续确认顺序。
- 首次处理结束后可执行 refresh-plan -> reconfirm -> begin 完整重试。
- 共享 Git common dir 仍严格串行，active batch、锁与用户输出边界不回归。
- 新工厂 HEAD 修复产品或 factory-only Batch 后，restart 能验证相应真实修复资产，禁止用无关提交绕过。

### 工厂发布与独立审计

- factory dogfood、skill metadata、manifest `--check`、project structure、mirror drift、完整
  downstream fixture、完整自动测试与 `git diff --check` 通过。
- 独立审计复核临时 Git 隔离、共享配置边界、事务错误语义、Batch 状态可达性、发布传播和
  真实恢复顺序，无未处理 Blocker / High / Medium。
- 工厂仅使用自身 `$git-sync` 提交并推送，最终 clean 且 `ahead=0 behind=0`。

### 真实恢复与批次闭环

- 受控恢复 StratusAgent `core.bare=false` 后，M2 与 master 均恢复 worktree 身份；HEAD、index、
  分支、upstream 和 M2 8 个骨架改动保持一致。
- 两个 StratusAgent 安装修复版并用各自 repo-local `$git-sync` 完成保存。
- 原 active batch 从头重跑四个目标；最终每个目标骨架为修复版、工作区干净、GitHub 已保存，
  `ahead=0 behind=0`，随后官方 `close` 释放 active batch。

## 合理假设与风险

- 假设报告中的受损 common config 确由 1.5.6 pre-commit 临时 `git init` 写入；恢复前仍须用
  当前配置、Git reflog/HEAD 和两个 worktree 元数据交叉证明。
- common config 为共享高风险元数据；错误恢复可能同时影响两个 worktree，因此禁止在产品
  修复发布前操作。
- 当前 M2 改动属于骨架更新但尚未完成真实 commit；任何测试或恢复必须避免刷新、暂存或重写
  其 index。
- 远端竞态、分叉、凭据失败或新增未知 dirty state 按现有 Skill 合同保留现场并停止，不自动解决。

## 自动化边界

- 允许：已确认范围内的源码、测试、Template/dogfood、manifest、VERSION、CHANGELOG、文档
  更新，以及官方 factory/downstream `$git-sync`。
- 单独确认：真正修改共享 Git config 的 `core.bare` 前，必须报告精确目标与影响后再执行。
- 禁止：手工 Git 提交/推送、历史改写、Hook 绕过、状态 JSON 删除、下游业务修改或未验证恢复。

## 实施与验证记录

- 实施：临时 Git 校验器与 project-sync 同型入口统一清除 repository-local、quarantine 和
  动态 Git config 环境；`git init` / `git check-attr` 共用隔离环境。
- 实施：`$git-sync` 固定 worktree、git-dir、common-dir、index、symbolic HEAD、`core.bare`
  与 common config digest；identity 漂移时停止自动恢复并输出 HIGH 收据。
- 实施：Batch pending 漂移/不可读原子 deferred 且不创建 attempt；reconfirm 恢复 pending；
  restart 按问题签名检查真实修复文件的 tracked blob 或产品 fingerprint。
- 验证：全量 unittest 307 项通过（1 项环境性 symlink 跳过）；审计修正后的三组核心回归
  81 项通过（同一项 symlink 跳过）。
- 验证：普通 checkout 与 linked worktree 各经一次真实 `git commit` 触发产品 pre-commit；
  两次均显式继承事故级仓库变量，common config 前后字节一致，`core.bare=false`，状态正常。
- 独立审计：首次发现并修复 deferred 重确认顺序与失败 pre-push identity 复核两处 Medium；
  复核后无 Blocker / High / Medium，发布与恢复顺序一致。
- 真实恢复与批次结果：待工厂发布后执行。
