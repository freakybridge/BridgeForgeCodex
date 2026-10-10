---
title: R01 受管 Markdown 基础解析收拢
lifecycle: completed
validation_status: verified
date: 2026-10-10
---

# R01 受管 Markdown 基础解析收拢

## 需求与授权

用户在 refactor-scan 中选择解释 R01，要求生成计划后明确回复“开始吧”。本次授权为 R01 本地开发、必要传播、验证与文档；不提交、推送、发布或写入真实下游。

基准为 `1978b367de7376b34e644ea84d6985dea5d12f58`，开工时工作区干净。

## 目标与边界

- 将标题定位、表格范围和行标识解析集中到 core 内部模块。
- project_sync 保留合并、升级、插入与事务；baseline 保留投影、哈希和漂移判定。
- 保持 CLI、合同 schema、输出字节、投影值、错误文本和副作用顺序。
- 显式保留两侧现有差异：标题层级边界、链接识别、大小写、换行归一化和多表处理。
- 不包含 R02–R04，不扩展 Markdown 语法，不修正发现的既有行为；无法等价时暂停受影响动作并说明。

## 规模与预算

M：两个解析消费者与合同生成链路；主对话实现，最多 1 名 review-auditor。累计实施上限 45 分钟 / 约 20k 新增 token（估算，未实测）/ 最多 2 轮验证；包含构建、强制检查、审计及合理重测缓冲。预计超额时停止新增工作并报告现场。

本机无适用周额度系数，不编造百分比；实施授权已明确，不以缺少换算数据重新索取开工授权。

## 实施计划

1. P0：先在旧实现上运行行为样本，固定结果与失败语义。
2. P1：增加 managed_markdown 内部模块，抽取共享解析并将差异显式命名。
3. P1：迁移 project_sync / baseline，保留薄适配及业务判定，移除重复有效实现。
4. P1：同步 dogfood、受管 manifest 与当前版本构建产物。
5. P2：按工厂开发清单执行强制检查、独立审计及受管准备；补充证据与经验。

## 传播四问

产品层通用能力，源码进入 templates/hooks；不是工厂专属配置或单纯元文档。同步自身 dogfood，禁止对工厂执行下游 project-sync apply。将来显式 release 按 [product] 升版与记录 CHANGELOG；本次不预写 VERSION。

## 验收

- 项目实表与注释示例表共存，受管表更新而项目表逐字保留。
- 重复执行幂等；缺失/重复标题、重复行、歧义与列数错误保持原行为。
- 标题层级、大小写、链接、LF/CRLF/CR、无尾换行保持现有两侧策略。
- 更新→投影→校验链路通过，篡改受管内容仍阻断。
- 同输入投影/哈希不变；新源码导致的构建来源哈希变化为正常传播。
- 先确认模板与 dogfood 全树一致，再执行 dogfood workspace 与完整工厂 fixture；其余检查以 .codex/development-checks.json 为唯一机器入口。

## 验证与交付记录

1. 旧实现行为基线：`cargo test --locked --config scripts/tests/factory-cargo.toml --manifest-path .codex/hooks/Cargo.toml -p bridgeforge-core markdown_characterization -- --test-threads=1`，5/5 通过。普通沙箱无法写 Cargo 锁，窄范围提权后通过。
2. 新实现定向验证：同一受管入口过滤 `markdown`，9/9 通过，含旧实现固定样本与真实模板用例。
3. 当前版本首次 build-assets 安装因 `os error 5` 报告事务回滚；随后按 `cargo build --locked --release --manifest-path .codex/hooks/Cargo.toml` 构建，从 `.codex/hooks/target/release/bridgeforge.exe build-assets --project-root .` 启动受管安装成功，Hook/CLI 自测及来源收据均完成。最初原因未单独定位到具体文件，换入口成功不作为根因定论。
4. 独立审计 `review-auditor:r01_audit` 已通过：未发现解析等价性、错误语义或职责边界回归。索引位置建议已修正，补充 Unicode/围栏断言并通过静态复核；新增断言已随完整工程检查通过。
5. 受管 `git-sync --prepare-release --message-file .runtime/R01-message.txt --audit-file .runtime/R01-audit.json` 返回 `prepared`，7 项检查全部通过。隔离目标版本 2.2.1，仅保存准备产物；工作区 VERSION 仍为 2.2.0，未提交、推送或发布。真实下游与 runtime 用户路径未验证。
6. `check factory-version --root .` 健康，工作区版本仍为 2.2.0；前后合同中 `codex.doc.readme` 的受管投影哈希一致，`git diff --check` 通过。开发指纹为 `sha256:14c1165fd734bc1057cadf2ae88424254056559e94c2e68ebcdf697504e03bb5`，独立审计记录已绑定该指纹。

### 最终工程证据

机器入口为 `.codex/development-checks.json`，收据位于本机 `.runtime/bridgeforge-codex/release-preparation/current.json`；日志是同目录 blobs 中按 SHA-256 保存的内容，不属于跨机器可复用的 runtime 验收。

| 检查 | 本轮结果 |
|---|---|
| baseline、skill metadata、manifest --check、project structure | 全部通过；结构仅有既有归档候选提示 |
| 模板与 dogfood 源码全树一致 | 1 项通过 |
| dogfood workspace | CLI 14、Core 214、Hook 22，共 250 项通过；5 项条件忽略；约 650 秒 |
| 完整工厂 fixture | 89 项通过，5 项条件忽略；约 400 秒 |
| 独立审计 | passed；建议已关闭 |
| 受管准备与隔离构建 | prepared，checks_passed=7，目标 2.2.1；不改写工作区版本 |

workspace 日志：`sha256:13f42a0d469595be4b59806aa4ada158c3045be18d14164541521eade65e2f27`。5 项忽略包括 4 个由父测试调用的子进程 helper/fixture，以及需显式输入的 live log 检查。

factory 日志：`sha256:b118f408c25e42d98b5dd37a7c0cacc5b476dfede5925b574eb0d8f56d262639`。5 项忽略为两项显式性能实验、需原生 Codex 入口的高成本提醒 smoke、进程 helper 和需单独授权来源的 Assist 迁移 fixture；不将这些条件项计为本轮通过。

本轮保持在 45 分钟实施上限内，使用 1 名独立审计 Agent，未发生源码实质修复失败；token 未实测。工程验证完成，用户已于 2026-10-11 通过 `$summary accept` 验收。

## 经验记录

两份解析代码的相似性不能证明语义完全相同。先在旧实现运行同一批固定样本，再移动基础能力并给差异明确命名，可避免将结构重构夹带成规则修复。合并需要原文偏移与字节保留，投影需要归一化与稳定哈希；这两项契约分别保留。注释中的示例表是已登记的受管内容，不能在抽取过程中按一般 Markdown 直觉将其过滤。

本轮不新增 AGENTS / Rule / Hook 治理约束，不修改原生 Memory。

## 用户验收

2026-10-11，用户明确调用 `$summary accept`，确认本会话 R01–R04 开发符合需求。R01 的旧行为样本、新实现回归、独立审计及累计工程检查满足约定，状态更新为 completed / verified。真实下游及 runtime 用户路径未验证的边界保留。累计验收与发布授权见 [R04 收口记录](../project-map-refresh/requirements_2026-10-11_R04.md#累计验收与发布授权)。
