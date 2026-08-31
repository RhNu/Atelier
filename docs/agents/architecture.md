# Architecture Overview

## Status

- Date: 2026-05-20
- Status: Current guidance

This document is the single architecture and backend layout overview for Atelier. It replaces the older split between general architecture guidance, crate layout planning, and rollout planning.

Names and module boundaries can still change when implementation proves a better shape, but changes should preserve the dependency direction and I/O boundaries described here.

## Product Boundary

Atelier is a desktop workspace for NovelAI image workflows. Internal language should stay close to the product domain: workspace, prompt, prompt resource, generation work, job, artifact, gallery item, Vibe document, Director result, resource reference, and safety assessment.

NovelAI protocol details belong behind the `novelai-bridge` adapter. The application should not become a generic provider abstraction unless a separate design note proves that need.

## Layer Shape

The backend follows a feature-first shape:

```text
foundation -> features -> kernel
foundation -> app-api
features/kernel/app-api/adapters -> app
app -> desktop host / frontend
```

Current implementation includes host-neutral `app` and frontend-facing `app-api` crates. Backend work is concentrated in `foundation`, feature crates, `kernel`, adapters, and the host-neutral app facade.

Hard boundaries:

- Feature crates own domain models, rules, ports, and tests.
- Feature crates do not perform real filesystem, database, keyring, HTTP, Tauri, clipboard, or notification I/O.
- `kernel` owns runtime state, events, queue orchestration, and cross-feature workflows.
- `kernel` does not call `novelai-bridge` directly and does not own persistence schema.
- Adapters implement ports and own concrete I/O.
- Tauri is a host adapter, not the application layer. It owns native dialogs, user-selected local file reads/writes, open/reveal guards, notifications, and desktop bundled resource path resolution.

### `novelai-bridge` in `features/generation`

`features/generation` is the one feature crate allowed to depend on `novelai-bridge`, and only for
its model catalog knowledge: `Model`, `ModelCapabilities`, `PromptStructure`, and the pricing entry
points. `ImageModel::bridge_model` is the single crossing point; everything else in the crate reads
capabilities through `ImageModel::capabilities`.

The rule this replaces (capabilities restated in Atelier and guarded by a drift test) produced two
capability tables that could disagree silently. Delegation makes upstream the only source.

`Client`, `Transport`, and every other request/response type stay behind `adapters/novelai`. No other
feature crate may add the dependency.

### Public Explore gallery

`features/explore` owns read-only discovery contracts and source query rules. The app exposes one
search/detail/media entry point with source-tagged DTOs for Danbooru Database and NovelAI Explore
Gallery. Source-specific content and transports remain separate; remote posts are not local artifacts.

The undocumented Explore protocol is an explicit Atelier-only exception to the bridge integration
rule: `adapters/novelai-explore` uses a fixed-host anonymous HTTP client, never reads generation
credentials, and handles nested metadata JSON itself. It must not expand the public `novelai-bridge`
API. Stable generation integration continues through the bridge adapter. Explore caches are ephemeral,
source/account scoped, and independent of workspace persistence. Source approval is not a local safety
assessment. See `docs/novelai-explore-read-only-research.md` for the observed public protocol.

## Current Workspace Layout

```text
crates/
  foundation/
  kernel/
  app/
  app-api/

  features/
    danbooru/
    explore/
    artifacts/
    director/
    downloadable-resources/
    gallery/
    generation/
    image-analysis/
    jobs/
    precise-reference/
    prompt/
    prompt-resources/
    resource-catalog/
    safety/
    secrets/
    settings/
    vibe/
    workspace/

  adapters/
    danbooru/
    novelai-explore/
    database/
    downloadable-resources-fs/
    image-codec/
    image-analysis-onnx/
    keyring/
    novelai/
    secrets-fs/
    settings-fs/
    storage-fs/

apps/
  desktop/src-tauri/
```

## Core Responsibilities

### `foundation`

Stable cross-feature primitives and small support types. It should not contain feature DTOs such as generation params, gallery items, prompt resources, or Vibe records.

### Feature Crates

Feature crates are the default owner for domain concepts:

- `workspace`: workspace root, layout, controlled paths, lock, manifest.
- `resource-catalog`: resource IDs, kinds, owners, lifecycle, blob and variant ports.
- `downloadable-resources`: application-global reconstructable runtime resource descriptors,
  groups, install state, resolution, and leases.
- `prompt`: NovelAI-oriented prompt syntax, formatting, functions, diagnostics.
- `prompt-resources`: chunks, prompt functions, compile trace, prompt resource ports.
- `generation`: NovelAI image generation params, normalization, Anlas estimate result model, request plan, generation client ports. Model capabilities and pricing formulas stay in `novelai-bridge`; capabilities are re-exported through `ImageModel::capabilities`, and pricing is invoked by `adapters/novelai`.
- `jobs`: job and batch state machine, retry/cancel policy, queue events.
- `artifacts`: generated artifact semantics, replay manifest, visual asset references.
- `gallery`: gallery item index model, query, source references, explicit unscanned/scanned/failed/
  unavailable safety state, and manual safety override.
- `image-analysis`: model-neutral rating and tag evidence plus analyzer/session-control ports.
  Downloading and package lifecycle belong to `downloadable-resources`. Rating-only requests avoid
  allocating WD general and character tag names.
- `vibe`: Vibe document import/export, encoding records, cache keys, Vibe client ports.
- `director`: Director tool request and client port.
- `safety`: versioned rating-cascade policy assets, primary/review evidence,
  manual-override-compatible labels, and scanner/policy-control ports. Runtime decisions and
  production-parity tooling consume the same immutable policy version. It consumes
  `image-analysis` rather than binding its ports to a specific tagger.
- `secrets`: API key registry, active key semantics, secret resolver, subscription probe.
- `precise-reference`: precise reference input processing and image reader port.
- `settings`: user-level application preferences and workspace-local NovelAI generation/image defaults, with separate repository ports for each persistence scope.

### `kernel`

`kernel` combines feature services and ports into explicit workflows. The current runtime exposes generation, streaming, Vibe, and precise-reference workflows. It emits events and records failure details, but keeps storage and external services behind ports.

### Adapters

Adapters are the boundary for real I/O:

- `storage-fs`: workspace filesystem operations, locks, resource blob storage.
- `database`: SQLite-backed repositories and adapter-local JSON DTOs.
- `image-codec`: PNG/JPEG/WebP probing plus deterministic gallery/export variant encoding.
- `keyring`: system credential storage for secret values.
- `novelai`: `novelai-bridge` integration and resolver-backed NovelAI clients.
- `downloadable-resources-fs`: HTTPS catalog caching, ranged downloads, mirror fallback,
  SHA-256 verification, atomic activation, and lease-aware deletion below the app data directory.
- `image-analysis-onnx`: resolves verified dbrating and WD resources, then owns preprocessing,
  lazy ONNX sessions, rating extraction, and future general/character tag output.
- `settings-fs`: user-level global settings stored below the desktop host-provided application configuration directory.
- `secrets-fs`: application-level NovelAI API key metadata stored below the desktop host-provided
  application configuration directory. Secret values remain in the system keyring.

Persistence and secret boundaries are adapter contracts:

- `adapters/database` owns the SQLite schema, exact format/version validation, and adapter-local
  JSON DTOs. New databases are created directly at the current schema. Each supported upgrade is
  an isolated `schema/migrations/vN_to_vN+1` module with one transaction and focused boundary
  tests; migration modules can be dropped when their input version is no longer supported. Future,
  unknown, unmarked, and wrong-format schemas are rejected. Global settings follow the same
  one-version-boundary rule below `settings-fs/src/migrations`. Feature and application models
  must not become persistence schemas through serialization derives.
- `features/secrets` stores application-level key metadata through its registry port. The
  `secrets-fs` adapter persists that metadata independently of any workspace. Secret values go
  through `SecretStore` and the keyring adapter, and must not appear in workspace SQLite, API
  responses, events, history, logs, or diagnostics. Workspace database migrations delete legacy
  API key metadata without importing it into application storage.

Desktop host glue lives inside `apps/desktop/src-tauri`, not a reusable adapter crate. The frontend should invoke Tauri commands that perform picker, read, write, and host actions together; it should not directly read arbitrary user files through frontend filesystem capabilities.

External library types should not leak upward into feature crates, `kernel`, or `app-api`.

### `app` and `app-api`

`app-api` should hold frontend-visible contracts only: request/response DTOs, event DTOs, error envelopes, pagination, and query DTOs.

`app` should be host-neutral. It should map `app-api` DTOs to feature/kernel inputs, hold runtime state, inject adapters, apply runtime guards, and expose use case groups. It must not depend on Tauri.

The process-level `AtelierRuntime` owns global settings, the application-level API key registry,
event listeners, injected external dependencies, and an optional `WorkspaceSession`. A
`WorkspaceSession` owns only services and runtime state tied to one opened workspace, while its
NovelAI adapter resolves the active key through the shared application registry. Opening a
replacement workspace builds the candidate session and persists its recent-workspace state before
publishing it.

## Resource Rule

Workspace-owned durable binary or semi-structured creative resources must go through
`resource-catalog`.

Feature crates may store `ResourceRef` plus feature-owned metadata. They should not create long-lived resource directories, encode physical path rules, or maintain private binary indexes.

Large application-level data and models are global, SHA-256-verified, reconstructable runtime
assets managed by `downloadable-resources` below
`app_data_dir/downloadable-resources/<id>/<version>`. Consumers resolve verified directories and
retain leases while files may be mmap-backed or held by ONNX sessions. They do not enter the
workspace `resource-catalog`.

Lexicon core and optional semantic capabilities have separate failure boundaries. Invalid semantic
metadata, files, or model initialization must leave lexical search, completion, and entity lookup
available, with semantic failures exposed through capability status. Download catalogs verify exact
transport bytes; runtime checks preserve strict binary integrity and vector dimensions, identify
tokenizers by their bounded decoded payload, and accept only SHA-256-proven LF/CRLF equivalence for
license text. Readable text alone is not evidence of integrity.

The stable catalog uses HTTPS for authenticity and per-file SHA-256 for integrity. It is not
separately signed. This intentionally accepts compromise of the HTTPS publication path as a trust
risk; application updater artifacts remain independently signed by Tauri.

## Generation Output Ownership

Generation history, Artifacts, and Gallery entries describe different facts and must not be treated
as interchangeable owners:

- A generation batch is the durable task created by one Generate action. Its request records and
  expected output slots remain valid history even when an output is later deleted.
- An Artifact owns the durable output provenance, including the request-level resolved seed,
  per-image embedded seed, replay prompt snapshots, and best-effort NovelAI metadata diagnostics.
- A Gallery item is an index/projection over an Artifact. It is not the source of generation
  provenance.
- Hard-deleting a Gallery item may remove its Artifact and unreferenced resource blobs, but the
  corresponding generation output record becomes a deleted tombstone instead of disappearing or
  retaining a loadable resource reference.
- Deleting generation task history does not implicitly delete Artifacts or Gallery entries. Output
  deletion must remain an explicit Gallery/Artifact action.

## Verification Boundary

Feature and kernel tests should run without Tauri, real filesystem access, real NovelAI access, or OS keyring access whenever possible. Real adapters should have focused contract or integration tests and preserve fake or in-memory replacements.

## Implementation Guidance

When adding a use case, answer:

- Which feature owns the domain model?
- Is a `kernel` workflow needed?
- Which ports are required?
- Does this introduce real I/O?
- Does it need `resource-catalog`?
- Can the main behavior be tested without Tauri and real external services?

When adding an adapter dependency, answer:

- Is the dependency limited to `adapters/*` or desktop host glue?
- Is there a fake or in-memory replacement?
- Are external errors converted at the adapter boundary?
- Are external library types kept out of feature crates, `kernel`, and `app-api`? The single
  exception is `novelai-bridge` model capability types in `features/generation`; see
  [`novelai-bridge` in `features/generation`](#novelai-bridge-in-featuresgeneration).
