use super::{
    Arc, AtelierRuntime, MemorySecretStore, RecordingFactory, SafetyScanner, TestApp,
    WorkspaceSession, support,
};

impl TestApp {
    pub async fn open(
        root: std::path::PathBuf,
        secrets: MemorySecretStore,
        factory: RecordingFactory,
    ) -> atelier_app::CommandResult<Self> {
        Self::open_with_scanner(root, secrets, factory, None).await
    }

    pub async fn open_with_scanner(
        root: std::path::PathBuf,
        secrets: MemorySecretStore,
        factory: RecordingFactory,
        scanner: Option<Arc<dyn SafetyScanner>>,
    ) -> atelier_app::CommandResult<Self> {
        let mut dependencies = support::dependencies(secrets, factory);
        dependencies.safety_scanner = scanner;
        let runtime = AtelierRuntime::new(dependencies);
        runtime
            .open_workspace(atelier_app_api::workspace::OpenWorkspaceRequestDto { root })
            .await?;
        let session = runtime.current_session()?;
        Ok(Self { runtime, session })
    }

    pub const fn account(
        &self,
    ) -> atelier_app::AccountUseCases<'_, MemorySecretStore, RecordingFactory> {
        self.runtime.account()
    }
}

impl std::ops::Deref for TestApp {
    type Target = WorkspaceSession<MemorySecretStore, RecordingFactory>;
    fn deref(&self) -> &Self::Target {
        &self.session
    }
}
