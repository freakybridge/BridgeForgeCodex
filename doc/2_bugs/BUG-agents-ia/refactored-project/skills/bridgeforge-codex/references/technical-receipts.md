# 内部技术收据

仅在根入口需要核对字段级成功条件、解释异常、证明回滚或回答用户技术追问时读取。默认用户结果必须来自同步器 `human` 区，不得倾倒本文件或 `machine` 区的字段清单。

项目骨架收据必须核对：用户级刷新 commit、`execution_status`、applied、preserved project asset IDs、blockers 原文、版本戳路径与终态、rollback 字段、验证命令和逐文件工作区清单。

Native Memory 收据必须核对：`project_readiness`、`user_native_memory_readiness`、长期授权状态、`hookInstalled`、`hookRuntimeVerified`、最近运行收据、Hook 修复结果和 `remote_reconcile=applied/declined/not_requested`。

禁止用项目 ready 掩盖用户级同步 gap，也禁止把本轮未执行的 reconcile 描述成已完成。只有用户追问原因、证据或技术细节时，才按问题范围展开对应字段；禁止一次性补发整份技术清单。

同步器输出合同：`machine` 保持旧 JSON 自动化合同；`human` 输出确定性的三段式用户结果；`combined` 返回两区供 Skill 同时判定流程和展示结果。主对话可以引用 `machine` 区核验事实，但不得用临场解释覆盖 `human` 区结论。
