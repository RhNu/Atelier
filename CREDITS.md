# CREDITS

- 快照日期：2026-07-30
- 目的：记录本仓库在设计、对齐、实现与专项核对阶段参考过的外部项目、许可证与映射范围。

## 参考清单

- [`DanbooruSearchOnline`](https://github.com/SuzumiyaAkizuki/DanbooruSearchOnline) — GPL-3.0；2026-07 Danbooru 词库重构研究。Atelier 的 Python、Rust 与 TypeScript 实现均为重新实现，未复制源代码。词库管线另行再分发了固定提交中的五个上游数据文件，完整许可证、来源提交和文件校验和保存在 `tools/lexicon-pipeline/sources/danbooru-search-online/`。
- [`danbooru-tag-pipeline`](https://github.com/SuzumiyaAkizuki/danbooru-tag-pipeline) — GPL-3.0；2026-07 Danbooru 词库重构研究。仅参考精选、翻译、Wiki、分组和共现管线设计；具体来源与快照由每个 bundle 的 `manifest.json` 记录。
- [Danbooru](https://danbooru.donmai.us/) — Danbooru 数据库条目及标签元数据；实际数据许可、API 快照时间和校验和随 bundle provenance 保存。
- [`multilingual-e5-small`](https://huggingface.co/intfloat/multilingual-e5-small) — MIT；固定 revision `614241f622f53c4eeff9890bdc4f31cfecc418b3` 的 qint8 ONNX 模型用于本地语义检索，许可证随 bundle 打包。
- [`NekoAI-JS`](https://github.com/Nya-Foundation/NekoAI-JS) — AGPL-3.0；2026-05 generation 参数设计。参考 Anlas 估算公式与系数，见 `crates/features/generation/src/estimate.rs`。
- [`deepghs/anime_dbrating`](https://huggingface.co/deepghs/anime_dbrating) — 模型卡只标识 `openrail` 许可证家族，未给出可据以再分发的确定条款。Atelier 不随安装包分发该模型；应用按用户运行需要下载固定 revision `7af21db648acdeb74f5c334abda9dd7403407b3c`，并校验文件大小与 SHA-256。
- [`SmilingWolf/wd-swinv2-tagger-v3`](https://huggingface.co/SmilingWolf/wd-swinv2-tagger-v3) — Apache-2.0；可选 WD 自动复核及未来图像标签能力。Atelier 下载并校验固定 revision `627aef95638667ddcaa3ac8ae625e88ea5b02f51`，不跟随仓库 head，且不随安装包预装模型权重。
- [`ONNX Runtime`](https://github.com/microsoft/onnxruntime) — MIT；提供 Windows x64 CPU 动态运行库 `v1.24.2`，见 `apps/desktop/src-tauri/resources/onnx-runtime/README.md`。运行库许可证随资源打包；无需系统级安装。
