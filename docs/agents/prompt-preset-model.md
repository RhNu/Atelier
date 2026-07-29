# Prompt Preset Model

Date: 2026-07-29

Status: Current compatibility guidance.

Prompt presets are explicitly selected during generation. They do not carry a
separate enabled state.

Positive and undesired-content prompt behavior use the same explicit domain
choice:

- `Surround { before, after }` adds text around the generation prompt.
- `Replace { text }` replaces the generation prompt.

`prompt-resources` owns this behavior model. `app-api` exposes a tagged DTO
union instead of inferring behavior from empty string fields.

The SQLite adapter retains the existing `enabled` and flat text columns for
workspace compatibility, while storing explicit `prompt_mode` and `uc_mode`
columns. Migration 8 infers the initial modes from legacy replacement text.
New writes set the compatibility `enabled` column to true and serialize only
the selected behavior. Persisting an empty replacement therefore remains
unambiguous.

The desktop editor owns a separate draft model so switching behavior tabs
preserves both editing buffers. Only the active buffer is mapped into an
upsert DTO.
