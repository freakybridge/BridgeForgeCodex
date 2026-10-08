# BridgeForge 工厂开发收口

本清单属于工厂自身，不下沉为所有项目的通用要求。通用 develop 调用项目清单，不复制工厂特例。

## 收口条件

开发完成前必须通过 factory dogfood、skill metadata、manifest `--check`、project structure、mirror drift、完整 fixture、完整自动测试与独立审计。机器检查唯一配置为 `.codex/development-checks.json`；完整 fixture 包含镜像检查。条件忽略项如实列明，不冒充已执行。

先证明模板与 dogfood 源码全树逐字一致，再对 dogfood 执行完整 workspace 测试；同一份测试代码不在两个镜像上重复执行。完整工厂 fixture 仍单独执行。

1. 完成实现、必要修复和当前版本受管构建，保证工作区 baseline 健康；不提前修改 VERSION。按 summary 流程完成本轮经验文档和索引，再进入最终审计与准备。
2. 核对新 CLI 自检能力包含 `git-sync-prepared-release-v1`。运行 `git-sync --release-status` 读取当前输入指纹，先做独立 review-auditor 复核，修复后重新核对指纹。
3. 将实际审计结论保存到忽略目录中的 JSON：`schema: 1`、`fingerprint`、`reviewer`、`verdict: passed`、`summary`。这是对真实审计输出的记录，工具不会代替独立审计，也不能伪造结论。
4. 执行受管 `git-sync --prepare-release --message-file <消息文件> --audit-file <审计记录>`。它运行配置中的检查、核验输入未变、在隔离快照构建目标版本并生成准备记录。检查与目标构建不会提交、推送或提前改写工作区版本。
5. 准备成功后才交付用户验收。收据位于 `.runtime/bridgeforge-codex/release-preparation/current.json`；检查日志与产物按内容哈希保存。质量失败时记录不能是 prepared。

## 验收与发布

- summary 将经验、交接、验收和 TODO 写入 doc。本项目在 development-checks.json 的 record_documents 显式声明记录类 Markdown：其变化只做 UTF-8/结构检查，复用开发检查与目标产物；本次完整文件清单仍绑定事务和提交树。声明不覆盖源码、Skill、AGENTS 或配置；清单本身变化也会使原准备失效。文档轻量检查不代表语义审计，summary 不自动补跑长检查。
- `git-sync release` 只快速核验记录，重新计算目标版本并使用准备产物更新版本、CHANGELOG、原生 manifest 后同步。提交说明可变，但目标版本必须仍一致。
- 缺失、损坏、跨仓库、分支变化、源码/依赖/检查配置或未声明文档变化时停止并指向 develop；发布阶段不自动补跑长检查、编译或审计。
- 若提交 Hook 改写已验证的暂存内容，保留本地提交并阻断推送；持久 guard 防止下一次普通同步绕过。必须先人工复核，禁止自动 reset 或删除现场。
- 普通 git-sync 保留现有同步与构建有效性要求，不因发布准备失败而回退为自动发布。开发验收与发布是不同授权。
- 准备记录是本机派生产物，不进入 Git。跨电脑、缺失记录时回到开发准备，不声称不存在的验证已完成。
- 性能以实际用户指令到最终回复的时间衡量；准备耗时单独披露，不能把内部 CLI 时间当端到端承诺。
