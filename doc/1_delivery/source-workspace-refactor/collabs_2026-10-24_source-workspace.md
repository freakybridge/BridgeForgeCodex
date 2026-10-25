# TODO-032 协作记录

需求与授权以 [唯一需求卡](requirements_2026-10-24_source-workspace.md) 为准。最新继续执行预算为剩余 8–12 小时 / 100,000–160,000 token，主对话、验证与独立复核共同计入；token 未实测。下列原始文件边界保留为分工历史，最终源码已迁入 `source/`。

| 任务 | 角色 | 唯一写入边界 | 接口与状态 |
|---|---|---|---|
| V1 混合版本角色 | implementation-worker | `templates/hooks/crates/bridgeforge-core/src/release/rust.rs`、`release/polyglot.rs`、`release/schema4.rs`、`release/schema4/configuration.rs`、`scripts/tests/unit/core_release_schema4.rs`；确有必要的新专用测试文件先通知主对话 | 已完成并迁入lifecycle；显式产品成员身份、旧产品整workspace映射与独立包边界均有实际测试 |
| 主集成 | 主对话 | 上述文件以外的构建/分发、记录与后续源码迁移；V1 完成前不移动其文件 | 实现、七包收口、最终运行时及必要验证已完成，待用户验收 |
| 最终审计 | review-auditor | 只读最终需求、真实差异及验证证据 | 整体、多轮增量及最终收据复核完成，未发现阻断 |

V1 不写 dogfood、版本配置、manifest、业务 VERSION 或 CHANGELOG，不提交。主对话在接收接口后统一传播和整合。首次独立登记、投影事务等后续实现不隐式交给 V1。

## 后续独立任务与证据

同一 implementation-worker 依次承接，不并发修改同一文件。V1 已完成混合角色合同与 6 项定向验证；其源码由主对话迁入 `source/lifecycle/`。

| 任务 | 边界 | 实际结果 |
|---|---|---|
| F1 工厂执行边界 | `source/factory/**` | 工厂命令、类型化 FactoryRunner、私有第三产物接入；2 项命令测试通过，运行时代码编译通过。主对话随后移除 factory 对 product CLI 的依赖，公共参数适配归 workflow，输出归 core |
| F2 测试归属 | `core_git_sync.rs`、测试 support、新 factory Git 测试、factory lib 的 cfg 接线 | 原 62 项分成 48+14，未删断言；主对话统一 workspace 测试编译通过 |
| F3 规则提取 | checks 新 instructions/encoding、hook post、相关 Hook 测试 | 保留诊断、退出码和计数器，5 项定向测试通过；主对话接入 CLI |
| V2 首次纳管来源 | lifecycle release 与版本定向测试/support | 已完成：原生core 6.0.0来源证明、不可变基线保护与重复收口；未由子Agent写真实配置 |
| 历史退休盲点复核 | review-auditor，只读 source-layout、manifest、baseline、plan 与 Git 证据 | 挑战“曾在 templates/hooks 即属本次范围”的假设；确认剔除 29 项 Python 资产，保留 92 项 Rust layout，补大小写 Hook 负例。主对话按已有 Rust 迁移范围采纳，4 项 manifest 测试通过 |

以下记录保留各阶段结论；最终整体审计及后续增量复核的状态以后文为准。

- V2 已完成：5 项首次纳管来源、原生版本连续性、防伪、重复收口与后续 move 场景通过，未改真实 baseline。
- V3 已完成：4 项按实际 workspace/锁依赖闭包计算的证据对照通过；原 V1 六项通过（35.29 秒）；V2 后续 move 复验通过（40.93 秒）。显式全锁检查输入仍保留原始字节绑定，不把产品版本字段当成所有库的改动。
- F4 已完成：公共产物 bootstrap 显式接收 CLI 产品版本，保留运行映像、安装范围、provenance、源码和编译器校验；隔离自重入测试 1 项通过（2.48 秒）。主对话已接 CLI 参数。
- F5 任务：修复拆包后测试定位工厂根目录的旧祖先层级，并适配三产物工厂 runtime fixture；不修改生产实现。完成证据见下一项。

- F5 已完成：5 项现代三产物 runtime fixture 与 1 项 Hook 路由验证实际通过。
- F6 已完成：`source/core/src/runtime.rs` 的 Windows 受管 peer 映像替换保留三程序精确目录/名称、链接保护与回滚；真实父 CLI/子 factory 测试 2 项通过，3 个忽略辅助入口由父测试显式重入，不宣称独立跳过即覆盖。
- 独立整体审计已完成，列出的入口、事务和严格零写入问题已逐项修复并定向验证。新增四个工厂 Skill 镜像由同一 review-auditor 增量复核，未发现实现阻断问题；指出测试取基线顺序问题，主对话已修正。完整回归、实际收口与三产物收据仍待最终复核。
- 最新授权预算为剩余 8–12 小时、100,000–160,000 token；所有后续主对话、验证与独立复核计入该预算。
- 2026-10-25 最终只读复核完成：ledger七条passed与当前policy一致，公开CLI/Hook receipt和合同/二进制一致，factory二进制hash匹配；主版本、HEAD、index、baseline保持原状。公开八包无factory/scripts，最终单元/镜像/编译日志及两个preview状态核实，未发现阻断。严格check文件时间保护复用主对话执行证据。需求卡标记awaiting_user_acceptance；未验收、未提交推送发布、未修改真实下游。
