---
lifecycle: active
validation_status: in_progress
---

# BUG：Windows 等价路径导致 Native Memory Hook 漂移误报

发现日期：2026-09-04。影响版本：1.12.4。

## 现场与根因

同一用户目录分别用 `C:/Users/<user>/.codex` 与
`C:\Users\<user>\.codex` 传给 `memory-sync status`，前者返回
`hookInstalled=true`，后者返回 `false`。`repair-hook` 使用反斜杠参数时在
SessionEnd 受管 handler 报 `managed hook content drifted`，拒绝写入。

`user_config::expected_document` 原先直接把调用参数的路径拼进命令。
ownership 检查器比较完整 handler 指纹；等价目录的不同路径文本因此产生不同预期。
该报错不能证明自动同步执行失败，真实生命周期运行收据须另行核验。

## 修复边界

- 新生成的普通 Windows 盘符绝对路径统一使用正斜杠。
- 现有 Memory Hook 的两种命令字段仅允许已知程序路径与用户目录路径中的斜杠差异；
  保留已有等价写法，避免为了检查而重写用户配置。
- status 和 repair 共用同一等价识别规则，通用 ownership 校验仍保持严格。
- 健康配置在持锁校验后直接返回，保留 JSON 格式与外部事件顺序；需要写入时仍走原 CAS。
- Windows 双引号命令禁止将尾斜杠转换为会转义闭合引号的反斜杠。
- 命令正文、引号转义、其他字段、其他用户 Hook、相对路径、UNC 与设备路径不放宽。
- 修复不读取或修改 Memory 正文，不前台触发远端 reconcile。

## 验证与传播

| 证据类别 | 当前记录 |
|---|---|
| 源码 | Template 的 `memory/user_config.rs`；完整 workspace 118 项通过、4 项子进程 helper 忽略；独立 review-auditor 增量复核通过 |
| 产品传播 | 由官方 git-sync 更新版本、CHANGELOG、合同与分发清单，尚未发布 |
| dogfood | `.codex/hooks` 对应源码与 Template 镜像一致；受管运行产物正在刷新 |
| fixture | 路径别名、无写入检查、repair no-op、真实修改拒绝与命令引号边界回归通过；完整分发 fixture 等待刷新产物后复跑 |
| 真实下游 | 用户授权按 StratusAgent、CausisRiskSuite、BridgePersonalAssist 顺序升级；尚未执行本次修复版本 |
| runtime | 本机用户级 CLI 的两种路径写法待复测；真实自动同步生命周期未验证 |

计划命令：

```powershell
cargo test --locked --config scripts/tests/factory-cargo.toml --manifest-path .codex/hooks/Cargo.toml --workspace
cargo test --locked --manifest-path scripts/tests/Cargo.toml
```

修复完成后的受管发布、批次与本机状态回执以本次执行结果为准；本记录不预先声明真实
Memory 自动同步成功。
