---
lifecycle: active
validation_status: in_progress
---

# Native Memory 有效旧快照被排序差异误判损坏

## 用户目标与范围

2026-09-05 用户明确要求修复 Native Memory 自动同步。恢复已授权用户的旧状态迁移、Hook 安装和实际同步，保留远端、授权范围与记忆正文。

## 事实与根因

- 受管 `memory-sync repair-hook` 获准执行后返回 `remote snapshot content does not match its SHA-256 manifest`。
- 旧基线 revision 22 的 61 个文件逐文件 SHA-256 全部匹配；manifest.files 原序列紧凑 JSON 的 SHA-256 与声明的 `b0bb7a7f235d4396f8feb581a17ed88962230abea7d01c533f061831bcd4725e` 一致。
- 新程序扫描排序与历史 manifest 不同，直接比较列表和重新计算摘要造成误报；安装 Hook 前的迁移因此阻断。
- 失败时 remote 已迁移，旧状态保留，未写完成标记、未安装 Hook、未执行同步。

## 修复与传播

修改 Template Memory 校验与恢复模块，同步工厂镜像，由正式发布流程升级 VERSION / CHANGELOG。摘要按 manifest 原顺序验证，扫描结果和声明列表按路径排序逐项比较；恢复暂存目录共用同一验证。保留原 manifest，不跳过完整性检查。

## 验证证据

| 层次 | 证据 |
|---|---|
| 源码 | 旧序迁移回归先红后绿；覆盖原 manifest 保留、伪造摘要、重复、多余、篡改及缺失文件；完整 workspace Core 106/106、CLI 9/9、Hook 15/15 通过，4 项子进程入口按设计忽略 |
| 产品传播 | Template 已同步，正式安装待验证 |
| dogfood | 源码镜像一致，受管 build-assets 成功，baseline clean，manifest --check / factory-version / project-structure / skill-metadata 通过 |
| fixture | 完整工厂测试 78 通过、3 失败、2 忽略；完成受管构建后 build_provenance 串行复验 11/11、真实项目内容保留复验 1/1，覆盖并通过全部 81 项；真实 init 与工厂 commit/push fixture 在整套中通过 |
| 真实下游 | 四项目未再次升级，本次修复用户级同步入口 |
| runtime | 安装、实际生命周期触发及远端同步待验证 |

独立审计 `audit_memory_snapshot_order` 指出的恢复暂存、no-op 和旧冲突包等价判断遗漏均已修复；最终复核无剩余发布阻断。新增回归证明无/有基线时相同旧序远端均 no-op 且远端 HEAD 不变；旧 captured-local 可决议，真实本地变化仍阻断。原摘要只验证 manifest 完整性，已验证文件集合的等价判断共用同一函数。

## 真实安装发现的第二层迁移遗漏

1.14.4 / `9e5ab28` 已正式推送并通过 updater 安装；真实旧基线迁移完成，证明排序兼容修复有效。随后 Hook 安装返回 `managed hook content drifted`：现场三个旧 Python/PowerShell handler 与历史官方 `fc94635:scripts/codex_memory_sync.py` 的 `_hook_handler` / `_windows_hook_command` 完全一致，但 Rust 未迁移这套历史 handler。

补充修复只在内存中精确识别完整旧 handler，转换后仍通过 ownership 校验，并在原用户 Hook 锁内以 CAS 核对原文件后原子写入。修改命令、Windows 命令、timeout、matcher 或 identity 的情况均零写阻断；第三方 handler 与顶层 metadata 保留。专项 user_config 12/12 通过，完整 workspace Core 109/109、CLI 9/9、Hook 15/15 通过；命令为 `cargo test --locked --config scripts/tests/factory-cargo.toml --manifest-path .codex/hooks/Cargo.toml --workspace`。

独立审计指出的缺失事件可变索引问题已改为不插入键的 get_mut，回归覆盖仅 SessionStart 存在时补齐 Stop/SessionEnd、保留第三方与 metadata、重复 repair 幂等。最终复核无剩余发布阻断。自动兼容限历史默认产品安装位置，自定义旧产品路径保持 fail-closed。

Codex 官方 hooks/list 证明旧三个 Hook 已 trusted；转换后必须重新核验新定义的信任与实际触发。

第二层修复发布前执行 `cargo test --locked --manifest-path scripts/tests/Cargo.toml -- --test-threads=1`：81 passed、0 failed、2 ignored（332.46 秒），包含真实 init、工厂受管 commit/push、Memory push/restore 与冲突保护。manifest --check、factory-version、project-structure、skill-metadata、baseline、mirror diff 与 diff --check 均通过。
