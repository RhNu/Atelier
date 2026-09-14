# Internal Agent tools

The Agent edits NovelAI text and generation settings through these application-owned tools.
It has no shell, arbitrary filesystem/network access, credentials, external plugins, Vibe editing,
image guidance editing, or pixel editing. Prompt-resource preview images are preserved by the host.

| Tool | Capability |
| --- | --- |
| `get_generation_context` | Current draft, model capabilities, registered prompt-function syntax. |
| `edit_generation_draft` | Ordered atomic operations: set/append/exact replace prompt text; create/copy/remove/reorder characters; partial character settings; model/preset selection; text-only generation parameters. |
| `undo_agent_action` | Restore a recorded draft edit when no subsequent draft change occurred. |
| `search_prompt_resources` | Search chunks and main/character presets across models, including text and aliases; filter before pagination. |
| `get_prompt_resource` | Complete resource contents, metadata, overrides and inbound references. Establishes the observation required for mutation. |
| `get_prompt_library` | Existing prompt-library folders and paths. |
| `create_prompt_resource` | Create chunks or main/character presets with names, aliases, description, model bindings, prompt/UC behavior and optional overrides. |
| `edit_prompt_resource` | Atomic exact text edits, metadata patches, behavior changes, quality/UC override changes. Omitted fields survive; null clears optional fields. |
| `copy_prompt_resource` | Copy an observed resource to a new path and name, preserving content, bindings, overrides and existing preview. |
| `delete_prompt_resource` | Delete an observed resource only when no saved draft or prompt resource refers to it. |
| `get_lexicon_context` | Installed lexicon availability, categories and groups. |
| `search_lexicon` | Lexical/semantic search for canonical tags, artists, translations and suggestions, with filters and pagination. |
| `get_lexicon_entity` | Aliases, translations, wiki text, groups and related terms. |
| `preview_generation` | Expanded and resolved prompts, all prompt-scope traces, effective preset overrides, token usage and optional Anlas estimate. |
| `submit_generation` | Submit the previously observed compiled preview. Requires a usable NovelAI account and unchanged preview inputs. |

## Observation and submission

Tools do not accept revision arguments. The host records snapshots delivered to the model and
promotes tool-result observations only at the next model request. Multiple queued writes cannot
silently use state the model has not read. On `outdated`, review the returned state and replan.

Draft edits are one transaction with the undo record. Resource operations stage changes on a copy
and validate before saving. Resource mutations and library renames share a writer lock. Chunk
renames rewrite prompt references and advance the same durable draft version used by ordinary
edits; an old undo cannot overwrite the rename.

Read a resource before editing, copying or deleting it. Search results are summaries, not edit
observations. A resource edit affects one resource; use its operations array to group related
changes. Resource mutations are not currently part of draft undo.

Preview compiles once into an immutable host-owned request. Submission approvals display that
preview, and submission rechecks inputs after approval. No prompt expansion occurs during
submission. Resolved text and token counts use `novelai-bridge`, including quality/UC additions.
Random seeds remain random until generation. A missing account does not prevent text preview;
it makes the estimate unavailable and prevents submission.

The first implementation conservatively invalidates a preview on any prompt-library change,
including unrelated resource metadata. This trades extra previews for a simple, complete
dependency check. Resource edits themselves compare only their target resource.

## Desktop QA

- Replace one phrase in a preset and verify its other behavior, overrides and model bindings survive.
- Copy a preset, select it, preview, and compare resolved main/negative/character prompts to generation.
- Rename a referenced chunk; verify references refresh and undoing an older draft action is rejected.
- Change a resource after preview; submission must require a fresh preview.
- Reject submission approval; no generation should enter the queue.
- Search, read and edit a resource, then interrupt the turn; the resource page should refresh.

Real provider, database failure injection and desktop interaction checks require user QA; automated
verification is limited to pure-logic unit tests and static/build checks.
