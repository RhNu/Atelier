# CREDITS

- 快照日期：2026-05-20
- 目的：记录本仓库在设计、对齐、实现与专项核对阶段参考过的外部项目、许可证与映射范围。

## 参考清单

| 项目 | 参考仓库 | 许可证 | 参考阶段 | 参考内容 | 对应位置 | 备注 |
|---|---|---|---|---|---|---|
| `Aaalice_NAI_Launcher` | `https://github.com/Aaalice233/Aaalice_NAI_Launcher` | MIT | 2026-05 Prompt 词库来源资产留痕 | Prompt Danbooru 翻译 CSV 资源映射 | `assets/prompt-lexicon/SOURCES.md`, `docs/agents/prompt-lexicon-workflow.md` | 依据只读 `nait` 参考仓 `4d5fe9abab48f942033288c439625ed3b2360dc5` 复制静态词库来源资产；本仓压缩未使用 CSV 列，不复制 Flutter 实现。 |
| `nai-codex` | `https://github.com/RhNu/nai-codex` | MIT | 2026-05 Prompt 词库行为对齐 | Prompt 词库生成流程、自动补全契约、Lexicon 浏览 / assembly 交互映射 | `assets/prompt-lexicon/SOURCES.md`, `docs/agents/prompt-lexicon-workflow.md` | 仅参考流程、浏览顺序和用户可见契约；Rust `xtask` builder 与 feature crate 查询代码为本仓实现。 |
| `NekoAI-JS` | `https://github.com/Nya-Foundation/NekoAI-JS` | AGPL-3.0 | 2026-05 generation 参数设计 | Anlas 估算公式与系数 | `crates/features/generation/src/estimate.rs` | 仅记录公式来源与行为对齐；Rust 实现按本仓类型、错误边界和测试要求重写。 |
