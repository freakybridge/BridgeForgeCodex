# 项目自有 Rust Hook

项目 Hook 的业务源码归项目所有，骨架只负责锁定构建、生成注册和校验。像工厂提供统一烤箱，项目提供自己的配方：升级烤箱不能把配方变成工厂资产。

## 配置与入口

在 `.codex/hooks/project_demo/entrypoint.rs` 实现 `pub fn run(args: Vec<String>) -> i32`。允许使用受管 workspace 已锁定的依赖（包括 `bridgeforge_core::ProcessRunner`）；不支持项目自带 Cargo.toml、额外依赖、外部 include 文件或任意构建脚本。业务处理直接读取 stdin 并写 stdout/stderr，返回原有退出码。

在项目所有的 `.codex/project-hooks.json` 中登记：

```json
{
  "schema_version": 1,
  "hooks": [{
    "id": "demo",
    "events": [
      {"event": "SessionStart", "args": ["check"], "timeout": 150},
      {"event": "Stop", "args": ["snapshot"], "timeout": 150}
    ]
  }]
}
```

ID 和参数仅允许小写字母开头、其后小写字母/数字/下划线，最长 64 字符；事件为当前骨架支持的 SessionStart、Stop、PreToolUse、PostToolUse、PostCompact，可指定 matcher，timeout 为 1–900 秒。一个程序可服务多个事件，共享内部业务逻辑。`--bridgeforge-self-test` 是骨架保留参数，不调用项目 run 函数；项目 Rust 源码属于受信任可执行代码，不是安全沙箱。

`.codex/hooks.json` 是 Codex 直接读取的原生配置，顶层只使用 `description`、`hooks`；禁止放入 `bridgeforgeProjectHooks` 等骨架专用顶层字段。项目登记与骨架原生命令仍在同一个项目中，但分为两个文件：`project-hooks.json` 给骨架构建器读，`hooks.json` 给 Codex 执行器读。无需写入用户级配置，也不自动授权信任。

## 构建和安装

使用 `$bridgeforge-codex` 的正式 project-sync plan/apply 流程。同步器合并旧的项目配置后，生成带 `bridgeforgeProjectHookId` 的命令注册，构建 `.codex/bin/project_demo.exe`（Windows）或无扩展名程序，并生成对应构建收据。源码、注册、产物、收据均在同一可回滚事务内；旧项目资产仍须逐项确认。已经生成注册的项目可使用 `bridgeforge build-assets --project-root <项目绝对路径>` 重建缺失或变更的产物；注册变化必须走 project-sync。

编译发生在隔离快照中，使用受管 Cargo.lock 和 `cargo build --locked --profile release`，不修改项目受管 Cargo.toml，不在 Hook 事件触发时编译。编译依赖清单必须完全属于已捕获源码；Windows 产物必须采用 GUI subsystem，子进程须使用隐藏窗口的 ProcessRunner。构建失败、输入漂移或无效锁文件必须停止，不回退 Python。

`bridgeforge check baseline --root <项目绝对路径>` 核对注册、源码、锁定 workspace 与二进制收据；暂存区校验同时检查项目注册和入口源码。只有通过构建及校验的事务才最后写骨架版本戳。同名既有产物没有匹配 ownership 收据时必须列为风险，不得静默覆盖。

旧版 `.codex/hooks.json` 顶层登记由同步器在同一风险确认事务中迁到 `.codex/project-hooks.json`，并移除原生配置中的旧字段；保留已有事件、参数和项目源码。两处登记同时存在且内容不一致时停止，不猜测采用哪份。独立登记文件由项目所有，骨架更新不得用模板覆盖；规划、构建、应用之间发生登记漂移必须拒绝写入。legacy 源文件迁移包应同时提供原生 `hooks.json` 与独立登记文件，二者使用 `hook-registration` 类型。

暂存区校验从 Git 暂存区读取两份配置和入口源码，不能用工作区文件补齐未暂存的登记。

本能力不自动执行项目业务验收。真实外部写入、网络副作用或镜像删除仍须由项目在明确授权的测试环境验证。
