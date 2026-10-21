# 项目自有 Rust Hook

项目 Hook 的源码、依赖、入口、构建、安装和注册归项目所有。普通骨架 update 只保留它们，不借用骨架 workspace 编译，不检查项目开发产物是否最新。公共 Hook 由骨架维护，两者只共用 Codex 的事件配置，不共享业务实现或构建依赖。

## 独立工程

已有 project_demo 可在 .codex/hooks/project_demo/ 维护自己的 Cargo.toml、Cargo.lock 和 main.rs。以下 manifest 的 [workspace] 使工程独立于父级骨架 workspace：

    [package]
    name = "project_demo"
    version = "0.1.0"
    edition = "2024"

    [workspace]

    [[bin]]
    name = "project_demo"
    path = "main.rs"

项目自行声明第三方依赖，不使用 bridgeforge_core 或骨架目录中的源码。Windows main 使用 #![cfg_attr(windows, windows_subsystem = "windows")]；启动子进程也需保持隐藏窗口、stdin/stdout/stderr、退出码及超时/子进程树语义。迁移时不能只替换成缺少超时处理的 Command::output。

在项目开发流程中生成并保存锁文件，用项目自己的 cargo build --locked --release 入口构建，验证原业务行为，再安装到 .codex/bin/project_demo.exe。不依赖下一次骨架升级来补构建。项目可采用自己的目录和构建脚本，但要自己维护完整调用链。

## 注册与调用

项目直接维护 .codex/hooks.json 中自己的 native handler，例如：

    {
      "hooks": {
        "SessionStart": [
          {"hooks": [{
            "type": "command",
            "command": ".codex/bin/project_demo check",
            "commandWindows": ".codex/bin/project_demo.exe check",
            "timeout": 150
          }]}
        ]
      }
    }

将自己的条目合入已有配置，不覆盖公共条目。bridgeforgeCodexId 的 bridgeforge-codex.project-hook.v1: 前缀归骨架所有，项目不能冒用。不同项目各自注册并调用自己的程序，不在用户级重复登记公共项目 Hook。

旧 .codex/project-hooks.json、bridgeforgeProjectHookId 等登记可在迁移完成前保留供一次性识别；新工程不需要骨架生成注册。公共更新不能因项目修改业务 Hook 就自动重写项目注册。

## 旧版本的一次性兼容迁移

旧单文件模式曾借用骨架依赖，普通 update 按固定声明提示一次。仅发现旧耦合不阻断，程序与注册原样保留；只有目标升级明确破坏原运行合同才阻断。骨架版本、workspace 哈希变化或“已经提示”都不能作为迁移完成证明。

用户明确授权迁移后，在项目内补齐独立 manifest/lock/main，去掉骨架私有依赖，保持原输入输出、参数、退出码、超时及隐藏启动行为。完整依赖分析、构建和业务验证只在显式迁移中执行；普通 update 不执行项目代码或业务事件。

仍保留 .codex/hooks/project_<id>/、.codex/bin/project_<id>.exe 和可识别旧登记的项目，在自己的构建、安装及行为验收完成后，可用项目 CLI 记录证据：

    .codex/bin/bridgeforge project-hook-migration status --root .
    .codex/bin/bridgeforge project-hook-migration verify --root . --confirmed-behavior

status 只读；verify 是独立维护动作，会核对独立 workspace/依赖、执行锁定构建并与已安装程序比对，成功才保存完成记录。它不修改业务代码、不生成入口、不替用户验证实际业务事件；项目采用不同布局时应按明确的项目迁移方案验证，不能猜测路径。

迁移状态与本机提示去重分别保存，待办跨骨架升版保留，已提示不是已完成。只读预览不保存提示状态；未知状态保留并报告。构建脚本和项目代码是受信任的可执行输入，不能把编译叫作安全沙箱。真实外部写入或删除仍须在明确授权的环境验收。
