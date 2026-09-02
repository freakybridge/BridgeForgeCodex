# bridgeforge-codex

> 为长期维护的 Codex 项目安装一套可升级、可检查、可回滚的协作骨架。

bridgeforge-codex 让 Codex 在不同项目中使用一致的基础规则、工作流程和安全检查，
同时保留每个项目自己的约束与定制。日常使用只需要记住一个入口：在目标项目根目录运行
`$bridgeforge-codex`。

它主要完成三件事：

- 为项目安装或更新 `AGENTS.md`、`.codex/`、文档结构和提交前检查等协作设施。
- 为用户安装 `confirm`、`develop`、`summary`、`git-sync` 等通用 Skills。
- 每次以最新正式版为完整基线；旧 Rule / 项目 Memory 逐文件确认迁移，无法安全判断时停止，验证失败则不写入或完整回滚。

bridgeforge-codex 只支持 Windows、Rust/Cargo 1.88+ 和 Codex。安装、同步、校验、Git、
Native Memory 与受管项目 Hook 全部由 Rust 承载，不要求 Python 或项目 `.venv`。

## 快速开始

先安装 bridgeforge-codex：

```powershell
git clone https://github.com/freakybridge/BridgeForgeCodex.git "$env:USERPROFILE\tools\bridgeforge-codex"
Set-Location "$env:USERPROFILE\tools\bridgeforge-codex"
cargo build --locked --release --manifest-path .\templates\hooks\Cargo.toml
& powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\install-shared-skills.ps1
```

然后新开 Codex 会话，在需要安装或更新骨架的项目根目录运行：

```text
$bridgeforge-codex
```

首次安装、旧版迁移和异常诊断分别见[安装与迁移说明](INSTALL.md)。

## 文档入口

需要了解当前架构、当前交付、开放 Bug 或历史资料时，从唯一完整导航
[文档索引](doc/README.md)开始；根 README 不复制这些动态状态。

## 它会修改什么

用户级位置：

- `~/.bridgeforge-codex`：完整、可信的产品仓库。
- `~/.codex/skills/bridgeforge-codex`：Codex 用来发现产品的薄入口。
- `~/.codex/skills/`：由产品维护的通用 Skills。
- `~/.codex/memories/`：Codex 原生 Memory；用户授权后可作为不透明目录跨电脑同步，
  同步器不解析正文语义；summary 可只读检索、阅读并提出建议，不改写正文。

目标项目：

- 根 `AGENTS.md`：公共区由产品更新；项目级专区逐字保留。
- `.codex/hooks/`、`.codex/bin/bridgeforge-hook.exe` 与 `.codex/bin/bridgeforge.exe`：受管 Rust
  源码与编译产物；`init`、`adopt`、`update` 及工厂 `$git-sync` 维护提交时构建，日常入口不启动脚本解释器。项目自有 Hook 继续使用
  `.codex/hooks/project_*/` 扩展目录，不与受管 Rust 源码混写。
- `.githooks/pre-commit`：执行提交前机器检查。
- `doc/`：以 `doc/README.md` 为索引的五层文档结构。
- 项目自己的嵌套 `AGENTS.md`、项目规则和明确保留的定制资产仍归项目所有。
- 旧项目的 `.codex/rules/*.md` 与 `.codex/memory/`：不再运行；`$bridgeforge-codex` 逐文件确认后迁往正确资产并在同一事务退役旧源。

## 更新时怎样保护项目

- 新项目直接安装当前骨架。
- 已有项目直接面向最新完整基线生成计划；每个 legacy Rule / Memory 源只确认一次，确认前零写入，不会静默替用户做选择。
- 无戳空项目初始化；无戳已有资产进入受控接入。双戳、非法戳、更新期间文件漂移或最终基线验证失败时停止或回滚。
- 同版本骨架需要修复时仍可生成计划；覆盖已有整文件资产需要确认，始终使用可信产品目录的当前基线。

不同项目版本的具体处理方式见[安装与迁移说明](INSTALL.md#项目升级与异常诊断)；内部同步
合同由[Codex 项目同步设计](doc/0_architecture/design/codex-project-sync.md)维护。

## 仓库结构

```text
BridgeForgeCodex/
├── .codex/                  # 本仓库 dogfood 配置、Rust 源码与生成资产
├── skills/bridgeforge-codex/ # $bridgeforge-codex 产品入口
├── skills/                   # 通用 Skills
├── templates/               # 唯一活跃下游模板根
├── scripts/                 # PowerShell 用户级安装入口
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

## License

MIT
