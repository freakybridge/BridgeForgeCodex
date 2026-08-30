# 项目 Python preflight

仅当根入口已经判定 `$MODE`，但项目 `.venv` 缺失或验证失败时读取。

- 双戳、缺戳、非法戳或项目身份不明必须在创建 `.venv` 前零写阻断。
- 每个项目只能使用自己的 CPython 3.11+ `.venv/Scripts/python.exe`。
- `.venv` 缺失时，只有空白 `init` 或已识别旧戳的 `adopt` 可以从 PATH 选择一次经验证的 CPython 3.11+；`update` 禁止创建或重建 `.venv`。
- 现有 `.venv` 损坏、低于 3.11、不是 CPython 或路径逃逸时必须阻断，禁止回退 PATH。

允许 bootstrap 时，PATH 解释器只能执行：

```powershell
& $BOOTSTRAP_PYTHON -B `
  (Join-Path $BRIDGEFORGE_CODEX_HOME "templates\scripts\project_runtime.py") `
  bootstrap --project-root . --mode $MODE --bootstrap-executable $BOOTSTRAP_PYTHON
```

创建成功后立即用新项目解释器运行：

```powershell
& .\.venv\Scripts\python.exe -B `
  (Join-Path $BRIDGEFORGE_CODEX_HOME "templates\scripts\project_runtime.py") `
  validate --project-root . --executable .\.venv\Scripts\python.exe
```

只有 validate 成功才允许把该解释器锁定为 `$HOOK_PYTHON`。本轮后续 Python 命令禁止使用裸 `python` 或切换解释器。
