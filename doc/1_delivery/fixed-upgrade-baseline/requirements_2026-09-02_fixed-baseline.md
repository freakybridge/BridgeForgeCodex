---
lifecycle: active
validation_status: awaiting_user_acceptance
record_type: requirements
confirmed_at: 2026-09-02
source: user -> confirm -> develop
---

# 固定升级基线与旧资产保护

## 原始需求与已确认规则

用户确认修复升级器，并以修复后的首个版本为固定基线；当前产品为 1.8.5，修复首版为
1.8.6。此需求替代旧的“每次最新版本都是重建分界线”约定，不改变资产迁移的逐文件确认。

1. 低于 1.8.6：不读取旧合同、旧 schema 或历史兼容链，盘点项目资产后重建最新骨架。
2. 等于或高于 1.8.6：兼容更新并保留项目定制；固定分界线不随发布自动上移。
3. 两条路径都逐文件展示旧 memory、rules 的迁移或删除方案；全部确认前零写入、零删除。
4. 确认之后，新目标、最新骨架与已确认旧源删除同一事务；源漂移阻断，失败完整回滚。
5. 无戳接入、非法或双戳、比产品新的版本仍按安全边界处理，不允许静默降级。

## 实施前已核实事实

- Rust 同步器按 `previous_version < release_version` 判定重建，分界线目前随发布移动。
- `render_asset` 在重建前调用严格 Markdown 合并；BridgePersonalAssist 1.5.8 缺少新版
  `doc/README.md` 受管章节，因此 planner 尚未给出迁移计划就报错。
- 旧 Rule/Memory 已有逐文件确认 manifest、源 hash 与事务校验；本次需验证其在两条路径都不可绕过。
- 工厂 Git 起始干净；真实下游尚未安装新骨架。

## 非目标与自动化边界

不修改知识库正文、Codex 原生 Memory 同步；不手工修补真实下游、不提交或推送。
真实下游安装必须等待用户另行授权发布，再通过官方产品 home 执行；旧资产的语义决策
仍由用户逐项确认，不用本卡替代。未知项目内容不得自动丢弃。

## 规模与预算

M 级：版本分流、Markdown 结构和资产保护联动，边界已明确。预算 45 分钟、约 20k
新增 token（未实测）、最多 1 个 `review-auditor`、最多 2 轮验证；预计超限先停止确认。
用户于本轮明确确认范围、预算、落盘和开工，不再重复访谈。

## 传播与实施计划

本次为通用产品层改动，进入 `templates/` 与共享 `skills/`；版本升至 1.8.6，写
`[product]` CHANGELOG，同步自身 dogfood 与机械清单。工厂专属事实不下沉。

1. 固定版本分流事实源；使低基线重建不依赖旧文档符合新结构，兼容更新保留项目内容。
2. 同步 Skill、当前设计、版本与镜像；旧确认卡仅作历史证据。
3. 增加版本矩阵、资产确认/漂移/回滚及 Markdown 回归；执行项目发布验证集与独立审计。

## 验收

- 1.8.5 及更早版本直接重建；1.8.6 到模拟后续版本兼容更新；同版本幂等；禁止降级。
- 低基线旧文档缺受管章节或旧表头可安装新受管结构，项目章节、行和布局保留。
- memory、rules 缺确认、部分确认、源漂移均不能写盘；完整确认才原子迁移和删除。
- 新旧版本路径均满足失败回滚、保留项目定制、指纹防漂移、版本戳最后写入。
- 完整自动测试、fixture、manifest、dogfood/mirror、项目结构和 Skill 检查有真实收据。
- 独立审计无未处理高风险问题；真实下游与 runtime 未执行时明确标为未验证。

## 实施与验证记录

- 产品版本已升至 1.8.6，合同新增固定 `compatibility_baseline: 1.8.6`；发布渲染只更新
  release，不自动移动分界线。非法分界线、未来版本与降级仍阻断。
- 低基线重建与基线内兼容更新分别执行；升级时允许补齐受管 Markdown 章节和同列数旧表头，
  保留项目段落、行与布局，歧义或列数变化仍拒绝。
- 旧 Rule/Memory 未全部确认时，apply 在创建项目锁之前拒绝；apply 重新扫描源路径、类型和
  hash，新增、丢失或改动的源均使旧确认失效。完整确认后在同一事务迁移、删除并最后写戳。
- 既有业务 Hook 整包按项目资产保留，不要求既有入口必须是 Rust；新迁移生成的 Hook 仍按
  Rust 入口合同验证。业务 Hook 整包指纹漂移会阻断。
- 已同步共享 Skill、设计、INSTALL、VERSION/CHANGELOG、Cargo 版本、manifest 与 dogfood。
  `$develop` 使用本卡保留需求、实施与验证事实；尚未验收、提交、推送或发布。

### 自动测试收据

1. `cargo test --locked --config scripts/tests/factory-cargo.toml --manifest-path .codex/hooks/Cargo.toml --workspace`
   最终退出 0：CLI 8、core 79、Hook 11 项通过；4 个子进程辅助入口按设计 ignored。
   覆盖固定分界版本矩阵、同版本幂等、降级阻断、双路径逐源确认、零写入、防漂移、回滚、
   Markdown 旧结构、真实模板投影以及既有 Python 业务 Hook 保留。
2. `cargo test --locked --manifest-path scripts/tests/Cargo.toml` 完整执行退出 1：64 passed、
   6 failed、1 ignored。5 项失败来自旧合成合同的基线字段；另 1 项为进程超时用例读取子进程
   PID 文件失败。不得把该次完整执行标为通过。
3. 更正合成合同后，
   `cargo test --locked --manifest-path scripts/tests/Cargo.toml distribution_regressions:: -- --test-threads=1`
   退出 0：12 passed，覆盖上述 5 项失败；旧 Hook 删除、邻近 Hook 保留、后续兼容升级不复活
   旧注册、源回滚断言保留或增强。
4. 未修改进程代码、超时或断言，原样独立执行
   `cargo test --locked --manifest-path scripts/tests/Cargo.toml process_runtime::timeout_kills_descendants_even_after_parent_exits -- --exact --test-threads=1`
   退出 0：1 passed，tree 与 orphan-pipe 两个模式均验证后代结束。并发失败根因尚未确认，
   孤立通过不代表稳定性问题已解决。

首次验证补全了合同精确字段清单；第二轮发现并修正批量维护夹具时误写的重复基线字段。
后续只修正合成版本/floor 的自洽性，并按固定分界更新后续兼容路径断言，没有放宽产品合同、
删除失败断言或再次修改产品实现。已使用两轮实现验证及上述夹具补跑，不继续扩张验证循环；
实际总耗时与 token 未精确计量。

### 构建与工厂硬闸收据

- `cargo build --locked --release --manifest-path templates/hooks/Cargo.toml` 与
  `cargo build --locked --release --manifest-path .codex/hooks/Cargo.toml` 均成功。
- `.codex/hooks/target/release/bridgeforge.exe manifest --root .` 与
  `.codex/hooks/target/release/bridgeforge.exe build-assets --project-root .` 成功；构建收据
  source tree 为 `c82e077a6fb87c7ea13e50cbae6eeff9b69cf50abc83f21a9cac692e53a191b1`。
- `.codex/bin/bridgeforge.exe manifest --root . --check`：`status=ok, changed=false`。
- `.codex/bin/bridgeforge.exe check baseline --root .`：`state=clean, project_version=1.8.6`；
  工厂项目索引按合同跳过，真实模板 Markdown 投影另由单测覆盖。
- `.codex/bin/bridgeforge.exe check factory-version --root .`：版本/合同 1.8.6、CHANGELOG 存在。
- `.codex/bin/bridgeforge.exe check skill-metadata --root .`：无 issues/warnings。
- `.codex/bin/bridgeforge.exe check project-structure --root .`：无 errors；仅既有
  `project-venv-hook-runtime-single-rule` topic 的归档候选提醒。
- `.codex/bin/bridgeforge-hook.exe config-health --strict` 与
  `.codex/bin/bridgeforge-hook.exe instruction-source --pre-commit` 退出 0。
- `cargo fmt --manifest-path scripts/tests/Cargo.toml --check` 和
  `git -c safe.directory=D:/Quant/BridgeForgeCodex diff --check` 退出 0。

### 独立审计与六类证据

同一名 `review-auditor` 复核后未发现未处理的本轮产品代码问题；确认夹具调整没有削弱验收。
审计保留并发进程测试风险，禁止宣称全部发布硬闸通过。

| 证据类别 | 本轮状态 |
|---|---|
| 源码 | 固定基线、合并与事务保护已实现，独立代码审计完成 |
| 产品传播 | Template、共享 Skill 与清单完成；远端发布和用户级安装未执行 |
| dogfood | 镜像、受管二进制、baseline 与工厂版本检查通过 |
| fixture | 核心通过；后续完整串行集成 70 passed、0 failed、1 ignored，见下节 |
| 真实下游 | 未写入 BridgePersonalAssist，实际升级与逐资产确认未执行 |
| runtime | 临时项目构建/apply、隔离 Git/pre-commit 通过；真实下游 Hook 运行未验证 |

## 风险与后续交接

兼容路径仍必须拒绝歧义和未知所有权，不能用“兼容”作为覆盖项目资产的授权。
发布后继续 BridgePersonalAssist 的只读计划与逐文件迁移确认，不能从模拟测试推断真实安装成功。
完整串行集成收据补齐后进入 `awaiting_user_acceptance`；此前并发调度下的进程用例失败根因
未确定，本次以串行执行隔离争用，没有改动产品代码、超时或断言。所有真实 memory/rules
的迁移或删除继续逐项询问用户。

## 后续同步授权与完整验证（2026-09-02）

用户明确调用 `$git-sync`，随后要求开始 BridgePersonalAssist 升级；此次授权覆盖工厂提交、
推送和官方产品刷新，不替代真实下游的逐资产确认。

- 提交前原样运行 `cargo test --locked --manifest-path scripts/tests/Cargo.toml -- --test-threads=1`：
  退出 0，70 passed、0 failed、1 ignored，489.14 秒；包括曾失败的进程后代超时、真实临时
  项目构建/apply 和隔离工厂 Git/pre-commit 全链路。
- 再次执行 baseline、factory-version、skill-metadata、project-structure 和 manifest --check，
  均退出 0；保留既有归档候选提醒，不影响本次升级。
- repo-local git-sync 会按现有自动升版规则将发布号从 1.8.6 升到 1.8.7；修复实现首版的
  固定兼容分界仍是 1.8.6，不随发布号上移。最终提交和推送结果以同步器真实收据为准。

## 真实下游触发的文件类型误判修复（2026-09-02）

1.8.7 已由 `6da99ae` 发布，官方 updater 取得同一提交；真实 BridgePersonalAssist 1.5.8
只读计划因 `.codex/hooks/project_structure_check.py` 被当作目录而阻断。该路径实测为 9009
字节普通文件、无 reparse 属性。同期盘点到 33 份旧 Memory，未作任何迁移或删除。

用户在说明修复并发布新补丁后明确要求“那你开始吧”。本次为通用产品层小修复，进入
Template 与 dogfood，补丁版本由本次 repo-local git-sync 自动升级；固定基线仍为 1.8.6。
不修改真实下游旧脚本，不绕过其逐资产确认，不改原生 Memory 或业务知识库。

- 普通 `project_*` 文件交给后续精确文件清单，不因名字要求其必须为目录。
- 非普通对象和链接仍阻断；真实业务 Hook 目录继续按整包确认。
- 独立审计指出已注册普通脚本删除时需同步清理注册；现已复用既有完整路径边界匹配，
  只在明确删除且已有 hooks.json 时清理对应注册，文件和注册同一事务，保留相似文件名。
- 新增普通 `.py`、`.rs`、无后缀文件的测试，修前真实复现三个目录误判；修后覆盖缺确认零
  写入、原样保留、源漂移、明确删除、回滚和业务 Hook 保留。
- 新增真实模板 Hook 合同测试，覆盖 `command`/`commandWindows` 注册精确退役与回滚、
  `.py.bak` 相似名保留；新增 Windows junction 测试，确认链接仍在写盘前被阻断。
- `cargo test --locked --config scripts/tests/factory-cargo.toml --manifest-path .codex/hooks/Cargo.toml -p bridgeforge-core project_prefixed_ -- --test-threads=1`
  最终退出 0：3 passed。junction 夹具首次因 mklink 不接受混合斜杠失败，规范化参数后原断言
  通过，未放宽链接保护。
- 同一 `review-auditor` 已确认注册遗漏修复，最终代码审计通过，无未处理发现。
- 完整 workspace 命令
  `cargo test --locked --config scripts/tests/factory-cargo.toml --manifest-path .codex/hooks/Cargo.toml --workspace -- --test-threads=1`
  退出 0：CLI 8、core 82、Hook 11 项通过，4 个子进程辅助入口 ignored。
- 从安装目录执行 build-assets 曾遇到访问拒绝并完整回滚；改从独立构建目录执行
  `.codex/hooks/target/release/bridgeforge.exe build-assets --project-root .` 后成功。
  source tree 收据为 `89563c3956b9a4d944d8a7dcf56927117f3b54346e97b4ddfc0c5320671fceef`。
- 首次完整集成为 69 passed、1 failed、1 ignored，因依赖工厂二进制的用例在产物更新前
  执行，报 `planned target is missing from transaction: .codex/bin/bridgeforge-hook.exe`。
  产物对齐后，该用例原样单独执行通过；随后不再并行更新测试输入，重新完整执行
  `cargo test --locked --manifest-path scripts/tests/Cargo.toml -- --test-threads=1`，退出 0：
  70 passed、0 failed、1 ignored，538.75 秒。未改失败断言或加入产品绕过逻辑。
- `check baseline --root .` 为 clean；skill-metadata 无 issues/warnings；project-structure
  无 errors，保留既有归档候选提醒；manifest --check 无漂移；config-health --strict、
  instruction-source --pre-commit、cargo fmt --all -- --check 与 git diff --check 均退出 0。
- 六类证据：源码、Template 传播、dogfood、fixture 与独立审计已完成；真实下游仍仅有
  1.8.7 失败计划，修复版 planner 和安装结果待官方刷新后核验；真实下游 runtime 未验证。
  最终发布、真实下游 planner 与安装结果分别以各自收据为准，不据测试宣称旧资产已迁移。
