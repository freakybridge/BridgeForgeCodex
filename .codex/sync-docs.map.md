<!-- bridgeforge-project-map schema=1 kind=sync-docs input=sha256:74715a89bf2ccd10abbcebff7d30310dd4bef4de3635e58739899af80a576e25 -->
# sync-docs 项目自动索引

> 此文件由 BridgeForge 自动生成，禁止手工维护。
> 输入指纹：`sha256:74715a89bf2ccd10abbcebff7d30310dd4bef4de3635e58739899af80a576e25`

## source_to_docs

| 源码路径 | 有明确引用的既有文档 |
|---|---|
| `AGENTS.md` | `doc/0_architecture/design/codex-native-instruction-architecture.md`<br>`doc/0_architecture/design/codex-project-sync.md`<br>`doc/0_architecture/design/design-rationale.md`<br>`doc/0_architecture/design/reverse-sync-playbook.md`<br>`doc/0_architecture/design/sync-from-upstream-playbook.md`<br>`doc/3_reference/codex-project-operating-guide.md`<br>`doc/README.md` |
| `INSTALL.md` | `doc/README.md` |
| `bridgeforge-codex-manifest.json` | `doc/0_architecture/design/design-rationale.md` |
| `scripts/**` | `doc/0_architecture/design/design-rationale.md`<br>`doc/0_architecture/design/memory-scoring-design.md` |
| `scripts/tests/**` | `doc/README.md` |
| `skills/**` | `doc/0_architecture/design/codex-project-sync.md`<br>`doc/0_architecture/design/design-rationale.md`<br>`doc/0_architecture/design/reverse-sync-playbook.md` |
| `templates/**` | `doc/0_architecture/design/antifabrication-framework.md`<br>`doc/0_architecture/design/codex-project-sync.md`<br>`doc/0_architecture/design/design-rationale.md`<br>`doc/0_architecture/design/reverse-sync-playbook.md`<br>`doc/README.md` |
| `templates/AGENTS.md` | `doc/0_architecture/design/antifabrication-framework.md`<br>`doc/0_architecture/design/reverse-sync-playbook.md` |
| `templates/doc/README.md` | `doc/0_architecture/design/downstream-document-lifecycle.md` |
| `templates/hooks/**` | `doc/0_architecture/design/antifabrication-framework.md` |
| `templates/managed-skeleton.json` | `doc/0_architecture/design/codex-project-sync.md` |

## source_roots

- `scripts/**`
- `scripts/tests/**`
- `scripts/tests/src/**`
- `skills/create-worktree/**`
- `templates/hooks/crates/bridgeforge-cli/src/**`
- `templates/hooks/crates/bridgeforge-core/src/**`
- `templates/hooks/src/**`

未命中路径时，`$sync-docs` 必须继续使用文档搜索 fallback；禁止据目录同名猜测关系。
