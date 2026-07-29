# CREDITS

- 快照日期：2026-05-20
- 目的：记录本仓库在设计、对齐、实现与专项核对阶段参考过的外部项目、许可证与映射范围。

## 参考清单

- [`Aaalice_NAI_Launcher`](https://github.com/Aaalice233/Aaalice_NAI_Launcher) — MIT；2026-05 Prompt 词库来源资产留痕。参考 Prompt Danbooru 翻译 CSV 资源映射，见 `assets/prompt-lexicon/SOURCES.md`。
- [`NekoAI-JS`](https://github.com/Nya-Foundation/NekoAI-JS) — AGPL-3.0；2026-05 generation 参数设计。参考 Anlas 估算公式与系数，见 `crates/features/generation/src/estimate.rs`。
- [`yahoo/open_nsfw`](https://github.com/yahoo/open_nsfw) — BSD-2-Clause；2026-07 NSFW 桌面资源打包。提供 `open_nsfw` ONNX 模型来源与二分类能力，见 `apps/desktop/src-tauri/resources/safety/README.md`。ONNX 文件来自 `opennsfw-standalone==0.0.6`；模型许可证随资源打包。
- [`opennsfw-standalone`](https://github.com/SectorLabs/opennsfw-standalone) — MIT（包装） / BSD-2-Clause（模型）；2026-07 NSFW 桌面资源打包。参考 Yahoo 模型的 ONNX 转换产物来源，见 `apps/desktop/src-tauri/resources/safety/README.md`。只采用已转换模型资产；Rust 推理实现不复制其代码。
- [`ONNX Runtime`](https://github.com/microsoft/onnxruntime) — MIT；2026-07 NSFW 桌面资源打包。提供 Windows x64 CPU 动态运行库 `v1.24.2`，见 `apps/desktop/src-tauri/resources/safety/README.md`。运行库许可证随资源打包；无需系统级安装。
