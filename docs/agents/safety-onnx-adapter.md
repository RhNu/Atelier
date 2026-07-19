# Safety ONNX Adapter

## Status

- Date: 2026-05-21
- Status: Implemented and bundled for Windows x64

## Decision

`crates/adapters/safety-onnx` provides an optional `SafetyScanner` implementation for OpenNSFW-style ONNX models. The adapter is host-neutral: callers pass the model path and ONNX Runtime library path explicitly. The app layer accepts the scanner as an injected dependency.

`apps/desktop/src-tauri` owns desktop path resolution for host-provided or bundled safety assets. The Tauri host resolves those assets on startup, builds the optional scanner, and injects it into the host-neutral `app` facade.

## Boundaries

- The host-neutral backend crate does not own model or runtime files.
- The Windows x64 desktop bundle includes the model and ONNX Runtime under
  `apps/desktop/src-tauri/resources/safety`; other desktop targets remain optional until their
  platform runtime is added.
- The desktop host initializes the process-global ONNX Runtime exactly once before constructing a
  scanner. Reusing the same canonical runtime path is idempotent; attempting to switch paths is an
  error.
- The desktop host leaves the scanner unset when no runtime assets are configured.
- Safety scanning receives resource bytes from app/kernel ports, not direct filesystem paths.
- Scan output preserves raw `safe` and `nsfw` model scores when present, with `nsfw` used as the canonical risk score.
- The native inference smoke test is ignored by the default workspace suite and runs as a dedicated
  process so ONNX Runtime lifecycle failures cannot make unrelated unit tests nondeterministic.

## Source Record

- Expected model family: `yahoo/open_nsfw` (`open_nsfw`), BSD-2-Clause.
- Common ONNX conversion source: `opennsfw-standalone`, MIT wrapper / BSD-2-Clause model lineage.
- Bundled source, version, checksum, and license records live beside the desktop assets.
