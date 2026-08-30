---
lifecycle: active
validation_status: awaiting_validation
---

# Hook 信号文档下发与新增 whole 资产冲突保护

> 状态：候选镜像实施完成，静态与 fixture 验证通过；真实安装和 runtime 未验证
> 调用来源：`$develop` L 级总交付中的卡点 #1；本卡按 M 级独立切片实施
> 用户确认：2026-08-28 选择“不同内容的既有同名文件进入 risk，明确确认后才允许覆盖”

## 背景与目标

Template `AGENTS.md` 引用 `doc/3_reference/codex-hook-signals.md`，但当前 Template 和 managed contract 没有下发该文档。补齐公共文档资产，并修复同步器对“新合同首次引入 whole 资产、下游已有同名自有文件”可能静默列为 safe replace 的安全缺口。

## 不做

- 不修改仓库真实根骨架、真实 Template 或真实下游。
- 不改变 merge、region、seed 资产的现有语义。
- 不在本切片修改 VERSION、CHANGELOG，不 commit、不 push。
- 不把 `.codex/hooks/AGENTS.md` 当作 Hook 触发时会自动加载的指令。

## 任务规模与预算

- 规模：M。原因是单一文档漏项扩大为 current-only update 的通用 risk 语义修改，跨 managed contract、planner、apply、CLI 收据和测试。
- 预算：45 分钟 / 20k 新增 token（估算，未实测）/ 0 个子 agent / 最多 2 轮验证。
- 超预算停止点：需要改变非 whole 资产语义、扩大到真实根/下游，或第二轮修复后仍失败。

## 已核实事实

- 工厂与 Template `AGENTS.md` 都引用 `doc/3_reference/codex-hook-signals.md`。
- 工厂文件存在，但 `templates/doc/3_reference/`、managed contract 和下游骨架没有该资产。
- V11 proposal 已提供公共文档候选及 `codex.doc.hook-signals` 稳定资产编号。
- 当前 planner 会在读取旧合同所有权之前，先把新合同中的不同内容目标生成 `safe replace`。
- CLI 已有 `--confirmed-risk`，但 current-only update 当前明确拒绝任何 risk action。

## 已确认行为

| 下游现场 | 计划与执行语义 |
|---|---|
| 目标不存在 | `safe create` |
| 目标与新资产字节相同 | 不重写内容，安全登记新合同 |
| 目标存在、内容不同且旧合同从未拥有 | `risk replace`；未经 `--confirmed-risk` 禁止 apply |
| 旧合同仅以 `merge`、`region`、`seed` 或其他部分所有权管理，新合同改为 `whole` | `risk replace`；禁止把“路径曾受管”误当成整文件所有权 |
| 旧合同已经拥有目标但现场漂移 | 保持现有 drift 阻断，不降级为普通 risk |

Risk apply 必须绑定紧邻计划的 aggregate fingerprint，apply 前重新规划；未确认时零写入，确认后仍使用事务快照、失败回滚和版本戳最后写入。

## 拟修改

- 完整候选镜像的 Template 与 dogfood Hook 信号参考文档。
- 候选 `templates/managed-skeleton.json` 及确定性生成镜像。
- 候选同步器对新增 whole 资产的旧合同所有权判定、risk apply 和计划收据。
- 候选 Template 文档索引、针对性同步测试和必要的 Bug/进度记录。

## 验收

1. fresh init 安装文档，引用路径存在。
2. update 时目标不存在为 safe create。
3. update 时同内容目标不重写并安全登记合同。
4. update 时不同内容且旧合同未拥有，计划列为 risk；未确认 apply 零写入。
5. `--confirmed-risk` 与正确 fingerprint 下允许替换；计划漂移仍阻断。
6. 故障注入恢复全部写入；版本戳最后写入。
7. 第二次 update 无 action、risk、gap 或 blocker。
8. Template/dogfood managed contract 一致，重建器 `--check` 通过。
9. 针对性 unittest、proposal/结构检查通过；未执行的真实下游和 runtime 明确标为未验证。

## 假设与风险

- 新 whole 资产的冲突保护是通用行为，未来同类资产自动复用。
- 单一 `--confirmed-risk` 会确认当前 fingerprint 中全部 risk action；因此计划必须完整展示目标及前后 hash，并在 apply 前重算 fingerprint。
- 候选镜像不是独立 Git 仓库，验证使用当前 BridgeForge 项目 `.venv`，不把候选静态结果冒充真实发布收据。

## 实施记录

- 在完整候选镜像新增公共 Template 文档 `templates/doc/3_reference/codex-hook-signals.md`，并同步候选 dogfood 文档。
- 新增稳定资产 `codex.doc.hook-signals`，目标为 `doc/3_reference/codex-hook-signals.md`，ownership strategy 为 `whole`。
- 同步器现在先读取旧合同所有权：新 whole 资产撞到不同内容的项目自有文件时列为 `risk replace`；同内容文件只登记合同，不重写。
- current-only update 可在正确 aggregate fingerprint 与 `--confirmed-risk` 下执行 risk；apply 前重算计划，失败继续走事务回滚，版本戳最后写入。
- 增加冲突确认、同内容接管、计划漂移和故障回滚测试；同步 Template 文档索引与候选 managed contract。
- 修正 proposal 合同中 `doc/README.md` 的迁移后哈希，并确认 `CHANGELOG.md` 指针在保存提交 `b269f8a` 时即为错误基线后更新其哈希。
- 独立审计发现旧部分所有权转 `whole` 时只按 target 判定的 P1；现改为复用“真正 whole-owned”判定，并补齐 `merge/region/seed → whole` 与真实 CLI 确认回归。

## 验证记录

- 候选 `test_current_baseline_project_sync.py`：55 项通过，覆盖新增 whole 资产 risk、三种部分所有权转 whole、CLI 确认执行、no-op 与回滚路径。
- 候选 `run_downstream_fixture.py`：3 项通过，覆盖 current init 幂等、旧项目确认重建、current drift 零写入阻断。
- 候选 `project_structure_check.py --pre-commit`：退出码 0，仅报告既有 archive advisory。
- 候选 `rebuild_shared_skill_manifest.py --check`：输出 `unchanged`。
- `proposal/contracts/validate_proposal.py`：`proposal-contract: PASS`。
- `git diff --check`：退出码 0。
- 独立有限复审：无 P0/P1，结论 `PASS`，允许进入 IA-01～IA-14。
- 未执行真实根骨架替换、真实下游同步、Codex Hook runtime smoke、版本发布或用户试用；这些证据不得标为通过。
