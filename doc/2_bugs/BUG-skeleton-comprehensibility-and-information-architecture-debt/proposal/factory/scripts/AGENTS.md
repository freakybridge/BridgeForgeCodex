# scripts 目录指令

本文件只在修改 `scripts/**` 源码和测试时生效。操作者从项目根执行骨架同步时，流程唯一 owner 是本仓库 `skills/bridgeforge-codex/SKILL.md`。

## 实现不变量

- 修改同步器、迁移器或发布检查源码前，必须读取并保持本仓库 `skills/bridgeforge-codex/SKILL.md` 的操作合同；禁止在本文件复制另一套操作算法。

## 验证

- 所有支持 `--check` 或 `--dry-run` 的工厂命令必须零写。
- 同步器本身的零写入口是“不带 `--apply` 的 plan”；相关测试必须覆盖它，以及漂移、失败回滚、项目内容保留和终态收据。不存在的参数不得伪装成已验证入口。
- 自动测试收据不得替代真实下游或运行时冒烟（runtime smoke）证据。
