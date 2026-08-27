# AGENTS 第九版独立复评

> 状态：未通过，已进入第十版
> 需求卡：`../requirements_2026-08-27_agents-iterative-validation.md`

## 议题

1. 足够精简，且每类决定只有一个规范 owner。
2. 人类友好，无单条承载四项以上独立义务。
3. 无语义损失，包括主体、强度、触发、顺序、例外和跨平台入口。
4. 自动验证可信，已知的 CRLF 与注释隐藏反例都转为可阻断机器门。

## 本轮候选

- 工厂根 135 行、66 个项目符号；Template 125 行、46 个项目符号。
- 非 README region 使用 raw span，只替换 marker 区间；新增 CRLF、LF、混合换行、无尾换行及二次 no-op fixture。
- 普通语义检查剥离 HTML 注释和 fenced code，并按章节检查；Template 项目占位注释是唯一显式例外。
- README 不再出现 `必须/禁止/不得/只能/不允许/只允许`，文档、环境和 Spike 的命令红线只留在 AGENTS；README 只解释位置、命令和样例。
- 恢复执行五步、可验证后台入口、Memory 严格顺序、双平台 pip、新增嵌套登记和填好删除占位注释。
- 拆开根文档规则、工厂事务结果和传播四问；新增 size/density 门。

## 自动收据

```text
proposal-contract: PASS
```

## 第一轮

- A 方：篇幅和整体可读性通过，但 `~~~`、缩进代码仍可藏住旧规则；“完整调用链”被弱化；README 与根红线重复；账本版本和状态过期。
- B 方：同意可读性改善，同时发现未闭合 HTML 注释 / fence 未 fail closed，且项目标题可藏在 HTML 注释中通过 Hook。

## 交叉第二轮

双方收敛为五项阻断：统一 Markdown 可见性模型、恢复“完整调用链”、清除 README 同义规范、补常用 Skill 发现入口、修正迁移账本。

## 结论

第九版不通过。已确认通过的部分是篇幅、整体冷读体验，以及非 README region 的外部字节保留与幂等；其余阻断进入第十版。
