# Rust Factory Regression Tests

这个目录放 bridgeforge-codex 自身的 Rust 回归检查。Git 集成测试只在隔离临时仓库中提交和推送，不修改真实项目或访问真实 Memory 远端。

开发收口按[工厂分层验证合同](../../doc/0_architecture/design/development-completion.md)选择必要检查并复用有效证据。下列是完整检查命令，不是每轮开发的默认执行列表。独立发布候选准备及其专用清单已退役；正常 develop 的确定性检查复用保留。工厂测试在根九包 workspace 执行；公共八包投影只做独立构建。

```powershell
cargo test --locked --config scripts/tests/factory-cargo.toml --manifest-path Cargo.toml --workspace -- --test-threads=1
cargo check --locked --manifest-path .codex/hooks/Cargo.toml --workspace
cargo test --locked --manifest-path scripts/tests/Cargo.toml -- --test-threads=1
```

私有单元测试实体位于本目录的 unit 子目录，通过工厂专用配置启用
`bridgeforge_factory_tests`。产品中只保留条件注册，不下发测试实体，也不通过 Cargo feature
自动启用。下游正常构建及 `cargo test --all-features` 不需要本工厂的 scripts/tests；
工厂私有单元测试必须使用带配置的根 workspace 命令，并按影响范围添加 package / test filter；筛选到 0 项不能当作验证通过。

当前覆盖：

- 仓库中不存在产品或测试 `.py` 文件。
- 显式八包公共投影与 factory dogfood 镜像逐字一致，factory 不进入公共分发。
- 受管 manifest 已重建且不再登记 Python 资产。
- pre-commit、共享更新器和 active Skills 只调用 Rust runtime。
- 工厂结构要求保留 Rust 规范检查；proposal 专项检查及批次功能已退役。
- Git 暂存守卫的启动失败、超时、非零退出及敏感文件保护。
- 混合 workspace、独立子包收口、原生来源证明、产品载体与派生锁一致性，预览零写入；旧整 workspace 产品映射保留隔离历史 fixture，旧 CPython 需求卡不再作为活动运行合同。
- project-sync apply/build-assets 全程互斥、Git 主仓库与 worktree 共享锁，以及已识别旧收据的事务退役和回滚。
- SessionStart、Stop、PostToolUse 生命周期及收据落盘失败；会话快照退役后不创建状态目录、保留历史文件、拒绝旧手动入口；Memory 恢复排除未声明文件并在损坏时保留原目录。
- 进程的大输入/双路大输出、非零退出、无人读取 stdin、父进程退出后后代占用管道及 Windows 后代终止；子进程辅助测试仅由回归显式启动。
- 构建收据的实测输入哈希、陈旧合同、原始输入/独立快照/产物漂移和失败零替换；另有真实 Cargo 构建与临时项目安装回归。
- Native Memory 的精确 GitHub 身份、私有性与 Git URL 改写保护；GitHub 查询使用测试替身。
- 产品目录 doctor、Rust 1.88 下限、骨架 8.0.0 最低支持版本拒绝零写入、8.x 定制保护、双戳拒绝、同版本修复及失败回滚。
- Memory 正式配置后运行授权与远端漂移拒绝、三个生命周期事件复用 worker、pending 后续事件、过期队列锁下并发单持有者、Git 凭证私有性校验且不泄露凭证。
- 真实本地 Git 收发、Memory 冲突恢复和单项目 project-sync 事务；测试进程适配器不会下发给下游。

白话：这是工厂的总巡检，专门防止以后有人把退役的 Python 入口或依赖偷偷带回来。
