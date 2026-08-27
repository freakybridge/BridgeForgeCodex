# skills 目录指令

本文件只约束 `skills/**` 通用 Skill 产品源；从项目根启动的任务必须先按根 `AGENTS.md` 的目录索引读取本文件。

## Skill 分发红线

- 新增、删除或改名用户级 Skill 时，必须同轮更新正式分发清单（manifest）、兼容 manifest 和两份路由表（routing）。
- 修改全局入口 Skill 时，必须同轮同步根与 Template `AGENTS.md` 的入口契约。
- 通用 Skill 改动必须同步产品源和工厂自验证镜像（dogfood）；禁止只修改其中一份。

## 验证

- Skill 改动必须通过 metadata、manifest `--check`、routing 和镜像漂移（mirror drift）检查。
