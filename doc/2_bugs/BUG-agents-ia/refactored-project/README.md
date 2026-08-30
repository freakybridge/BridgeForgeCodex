# bridgeforge-codex

> 为长期维护的 Codex 项目安装一套可升级、可检查、可回滚的协作骨架。

bridgeforge-codex 让 Codex 在不同项目中使用一致的基础规则、工作流程和安全检查，
同时保留每个项目自己的约束与定制。日常使用只需要记住一个入口：在目标项目根目录运行
`$bridgeforge-codex`。

它主要完成三件事：

- 为项目安装或更新 `AGENTS.md`、`.codex/`、文档结构和提交前检查等协作设施。
- 为用户安装 `confirm`、`develop`、`summary`、`git-sync` 等通用 Skills。
- 更新前先检查现有项目；需要用户决定的保留或删除项会先停下来确认，无法安全判断的冲突直接停止，验证失败则不写入或回滚。

bridgeforge-codex 只支持 Windows、Python 3.11+ 和 Codex。

## 快速开始

先安装 bridgeforge-codex：

```powershell
git clone https://github.com/freakybridge/BridgeForgeCodex.git "$env:USERPROFILE\tools\bridgeforge-codex"
Set-Location "$env:USERPROFILE\tools\bridgeforge-codex"
& powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\install-shared-skills.ps1
```

然后新开 Codex 会话，在需要安装或更新骨架的项目根目录运行：

```text
$bridgeforge-codex
```

首次安装、旧版迁移和异常诊断分别见[安装与迁移说明](INSTALL.md)。

## 它会修改什么

用户级位置：

- `~/.bridgeforge-codex`：完整、可信的产品仓库。
- `~/.codex/skills/bridgeforge-codex`：Codex 用来发现产品的薄入口。
- `~/.codex/skills/`：由产品维护的通用 Skills。
- `~/.codex/memories/`：Codex 原生 Memory；用户授权后可作为不透明目录跨电脑同步，
  bridgeforge-codex 不读取或改写其中语义。

目标项目：

- 根 `AGENTS.md`：公共区由产品更新；项目级专区逐字保留。
- `.codex/hooks/` 与 pre-commit：执行可机器判定的安全检查。
- `doc/`：以 `doc/README.md` 为索引的五层文档结构。
- 项目自己的嵌套 `AGENTS.md`、项目规则和明确保留的定制资产仍归项目所有。
- 旧项目已有的 `.codex/memory/`：只作为待迁移的 legacy 资产原样保留，不再安装或运行项目 Memory 链。

## 更新时怎样保护项目

- 新项目直接安装当前骨架。
- 已有项目先生成更新计划；需要保留或删除项目资产时逐项询问，不会静默替用户做选择。
- 公共区被人工修改、版本戳缺失或冲突、当前基线损坏时，整轮更新在写盘前停止。

不同项目版本的具体处理方式见[安装与迁移说明](INSTALL.md#项目升级与异常诊断)；内部同步
合同由[Codex 项目同步设计](doc/0_architecture/design/codex-project-sync.md)维护。

## 仓库结构

```text
BridgeForgeCodex/
├── skills/bridgeforge-codex/ # $bridgeforge-codex 产品入口
├── skills/                   # 通用 Skills
├── templates/               # 唯一活跃下游模板根
├── scripts/                 # 安装、项目同步与 tests
├── doc/                     # 架构、交付、Bug、参考与归档
├── bridgeforge-codex-manifest.json # Codex-only 正式分发清单
├── VERSION
└── CHANGELOG.md
```

`templates/`、`skills/` 属于产品层；`.codex/` 是本仓库的自用镜像；`doc/` 和 README
属于元文档。产品层变化需要同步版本、CHANGELOG、分发清单与自用镜像。

## 兼容边界

旧 `$bridgeforge`、旧用户级受管记录、旧 `.bridgeforge` 产品目录和 Claude 用户级资产
不再受支持，也不会由当前产品读取、接管或删除。Claude 骨架、host switch 和双骨架
一致性机制已经退役；项目中发现 Claude 遗留文件时只报告存在，不读取或修改。

详细入口：[文档索引](doc/README.md)、
[设计依据](doc/0_architecture/design/design-rationale.md)、
[原生指令架构](doc/0_architecture/design/codex-native-instruction-architecture.md)、
[Codex 项目同步设计](doc/0_architecture/design/codex-project-sync.md)。

## License

MIT
