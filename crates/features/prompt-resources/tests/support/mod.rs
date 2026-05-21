use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use atelier_prompt_resources::{
    ChunkReference, PromptChunk, PromptChunkId, PromptChunkKey, PromptResourceReader,
    PromptResourceRepository, PromptResourceResult,
};

#[derive(Clone, Debug, Default)]
pub struct MemoryPromptResourceRepository {
    state: Arc<Mutex<State>>,
}

#[derive(Debug, Default)]
struct State {
    next_id: u64,
    chunks: BTreeMap<PromptChunkId, PromptChunk>,
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
        Ok(self
            .chunks()
            .into_iter()
            .filter(|chunk| chunk.references_chunk(key))
            .map(|chunk| ChunkReference {
                chunk_id: chunk.id,
                key: chunk.key,
            })
            .collect())
    }
}
