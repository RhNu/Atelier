# Prompt lexicon workflow 决策记录

## 状态

- Date: 2026-05-20
- Status: Implemented direction

## 决策

`features/prompt-lexicon` 拥有 NovelAI prompt 词库的运行时读取、catalog、browse 与 search 规则。它是纯 feature crate，不接 Tauri、不暴露 `app-api` DTO，也不执行文件系统 I/O。

词库来源资产放在 `assets/prompt-lexicon/sources`，生成产物放在 `assets/prompt-lexicon/generated/lexicon.json`。生成产物提交入仓，运行时通过 `PromptLexicon::load_embedded()` 读取。

生成工作流由 Rust `xtask` 负责：

```powershell
cargo xtask lexicon build
cargo xtask lexicon check
```

生成格式是本仓 v1：

- `schema = "nai-atelier-prompt-lexicon"`
- `version = 1`
- compact arrays: `categories`, `subcategories`, `tags`, `translations`
- `sources` metadata copied from the source manifest for auditability
- generated statistics at the 2026-05-20 snapshot: 228304 tags, 310957
  translations, 16 categories, 333 subcategories

## 与 `nait` 的关系

本阶段复制 `nait` 的静态词库来源资产和整体构造规则，但不复制旧 `nait` 的 JS 构造脚本、Rust application-service、Tauri commands 或 Vue 前端实现。

本次导入使用的本地 `nait` prompt lexicon 资产快照提交为
`4d5fe9abab48f942033288c439625ed3b2360dc5`。`nait` 工作区当时有其他
未提交文件，但 prompt lexicon source/generated 资产的最后变更提交是该
commit。

`nait` 的来源留痕提到：

- `Aaalice_NAI_Launcher`: Prompt Danbooru 翻译 CSV 资源映射。
- `nai-codex`: Prompt 词库生成流程、自动补全契约、Lexicon 浏览与 assembly 交互映射。

这些来源同步记录在 `assets/prompt-lexicon/SOURCES.md` 与 `CREDITS.md`。

## 构造规则

`translation-sources.json` 控制 CSV 来源顺序、parser、priority 与 primary translation 权限。当前支持：

- `weighted_csv`
- `simple_csv`
- `reversed_csv`
- `github_csv`
- `alias_csv`

分类 JSON 先建立 category/subcategory 与最高优先级 primary translation。Manifest sources 再按 priority 降序合并 translation 与 weight。`alias_only` 来源只贡献 alias，不参与 primary translation 选择。

`category-order.json` 固定 `nait` 生成词库的 category/subcategory 浏览顺序，避免 Windows 文件系统顺序、Unicode codepoint 排序或 JSON map 排序改变用户可见 Lexicon catalog。

CSV parser 会丢弃列数不足的 malformed 行。`github_csv` 同时兼容 `nait`
复制来的 4 列形态和本仓压缩后的 2 列形态；压缩形态只保留实际用于生成的
`tag` 与 `danbooru_translation`，不提交 Danbooru wiki 正文和 URL 列。

搜索规则沿用 `nait` 的用户可见行为：

- `_` 与空白在匹配中等价。
- exact 优先于 prefix，prefix 优先于 substring。
- tag 命中优先于 primary translation，primary translation 优先于 alias。
- 同级结果按 weight 降序，再按 tag 升序。

Rust CSV parser 会丢弃旧 JS 简易 parser 误切出的 malformed 行。例如旧产物中由坏 CSV 行产生的 `right? Incidentally`、以及 CSV header/坏行产生的 `tag` 不进入本仓 v1 产物。

## 非目标

- 本决策初版不新增 `app-api`、`app` 或 Tauri command；当前实现已经通过 `app-api`、`app` command facade 和 Tauri command 暴露 embedded lexicon catalog/list/search。
- 前端 Lexicon 页面仍不是本文决策范围。
- 不把词库做成通用多模型 tag 平台。
- 不在 feature crate 内做文件 I/O、下载、增量更新或用户自定义词库编辑。
- 不通过 SQLite 持久化词库；`ResourceKind::LexiconBundle` 仍保留给后续 workspace-scoped/imported bundle 设计。
