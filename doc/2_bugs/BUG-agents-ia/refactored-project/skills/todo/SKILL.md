---
name: todo
description: 归档对话中新出现的问题、情报或待办，不打断当前主线；用户调用 /todo、$todo，或明确要求记录新问题、历史线索、待查事项时使用。
user_invocable: true
argument: 问题描述（可附精确文档路径或记录 ID）
---

# todo — 归档新问题

## 定位与边界

把单条问题写入项目的既有 delivery / Bug / TODO 事实源。默认只记录，不修改代码；`todo`
负责新问题，`summary` 负责对话总结与验收收口。

## 核心流程

1. 用户给出精确路径、记录 ID，或当前任务锚能唯一定位时，直接读取该记录，不启动子 agent。
2. 目标不唯一且项目指令允许委派时，`light-explorer` 只读扫描 `doc/1_delivery/`、
   `doc/2_bugs/` 和既有 TODO；禁止扫描项目或原生 Memory。
3. 按类型写入：
   - Bug：新建或追加 `doc/2_bugs/BUG-<id>-<topic>.md`，只记录现象、影响、线索和待验证项。
   - 已确认需求的附加事项：追加至该 delivery topic 已存在的 `README.md`、`requirements`、
     `plan` 或 TODO 单一事实源。
   - 尚未确认的新需求：停止并建议进入 `$confirm`，禁止凭空建立 delivery 包。
4. 修改 `doc/**` 文件路径或索引成员时同步 `doc/README.md`；只追加或精确更新本条记录。

## 输出与停止条件

- 输出实际修改的可点击路径、记录内容摘要和未验证项；未 commit、未 push。
- 完成归档后立即停止，不自动恢复旧主任务，也不追加扩展任务。
- 找不到可靠位置或写入会造成重复事实源时，只报告候选位置并等待用户决定。

## 禁止事项

- 禁止修改代码、Rule、AGENTS、Hook 或配置；代码问题只能记录。
- 禁止创建、读取、更新、移动或删除项目 `.codex/memory/`，也禁止调用 `$find-memory`、writer、
  rebuild、lint、duplicate、usage 或索引机制。
- 禁止直接写入或把内容路由到 Codex 原生 `~/.codex/memories/`。
- 禁止删除或覆盖既有 TODO；禁止把“btw / 顺便”等日常语气当作自动触发信号。

$ARGUMENTS
