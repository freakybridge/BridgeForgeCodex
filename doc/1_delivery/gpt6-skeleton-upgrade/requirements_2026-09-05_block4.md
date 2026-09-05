---
lifecycle: active
validation_status: awaiting_validation
topic: gpt6-skeleton-upgrade
date: 2026-09-05
---

# 板块四：一致性与升级验证

验收收据（2026-09-05）：用户明确“同意验收”，接受本地测试补充；前三板块的未执行行为场景及完整模型 A/B 仍待验证，本板块保持 active，不把用例校验计为行为通过。用户另行授权 Git 同步；不包含安装传播，无整题归档候选。

用户已授权修改，继续控制文字膨胀。本轮只补工厂测试与验收材料，属 `[repo] / [meta]`；产品版本保持 `1.14.11`，不改运行代码、公共规则、Skill、角色或分发清单，不安装、不提交推送。

## 范围与方法

- 扩展既有真实合同测试，不复制框架：合成旧公共区和旧角色，使用现行完整合同升级；核对项目专区、Skill、文档行和 Hook 保留，旧角色退役、版本落盘及再次规划无动作。此夹具不是完整历史安装回放，也不测试用户级 Skill 分发。
- [行为用例](../../../scripts/tests/fixtures/gpt6-behavior-cases.json) 是场景与预期的唯一事实源；Rust 只检查用例完整性和引用，不调用模型、不自动安装。
- 对照运行时固定模型、effort、工具及初始现场，旧规则取用例中的已核实 Git 基线，新规则取候选工作树；仅替换被测指令，隔离上下文并保留加载证据。无法证明隔离时不得报告有效 A/B。
- 每例记录输入、工具动作、文件差异、提问及结果；Agent 主动动作与后台 Hook 分开记账。原始日志留 `.runtime/`，本文只记证据指针；比较行为而非措辞，有限样本不得宣称普遍改善。

## 验证与传播状态

使用 `cargo test --locked --manifest-path scripts/tests/Cargo.toml <过滤名> -- --test-threads=1`：

| 过滤名 | 实际结果 |
|---|---|
| `project_sync_combined_upgrade` | 1 项通过：受管公共区和四角色更新、退役旧角色、项目自有内容保留、再次规划无动作 |
| `gpt6_behavior_cases` | 1 项通过：6 个用例 ID 唯一、输入完整、指令路径可解析；不代表模型行为通过 |

去掉过滤名运行完整工厂回归：**83 通过、0 失败、2 ignored，319.17 秒**；跳过项为子进程辅助入口和需额外授权的 Assist 来源夹具。用例措辞收紧后再次单独运行用例校验，1 项通过。

受管 CLI 的 `manifest --root . --check` 为 `changed=false`，`check factory-version --root .` 为 `healthy=true / 1.14.11`，`check baseline --root .` 为 `clean`，`self-test --json` 为 `ok`；metadata 无问题，structure 仅既有归档提示，instruction-source、格式与 diff 检查退出 0。

本轮前后 158 个产品及镜像路径的 SHA-256 / 删除状态一致：运行规则文字增量为零。只扩展一项测试、增加一项用例校验，JSON 保存完整场景，本文不复制场景正文；没有新增测试调度器。未改产品代码，Template / dogfood 回归沿用板块三证据，不重复宣称本轮执行。

模型 A/B、用户级安装及真实下游升级均未执行；前三区块记录保留。这里只交付测试补充，不将板块四整体标为验收完成。
