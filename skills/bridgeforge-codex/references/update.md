# Codex 骨架更新

仅在根 skill 判定为 `update` 后读取。

1. 只运行 `bridgeforge project-sync --mode update` 生成计划。
2. 接受任意不高于产品 home 的合法单戳。以当前产品合同 `compatibility_baseline` 为固定分界：低于基线或使用旧戳文件名时直接重建最新骨架，禁止读取旧合同或历史兼容分支；等于或高于基线时兼容更新、保留项目定制，不做全目录重建。分界线必须固定，不得随 `release_version` 自动抬高。
3. 旧戳文件名由同一 latest 事务退役；终态只保留 `.codex/.bridgeforge_codex_version`。
4. 两条路径中的 Rule / Memory 都进入 `project-asset-migration.md` 的逐文件迁移，全部确认前零写入；其他 project-owned、未知文件和人工定制必须保留或逐项确认。不存在任何决策资产时零确认直接安装最新基线。
5. 准备 apply 时返回根入口并读取 `references/transaction.md`。

不得调用已退役的 switch、finalizer、parity 或布局迁移工具，不得手工写戳。
