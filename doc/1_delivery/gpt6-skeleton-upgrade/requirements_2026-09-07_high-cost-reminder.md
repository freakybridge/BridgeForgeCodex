---
lifecycle: completed
validation_status: verified
---

# 持续高耗能设置提醒 Hook

## 用户验收（2026-09-07）

用户在收到安装、测试结果及未验证边界后明确调用 `$summary 同意验收`，接受本次 Hook 实现与本仓库安装交付，并单独调用 `$git-sync` 授权保存当前变更。依据已有核心测试、31 轮二进制测试、原生 app-server 提醒通知与 baseline clean 收据关闭本需求；不重新运行验收测试。

本次关闭不代表桌面卡片外观、长期运行、完整工厂 fixture、独立 Agent 审查或真实下游升级已验证；这些边界保持下文记录。未修改其他需求验收状态、Rule、AGENTS 或 Memory，未自动归档。原生 Memory 索引按“高耗能 / HIGH-COST / reminder”检索，无与本需求相关的历史正文命中；不据无关条目新增规则建议。本 topic 仍有其他未完成需求，当前不整体归档。

## 需求、授权与预算

承接 block4 的 `TODO-HIGH-COST-REMINDER`。用户已确认：GPT-6 Astra 的 High 或更高强度，或者任意模型 Fast，连续完成 10 轮后提醒，持续使用时第 20、30……轮继续提醒。用户在自动模型路由回滚、待办提交后明确“开始开发”。仅显示提醒，不修改模型、effort、速度，不阻断或续开任务，不调用额外模型。

L 级：新增跨生命周期的持久计数，且 Fast 依赖非稳定的本机日志接口。用户明确批准 60 分钟、0 个子 Agent、最多两轮验证；原始 token 上限 6,000,000，其中非缓存输入与输出合计不超过 200,000，任一预计超限暂停。开始 2026-09-07 19:38:31（Asia/Shanghai）；预算原生基线时间 19:37:52，累计输入 36,832,146、缓存输入 35,663,488、输出 108,800，总量 36,940,946。缓存属于输入，推理属于输出，均不重复加总。不提交、推送或写真实下游。

## 本次实现口径

- 只在主对话 Stop 事件计数，一次成功完成的 turn_id 算一轮；工具、子 Agent、压缩、取消和停止后续开的重复事件不另计。空回复不计。
- 用当前 turn_id 对应的最后一条实际 turn_context 获取模型与强度；未发送到模型的界面选择不影响本轮。轮中状态不一致时按未知处理，不猜测。
- 普通档完成后连续计数归零；状态未知时记录 unknown 并中断连续性，不当作已知普通档。恢复会话读取本地计数，不补发安装前历史提醒。
- 同一 turn_id 幂等；过期回合不改变当前连续性；并发回调用操作系统文件锁保护。读写或状态解析失败只记录诊断，不阻断用户任务。
- 第 10、20、30……轮通过原生 `systemMessage` 提醒；禁止用 `decision:block` 或模型额外回复实现提示。状态先持久化再输出，因此异常退出极端情况下可能漏一次提醒，不能重复计轮。

## 事实来源与范围

官方 Hook 输入有 session_id、turn_id、model、transcript_path，Stop 支持 systemMessage。effort 从当前 transcript 的匹配回合读取；Fast 从当前对话的本机日志读取。日志是兼容性适配，不是官方稳定合同。必须分别处理线程设置覆盖和单轮覆盖，未知值不得解释为普通速度；不用全局 TOML 推测当前回合。

读取须限定当前对话、设行数/字节/执行时间上限，并以只读方式访问数据库。Windows 使用系统 SQLite 库，缺失或不兼容时记录未知；不引入脚本运行时、常驻服务或模型任务。非 Windows 无法确认 Fast 时同样未知，但可独立确认的 Astra 高强度仍可计数。

传播四问：属于通用产品能力；核心逻辑进入 templates/hooks、Stop 接入既有 Hook，并同步 .codex/hooks dogfood；产品版本、CHANGELOG、锁定本地包版本及受管 manifest/生成资产随修改更新；需求卡和测试为工厂元文档与测试资产。保留项目专区和现有 Hook 行为，不修改用户级设置、自动模型路由或真实下游。

## 验证计划与边界

定向 Rust 测试覆盖 9/10/20/30 阈值、恢复普通档、未知、重复与过期回合、取消/子任务排除、锁冲突、损坏状态、路径边界、日志继承和单轮覆盖；隔离数据库与 transcript fixture 验证真实读取；实际 Hook 二进制验证输出是非阻断 systemMessage，并核对构建及 dogfood 一致性。

不为凑轮次调用付费模型，不自动打开 Fast。真实桌面提醒呈现与下游安装单独记录证据，未取得时标未验证。0 子 Agent 预算下不启动独立 Agent 审查，不能声称已有独立审查收据。完成后留待用户试用验收。

官方参考：[Hooks](https://learn.chatgpt.com/docs/hooks)。

## 当前交付结果（1.15.0）

已实现并安装本仓库 Hook。首次安装后从新完成的主对话回合开始计数，不回补旧轮次。达到第 10、20、30……轮时出现一次高耗能提醒；用户自行决定是否切换设置。未修改用户级模型、effort 或 Fast 配置。真实下游尚未升级。

- 核心：`templates/hooks/crates/bridgeforge-core/src/high_cost.rs`、`high_cost_windows.rs`；Stop 接入 `templates/hooks/src/lib.rs`；六个源码/版本文件同步 dogfood。新增资产已由受管 manifest 登记。
- 安装：官方 `build-assets --project-root D:\Quant\BridgeForgeCodex` 返回 built，Hook SHA256 为 `d21b71d783ff798b2a0050ea5a94601b27865493c20f2422d3185d63edf2bb74`，source tree 为 `657eded6c7bfe246ebd85abd6f46ab0255b0d378b48e1fa7622fdedc5b82aab8`。使用同哈希的官方构建器临时副本启动，避免 Windows 占用当前 EXE 时无法替换自身。
- 已安装产物验证：`cargo test --locked --offline --manifest-path scripts/tests/Cargo.toml high_cost -- --include-ignored --nocapture --test-threads=1` 两项通过。覆盖实际二进制 31 轮与重复 Stop，以及本机 Codex 0.153.4 app-server 的真实 Stop 输入、10 轮计数和 `hook/completed` 提醒通知；假响应仅请求一次，未调用付费模型。收据目录 `C:\Users\bridg\AppData\Local\Temp\bf-cost-native-35064-1788783558675306200`。
- 核心定向测试 9 项通过；真实历史 Fast 开关日志的只读适配测试通过。已有 Hook 单测 24 项通过、2 项忽略。源码镜像相同测试通过。
- `check baseline --root .` 返回 clean、版本 1.15.0；factory-version、skill-metadata、project-structure、manifest --check 和 git diff --check 通过。误用不存在的 `check factory-dogfood` 返回 unknown check；已改用实际 baseline 命令及源码镜像测试，不将误用算通过。
- app-server 收到 UI warning 是已验证事实；桌面窗口内卡片外观、长期真实使用和真实下游仍待用户试用。`codex exec --json` 不展示这段 Hook 正文，不能把它作为桌面呈现验收入口。
- Fast 依赖内部日志，日志缺失、轮转、格式变化、读取超限或数据库不可用时为 unknown，中断连续性。当前 transcript 回合落在限定尾部范围以外也如此。计数限本项目本机 runtime；移动项目、清理 runtime 或跨电脑不会自动迁移。其他 Stop Hook 若强制续开，本 Hook 对同一 turn_id 仍只计一次；不把整个外部 Hook 组合声称为已验收。
- 本次按用户明确的 0 子 Agent 预算执行，未做独立 Agent 审查或完整工厂 fixture 回归，因此不是完整发布验收。保留并行 TODO Skill 的 1.14.22 改动；无提交、推送或真实下游写入。

## 追加预算授权

用户先将非缓存输入＋输出上限追加到 300,000、原始 token 到 8,000,000，并允许增加一轮收尾验证；再次暂停后明确授权将前者追加到 500,000。60 分钟截止时间仍为 20:38:31，0 子 Agent、不提交推送不变。此前两次超限已向用户说明并暂停，不将追加授权倒算为此前未超限。

收尾复核（20:21 左右）：原始累计增量 7,035,297，非缓存输入＋输出 357,025；后续仅有本段记录、临时构建器清理和最终交付文字。强化通知断言后再次通过：原生 `hook/completed` 的状态必须是 completed，entries 必须包含第 10 轮的 warning 正文；收据 `C:\Users\bridg\AppData\Local\Temp\bf-cost-native-23760-1788783678489062700`。预算报告中的原始 token 含缓存输入，不能与非缓存数字相加。

## 2026-09-07 开发中途收据

- 核心定向测试 9 项通过；已有 Hook 单测 24 项通过、2 项忽略。实际 Windows SQLite 读取历史 Fast 开关对照日志测试通过。
- debug Hook 二进制完成 31 轮、每轮重复回调测试：仅第 10、20、30 轮返回 systemMessage；退出码 0，无 decision/continue 字段，用户配置字节不变。
- 原生 Codex 离线探针已证明真实 Stop 将预置 9 轮计数推进到 10，且只有一次假模型请求。最初的测试配置未启用 Hook 信任，随后 Windows 测试命令缺少调用运算符；修正测试配置后触发成功。最终提醒可见性断言仍失败：CLI JSON/stdout/stderr 未出现 systemMessage，不能宣称桌面呈现通过。探针保留为显式忽略测试，等待继续调查。
- templates 源码尚未同步 dogfood 或安装运行产物；产品版本与 manifest 尚未完成本需求收尾。工作区的 todo Skill、1.14.22 版本及相关清单属于另一个并行任务，未回滚。
- 20:10 左右预算复核：原始增量 3,972,592；非缓存输入与输出增量 218,864，已超出 200,000 上限。发现后暂停实施与验证；本次预算检查不够及时，后续不得沿用旧授权继续。0 个子 Agent，无提交推送，无真实下游写入。
