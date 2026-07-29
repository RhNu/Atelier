# CREDITS

- 快照日期：2026-05-20
- 目的：记录本仓库在设计、对齐、实现与专项核对阶段参考过的外部项目、许可证与映射范围。

## 参考清单

- [`DanbooruSearchOnline`](https://github.com/SuzumiyaAkizuki/DanbooruSearchOnline) — GPL-3.0；2026-07 Danbooru 词库重构研究。仅参考多视图检索、别名归一、NPMI 推荐和数据分层思路；Atelier 的 Python、Rust 与 TypeScript 实现均为重新实现，未复制源代码。
- [`danbooru-tag-pipeline`](https://github.com/SuzumiyaAkizuki/danbooru-tag-pipeline) — GPL-3.0；2026-07 Danbooru 词库重构研究。仅参考精选、翻译、Wiki、分组和共现管线设计；具体来源与快照由每个 bundle 的 `manifest.json` 记录。
- [Danbooru](https://danbooru.donmai.us/) — Danbooru 数据库条目及标签元数据；实际数据许可、API 快照时间和校验和随 bundle provenance 保存。
- [`multilingual-e5-small`](https://huggingface.co/intfloat/multilingual-e5-small) — MIT；固定 revision `614241f622f53c4eeff9890bdc4f31cfecc418b3` 的 qint8 ONNX 模型用于本地语义检索，许可证随 bundle 打包。
- [`NekoAI-JS`](https://github.com/Nya-Foundation/NekoAI-JS) — AGPL-3.0；2026-05 generation 参数设计。参考 Anlas 估算公式与系数，见 `crates/features/generation/src/estimate.rs`。
- [`yahoo/open_nsfw`](https://github.com/yahoo/open_nsfw) — BSD-2-Clause；2026-07 NSFW 桌面资源打包。提供 `open_nsfw` ONNX 模型来源与二分类能力，见 `apps/desktop/src-tauri/resources/safety/README.md`。ONNX 文件来自 `opennsfw-standalone==0.0.6`；模型许可证随资源打包。
- [`opennsfw-standalone`](https://github.com/SectorLabs/opennsfw-standalone) — MIT（包装） / BSD-2-Clause（模型）；2026-07 NSFW 桌面资源打包。参考 Yahoo 模型的 ONNX 转换产物来源，见 `apps/desktop/src-tauri/resources/safety/README.md`。只采用已转换模型资产；Rust 推理实现不复制其代码。
- [`ONNX Runtime`](https://github.com/microsoft/onnxruntime) — MIT；2026-07 NSFW 桌面资源打包。提供 Windows x64 CPU 动态运行库 `v1.24.2`，见 `apps/desktop/src-tauri/resources/safety/README.md`。运行库许可证随资源打包；无需系统级安装。
