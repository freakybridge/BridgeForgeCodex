# Native Memory 状态处理

仅当根入口的只读 `codex_memory_sync.py status` 结果不是 `declined`，也不是当前策略下完整健康的 no-op 时读取。

- `approved + enabled + hookInstalled + !hookRuntimeVerified`：只记用户级 runtime gap；禁止把“已安装”描述成“健康”，禁止重复 repair 或触发 reconcile，等待下一次真实生命周期事件产生当前 handler revision 收据。
- `approved + enabled + !hookInstalled`：把本地-only `repair-hook` 归为 safe。该 safe 来自已保存的长期授权，不是项目更新授权。
- `approved + disabled_by_user`：保留现状并记 gap，禁止擅自重开。
- `consent=null + disabled`：把首次 `setup`、private 仓库和用户 Hook 安装合并为本轮唯一 risk；拒绝后才运行 `decline --confirmed`，同意后才运行 `setup --confirmed-enable`。
- `consent=null + enabled`：授权状态损坏，保留现场并阻断；禁止猜测修复或补写授权。

首次 risk 卡必须披露：同步整个 `~/.codex/memories/**`、本地较新自动上传、远端较新自动恢复、生命周期 Hook 持续自动同步，并且目标必须是指定 private 仓库。确认后形成长期授权；目录、远端、可见性或协议未变化时，日常同步和 hook 修复不得重复询问。

`repair-hook/setup/decline` 必须传 `--project-root .`，并进入根入口的统一 safe/risk/gap accumulator；禁止提前执行或另问一次。`repair-hook` 只能修改用户 hooks 并验证解释器，禁止访问 GitHub、Git、读取 Memory 或调用 `reconcile`。

实际同步只由已授权的生命周期 Hook 触发，每次同步前必须验证长期授权、远端身份和 private 状态。用户级 Hook 必须通过当前 Git 根动态调用项目 `.venv`；禁止持久化任一项目的绝对 Python 路径。
