# Safety ONNX Adapter

## Status

- Date: 2026-05-21
- Status: Implemented direction

## Decision

`crates/adapters/safety-onnx` provides an optional `SafetyScanner` implementation for OpenNSFW-style ONNX models. The adapter is host-neutral: callers pass the model path and ONNX Runtime library path explicitly. The app layer accepts the scanner as an injected dependency.

`apps/desktop/src-tauri` owns desktop path resolution for host-provided or bundled safety assets. The Tauri host resolves those assets on startup, builds the optional scanner, and injects it into the host-neutral `app` facade.

## Boundaries

- No model file or ONNX Runtime binary is committed by this backend crate.
- The adapter returns `None` when no runtime assets are configured.
- Safety scanning receives resource bytes from app/kernel ports, not direct filesystem paths.
- Scan output preserves raw `safe` and `nsfw` model scores when present, with `nsfw` used as the canonical risk score.

## Source Record

- Expected model family: `yahoo/open_nsfw` (`open_nsfw`), BSD-2-Clause.
- Common ONNX conversion source: `opennsfw-standalone`, MIT wrapper / BSD-2-Clause model lineage.
- If a desktop bundle ships an ONNX model or runtime library, that bundle must carry the matching upstream license and source notes with the artifact.
