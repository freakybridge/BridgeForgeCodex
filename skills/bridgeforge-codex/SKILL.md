---
name: bridgeforge-codex
description: 初始化、升级或诊断 Windows Codex 骨架及受管 Skills；咨询和只读诊断不触发维护，仅提及产品名不触发本流程。
user_invocable: true
argument: 仅支持无参数
---

# bridgeforge-codex

仅由主对话编排无参数 `$bridgeforge-codex`；不支持旧 `$bridgeforge`、`switch`、Claude 维护或内部参数。

## 0. 先分流意图

咨询直接回答；诊断、审计或只读预览仅检查已有相关文件，涉及运行时先读 [runtime preflight](references/runtime-preflight.md)，不进入下文维护流程，不刷新、安装或修复。未查上游时禁止称本机版本为最新。

明确安装、升级或无附加限制的显式调用才进入 §1；附带的只读要求优先。意图不明且会写入时，只补问该决定，禁止从产品名称推断维护授权。

## 1. 刷新产品入口

维护仅支持 Windows；其他平台在下载或写入前停止。维护同时更新用户级 `~/.codex/AGENTS.md`：采用仓库统一模板，本地内容不同则先备份再覆盖，相同则不写。向用户说明这一范围后，复用已有维护授权，从薄入口运行一次 updater：

```powershell
$BRIDGEFORGE_CODEX_ENTRY = Join-Path $env:USERPROFILE ".codex\skills\bridgeforge-codex"
$BRIDGEFORGE_CODEX_HOME = Join-Path $env:USERPROFILE ".bridgeforge-codex"
& powershell -NoProfile -ExecutionPolicy Bypass -File `
  (Join-Path $BRIDGEFORGE_CODEX_ENTRY "scripts\bridgeforge_codex_shared_update.ps1")
```

成功收据必须包含 `BRIDGEFORGE_CODEX_SHARED_UPDATE_RECEIPT`。随后重新读取 `$BRIDGEFORGE_CODEX_HOME\skills\bridgeforge-codex\SKILL.md` 并以新版本继续。仅在旧入口成功收据完全缺少 `user_agents` 字段时，说明新增覆盖范围并从刚更新的薄入口补跑一次 updater；这是一次性版本过渡，其他情况本轮禁止再次刷新。过渡前的授权若明确限于 Skills 或项目文件，必须先确认用户级覆盖；无范围限制的维护授权直接复用。第二次仍缺字段、字段为空或状态不是 `updated/noop` 时停止，不循环重试或声称用户指令已更新。

结果简要展示 `user_agents.status`；覆盖时给出收据中的备份路径。`override_present=true` 时说明原有 `AGENTS.override.md` 可能优先生效，禁止删除它。用户指令由新会话加载，不声称当前会话已经重载。

薄入口只是 Codex 可发现入口，完整产品只能来自包含 `templates/hooks/Cargo.toml` 与受管 `bridgeforge` 二进制的官方产品 home。updater 失败、产品 home 缺文件、非普通目录、工作树不干净或 origin 不匹配时，读取 [用户级受管 Skill 维护](references/user-skill-maintenance.md) 后停止；禁止从旧用户目录、本地 clone 或当前项目补文件，也禁止读取、迁移或删除旧 BridgeForge/Claude 遗留。

## 2. 判断模式并锁定已验证工具

项目写入前，只读检查版本戳并锁定唯一 `$MODE`：

1. `.codex/.bridgeforge_codex_version` 与 `.codex/.bridgeforge_version` 双戳或非法戳：零写阻断。
2. 恰好一个合法戳：`update`；由当前合同的固定 `compatibility_baseline` 判定低基线重建或基线内兼容更新。
3. 没有合法戳但已有 `.codex/`、`AGENTS.md` 或其他骨架资产：`adopt`。
4. 没有骨架身份的空白项目：`init`。

每轮以刷新后的产品 home 为安装目标，固定升级分界线不得随发布上移。低于分界线时禁止读取旧 manifest、旧 schema、旧 hash lineage 或逐版本迁移链来选择实现；等于或高于分界线时兼容更新并保留项目定制。只允许使用 updater 事务安装并验证的受管 Rust CLI：

```powershell
$BRIDGEFORGE = Join-Path $env:USERPROFILE ".codex\bin\bridgeforge.exe"
```

从 updater 成功收据读取 `public_artifacts.receipt` 并保存为 `$PUBLIC_ARTIFACTS`；其状态必须为 `prepared`。工具与公共产物验证归用户级事务，普通维护不再重复运行 doctor。缺少有效收据时停止，不切换到项目脚本、Python fallback 或其他 clone。主动诊断或异常定位才读取 [Rust runtime preflight](references/runtime-preflight.md)。

## 3. 用户级与项目级 Hook 边界

普通 update 不检查 Native Memory 同步状态，不执行 reconcile、首次启用或常规 repair。只有目标版本明确要求迁移用户级 Hook 注册时才读取 [Native Memory 状态处理](references/native-memory.md)，按既有授权保留启用/禁用状态；主动 Memory 维护仍是独立任务。

项目自有 Hook 由项目独立开发、构建和注册，普通 update 保留其程序与注册，不检查开发产物是否最新，不扫描完整源码或编译。同步器只按明确迁移声明做有限的一次性识别：仅有旧耦合提示可继续；明确不兼容才阻断。收到首次迁移提示时读取 [项目 Hook 独立迁移](references/project-hook-independence.md)；不得自动实施迁移或把“已提示”记为完成。

## 4. 读取唯一模式手册并生成计划

只读取当前模式对应的一份手册：

| 模式 | 必读手册 |
|---|---|
| `init` | [新项目初始化](references/init.md) |
| `adopt` | [旧骨架受控接入](references/adopt.md) |
| `update` | [当前骨架更新](references/update.md) |

然后只用已锁定的 `$BRIDGEFORGE` 运行 planner：

```powershell
& $BRIDGEFORGE project-sync --project-root . `
  --template-root $BRIDGEFORGE_CODEX_HOME --mode $MODE --output-format combined `
  --public-artifacts $PUBLIC_ARTIFACTS
```

plan 必须零写入。同步器 `machine` 区负责 fingerprint、safe、risk、gap、blocker 与一次性 `PreservationManifest`；`human` 区负责稳定的用户结果。blocker 必须立即停止；无 risk 的 current baseline 更新零确认。存在 risk 时，主对话只能把所有用户决策合并为本轮一次确认，确认前禁止执行 safe 或 risk 动作。

两条升级路径只要 `asset_migration.source_count > 0`，都必须读取 [legacy Rule / Memory 逐文件迁移](references/project-asset-migration.md)，在同一连续流程中逐源文件确认完整迁移包；全部确认前禁止写入或删除，再以 stdin 重新规划。迁移确认本身就是对应旧源删除授权；禁止另问清理确认。

## 5. Apply 与用户结果

准备 Apply 时复用已取得的计划指纹、用户选择和同一份内存迁移 manifest；不在 Skill 中额外跑一遍完整 planner。读取 [事务与回滚](references/transaction.md)，Apply 传 `--plan-fingerprint` 与同一 `$PUBLIC_ARTIFACTS`；执行入口集中复核，漂移时零写停止并重新展示。安装准备是内部步骤，没有单独命令。禁止人工 copy、merge、删除或写版本戳。

维护结果逐项原样展示同步器 `human` 区的“结论、待处理事项、下一步”，只使用“已完成、无需处理、可直接执行、等待确认、未完成、已完成但仍有待处理项”六类状态。safe-only 必须为“可直接执行”，禁止改成“等待确认”、改写结论或倾倒 `machine` 内部字段。用户追问未覆盖的背景时可补充，但不得改变同步器结论。

- 成功：说明当前骨架版本；只把本轮真实收据证明产生的未提交骨架文件计入数量，并把 `$git-sync` 作为需要保存到 GitHub 时的唯一下一步。
- no-op：说明当前版本和“本次骨架操作已结束，无需继续处理”，不展示 planner、validator 或健康 Native Memory 明细。
- risk：用白话说明范围、影响、保留项和不可恢复边界，并且只问一个决策问题。
- blocker/失败：直接展示同步器 `human` 区确定的中文停止原因、写入/回滚状态与安全动作；未分类错误只能显示通用中文停止说明，原始异常只保留在 `machine` 技术收据中。禁止把状态码、blocker 列表、原始异常或 traceback 当结论。
- gap/advisory：只有影响当前结果、需要用户操作或会改变后续行为时才展示；等待真实生命周期事件验证时，只说明仍在等待正常使用完成验证。

Git 同步和显式升版不依赖项目开发检查清单。骨架维护不创建、覆盖或删除已有清单，不报告缺清单会阻断发布；项目可在独立开发验证流程维护该数据。

既有改动禁止归为本轮成果。核对成功条件、Native Memory readiness、回滚或解释技术证据时，读取 [内部技术收据](references/technical-receipts.md)；未追问不补发技术清单。
