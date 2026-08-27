# AGENTS 信息架构优化提案（第十一版）

> 状态：迭代已由用户中断；第十一版未通过，最后一项精确命令合同修补尚未复跑完整验证。本文和同目录文件不参与当前运行时，尚未替换真实 Template、Hook、Skill、README、manifest 或同步器。

## 结论

第十一版不再把“变短”当成唯一目标，而是先解决一句规则到底由谁负责：

| 载体 | 唯一职责 |
|---|---|
| 根 `AGENTS.md` | 跨目录、高损害、无法完全机判的结果红线 |
| `$bridgeforge-codex` 等 Skill | 用户从项目根发起的一次完整操作流程 |
| 嵌套 `AGENTS.md` | 修改该目录内容时必须保持的局部不变量 |
| 根 README 公共区 | 面向人的位置说明、操作入口和概念解释 |
| Hook / manifest / 测试 | 标题、hash、区域所有权和零写等可机判合同 |

第十一版补齐第十版复评发现的缺口：恢复公共区禁止直接修改、禁止隐式写入用户级配置和换机触发强度；补齐工厂快速命令与 Template 目录索引的来源覆盖；Markdown 扫描器先识别 fence / 缩进代码，再处理 HTML 注释，既拦隐藏规则，也不误伤代码块中的字面量 `<!--`。非 README region 继续使用原始字节区间拼接，保证 marker 外内容不变。

## 候选文件

| 文件 | 实施目标 |
|---|---|
| [`factory/AGENTS.md`](factory/AGENTS.md) | 工厂根 `AGENTS.md` |
| [`template/AGENTS.md`](template/AGENTS.md) | `templates/AGENTS.md` |
| [`factory/scripts/AGENTS.md`](factory/scripts/AGENTS.md) | 工厂 `scripts/AGENTS.md` |
| [`factory/skills/AGENTS.md`](factory/skills/AGENTS.md) | 工厂 `skills/AGENTS.md` |
| [`factory/doc/2_bugs/AGENTS.md`](factory/doc/2_bugs/AGENTS.md) | 工厂 `doc/2_bugs/AGENTS.md` |
| [`readme/bridgeforge-public-section.md`](readme/bridgeforge-public-section.md) | 工厂与下游根 README 的同一受管区 |
| [`shared-docs/codex-hook-signals.md`](shared-docs/codex-hook-signals.md) | `doc/3_reference/codex-hook-signals.md` |
| [`architecture/codex-native-instruction-architecture.md`](architecture/codex-native-instruction-architecture.md) | `doc/0_architecture/design/` 的正式设计 |
| [`semantic-migration-matrix.md`](semantic-migration-matrix.md) | 逐条语义迁移账本 |
| [`contracts/instruction-contract.json`](contracts/instruction-contract.json) | 标题、区域、资产和旧指针机器合同 |
| [`contracts/semantic-contract.json`](contracts/semantic-contract.json) | 来源行、目标文件和必需语义的可执行合同 |
| [`contracts/bridgeforge-codex-skill-patch.json`](contracts/bridgeforge-codex-skill-patch.json) | 完整 `$bridgeforge-codex` 候选的确定性 patch 与预期 hash |
| [`contracts/region_migration.py`](contracts/region_migration.py) | README 专属追加、其他 region 失败关闭的实现候选 |
| [`contracts/implementation-patch.md`](contracts/implementation-patch.md) | 真实实施时的精确改动清单 |
| [`contracts/validate_proposal.py`](contracts/validate_proposal.py) | proposal 静态与临时覆盖验证器 |

README 公共区使用 `BRIDGEFORGE:README:BEGIN/END` 标记。工厂和下游共用同一内容，标记外始终由各项目所有。

## 精简结果

| 指标 | 当前 | 第十版 | 第十一版 |
|---|---:|---:|---:|
| 工厂根总行数 | 188 | 135 | 135 |
| 工厂根项目符号 | 72 | 66 | 66 |
| Template 总行数 | 165 | 125 | 125 |
| Template 项目符号 | 50 | 46 | 46 |

第十一版没有增加根 `AGENTS.md` 行数，仍受 Template 125 行、工厂根 135 行的自动门约束。它不把删除语义算作有效精简。

更关键的精简来自去重：计划/执行、保留差异（gap）、聚合指纹（fingerprint）、回滚和版本戳顺序只由 Skill 负责操作流程；README 只解释入口；`scripts/AGENTS.md` 只要求实现与测试保持 Skill 合同。行数不再是唯一判据。

## 人类友好性处理

- 第一次出现 dogfood、gap、manifest、fixture、runtime smoke、asset id、target、hash、ownership strategy 等词时先给中文，再保留英文检索词。
- 把“有效流程”“独立升级研判”“项目执行上下文”等模糊说法换成真实 Skill 名或“项目级专区”。
- 根文件先给位置地图，再按交付、证据、环境、排障和稳定边界组织；README 用“怎么放 / 怎么用 / 为什么不同”说明操作。
- 长流程和背景不再塞回根文件，但每条被移出的语义都在迁移账本中记录唯一 owner、加载时点和验证门。

## 验证口径

运行：

```powershell
.venv\Scripts\python.exe -B doc/2_bugs/BUG-skeleton-comprehensibility-and-information-architecture-debt/proposal/contracts/validate_proposal.py
```

验证器从候选路径加载完整生成的 schema 3 parser 与同步器，再执行 payload、plan/apply/no-op、结构化阻断和故障注入；其中 marker blocker 还通过真实子进程 CLI 核验退出码、唯一 JSON stdout、与 JSON error 严格一致的 `BLOCKED:` stderr 提示及零写。README 无 marker 可逐字追加；非 README region 使用 CRLF、LF、混合换行和无尾换行样例验证 marker 外 prefix/suffix 原始字节不变，二次执行 byte-identical。plan、阻断、no-op 和回滚比较“受管可见树”，明确排除 `.git`、`.venv` 与 `__pycache__`，不宣称覆盖这些边界。完整候选 Skill 通过 root-skill 测试和正式分发 manifest 检查。工厂 instruction Hook 对根项目区、Template 和三个嵌套指令执行工作树与 staged 的规范化全文 hash 破坏矩阵。语义合同锁定来源 hash，按章节检查可执行正文；HTML 注释、反引号 / 波浪线 fence 和缩进代码不能满足规则，未闭合注释或 fence 失败关闭。Template 项目占位注释另按“可见标题后紧邻指定注释”精确验证。

当前只实跑 manifest `--check` 和同步器“不带 `--apply` 的 plan”；“所有支持 `--check` / `--dry-run` 的工厂命令必须零写”已迁移为规则与测试义务，但全命令行为矩阵要在真实实施后验证。旧指针检查也只针对合同列出的迁移项和枚举的活跃根，不宣称扫描了全部历史材料。

## 仍未验证

以下内容必须留到用户批准真实实施后验证：Codex 对三个新嵌套文件的实际加载、真实下游仓库、正式 release suite、runtime smoke 和用户本人试读。中断前的上一份完整候选曾通过 proposal 自动验证，但最后一项合同修补尚未复跑；不能称“骨架问题已经解决”。

## 中断进度

- 已落盘 11 个 debate 版本；V1 至 V10 已形成结论，V11 独立复评未完成。
- V11 复评发现快速命令合同允许尾缀污染；候选验证器已改为逐行精确、唯一、顺序一致，但中断前未复跑完整验证。
- 当前没有最终通过版，也没有第十二版 debate。
- 已确认“无限 debate 直到不存在任何新反例”不是有限可完成的验收命题；本目录保留当前候选与失败证据，等待未来先定义有限验收边界再继续。
