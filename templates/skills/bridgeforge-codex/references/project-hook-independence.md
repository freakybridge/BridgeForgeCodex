# 项目 Hook 独立迁移

仅在同步器报告一次性兼容迁移、或用户明确要求迁移/诊断项目 Hook 时读取。普通 update 不因此扫描全部源码、编译、补依赖或改业务逻辑。

## 提示与决定

- 迁移声明来自产品中的 `project-hook-policy.json`；旧耦合本身只提示，既有程序与注册保留。只有明确不兼容才阻断相关更新。
- 待迁移状态跨骨架升版保留；提示去重不是完成证明。只读预览不写去重状态，主动状态查看可使用 `bridgeforge project-hook-migration status --root <项目>`。
- 用户明确授权该项目迁移后，先读取项目源码、注册和现有行为，形成具体迁移范围，再在项目自身开发流程实施。工厂维护授权不自动覆盖真实下游。

## 独立工程与完成证据

1. 每个项目 Hook 自己维护 Cargo.toml、Cargo.lock、main 入口和依赖；不引用 `bridgeforge_core` 或骨架 workspace。进程管理等原能力由项目维护，保留 stdin/stdout/stderr、退出码、超时与 Windows 隐藏启动语义。
2. 使用项目自己的锁定 Cargo 构建入口，测试原业务行为，由项目安装可执行文件并维护自己的 native Hook 注册。骨架只保留注册，不为项目生成运行入口。
3. 项目 Hook 源码继续位于 `.codex/hooks/project_<id>/`，编译产物位于 `.build/bridgeforge/bin/`。旧 `.codex/bin/` 中具有登记、精确收据及匹配字节的受管 Hook 由布局迁移事务搬迁并更新对应注册，保持 Hook 身份。自定义或无法证明归属的旧入口保留并阻断相关迁移，不猜测路径或自动重写。
4. 在行为已明确验证并获准记录完成后，运行 `bridgeforge project-hook-migration verify --root <项目> --confirmed-behavior`。它核对独立 workspace/依赖，执行锁定构建并比对项目已安装程序，记录完成证据；不是只读命令，也不替用户执行业务事件测试。缺自己的 manifest/lock/main 或安装产物不匹配时停止，不回退到骨架隐式构建。

独立验证依赖项目已构建过所需依赖；离线 metadata 无法确认时先在项目自己的开发环境解决。构建脚本和项目代码属于受信任可执行输入，不将 Cargo 构建称为安全沙箱。迁移失败不能标成完成，也不能通过伪造新收据绕过旧构建绑定。
