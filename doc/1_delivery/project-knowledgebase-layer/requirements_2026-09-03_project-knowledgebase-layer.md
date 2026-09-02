---
lifecycle: active
validation_status: awaiting_user_acceptance
size: M
---

# 固定项目知识库层

## 已确认需求

来源：本轮用户要求“允许骨架统一新增 5_project_knowledgebase，作为知识类项目存放话题的地方”，并确认实施。

目标：将 `doc/5_project_knowledgebase/` 纳入固定六层文档结构，为项目自有知识话题提供合法、持续维护的存放位置。

- 新建项目提供该空目录；现有五层项目可正常升级，不能因原来没有第六层而阻断。
- 知识话题、资料和项目索引属于项目，升级不得覆盖或删除；仅目录占位文件使用显式 seed 资产，不得使用整树或 glob ownership。
- 该层不要求套用软件需求验收状态；不得因日期较早或 completed 状态自动成为归档候选。
- 其他任意 doc 同级目录仍禁止；新增 Markdown 仍须登记唯一索引。
- legacy Rule / Memory 可逐源确认迁往该层，必须保留源覆盖、目标 before hash、路径、索引校验和整体事务回滚。
- 兼容性基线固定为 1.8.6：低于基线重建，基线内兼容更新；本功能不改该分界。

非目标：不引入项目类型或任意目录扩展配置；不吸收私人姓名、课题或业务规则；不修改 Assist、vault 或用户级安装；不提交或推送 Git。

## 范围与预算

M 级：涉及模板、机器校验、迁移、分发合同及回归。用户批准 45 分钟、约 20k 新增 token（估算，非实测）、最多 1 个 review-auditor、最多 2 轮验证。超预算、需要额外授权或范围扩张时停止并说明。

2026-09-03 用户追加批准最多 10 分钟，仅收取剩余测试结果并补齐交付记录；未扩大实现范围、未增加审计 Agent。续接后测试已结束，收取结果并完成记录。规模保持 M；token 未实测，不将估算记为实际消耗。

已查证：目录白名单由 project_structure 控制；迁移目标由 asset_migration 校验；archive_scan 仅扫描 delivery 与 bugs；seed 保留既有目标字节；doc/README 使用标题与稳定表键合并。

## 实施计划

1. 更新公共 AGENTS、模板索引和工厂镜像，明确知识正文的项目所有权。
2. 扩展固定目录和 documentation 迁移白名单；新增显式 seed；保持事务及归档边界。
3. 新功能版本升至 1.9.0，同步锁文件、CHANGELOG 和生成合同；基线不变。
4. 测试新建、旧五层升级、已有知识保留、迁移索引与失败回滚、非法目录拒绝和归档排除；运行工厂硬闸、全量测试及独立审计。

## 验收与风险

自动验证必须证明 plan 零写、新目录可创建、重复升级不改知识正文、旧索引保留、非法目标零写失败及事务失败逐字恢复。独立审计不得代替用户确认旧资产。

主要风险是新目录漏分发、受管索引扩展误吞项目行、seed 所有权过宽；以显式文件资产和真实同步器 fixture 覆盖。已有外部知识库必须保留唯一事实源，不得自动生成第二份持续维护副本。

## 实施与验证记录

### 实现

- 1.9.0：公共 AGENTS 和模板/工厂文档同步六层结构；project_structure 与 asset_migration 扩展固定白名单。
- 合同仅拥有明确的 `doc/5_project_knowledgebase/.gitkeep` seed；补齐同步器 seed 目标不存在时的创建分支，既有目标继续逐字保留。
- manifest 重建器从真实模板重新计算 Markdown projection hash，避免修改受管正文后遗留旧校验值；版本和 Rust 锁文件同步，兼容性基线保持 1.8.6。
- 知识通用规则放在既有受管“文档生命周期”节内。新增模板导航行和话题标题不扩大受管所有权；旧项目通过生命周期节内链接得到新入口，已有同名知识索引和导航行保留。

### 审计与验证过程

独立 review-auditor 发现新增受管标题可能吞掉旧项目同名知识索引，已按上述所有权边界修正，并增加同名标题/导航/链接保留及幂等合并断言；最后静态复核无阻断。首次回归另暴露 seed 首次创建遗漏，已补齐。首次测试与清单生成并行产生旧 hash 断言，最终轮改在合同稳定后执行；旧测试进程占用 Windows 测试程序造成链接失败，停止本轮旧进程后续跑。构建期间源码变动触发安全停止，源码稳定后重建，未绕过检查。

| 证据类别 | 当前结果 |
|---|---|
| 源码 | 固定目录/迁移白名单、seed 分支与 Markdown projection 生成已实现；独立审计无阻断 |
| 产品传播 | 模板和分发清单已重建；`manifest --root . --check` 返回 changed=false；未发布或安装用户级版本 |
| dogfood | 工厂公共区及 Rust 源码镜像已同步；完整 baseline=clean（包含生成 CLI/Hook）、factory-version=healthy、project-structure 无 errors、skill-metadata 无 issues |
| fixture | 完整最终轮通过：工厂回归 75 passed / 0 failed / 1 ignored，含新建/升级/正文保留、迁移/回滚、索引/路径/确认/hash 拒绝、归档排除、真实临时项目初始化构建和模板/dogfood 源码一致性 |
| 真实下游 | 未验证；Assist 与 vault 零写，旧源确认不代办 |
| runtime | 工厂 CLI/Hook 生成成功且完整 baseline 核验通过；真实下游/Codex 交互 smoke 未验证 |

### 最终验证命令

- `cargo test --locked --config scripts/tests/factory-cargo.toml --manifest-path .codex/hooks/Cargo.toml --workspace -- --test-threads=1`：102 passed，0 failed，4 ignored（子进程辅助用例）；含同名知识索引/导航保留、幂等合并和固定基线回归。
- `cargo test --locked --manifest-path scripts/tests/Cargo.toml -- --test-threads=1`：75 passed，0 failed，1 ignored（子进程辅助用例），606.87 秒；全新临时项目初始化、生成资产构建、真实合同项目区/索引/Hook 保留与镜像一致性均通过。两套测试合计 177 passed / 0 failed / 5 ignored。
- `.codex/hooks/target/release/bridgeforge.exe build-assets --project-root D:/Quant/BridgeForgeCodex`：built；CLI 与 Hook 均生成 schema-v2 收据并完成 self-test。
- `.codex/bin/bridgeforge.exe check baseline --root .`：clean，覆盖受管源码及生成运行文件；fingerprint=`sha256:1dca5f88058d252f7999e363eeb5816efef427b3184877fea2cf0e5de06563aa`。
- `.codex/bin/bridgeforge.exe check factory-version --root .`：1.9.0，healthy。
- `.codex/bin/bridgeforge.exe check project-structure --root .`：0 errors；另有既有交付归档建议，未执行归档。
- `.codex/bin/bridgeforge.exe check skill-metadata --root .`：0 issues / warnings。
- `.codex/bin/bridgeforge.exe manifest --root . --check`：changed=false。
- `git -c safe.directory=D:/Quant/BridgeForgeCodex diff --check`：通过。

实现、工厂验证及独立审计已完成，等待用户验收，lifecycle 保持 active。试用入口为模板索引的“项目知识库”说明及 factory 空目录 `doc/5_project_knowledgebase/`；本轮不写入示例或私人话题。未提交或推送 Git，未发布用户级 1.9.0；后续正式同步发布后，再继续 Assist 的 legacy 源逐项确认与事务迁移。真实下游和 Codex 交互 smoke 仍未验证。
