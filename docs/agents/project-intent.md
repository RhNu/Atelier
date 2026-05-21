# Project Intent

## Goal

Atelier is a desktop creative workspace for NovelAI image workflows. It is meant for creators who use NovelAI regularly, not for users who need a generic multi-provider image generation tool.

The core experience should grow around:

- Prompt writing, parsing, reuse, and prompt resources.
- Stable submission of NovelAI parameters, models, Vibe data, reference images, and Director tools.
- Observable, cancellable, retryable, and replayable generation work.
- Artifact gallery, metadata, export, replay, and safety markings.
- Multiple API keys with explicit user-controlled switching.

## Design Preferences

- Product language should acknowledge that the app is NovelAI-specific.
- NovelAI network details should stay behind the `novelai-bridge` adapter.
- Internal domain language should use application concepts such as workspace, job, artifact, resource ref, prompt resource, gallery item, Vibe document, and Director result.
- Durable resources should go through `resource-catalog` instead of feature-owned directories or binary indexes.
- The UI should not simply copy old `nait` pages. It should be organized around high-frequency creative workflows.

## Non-Commitments

- Do not preserve old `nait` command names or DTOs by default.
- Do not promise old database or workspace migration.
- Do not require the first runnable slice to include full orchestration, NSFW detection, Director, or all Vibe features.
- Do not freeze the frontend framework without separate evaluation.
