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

## 真实 Codex 生命周期发现的启动语法遗漏

1.14.5 / `5d3463e` 已推送并正式安装；repair-hook 返回 applied，三个 Rust user Hook 经 Codex 官方 API 核验 enabled/trusted。2026-09-05 01:52 UTC 临时、不持久化的 Codex 0.153.3 会话首轮实际发出 SessionStart，Hook 在 653ms 内退出 1，未产生 hookAttempt，证明尚未进入 CLI。仅启动 thread 而未发送首轮不会执行 SessionStart；子 agent 续接也未提供该事件。

旧 commandWindows 以引号路径开头，没有 PowerShell 调用符。真实 PowerShell self-test 复现 ParserError，独立源码复核确认 Codex Hook 继承会话 shell，并非固定 cmd。修复针对本机 PowerShell 5/7 宿主：单引号字面量参数、调用符、显式原样传递 native exit code，不添加脚本包装资产。cmd 宿主不在本轮验证与支持声明内。

新增真实 Rust native probe，覆盖路径空格/单引号/美元符/反引号，中文 JSON stdin、stdout/stderr 逐字节和 0/1/2 退出码；PowerShell 5/7 六个组合均通过。旧无调用符 Rust handler 必须精确迁移，不能继续当健康别名，附加命令仍零写阻断。迁移完成后从新 payload 重算 ownership，覆盖两字段四种独立路径分隔符组合。

最终 workspace 验证 Core 111/111、CLI 9/9、Hook 15/15 通过。独立审计发现的启动失败误报成功已通过 ErrorActionPreference Stop 修复，PS5/7 缺失程序均实测退出 1；最终审计无剩余源码阻断。最终受管 build-assets、manifest --check、factory-version、baseline、project-structure、skill-metadata 通过；真实安装后仍需新的生命周期和同步收据。

最终完整工厂 fixture 以同一串行命令完成：81 passed、0 failed、2 ignored（375.14 秒），真实新项目安装与工厂发布均通过。

## SessionEnd 关闭阶段的句柄继承缺陷

1.14.6 正式安装后，真实 SessionStart 已自动推送 revision 23，远端 HEAD 与本地收据均为 `44a630aebd3eb1290e476b30b3e27a84c0ecb017`。但 SessionEnd 后存在死亡 worker 与未完成 pending，不能因此宣称整条自动同步链已验收。

2026-09-05 02:45–02:47 UTC 用不调用 Kill 的临时 Codex 会话观察：SessionStart 与 Stop 分别正常同步并清队列；02:47:00.650 只关闭 app-server stdin，SessionEnd worker 于 02:47:01.182 启动，02:47:04.042 随服务自然退出消失，遗留 pending。排除旧测试程序五秒强杀的干扰。

离线回归证明旧 `Command::spawn` 即使三个 stdio 为 null，仍泄漏额外可继承句柄：父进程关闭独占测试文件后，子进程仍持有该句柄，重新打开返回 WinError 32。修复仅替换 Memory worker 的 Windows 创建入口：固定 native exe、UTF-16 参数、`bInheritHandles=FALSE`、单独 `DETACHED_PROCESS`，不更改父 Job 策略。`CREATE_NO_WINDOW` 单独使用的试验在 stdin 读取处阻塞，故不采用。最终回归同时断言三个标准句柄为 NULL、无 console、stdin EOF、Rust stdout/stderr/println 成功、参数逐字保留及子进程存活时额外句柄已经释放。

此阶段 `cargo test --locked --config scripts/tests/factory-cargo.toml --manifest-path .codex/hooks/Cargo.toml --workspace` 完成 Core 113、CLI 9、Hook 15 项通过，4 项子进程入口按设计忽略；`cargo test --locked --manifest-path scripts/tests/Cargo.toml -- --test-threads=1` 完成 81 passed、0 failed、2 ignored（354.24 秒）。受管 build-assets、manifest --check、factory-version、baseline、project-structure、skill-metadata 与镜像检查通过。独立审计未发现源码阻断；MSRV 1.88 未单独实编。仍需正式安装与关闭后真实同步收据，不能以启动收据代替完成收据。
