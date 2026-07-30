# ONNX Runtime

Atelier bundles the Windows x64 CPU ONNX Runtime dynamic library so image-analysis
models can run without a system installation.

- Version: 1.24.2
- Library: `onnxruntime.dll`
- License: MIT, copied in `LICENSE-onnxruntime.txt`

Image-analysis model weights are not bundled here. Atelier downloads pinned,
SHA-256-verified model revisions into the application's data directory.
