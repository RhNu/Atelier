use super::*;
use atelier_app_api::prompt::UpsertPromptChunkRequestDto;

#[test]
fn queued_prompt_is_frozen_across_resource_edits_and_workspace_reopen() {
    block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let factory = RecordingFactory::with_image_bytes(valid_png_bytes(2, 1));
        let secrets = MemorySecretStore::default();
        let mut app = TestApp::open(temp.path().to_path_buf(), secrets.clone(), factory.clone())
            .await
            .unwrap();
        activate(&app).await;
        let chunk = app
            .prompt()
            .upsert_chunk(chunk_request(None, "before"))
            .await
            .unwrap();
        app.generation()
            .submit(submit_request(
                "snapshot-batch",
                "snapshot-job",
                "$chunk(hero)",
            ))
            .await
            .unwrap();
        drop(app);
        app = TestApp::open(temp.path().to_path_buf(), secrets, factory.clone())
            .await
            .unwrap();
        activate(&app).await;
        app.prompt()
            .upsert_chunk(chunk_request(Some(chunk.chunk_id), "after"))
            .await
            .unwrap();
        app.generation().resume().await.unwrap();
        app.generation().run_job("snapshot-job").await.unwrap();
        assert_eq!(factory.generated_requests()[0].prompt, "before");
    });
}

fn chunk_request(chunk_id: Option<String>, content: &str) -> UpsertPromptChunkRequestDto {
    UpsertPromptChunkRequestDto {
        chunk_id,
        key: "hero".to_owned(),
        content: content.to_owned(),
        category: None,
        description: None,
        preview: None,
        models: vec![ImageModelDto::NaiDiffusion45Full],
    }
}

async fn activate(app: &TestApp) {
    app.account()
        .create_api_key(CreateApiKeyRequestDto {
            id: "snapshot-key".to_owned(),
            display_name: "Snapshot".to_owned(),
            secret: "secret".to_owned(),
        })
        .await
        .unwrap();
    app.account()
        .set_active_api_key("snapshot-key")
        .await
        .unwrap();
}
