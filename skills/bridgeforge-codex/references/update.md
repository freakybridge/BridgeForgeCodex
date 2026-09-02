# Codex 骨架更新

仅在根 skill 判定为 `update` 后读取。

1. 只运行 `bridgeforge project-sync --mode update` 生成计划。
2. 接受任意不高于产品 home 的合法单戳。版本落后或使用旧戳文件名时直接生成 latest current-only rebuild，禁止加载旧合同或历史兼容分支；版本与产品 home 相同才校验已安装 current baseline。
3. 旧戳文件名由同一 latest 事务退役；终态只保留 `.codex/.bridgeforge_codex_version`。
4. Rule / Memory 进入 `project-asset-migration.md` 的逐文件迁移；其他 project-owned、未知文件和人工定制必须保留或逐项确认。不存在任何决策资产时零确认直接安装最新基线。
5. 准备 apply 时返回根入口并读取 `references/transaction.md`。

不得调用已退役的 switch、finalizer、parity 或布局迁移工具，不得手工写戳。
