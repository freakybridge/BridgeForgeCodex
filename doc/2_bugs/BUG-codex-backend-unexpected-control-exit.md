---
lifecycle: active
validation_status: in_progress
---

# BUG：Codex 本地后端异常控制退出

**状态**：investigating  
**发现日期**：2026-08-26  
**影响版本**：Codex Desktop `26.818.8289.0`、`26.820.7780.0`  
**影响范围**：Windows Codex Desktop 本地任务运行；BridgeForgeCodex 仅记录现场，不主张问题属于本仓库

## 现象

Codex Desktop 在任务运行期间偶发显示“ChatGPT 意外停止”恢复页。前台应用仍能响应，点击
“重启”后通常可以继续，但当时正在执行的任务可能被中断。

2026-08-26 本机日志至少记录到三次本地 `codex.exe` 后端异常退出：北京时间约
09:12、13:10、13:21。三次均被桌面端标记为非预期退出。

## 影响

- 当前 turn 可能中断，需要恢复或重新发起。
- 长时间工具调用的结果可能没有完整写入 session transcript。
- 问题重复出现时会影响长任务可靠性，但尚无项目文件损坏证据。

## 已确认线索

- 三次退出码均为十进制 `3221225786`，即 Windows `0xC000013A`
  (`STATUS_CONTROL_C_EXIT`)。
- Windows Application Event Log、Reliability Records 未发现对应的应用崩溃记录；本机未生成
  `ChatGPT.exe` 或 `codex.exe` crash dump。
- 其中两次退出前，Codex Core 记录
  `Custom tool call output is missing for call id`。
- 对应 transcript 中存在已完成的 `functions.exec` 自定义工具调用，但缺少配对的
  `custom_tool_call_output`；两次都涉及对长运行命令执行 `write_stdin` 轮询。
- 第三次退出前最后一条 Core 错误是命令被安全策略拒绝。该错误与四分钟后的进程退出是否存在
  因果关系，当前没有证据。
- 第一次发生在 Codex App Server `0.149.0-alpha.4.3`，后两次发生在
  `0.150.0-alpha.8`；升级后问题仍曾复现。
- 未发现配置解析失败、GPU 进程崩溃、内存不足或当天 OpenAI 官方服务事故与三次时间点对齐。

## 当前判断

已定位直接故障点为本地 Codex 后端收到控制终止并退出。工具输出记录缺失是两次现场中的高相关
线索，但不能据此断言它是根因，也不能排除它只是中断过程留下的结果。

目前不知道哪个组件发送了控制终止事件，因此最底层根因未确认。禁止将本问题表述为已证实的
BridgeForge 配置故障、Codex 产品缺陷或用户操作错误。

## 待验证项

1. 再次复现时记录准确时间、任务 ID、是否进行了取消、切换任务或发送新消息。
2. 核对缺失 `custom_tool_call_output` 是发生在控制终止之前，还是 transcript 落盘中断造成。
3. 确认发送 `CTRL+C` / console control event 的进程或 Codex 内部代码路径。
4. 使用同版本对照测试：普通短工具调用、长命令自然完成、长命令轮询、主动取消和切换 turn。
5. 若能稳定复现，整理脱敏桌面日志与 session 片段，通过 Codex `/feedback` 提交上游。

## 现场位置

- 桌面日志：`%LOCALAPPDATA%\Packages\OpenAI.Codex_2p2nqsd0c76g0\LocalCache\Local\Codex\Logs\2026\08\26\`
- Session transcript：`%CODEX_HOME%\sessions\` 与 `%CODEX_HOME%\archived_sessions\`

日志和 transcript 可能包含项目路径、命令和对话内容；对外提交前必须脱敏。
