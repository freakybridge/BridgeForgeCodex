---
lifecycle: completed
validation_status: verified
topic: gpt6-skeleton-upgrade
date: 2026-09-05
scale: M
---

# 板块三：Agent 角色边界与安全退役

验收收据（2026-09-05）：用户明确“同意验收”，本地角色与迁移实现按下列证据收口；新角色模型行为仍未验证，留板块四跟进；不包含用户级安装或真实下游。用户另行授权 Git 同步，结果以同步收据为准。

## 已确认范围

用户对修改方案明确回复“同意”，沿用控制文字膨胀要求；本卡整理已有授权，不重复确认。

- 收紧调研、审计的只读边界；实现角色只处理授权文件。保留角色名，删除误导性的算力描述；xhigh 保留额外审计明确授权门槛，默认继承会话 model / effort。
- 删除 mechanical-sync-worker 模板、dogfood 与分发登记；Git 同步仍由主对话执行，不改已经一致的 Skill。
- 退役仅匹配三个经 Git 核实的官方历史 hash；未知或定制内容、项目仍有引用时保留并阻断。退役与版本升级同事务，计划后变化零写拒绝，失败回滚。
- 修订根 AGENTS 工厂登记要求与架构说明；不新建路由表或通用迁移框架。
- 通用产品层进入 templates 并同步 dogfood，更新 VERSION / CHANGELOG / manifest；工厂条款为自身配置层，交付记录为元文档。
- 不运行安装器、不修改真实下游或用户级配置、不提交推送；保留板块一、二及初审记录。

## 验收

四个角色与引用一致，角色文字总量净减少；新安装不分发旧角色。真实函数测试覆盖历史载荷、换行、低基线、init / adopt、定制、引用、漂移、失败回滚及大文件扫描。完成镜像、manifest、Rust 测试、工厂硬闸、runtime 自检与独立审计；不得以静态文字匹配冒称模型行为验证。

## 实施与证据

产品版本 `1.14.11`，四个保留角色及 Rust 同步器已同步 dogfood；旧角色不再分发，原文保留为非运行时测试夹具，Git 历史可恢复。未修改共享 Skill 正文或公共 AGENTS，已有引用均指向保留角色。

### 文字控制

只统计角色模板，镜像不重复计数；非空白字符 **3,200 → 1,515（−52.66%）**，LF 行数 **60 → 31**。四份保留角色各自缩短，另删除一份角色；运行代码、测试、版本及本交付记录不计作提示词缩减。

| 角色 | 字符（前→后） | 行数（前→后） |
|---|---:|---:|
| implementation-worker | 544→367 | 12→8 |
| light-explorer | 543→301 | 11→7 |
| review-auditor | 530→368 | 12→8 |
| xhigh-auditor | 768→479 | 14→8 |
| mechanical-sync-worker | 815→0 | 11→0 |

### 验证证据

- `cargo test --locked --config scripts/tests/factory-cargo.toml --manifest-path templates/hooks/Cargo.toml --workspace retired_role -- --test-threads=1`：7 组通过。覆盖三份历史载荷及 LF/CRLF、低基线与兼容更新、init/adopt、未知与定制、伪造旧 manifest、引用、计划后变化、失败恢复和事务晚期引用保留。
- 同一命令去掉 `retired_role`：Template 完整 workspace 退出 0；manifest 改为 `.codex/hooks/Cargo.toml`：dogfood 完整 workspace 退出 0；各 144 通过、4 个子进程辅助入口 ignored。
- `cargo test --locked --manifest-path scripts/tests/Cargo.toml distributed_roles_match_skill_references_and_dogfood -- --test-threads=1`：1 组通过，验证四角色登记、镜像、无算力配置及当前 Skill 显式角色引用。
- `cargo run --locked --release --manifest-path .codex/hooks/Cargo.toml --package bridgeforge-cli -- build-assets --project-root .`：`built`，生成 CLI / Hook 两份收据；两者 `self-test --json` 均 `ok`，CLI 版本 `1.14.11`。
- `.codex/bin/bridgeforge.exe manifest --root . --check`：`changed=false`；`check baseline --root .`：`clean / 1.14.11`；`check factory-version --root .`：`healthy=true`。
- 同一 CLI 的 `check skill-metadata --root .` 无 issue / warning；`check project-structure --root .` 无 error、仅既有归档候选提示；`check instruction-source` 退出 0。
- `cargo test --locked --manifest-path scripts/tests/Cargo.toml -- --test-threads=1`：82 通过、0 失败、2 ignored，耗时 326.70 秒；跳过项为子进程辅助入口及需额外授权的 Assist 来源夹具。实际覆盖隔离工厂 CLI 同步构建、pre-commit、新项目初始化构建与事务应用；不等于真实下游验收。
- `git diff --check` 及 Template / dogfood / 工厂测试的 `cargo fmt --check` 均退出 0。manifest 与 dogfood 构建目录的权限拒绝均通过同一命令授权重试解决，未更换工具或安装来源。

### 独立审计与边界

`review-auditor` `/root/block3_review` 独立核实三个历史 hash、真实 diff、退役与回滚链路及测试。初审发现整树载荷留存风险；已改为 64 KiB 流式读取，仅存摘要和匹配标记。2 MiB 文本跨块匹配、摘要正确性、二进制附件跳过及附件原样保留均有实测。复核未发现新的阻断问题；审计者未代跑写盘测试。

退役引用检测仅覆盖根 AGENTS、Codex config 及 `.codex/skills` / `.codex/agents` 下已列文本扩展名的字面量，不是语义分析，不覆盖动态拼接、外部 Skill 或未列格式。工厂测试核对当前反引号标注及角色命名约定，不是通用 TOML / 委派解析器。定制文件或引用 gap 必须处理后重新规划，不能强行盖新版本戳。

源码、工厂运行文件、可运行自动回归及独立审计完成，待用户试用。真实下游升级、已安装用户级配置、新角色模型前向行为未验证；本轮没有安装、提交或推送。已有会话角色是否热重载不作承诺。
