---
lifecycle: active
validation_status: in_progress
record_type: requirements
confirmed_at: 2026-09-01
source: $confirm -> $develop
---

# 旧版 Markdown 受管结构升级需求

## 原始需求与目标

BridgePersonalAssist 从旧版骨架升级时，先后因 `doc/README.md` 的旧受管表头和缺失受管
标题而硬停。修复升级器，使合法旧版本在 latest current-only rebuild 中既能升级列数不变的
受管表头，也能按当前模板顺序补齐缺失的受管标题，同时更新受管行并保留项目自有内容；发布
新版后继续完成 BridgePersonalAssist 骨架安装。

## 不做

- 不加载历史合同、旧 schema 或逐版本兼容分支。
- 不手工修改 BridgePersonalAssist 绕过事务。
- 不放宽无戳 `adopt`、同版本漂移、重复表格或列数变化的硬闸。
- 不触碰 BridgePersonalAssist 的 project-owned 内容或其他业务文件。
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

- BridgePersonalAssist 当前使用合法单戳 1.5.8，项目 `.venv` 为 CPython 3.12.13。
- 正式产品 home 已刷新到 1.7.2；目标 planner 在 `## 文档生命周期` 缺失处零写阻断。
- 目标 `doc/README.md` 同时缺少 `## 从这里开始` 与 `## 文档生命周期`，但保留项目自有索引内容。
- 目标项目 Git 工作区干净，阻断后仍为 1.5.8，尚未发生本轮骨架写入。
- Native Memory 已授权并安装 Hook，当前仍在等待下一次正常生命周期完成 runtime 验证；该等待不阻断骨架安装。
- 旧表头升级修复已随 1.7.2 发布；缺失标题补齐的 4 项针对性边界测试已通过。

## 已确认规则

1. 只有旧版本 rebuild 可把列数不变的受管表头替换为当前 canonical 表头。
2. 只有 obsolete stamp，或合法版本低于产品版本时，才允许按 canonical 顺序插入缺失的普通受管标题和受管表格标题。
3. 受管行按当前合同更新，项目自有行与项目自有章节逐字保留。
4. 无戳 `adopt`、同版本缺失或漂移、列数变化及重复/歧义表格继续 fail-closed。
5. 修复属于通用产品能力，必须 bump patch 版本并写 `[product]` CHANGELOG。
6. 发布后只通过正式产品 home 重新规划和安装 BridgePersonalAssist。

## 拟修改

- `scripts/bridgeforge_codex_project_sync.py`
- `scripts/tests/test_current_baseline_project_sync.py`
- `VERSION`、`CHANGELOG.md` 与机械生成的分发/受管清单
- 本需求包与 `doc/README.md`

## 验收

- 针对性测试证明旧表头升级、缺失标题补齐、项目内容保留、无戳/同版本拒绝、歧义拒绝和幂等。
- 发布硬闸、完整测试、fixture、manifest、dogfood/mirror 检查通过。
- `review-auditor` 独立读取需求卡、真实 diff 和测试收据后判定可发布。
- 新版提交并推送，用户级产品 home 更新到该正式版本。
- BridgePersonalAssist 重新规划不再被表头阻塞；风险/迁移仍按合同确认。
- 应用后版本戳为最新，no-op replan 通过，原有用户改动保持不变。

## 实施与验证记录

### 1.7.2 表头补丁历史收据

- 已将许可收窄为“存在 obsolete stamp，或合法版本低于当前版本”；无戳 `adopt` 不继承该权限。
- 7 项边界测试通过：旧版升级与项目行保留、无戳 adopt 拒绝、同版本漂移拒绝、列数变化拒绝、重复/歧义拒绝、幂等和 migration composition。
- 完整自动测试 402 项通过、1 项既有跳过；downstream fixture 5/5 通过。
- factory dogfood、instruction source、hook config、encoding、project structure、skill metadata、factory version、manifest 与 `git diff --check` 均通过。
- `review-auditor` 首审发现 adopt 权限越界；修复后复审确认 P1/P2 均关闭，可以发布 1.7.2。
- 1.7.2 已正式发布并刷新产品 home；真实目标随后暴露缺失 `## 从这里开始` 与 `## 文档生命周期` 的后续兼容缺口。

### 当前缺失标题补丁收据

- 已实现仅限 obsolete stamp 或合法低版本的缺失标题补齐；无戳 adopt 与同版本缺失继续零写阻断。
- 5 项针对性测试与核心代码体积闸通过，包含 current stamp、obsolete stamp、无戳 adopt、同版本和 migration composition 边界。
- 第二轮完整自动测试 406 项通过、1 项既有跳过；downstream fixture 5/5 通过。
- factory dogfood、instruction source、hook config、encoding、project structure、skill metadata、factory version、manifest 与 `git diff --check` 均通过。
- `review-auditor` 首审未发现 P0/P1，提出 obsolete stamp 直接测试与交付文档收据分层两项 P2；修正后复审确认两项均关闭，当前无 P0/P1/P2，可以发布。
- 后续补丁的正式发布、产品 home 刷新及 BridgePersonalAssist 安装仍待完成。
