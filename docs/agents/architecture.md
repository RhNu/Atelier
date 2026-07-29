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

## Current Workspace Layout

```text
crates/
  foundation/
  kernel/
  app/
  app-api/

  features/
    artifacts/
    director/
    gallery/
    generation/
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
    database/
    image-codec/
    keyring/
    novelai/
    safety-onnx/
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
- `prompt`: NovelAI-oriented prompt syntax, formatting, functions, diagnostics.
- `prompt-resources`: chunks, prompt functions, compile trace, prompt resource ports.
- `generation`: NovelAI image generation params, normalization, Anlas estimate, request plan, generation client ports.
- `jobs`: job and batch state machine, retry/cancel policy, queue events.
- `artifacts`: generated artifact semantics, replay manifest, visual asset references.
- `gallery`: gallery item index model, query, source references, safety override.
- `vibe`: Vibe document import/export, encoding records, cache keys, Vibe client ports.
- `director`: Director tool request and client port.
- `safety`: safety scores, assessments, scanner port.
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
- `safety-onnx`: optional OpenNSFW-style ONNX safety scanner built from host-provided model/runtime paths.
- `settings-fs`: user-level global settings stored below the desktop host-provided application configuration directory.

Persistence and secret boundaries are adapter contracts:

- `adapters/database` owns SQLite schema, migrations, and adapter-local JSON DTOs; feature and
  application models must not become persistence schemas through serialization derives.
- `features/secrets` stores only key metadata through the database port. Secret values go through
  `SecretStore` and the keyring adapter, and must not appear in SQLite, API responses, events,
  history, logs, or diagnostics.

Desktop host glue lives inside `apps/desktop/src-tauri`, not a reusable adapter crate. The frontend should invoke Tauri commands that perform picker, read, write, and host actions together; it should not directly read arbitrary user files through frontend filesystem capabilities.

External library types should not leak upward into feature crates, `kernel`, or `app-api`.

### `app` and `app-api`

`app-api` should hold frontend-visible contracts only: request/response DTOs, event DTOs, error envelopes, pagination, and query DTOs.

`app` should be host-neutral. It should map `app-api` DTOs to feature/kernel inputs, hold runtime state, inject adapters, apply runtime guards, and expose use case groups. It must not depend on Tauri.

The process-level `AtelierRuntime` owns global settings, event listeners, injected external dependencies, and an optional `WorkspaceSession`. A `WorkspaceSession` owns only services and runtime state tied to one opened workspace. Opening a replacement workspace builds the candidate session and persists its recent-workspace state before publishing it.

## Resource Rule

Any durable binary or semi-structured resource must go through `resource-catalog`.

Feature crates may store `ResourceRef` plus feature-owned metadata. They should not create long-lived resource directories, encode physical path rules, or maintain private binary indexes.

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
- Are external library types kept out of feature crates, `kernel`, and `app-api`?
