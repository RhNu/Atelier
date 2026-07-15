# Generate Workbench and Workspace Draft

Date: 2026-07-15

Generate uses a full-height workbench with one scrollable settings column and a fixed action dock. The sidebar defaults to 420px and stores only its versioned UI width preference in `localStorage`. Durable prompt, parameter, character, I2I, Vibe, and Precise Reference state is workspace data and is stored through the backend.

`D:\Source\_Rust\nait` is a read-only layout and interaction reference for the dense sidebar, hard-edged sections, and always-visible generation action. Atelier does not copy its Vue source, assets, commands, or DTOs.

The `generation` feature owns `GenerationDraftSnapshot`, validation, its repository port, and service. The SQLite adapter stores adapter-versioned JSON under the independent `workspace_settings` key `generation.draft`; the normal workspace settings payload remains under `workspace`. Missing drafts initialize from current workspace generation defaults. Corrupt or unsupported payloads are recoverable and can be cleared without blocking the rest of the workspace.

Draft resources use the `Workspace / generation-draft` owner. Save attaches new draft links before replacing JSON, then releases removed links and any matching import-staging links after persistence succeeds. A failed save rolls back newly attached links. Clear removes the JSON and detaches every link owned by the draft, including when the stored JSON cannot be decoded, then runs normal resource-catalog cleanup.

The frontend loads the draft with TanStack Query and keeps responsive edits in a page-local controller. Text and continuous controls use a short debounce; discrete resource changes, reset, blur, and slider commit flush immediately. Saves are serialized and coalesced so an older request cannot overwrite the latest draft.

Image Guidance uses compact conditional sections: empty I2I, Vibe, Precise Reference, and Character groups expose only their add actions, while parameters appear only after relevant data exists. Vibe activation is derived from a non-empty slot stack and the absence of Precise References; the persisted `enabled` field remains compatibility data rather than a user-facing switch. Vibe library selection uses a thumbnail dialog, repeated bounded numeric parameters use range controls, and technical resource metadata is hidden unless global developer mode is enabled.
