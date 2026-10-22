# schema 4 发布分析

## 输入与分析

先运行不带分析的 `--release-preview`，读取 fingerprint、changes、affected_packages、各包现版本和 required_checks。按最后正式发布与当前最终内容审阅实际差异；Git 标题只用于提交说明，不作为全包统一判级依据。

本 reference 继承根 Skill 的调用级一次确认上限和执行环境合同，不重新开启确认轮次。最终无分析预览、完整分析生成、带分析预览与实际发布在同一宿主进程、最终权限/沙箱、账户、环境及工具链上下文中连续完成；跨进程的前置调查仅供语义审阅，最终段须核对当前实际差异与已审阅事实一致。若中途提权、进程中断或环境变化，旧分析作废，整段重新预览并生成完整分析。不得只改旧 fingerprint、复用失效验证声明或通过修改 TEMP/TMP/LOCALAPPDATA 等变量伪装环境。

在项目忽略的 `.runtime/release-analysis.json` 保存：

```json
{
  "schema": 1,
  "fingerprint": "<输入预览返回的完整值>",
  "product": {"level": "minor", "reason": "本次程序新增兼容功能", "compatibility": "compatible"},
  "packages": {
    "stable-package-id": {"level": "patch", "reason": "该包仅修复自身行为", "compatibility": "compatible"}
  },
  "targets": {},
  "checks": []
}
```

仅列实际受影响或明确指定纯升版的包；移除包仍须分析影响，但不生成虚假新版本。若带分析预览指出调用方有效依赖要求必须改变，审阅该实际变化并补上调用方分析。不能仅因存在依赖边就主动带升所有包。

调用方文件及依赖要求没改，但转发接口确有变化时，另提供 `interface_impacts`，例如 `{"caller-id":{"dependencies":["changed-dependency-id"],"reason":"转发的公开类型新增方法"}}`。同时提供调用方自己的 packages 判级；CLI 核对基线/当前依赖和本次实际变化，允许已说明的转发链，不自动传播。无相关变化、无依赖或无理由的声明拒绝；纯版本目标不能充当接口变化的起因。语义影响仍须 Agent 根据真实公开合同判断。

level 为 patch/minor/major；程序级别覆盖最高实际影响。修复、重构、注释、格式、文档一般 patch，兼容功能 minor，不兼容接口/数据/使用要求 major；说明必须与事实一致。源码差异不能自动证明语义兼容；不确定使用 unknown 并停止，不伪造 compatible。

## 显式版本及不兼容

`targets.product` 指定程序目标，其他键使用稳定包 id。程序和包可同时指定多个目标；低于建议、未知目标、非递增或复用旧版本直接拒绝。无实际内容变化的目标必须有顶层 `pure_version_reason`；无目标且无发布变化为 no-op。统一策略只接受程序目标，包跟随该版本。

Python 保留 PEP 440 原生版本语义；非规范稳定三段版本需要明确目标，不猜测升版。兼容性为 breaking 时 level 必须 major，并在对应对象记录 acceptance：本次 fingerprint、scope（含该对象 id，程序为 product）、用户接受的 reason 及可核对的 source。只复用适用的用户明确接受，不以 major 或发布命令代替不兼容接受。

所有 breaking 对象与策略、迁移/补正及其他必要决定合并为一次确认，不逐包轮流问。环境或 Git 状态变化后，只有实际影响、scope 和接受理由仍被原用户决定完整覆盖，才可把该决定及原来源记录到新分析对应的 acceptance；这不代表旧验证证据适用于新环境。出现新的实质风险或决定不完整时停止，不能以一次确认上限为由自动接受，也不能在已确认后追加第二问。

## 项目验证证据

`record_inputs` 首期只接受明确的 JSON/JSONL 自动记录文件；登记不代表可以把实际参与业务的 JSON 资源排除。源码、版本声明、锁、构建产物和业务生成资源继续按实际职责归属。

只核对项目显式 required_checks 及实际已提供证据，不组织全量测试、不要求工厂 prepared/审计 JSON。每项证据包括 id、status、fingerprint、scope、reason、artifact（项目 `.runtime/` 相对路径）、sha256（`sha256:` 前缀）。这里 check.fingerprint 使用预览的 validation_fingerprint：绑定最终源码、依赖、构建输入及环境，排除 VERSION、CHANGELOG 和声明的自动记录，不因暂存或普通提交本身变化。分析顶层与不兼容 acceptance 仍使用完整 fingerprint，防止 Git 事务漂移。artifact 是实际项目验证输出的结构化记录，id/status/fingerprint/scope/reason 必须与声明一致，摘要匹配真实字节。Agent 不得补造 passed 记录。

passed 必须覆盖适用范围；failed、未知或新失败阻断。historical-unrelated 必须有实际依据，scope 不得覆盖本次影响对象，也不能替代必需检查的通过结果。证据失效时停止依赖它的发布，复用仍有效部分。

配置可用 `check_scopes` 指定各 required_checks 的适用对象，例如 `{"unit-a":["a"],"unit-b":["b"],"integration":["product"]}`。未指定的检查保持全局范围；没有受影响对象的局部检查无需重跑。每项适用的必需检查必须通过并覆盖自身范围，通过证据合计覆盖本次影响对象；不能把单包测试虚报成全项目证据。

Python 项目提供验证证据时，还须在配置明确 `python_environment`：`interpreter` 为实际解释器文件，`runtime_roots` 为实际运行时/标准库目录列表，`dependency_roots` 为验证使用的依赖目录列表。路径可为项目相对路径或绝对路径；必须覆盖虚拟环境、基础运行时，以及 editable/local 依赖的外部实际目录。Forge 只读计算这些文件的内容身份，不执行 Python、安装脚本或 backend；新增/删除/修改文件、原路径替换解释器或更新依赖都会使旧证据失效，应用前再次核对。目录内字节码也参与身份，遇到未声明实际目录的嵌套链接则停止。未配置环境时可查看包分析，但不得把 Python 的 passed 记录当作有效发布证据。

## 原生格式边界

Rust 使用显式 Cargo 包/workspace、依赖约束和锁；Python 支持现有静态 PEP 621、setup.cfg、传统 Poetry、明确 text/TOML/静态字符串版本来源及 local_sources。不得执行任意 setup.py、SCM 动态版本后端或安装脚本。Cargo 3/4、uv、Poetry、PDM、pylock 和 requirements-export 沿用适配器明确支持边界；未知格式、无法证明的条件或制品阻断，不升级第三方依赖来凑通过。

所有 Cargo.lock 和已声明 Python lock 不直接升包；全局 build_inputs 不能覆盖包源码/原生声明；record_inputs 只用于明确自动记录，业务生成源码/资源不能排除。包改名、移动、移除、拆分/合并保留稳定身份、显式生命周期和历史，退役身份不复用。
