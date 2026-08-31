---
lifecycle: active
validation_status: in_progress
record_type: requirements
confirmed_at: 2026-09-01
source: $confirm -> $develop
---

# 旧版 Markdown 表头升级需求

## 原始需求与目标

BridgePersonalAssist 从 bridgeforge-codex 1.5.7 升级时，1.7.1 planner 因
`doc/README.md` 的旧受管表头与当前表头不同而硬停。修复升级器，使合法旧版本在 latest
current-only rebuild 中能升级列数不变的受管表头、更新受管行并保留项目自有行；发布新版后
继续完成 BridgePersonalAssist 骨架安装。

## 不做

- 不加载历史合同、旧 schema 或逐版本兼容分支。
- 不手工修改 BridgePersonalAssist 绕过事务。
- 不放宽同版本漂移、重复表格或列数变化的硬闸。
- 不触碰 BridgePersonalAssist 当前 `vault-mirror/` 改动。
- 不绕过 legacy Rule / Memory 的逐源确认。

## 规模与预算

| 项目 | 内容 |
|---|---|
| 规模 | M |
| 依据 | 修复本身单一明确，但产品发布必须完成完整验证与独立审计 |
| 时间 | 45 分钟 |
| Token | 约 20k，平台无可靠计量器，未实测 |
| Agent | 最多 1 个 `review-auditor` |
| 验证 | 最多 2 轮完整验证；用户已额外批准 1 轮针对性测试修正 |
| 超预算停止点 | 范围、agent 或验证轮次预计越界时停止并重新确认 |

## 已核实事实

- BridgePersonalAssist 使用合法单戳 1.5.7，项目 `.venv` 为 CPython 3.11.9。
- Native Memory 已启用，Hook 已安装且 runtime verified，远端已配置，状态 healthy。
- 1.7.1 planner 在 `## 3_reference/` 报告 `managed Markdown table header drifted`。
- 旧表头为 `文件 / 子目录 | 说明`，当前表头为 `文件 | 说明`，列数相同。
- 目标项目尚未发生骨架写入；其现有 `vault-mirror/` 修改保持原样。
- 本地修复已通过表头升级、歧义拒绝和同版本幂等 3 项针对性测试。

## 已确认规则

1. 只有旧版本 rebuild 可把列数不变的受管表头替换为当前 canonical 表头。
2. 受管行按当前合同更新，项目自有行逐字保留。
3. 列数变化、重复/歧义表格和同版本 baseline 漂移继续 fail-closed。
4. 修复属于通用产品能力，必须 bump patch 版本并写 `[product]` CHANGELOG。
5. 发布后只通过正式产品 home 重新规划和安装 BridgePersonalAssist。

## 拟修改

- `scripts/bridgeforge_codex_project_sync.py`
- `scripts/tests/test_current_baseline_project_sync.py`
- `VERSION`、`CHANGELOG.md` 与机械生成的分发/受管清单
- 本需求包与 `doc/README.md`

## 验收

- 针对性测试证明旧表头升级、项目行保留、歧义拒绝和同版本幂等。
- 发布硬闸、完整测试、fixture、manifest、dogfood/mirror 检查通过。
- `review-auditor` 独立读取需求卡、真实 diff 和测试收据后判定可发布。
- 新版提交并推送，用户级产品 home 更新到该正式版本。
- BridgePersonalAssist 重新规划不再被表头阻塞；风险/迁移仍按合同确认。
- 应用后版本戳为最新，no-op replan 通过，原有用户改动保持不变。

## 实施与验证记录

- 已将许可收窄为“存在 obsolete stamp，或合法版本低于当前版本”；无戳 `adopt` 不继承该权限。
- 7 项边界测试通过：旧版升级与项目行保留、无戳 adopt 拒绝、同版本漂移拒绝、列数变化拒绝、重复/歧义拒绝、幂等和 migration composition。
- 完整自动测试 402 项通过、1 项既有跳过；downstream fixture 5/5 通过。
- factory dogfood、instruction source、hook config、encoding、project structure、skill metadata、factory version、manifest 与 `git diff --check` 均通过。
- `review-auditor` 首审发现 adopt 权限越界；修复后复审确认 P1/P2 均关闭，可以发布 1.7.2。
- 正式发布、产品 home 刷新、BridgePersonalAssist 安装与用户试用仍待完成。
