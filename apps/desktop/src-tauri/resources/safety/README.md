# Bundled safety assets

These files let the Windows x64 desktop build run NSFW scanning without a separately installed
ONNX Runtime:

- `open_nsfw.onnx` was extracted from `opennsfw-standalone==0.0.6`, which packages an ONNX
  conversion of Yahoo's `open_nsfw` model. SHA-256:
  `864bb37bf8863564b87eb330ab8c785a79a773f4e7c43cb96db52ed8611305fa`.
- `onnxruntime.dll` is the Windows x64 CPU runtime from the official ONNX Runtime `v1.24.2`
  release archive `onnxruntime-win-x64-1.24.2.zip`. The archive SHA-256 is
  `8e3e9c826375352e29cb2614fe44f3d7a4b0ff7b8028ad7a456af9d949a7e8b0`; the extracted DLL
  SHA-256 is `114947d633e6844ce3c4b51ef6678f776628571d08a5763859c61642c8dcca9c`.

The model is distributed under the Yahoo `open_nsfw` BSD-2-Clause license in
`LICENSE-open-nsfw.md`. ONNX Runtime is distributed under the MIT license in
`LICENSE-onnxruntime.txt`.

The Tauri resource map places this directory at `$RESOURCE/safety`, matching the desktop host's
runtime lookup path.

The native smoke test is intentionally excluded from the default unit-test suite. Run it in its
own process on Windows with:

```powershell
cargo test -p atelier-adapter-safety-onnx tests::bundled_onnx_smoke_test -- --ignored --exact --test-threads=1
```
