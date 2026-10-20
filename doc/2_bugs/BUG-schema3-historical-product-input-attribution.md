---
lifecycle: active
validation_status: awaiting_user_acceptance
---

# schema 3 历史项目级文件缺少发布归属

## 范围与授权

2026-10-20 用户确认：项目级启动工具及其测试逐文件登记为 product_inputs，不因这些文件单独触发组件升版；授权修复并发布上游、更新用户级工具及本项目后继续下游 release。通用模板只实现补认机制，不包含下游项目名或文件清单。

本修复按 M 级执行：默认 45 分钟、20k 新增 token 估算（token 未实测）、最多 1 个 review-auditor、最多 2 轮验证；复用已完成的隔离草案调查和 26 项相关回归。实际现场和发布状态分别记录，已有授权不重复确认。

传播四问：属于产品层的通用历史归属修复；进入 templates/hooks 并同步 dogfood 与受管 manifest；版本和 [product] CHANGELOG 由正式 release 事务生成，不预写 VERSION。

## 现象与原因

已有 schema 3 项目先提交了未登记归属的项目工具，后续 release 报 `unattributed historical release input`。即使当前配置新增精确 product_inputs，受管 setup 仍在回放旧提交时被同一缺口阻断，无法生成可应用的配置 fingerprint。

原因是历史归属只检查提交前后的配置；两者都没有声明时直接失败，没有使用当前明确的单文件项目归属补认这一原先未归属的输入。历史已有组件归属、原生 adapter 输入和真正未知文件仍必须严格处理。

## 修复合同与经验

1. 仅在历史前后都无归属、当前 product_inputs 精确登记同一文件，且文件不属于历史原生包源码根、manifest 或 adapter 输入时，允许补认项目级输入。
2. 目录前缀不能作为历史补认依据；历史已有归属优先，不能把已有组件影响改成项目级影响。
3. 补认不产生组件影响，但该业务提交仍按原消息参与软件 VERSION 的累计升版判断；登记本身不即时改版本，普通同步仍不升版。
4. 锁文件、包身份、生命周期、版本漂移、工作区与 index 保护均保持，预览仍零写入，不改写历史。

经验：文件属于项目与发布输入归属是两个问题。补齐当前配置前，必须验证历史回放能否接受该补认；不能靠反复逐文件重试、回退版本或跳过归属检查恢复发布。

## 六类验证证据

| 类别 | 当前证据 |
|---|---|
| 源码 | 精确单文件补认机制已应用，独立 review-auditor passed；开发指纹 sha256:6681555a176f5367ce923e3929d5a3f0d00fdbfe20693af000150f74a58f856e |
| 产品传播 | 模板、manifest 与镜像检查通过，正式上游发布待执行 |
| dogfood | 全树镜像一致；完整 workspace 440 passed、0 failed、13 ignored；当前版本受管 CLI/Hook 构建、自检和 baseline 通过 |
| fixture | 规范验证环境后完整工厂 fixture 93 passed、0 failed、7 ignored；七项工程检查全部通过。隔离相关发布回归另有 26 passed，包含项目升版而组件不变、已有组件影响保留、目录不补认、锁文件不豁免、其他未登记文件仍阻断 |
| 真实下游 | 待正式更新后经受管 setup 应用确认配置，再执行真实 release |
| runtime | 本轮不声称桌面或 Native Memory 生命周期验证通过 |

## Windows PowerShell 验证环境恢复（2026-10-21）

第一轮完整工厂 fixture 为 92 passed、1 failed、7 ignored，唯一失败是 shared_cli_reuse_is_verified_in_isolated_profile 找不到 Get-FileHash。未改任何产品源码、测试或断言，使用原用例做进程级环境对照：PSModulePath 临时限定为 Windows PowerShell 自带 Modules 时通过，恢复继承的 PowerShell 7 模块路径后原错复现；直接由 PowerShell 宿主启动同一脚本也通过。

随后仅在正式验证进程中规范 PSModulePath，并在 finally 恢复父进程原值，重新执行全部检查。正式缓存按环境绑定，因此没有篡改或人工补写检查缓存；完整 workspace 和工厂 fixture 均重新通过。工程验证耗时超过最初估计，主要额外时间来自此环境诊断和整轮重测。忽略项不计为通过，未修改任何持久模块路径或用户配置。

正式目标构建已经通过，受管收据 status=prepared、target_version=2.6.2、checks_passed=7；完整原始记录保存在本机 `.runtime/bridgeforge-codex/release-preparation/current.json`。两个仓库的发布结果以本节后续实际收据补充；发布授权不等于用户验收，不自动关闭该记录。
