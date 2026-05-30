use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use atelier_prompt_resources::{
    ChunkReference, PromptChunk, PromptChunkId, PromptChunkKey, PromptPreset, PromptPresetId,
    PromptPresetKind, PromptResourceReader, PromptResourceRepository, PromptResourceResult,
};

#[derive(Clone, Debug, Default)]
pub struct MemoryPromptResourceRepository {
    state: Arc<Mutex<State>>,
}

#[derive(Debug, Default)]
struct State {
    next_id: u64,
    next_preset_id: u64,
    chunks: BTreeMap<PromptChunkId, PromptChunk>,
    presets: BTreeMap<PromptPresetId, PromptPreset>,
}

impl MemoryPromptResourceRepository {
    pub fn chunks(&self) -> Vec<PromptChunk> {
        self.state
            .lock()
            .expect("lock memory repo")
            .chunks
            .values()
            .cloned()
            .collect()
    }

    pub fn presets(&self) -> Vec<PromptPreset> {
        self.state
            .lock()
            .expect("lock memory repo")
            .presets
            .values()
            .cloned()
            .collect()
    }
}

#[async_trait]
impl PromptResourceReader for MemoryPromptResourceRepository {
    async fn get_chunk_by_id(
        &self,
        id: &PromptChunkId,
    ) -> PromptResourceResult<Option<PromptChunk>> {
        Ok(self
            .state
            .lock()
            .expect("lock memory repo")
            .chunks
            .get(id)
            .cloned())
    }

    async fn get_chunk_by_key(
        &self,
        key: &PromptChunkKey,
    ) -> PromptResourceResult<Option<PromptChunk>> {
        Ok(self
            .state
            .lock()
            .expect("lock memory repo")
            .chunks
            .values()
            .find(|chunk| &chunk.key == key)
            .cloned())
    }

    async fn list_chunks(&self) -> PromptResourceResult<Vec<PromptChunk>> {
        Ok(self.chunks())
    }

    async fn get_preset_by_id(
        &self,
        id: &PromptPresetId,
    ) -> PromptResourceResult<Option<PromptPreset>> {
        Ok(self
            .state
            .lock()
            .expect("lock memory repo")
            .presets
            .get(id)
            .cloned())
    }

    async fn list_presets(
        &self,
        kind: Option<PromptPresetKind>,
        include_disabled: bool,
    ) -> PromptResourceResult<Vec<PromptPreset>> {
        Ok(self
            .presets()
            .into_iter()
            .filter(|preset| kind.is_none_or(|kind| preset.kind == kind))
            .filter(|preset| include_disabled || preset.enabled)
            .collect())
    }
}

#[async_trait]
impl PromptResourceRepository for MemoryPromptResourceRepository {
    async fn allocate_chunk_id(&self) -> PromptResourceResult<PromptChunkId> {
        let next_id = {
            let mut state = self.state.lock().expect("lock memory repo");
            state.next_id += 1;
            state.next_id
        };
        Ok(PromptChunkId::new(format!("chunk-{next_id}")))
    }

    async fn save_chunk(&self, chunk: PromptChunk) -> PromptResourceResult<()> {
        self.state
            .lock()
            .expect("lock memory repo")
            .chunks
            .insert(chunk.id.clone(), chunk);
        Ok(())
    }

    async fn save_chunk_and_rewrite_references(
        &self,
        chunk: PromptChunk,
        old_key: &PromptChunkKey,
    ) -> PromptResourceResult<()> {
        let mut state = self.state.lock().expect("lock memory repo");
        state.chunks.insert(chunk.id.clone(), chunk.clone());
        for item in state.chunks.values_mut() {
            if item.id == chunk.id {
                continue;
            }
            item.content = atelier_prompt_resources::rewrite_chunk_references(
                &item.content,
                old_key,
                &chunk.key,
            );
        }
        for preset in state.presets.values_mut() {
            preset.before = atelier_prompt_resources::rewrite_chunk_references(
                &preset.before,
                old_key,
                &chunk.key,
            );
            preset.after = atelier_prompt_resources::rewrite_chunk_references(
                &preset.after,
                old_key,
                &chunk.key,
            );
            preset.replace = atelier_prompt_resources::rewrite_chunk_references(
                &preset.replace,
                old_key,
                &chunk.key,
            );
            preset.uc_before = atelier_prompt_resources::rewrite_chunk_references(
                &preset.uc_before,
                old_key,
                &chunk.key,
            );
            preset.uc_after = atelier_prompt_resources::rewrite_chunk_references(
                &preset.uc_after,
                old_key,
                &chunk.key,
            );
            preset.uc_replace = atelier_prompt_resources::rewrite_chunk_references(
                &preset.uc_replace,
                old_key,
                &chunk.key,
            );
        }
        drop(state);
        Ok(())
    }

    async fn delete_chunk(&self, id: &PromptChunkId) -> PromptResourceResult<()> {
        self.state
            .lock()
            .expect("lock memory repo")
            .chunks
            .remove(id);
        Ok(())
    }

    async fn list_chunk_references(
        &self,
        key: &PromptChunkKey,
    ) -> PromptResourceResult<Vec<ChunkReference>> {
        let mut references = self
            .chunks()
            .into_iter()
            .filter(|chunk| chunk.references_chunk(key))
            .map(|chunk| ChunkReference {
                chunk_id: chunk.id,
                key: chunk.key,
            })
            .collect::<Vec<_>>();
        references.extend(
            self.presets()
                .into_iter()
                .filter(|preset| preset.references_chunk(key))
                .map(|preset| ChunkReference {
                    chunk_id: PromptChunkId::new(preset.id.as_str()),
                    key: key.clone(),
                }),
        );
        Ok(references)
    }

    async fn allocate_preset_id(&self) -> PromptResourceResult<PromptPresetId> {
        let next_id = {
            let mut state = self.state.lock().expect("lock memory repo");
            state.next_preset_id += 1;
            state.next_preset_id
        };
        Ok(PromptPresetId::new(format!("preset-{next_id}")))
    }

    async fn save_preset(&self, preset: PromptPreset) -> PromptResourceResult<()> {
        self.state
            .lock()
            .expect("lock memory repo")
            .presets
            .insert(preset.id.clone(), preset);
        Ok(())
    }

    async fn delete_preset(&self, id: &PromptPresetId) -> PromptResourceResult<()> {
        self.state
            .lock()
            .expect("lock memory repo")
            .presets
            .remove(id);
        Ok(())
    }
}
