# Bundled safety assets

These files let the Windows x64 desktop build run NSFW scanning without a separately installed
ONNX Runtime:

- `open_nsfw.onnx` was extracted from `opennsfw-standalone==0.0.6`, which packages an ONNX
  conversion of Yahoo's `open_nsfw` model. SHA-256:
  `864bb37bf8863564b87eb330ab8c785a79a773f4e7c43cb96db52ed8611305fa`.
- `onnxruntime.dll` is the Windows x64 CPU runtime from ONNX Runtime `v1.22.0`. SHA-256:
  `579b636403983254346a5c1d80bd28f1519cd1e284cd204f8d4ff41f8d711559`.

The model is distributed under the Yahoo `open_nsfw` BSD-2-Clause license in
`LICENSE-open-nsfw.md`. ONNX Runtime is distributed under the MIT license in
`LICENSE-onnxruntime.txt`.

The Tauri resource map places this directory at `$RESOURCE/safety`, matching the desktop host's
runtime lookup path.
