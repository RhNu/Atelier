mod registry;
use async_trait::async_trait;
use atelier_adapter_novelai::NovelAiEmbeddedVibeExtractor;
use atelier_agent::{AgentRegistry, AgentRegistryRepository, AgentResult};
use atelier_app::RuntimeDependencies;
use atelier_settings::{
    GlobalSettings, GlobalSettingsRepository, GlobalSettingsService, SettingsResult,
};
use std::sync::{Arc, Mutex};

pub fn dependencies<S, F>(secrets: S, factory: F) -> RuntimeDependencies<S, F> {
    RuntimeDependencies::new(
        secrets,
        factory,
        NovelAiEmbeddedVibeExtractor,
        GlobalSettingsService::new(Arc::new(MemorySettings::default())),
        Arc::new(registry::MemoryApiKeyRegistry::default()),
        Arc::new(MemoryAgentRegistry::default()),
    )
}

#[derive(Default)]
struct MemoryAgentRegistry(Mutex<AgentRegistry>);

#[async_trait]
impl AgentRegistryRepository for MemoryAgentRegistry {
    async fn load_registry(&self) -> AgentResult<AgentRegistry> {
        Ok(self.0.lock().unwrap().clone())
    }

    async fn save_registry(&self, registry: AgentRegistry) -> AgentResult<()> {
        *self.0.lock().unwrap() = registry;
        Ok(())
    }
}

#[derive(Default)]
struct MemorySettings(Mutex<GlobalSettings>);

#[async_trait]
impl GlobalSettingsRepository for MemorySettings {
    async fn get_global_settings(&self) -> SettingsResult<GlobalSettings> {
        Ok(self.0.lock().unwrap().clone())
    }
    async fn save_global_settings(&self, settings: GlobalSettings) -> SettingsResult<()> {
        *self.0.lock().unwrap() = settings;
        Ok(())
    }
}
