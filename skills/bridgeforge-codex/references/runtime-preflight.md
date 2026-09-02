# Rust runtime preflight

仅当根入口已经判定 `$MODE`，但受管 CLI 或产品 workspace 验证失败时读取。

- 双戳、非法戳必须阻断；无戳空项目允许 init，无戳已有资产允许 adopt，不要求下游已经安装 Rust 骨架。
- 只检查刷新后的产品 home：`templates/hooks/Cargo.toml`、`Cargo.lock` 和 updater 安装的用户级二进制。
- Cargo 与 rustc 必须满足 workspace 声明的最低版本；产品版本与 CLI 必须相同。缺失、版本不足、锁文件校验失败或 self-test 失败都必须停止。
- 禁止使用 Python、旧脚本、其他 clone 或 PATH 中同名非受管二进制兜底。

只读验证命令：

```powershell
& $BRIDGEFORGE doctor --product-root $BRIDGEFORGE_CODEX_HOME --json
```

doctor 检查工具链版本、`cargo metadata --locked --offline --no-deps` 和实际受管 CLI 的自检。收据须为 `schema=1`、`status=ok`，且 manifest/lockfile 位于当前产品 home。

需要恢复时，先修复已报告的 Cargo/rustc 或文件问题，本轮停止；下一轮重新运行根入口。构建和替换只由官方 updater 完成：仓库外临时 target 构建 → 对新产物执行 `self-test --json` → 核对 `schema=1/name=bridgeforge/status=ok` → 事务替换用户级二进制，失败恢复原文件。禁止手工覆盖二进制，禁止用旧已安装产物的自检代替新构建产物自检，也禁止在同一轮再次刷新。
