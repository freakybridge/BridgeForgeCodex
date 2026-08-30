# Codex 骨架更新

仅在根 skill 判定为 `update` 后读取。

1. 只运行 `bridgeforge_codex_project_sync.py --mode update` 生成计划。
2. 只接受 `>=1.4.31` 的合法单戳并先校验已安装 current baseline；缺戳、双戳、非法戳、合同损坏或公共资产漂移必须零写阻断。
3. 旧戳文件名只允许在通过 current-only 校验后由同一事务迁移；终态只保留 `.codex/.bridgeforge_codex_version`。
4. 常规更新必须保留 project-owned、未知文件和人工定制；存在 risk 时返回根入口走本轮唯一确认。
5. 准备 apply 时返回根入口并读取 `references/transaction.md`。

不得调用已退役的 switch、finalizer、parity 或布局迁移工具，不得手工写戳。
