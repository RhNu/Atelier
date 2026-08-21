# Prompt Preset Model

Date: 2026-08-21

Status: Current guidance.

Prompt presets are explicitly selected during generation. They do not carry a
separate enabled state.

Positive and undesired-content prompt behavior use the same explicit domain
choice:

- `Surround { before, after }` adds text around the generation prompt.
- `Replace { text }` replaces the generation prompt.

`prompt-resources` owns this behavior model. `app-api` exposes a tagged DTO
union instead of inferring behavior from empty string fields.

The SQLite adapter stores explicit `prompt_mode` and `uc_mode` columns alongside
the text fields for each behavior. It has no compatibility-only `enabled`
column and does not infer modes from older records. Persisting an empty
replacement therefore remains unambiguous.

The desktop editor owns a separate draft model so switching behavior tabs
preserves both editing buffers. Only the active buffer is mapped into an
upsert DTO.

Prompt chunks and presets bind to one or more concrete NovelAI image models.
Family names are only an editor grouping; persistence and compile requests use
exact model identifiers. Chunk keys remain globally unique across every model.

Compilation is model-scoped. A selected preset must include the requested
model, and `$chunk(...)` only resolves chunks that include it. Every referenced
chunk must cover all models of the resource that references it. Saving a
binding reduction is rejected when it would invalidate an existing dependent.

Generation drafts retain a separate prompt state for each concrete model. That
state contains the positive and undesired-content prompts, main preset,
character prompts, and character presets. Switching models restores its state;
other generation parameters are shared, except scale is set to the selected
model's official default. Capability-gated guidance stays in the draft while
inactive and is omitted from generation requests.

Vibe resources do not have a separate model-binding table. Their model
availability is derived exclusively from their actual encoding configs, and
model filtering happens before pagination.
