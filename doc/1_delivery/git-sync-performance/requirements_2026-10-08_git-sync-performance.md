---
lifecycle: completed
validation_status: verified
---

# git-sync 构建耗时优化

## 授权与范围

- 用户要求开始优化，并确认 BridgeForge 自己也 dogfood：本仓库切换为 `explicit_release`，普通同步不升版，仅 `git-sync release` 升版；其他项目不自动切换。
- 2026-10-08 用户明确调用 `$summary 同意验收` 和 `$git-sync release`，授权本次性能优化及入口更名验收收口、提交并推送到当前 origin/main。复用已有定向验证与独立审计，补齐发布必需的完整测试后关闭记录。
- 保留此前未提交的 `release` 用法更名。开发阶段不提交、不推送；用户后续发布授权见上条。始终不写真实下游或执行用户级安装。
- M 级：约 45 分钟 / 20k 新增 token 估算（未实测）/ 主对话实施，最多 1 名独立 review-auditor 做等价性复核 / 最多两轮定向验证。没有匹配当前模型的周额度系数，不伪报百分比；如范围或预算明显扩大先说明，不循环执行完整发布回归。
- 四问：共享构建与复用属于产品通用能力，进入 templates/ 并同步 dogfood；本仓库策略属于自身配置，不下沉；产品版本与正式 CHANGELOG 由下一次受管发布事务统一生成。

## 计划与验收

| 编号 | 优先级 | 改动 | 验收 |
|---|---|---|---|
| 1 | P0 | 工厂同步复用与目标源树、lock、配方、自检和二进制哈希均匹配的已安装产物 | 无关文档变更不调用 Cargo、不改二进制/收据；版本保持不变 |
| 2 | P0 | Hook/CLI 共用本次隔离快照的 Cargo target | 两者共用依赖产物；每个目标二进制构建前清除旧输出，不能把 Cargo 空成功当新产物 |
| 3 | P0 | 复用前后继续检查输入、二进制与收据，并复用原事务锁、回滚 | 失配重新构建；并发漂移阻断；无静默绕过 |
| 4 | P1 | 同步收据报告复用/重建数 | 可以从机器收据判断是否走了慢路径 |
| 5 | P1 | 本工厂配置 explicit_release，调整实际发布 fixture | 普通同步版本不动；显式 release 保持原版本规则 |
| 6 | P1 | 同一隔离仓库场景对比优化前后，并运行构建来源/事务定向测试 | 输出实测耗时、Cargo 次数和覆盖限制 |

## 边界

- 本轮复用已有生成产物，不引入长期跨项目缓存、不复用未经验证的二进制。
- 正式 release 版本变化会改变 Rust 输入，不能复用旧版本产物；可通过共享依赖编译降低成本。
- 不修改发布前完整回归与独立审计的要求，不把此前 47 分钟整轮耗时当成 CLI 性能基线。
- 不自动复用 LLM 审计或测试结果；机器验证收据的通用持久缓存不在本轮实现范围内。
- 普通同步目标 10–30 秒是目标，不是保证；本地 bare remote 测量不覆盖 GitHub 网络延迟或大型业务项目检查。

## 实测记录

- 历史实际工厂发布命令 226.576 秒；含升版构建，不与不升版的普通同步混为同一场景。
- 旧 CLI 基线：同一基准函数、explicit_release、文档单点修改、真实 pre-commit、本地 bare remote。定时部分 224318 ms，普通同步成功、版本 1.23.0 不变、工作区 clean、0/0。基线夹具复制完成后才修改产品源码。
- 复用/失效/并发和回滚相关 `factory_` 定向测试 6 passed、0 failed；build_provenance 12 passed、0 failed。
- review-auditor 独立等价性复核通过，未发现阻断项；指出的部分命中后提交失败组合已补测：1 passed，复用 Hook 未被重写，CLI/收据/Git index 恢复，用户修改保留。
- 新 CLI 与 Hook 已经由受管 workspace 构建、安装到本仓库 dogfood 并通过完整 baseline。构建源码 SHA-256 为 `ef51f9acbc517b663b2faaafc85302f2736f7eee2023fc48c02c01cbfc462977`。
- 本工厂配置已改为 explicit_release，并同步工厂专属 AGENTS 版本说明；不修改公共模板版本政策。Skill 输出补充复用/重建数。
- 根版本暂维持已发布 1.23.0，避免预写版本导致发布返工。

## 结果与验证

| 编号 | 场景 | 耗时与断言 |
|---|---|---|
| 1 | 优化前普通同步 | 224318 ms；不升版但仍重建 |
| 2 | 优化后相同普通同步场景 | 23963 ms，缩短约 89.3%；复用 2，重建 0；CLI 修改时间不变；版本不变、clean、0/0 |
| 3 | 优化后显式 release | 149796 ms；复用 0，重建 2；隔离项目 1.23.0 → 1.23.1，clean、0/0 |

测试入口：`cargo test --locked --manifest-path scripts/tests/Cargo.toml git_sync_runtime::factory_ordinary_sync_performance -- --ignored --exact --nocapture`。复测设置 `BRIDGEFORGE_PERF_EXPECT_REUSE=1`、`BRIDGEFORGE_PERF_RELEASE=1`；基线不设置。每次在临时目录复制工厂，使用真实 pre-commit 和本地 bare remote，计时仅覆盖 CLI 调用，不含夹具准备、编译测试程序和 Agent 思考。

这是同机单次前后样本，不是统计分位数或 GitHub 网络耗时保证。显式 release 的 149.8 秒为本次测量，不把历史不同负载的 226.6 秒当作严格对照。目标版本变化仍需重建，首次完整回归成本没有被隐藏或跳过。

验证收据：

1. `cargo test --locked --config scripts/tests/factory-cargo.toml --manifest-path templates/hooks/Cargo.toml -p bridgeforge-core git_sync::tests -- --test-threads=1`：26 passed、0 failed，含全部新增复用/失效/部分命中回滚及既有 Git 安全测试。
2. `cargo test --locked --manifest-path scripts/tests/Cargo.toml build_provenance -- --test-threads=1`：12 passed、0 failed，覆盖共享依赖目录和清除旧目标输出。
3. 性能基线与复测各 1 passed；仅临时仓库创建提交和推送，本仓库没有提交推送。
4. 源码模板与 dogfood 全树一致测试 1 passed；manifest --check、skill metadata、project structure、格式/diff 与完整 baseline 通过。
5. 独立 review-auditor 等价性复核通过；补充组合回归也通过。没有发现阻断问题。

开发阶段未执行发布前完整工厂 suite，其后续结果见下节。真实 GitHub 普通同步耗时、真实下游验证、用户级 Skill 安装仍未执行；其他项目配置不变。只有构建输入和产物均有效时走复用路径，Rust 源码、版本、lock 或收据变化仍可能进入编译。

## 验收发布阶段

- 用户已同意验收并授权 `git-sync release`。目标 origin/main，提交消息 `perf: 复用工厂构建产物并统一 release 升版入口`；只读预览确认 1.23.0 → 1.23.1。
- 复用此前独立 review-auditor 复核和定向验证，产品源码未再修改；manifest、版本一致性、skill metadata、project structure 与完整 baseline 检查通过。
- 模板与 dogfood 的完整 workspace 分别通过：CLI 14 + core 172 + Hook 25 = 211 passed，5 ignored，0 failed。
- 完整工厂 suite 首轮 88 passed、1 failed、4 ignored。唯一失败是 `runtime_flows::project_sync_real_init_builds_and_applies_generated_assets`：Cargo 在新临时 target 写 `invoked.timestamp` 时返回 Windows 路径不存在（os error 3）。代码保持不变，单独复测失败 fixture，不将环境原因当作已证实结论。
- 单项原路径复测：`cargo test --locked --manifest-path scripts/tests/Cargo.toml runtime_flows::project_sync_real_init_builds_and_applies_generated_assets -- --exact --test-threads=1`，1 passed、0 failed，317.00 秒；产品代码没有修改，原临时目录错误未复现，根因未确定。合并首轮与单项收据，89 个默认工厂测试均已有通过结果；4 个条件忽略项未冒充执行。
- 2026-10-08 按用户明确同意完成验收，状态 completed / verified；符合归档候选条件但未执行归档。发布由本次受管 `git-sync --release` 完成，实际 commit、推送成败与最终 0/0 以机器收据为准，不在文档中预写成功结果。历史 autostash 保留；不执行用户级 Skill 安装或真实下游升级。
- 已核对原生 Memory 中构建收据、受管入口和源码/安装状态边界；现有约束已覆盖，无新增常驻规则建议，未写 Memory 或自动归档。
