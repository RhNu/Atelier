use super::*;

#[test]
fn vibe_repository_round_trips_documents_and_cached_encodings() {
    block_on(async {
        let repository = DatabaseVibeRepository::new(DatabaseConnection::open_memory().unwrap());
        let settings = VibeEncodeSettings::new(VibeModel::NaiDiffusion45Full, 0.7).unwrap();
        let entry = VibeDocumentEntry {
            summary: VibeDocumentSummary {
                document_id: VibeId::new("vibe-1"),
                display_name: "Style A".to_owned(),
                has_image: true,
                available_model_keys: vec!["v4-5full".to_owned()],
                available_encoding_configs: vec![nai_atelier_vibe::VibeEncodingConfig {
                    model: VibeModel::NaiDiffusion45Full,
                    settings: settings.clone(),
                }],
            },
            resources: VibeDocumentResources {
                document: ResourceRef::base(ResourceId::new("vibe-document")),
                source_image: Some(ResourceRef::base(ResourceId::new("vibe-source"))),
                preview: Some(ResourceRef::base(ResourceId::new("vibe-preview"))),
                encodings: vec![ResourceRef::base(ResourceId::new("vibe-encoding"))],
            },
        };
        let source = VibeSourceIdentity::new_sha256("source-hash");
        let encoding = VibeEncodingRecord {
            vibe_id: VibeId::new("vibe-1"),
            source: source.clone(),
            settings: settings.clone(),
            resource: ResourceRef::base(ResourceId::new("cached-encoding")),
        };

        repository.insert_document(entry.clone()).await.unwrap();
        repository.save_encoding(encoding.clone()).await.unwrap();

        assert_eq!(
            repository
                .get_document(&VibeId::new("vibe-1"))
                .await
                .unwrap(),
            Some(entry)
        );
        assert_eq!(
            repository
                .find_cached_encoding(&source, &settings)
                .await
                .unwrap(),
            Some(encoding)
        );
    });
}

#[test]
fn artifact_and_gallery_persistence_supports_query_and_manual_override() {
    block_on(async {
        let connection = DatabaseConnection::open_memory().unwrap();
        let artifacts = DatabaseArtifactRepository::new(connection.clone());
        let gallery = DatabaseGalleryIndex::new(connection);
        let first = artifact_record(
            "artifact-1",
            11,
            ArtifactSource::GenerationJob {
                job_id: "job-1".to_owned(),
                batch_id: Some("batch-1".to_owned()),
            },
        );
        let second = artifact_record(
            "artifact-2",
            22,
            ArtifactSource::DirectorRun {
                run_id: "director-1".to_owned(),
            },
        );
        let first_item = GalleryItem {
            id: GalleryItemId::from_artifact_id(&first.id),
            artifact_id: first.id.clone(),
            artifact_kind: first.kind,
            source: first.source.clone(),
            primary_resource: first.primary_resource.clone(),
            assets: first.assets.clone(),
            metadata: first.metadata.clone(),
            safety_assessment: Some(SafetyAssessment::new(
                first.primary_resource.clone(),
                ImageSafetyScore::new(0.25).unwrap(),
            )),
            manual_safety_override: None,
            indexed_at_ms: 100,
        };
        let second_item = GalleryItem {
            id: GalleryItemId::from_artifact_id(&second.id),
            artifact_id: second.id.clone(),
            artifact_kind: second.kind,
            source: second.source.clone(),
            primary_resource: second.primary_resource.clone(),
            assets: second.assets.clone(),
            metadata: second.metadata.clone(),
            safety_assessment: None,
            manual_safety_override: None,
            indexed_at_ms: 200,
        };

        artifacts.insert_artifact(first.clone()).await.unwrap();
        artifacts.insert_artifact(second).await.unwrap();
        gallery.upsert_item(first_item.clone()).await.unwrap();
        gallery.upsert_item(second_item).await.unwrap();
        let updated = gallery
            .set_safety_override(&first_item.id, Some(GallerySafetyOverride::Hidden))
            .await
            .unwrap();

        assert_eq!(
            updated.manual_safety_override,
            Some(GallerySafetyOverride::Hidden)
        );
        assert_eq!(
            gallery
                .query_items(GalleryQuery {
                    source_kind: Some(GallerySourceKind::Generation),
                    manual_safety_override: Some(GallerySafetyOverride::Hidden),
                    ..GalleryQuery::default()
                })
                .await
                .unwrap()
                .into_iter()
                .map(|item| item.id)
                .collect::<Vec<_>>(),
            vec![first_item.id]
        );
        assert_eq!(
            gallery
                .query_items(GalleryQuery {
                    offset: 0,
                    limit: 1,
                    ..GalleryQuery::default()
                })
                .await
                .unwrap()[0]
                .artifact_id
                .as_str(),
            "artifact-2"
        );
    });
}

#[test]
fn generation_workflow_can_use_database_backed_payload_resource_artifact_and_gallery_ports() {
    block_on(async {
        let connection = DatabaseConnection::open_memory().unwrap();
        let ports = DatabaseWorkflowPorts::new(connection.clone());
        let mut runtime = KernelRuntime::new(ports.clone());
        let job_id = JobId::new("job-db");

        runtime
            .submit_generation_work(SubmitGenerationWork {
                batch_id: BatchId::new("batch-db"),
                job_id: job_id.clone(),
                request: GenerationWorkRequest::Image(GenerateImageRequest {
                    prompt: "1girl".to_owned(),
                    model: ImageModel::NaiDiffusion45Full,
                    ..Default::default()
                }),
                context: GenerationPlanContext::default(),
            })
            .await
            .unwrap();
        runtime.run_scheduled_generation_job(&job_id).await.unwrap();

        let gallery = DatabaseGalleryIndex::new(connection);
        let items = gallery.query_items(GalleryQuery::default()).await.unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].artifact_id.as_str(), "artifact:job-db:sample:0");
        assert_eq!(ports.generate_call_count(), 1);
    });
}
