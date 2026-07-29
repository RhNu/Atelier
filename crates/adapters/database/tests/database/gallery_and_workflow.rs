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
                hidden: false,
                available_model_keys: vec!["v4-5full".to_owned()],
                available_encoding_configs: vec![atelier_vibe::VibeEncodingConfig {
                    model: VibeModel::NaiDiffusion45Full,
                    settings: settings.clone(),
                }],
                created_at_ms: 10,
                updated_at_ms: 10,
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
            Some(entry.clone())
        );
        assert_eq!(
            repository.list_documents(0, 10, false).await.unwrap(),
            vec![entry.clone()]
        );
        let hidden = repository
            .set_document_hidden(&VibeId::new("vibe-1"), true, 20)
            .await
            .unwrap()
            .unwrap();
        assert!(hidden.summary.hidden);
        assert_eq!(hidden.summary.updated_at_ms, 20);
        assert!(
            repository
                .list_documents(0, 10, false)
                .await
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            repository.list_documents(0, 10, true).await.unwrap(),
            vec![hidden]
        );
        assert_eq!(repository.count_documents(false).await.unwrap(), 0);
        assert_eq!(repository.count_documents(true).await.unwrap(), 1);
        let renamed = repository
            .rename_document(&VibeId::new("vibe-1"), "Renamed".to_owned(), 30)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(renamed.summary.display_name, "Renamed");
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
            replay: first.replay.clone(),
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
            replay: second.replay.clone(),
            safety_assessment: None,
            manual_safety_override: None,
            indexed_at_ms: 200,
        };
        let second_item_id = second_item.id.clone();

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
        let hidden_items = gallery
            .query_items(GalleryQuery {
                source_kind: Some(GallerySourceKind::Generation),
                manual_safety_override: Some(GallerySafetyOverride::Hidden),
                ..GalleryQuery::default()
            })
            .await
            .unwrap();
        let default_items = gallery
            .query_items(GalleryQuery {
                offset: 0,
                limit: 10,
                ..GalleryQuery::default()
            })
            .await
            .unwrap();

        assert_eq!(gallery_item_ids(&hidden_items), vec![first_item.id]);
        assert_eq!(gallery_item_ids(&default_items), vec![second_item_id]);
        assert_eq!(default_items[0].artifact_id.as_str(), "artifact-2");
    });
}

#[test]
fn gallery_query_filters_safety_label_before_sql_pagination() {
    block_on(async {
        let connection = DatabaseConnection::open_memory().unwrap();
        let gallery = DatabaseGalleryIndex::new(connection);
        let newer_sensitive = gallery_item("sensitive-newer", 30, 0.91);
        let older_safe = gallery_item("safe-older", 10, 0.04);
        let older_safe_id = older_safe.id.clone();

        gallery.upsert_item(newer_sensitive).await.unwrap();
        gallery.upsert_item(older_safe).await.unwrap();

        let page = gallery
            .query_items(GalleryQuery {
                offset: 0,
                limit: 1,
                safety_label: Some(SafetyLabel::Safe),
                ..GalleryQuery::default()
            })
            .await
            .unwrap();
        let total = gallery
            .count_items(GalleryQuery {
                safety_label: Some(SafetyLabel::Safe),
                ..GalleryQuery::default()
            })
            .await
            .unwrap();

        assert_eq!(gallery_item_ids(&page), vec![older_safe_id]);
        assert_eq!(total, 1);
    });
}

#[test]
fn gallery_delete_items_returns_deleted_records_and_ignores_missing_ids() {
    block_on(async {
        let connection = DatabaseConnection::open_memory().unwrap();
        let gallery = DatabaseGalleryIndex::new(connection);
        let first = gallery_item("delete-first", 30, 0.04);
        let second = gallery_item("delete-second", 20, 0.04);
        let first_id = first.id.clone();
        let second_id = second.id.clone();

        gallery.upsert_item(first).await.unwrap();
        gallery.upsert_item(second).await.unwrap();

        let deleted = gallery
            .delete_items(&[
                first_id.clone(),
                GalleryItemId::new("artifact:missing"),
                second_id.clone(),
            ])
            .await
            .unwrap();

        assert_eq!(gallery_item_ids(&deleted), vec![first_id, second_id]);
        assert!(
            gallery
                .query_items(GalleryQuery::default())
                .await
                .unwrap()
                .is_empty()
        );
    });
}

#[test]
fn gallery_hard_delete_commits_related_database_changes_together() {
    block_on(async {
        let connection = DatabaseConnection::open_memory().unwrap();
        let gallery = DatabaseGalleryIndex::new(connection.clone());
        let artifacts = DatabaseArtifactRepository::new(connection.clone());
        let resources = DatabaseResourceCatalogRepository::new(connection);
        let catalog = ResourceCatalog::new(
            resources.clone(),
            MemoryBlobStore::default(),
            NullVariantBuilder,
        );
        let item = gallery_item("delete-atomic", 30, 0.04);
        let gallery_owner = ResourceOwner::new(ResourceOwnerKind::GalleryItem, item.id.as_str());
        catalog
            .register_resource(RegisterResourceRequest {
                resource_id: item.primary_resource.id.clone(),
                owner: gallery_owner.clone(),
                ..generated_resource("unused", vec![9])
            })
            .await
            .unwrap();
        artifacts
            .insert_artifact(artifact_record("delete-atomic", 30, item.source.clone()))
            .await
            .unwrap();
        gallery.upsert_item(item.clone()).await.unwrap();

        let deleted = gallery
            .hard_delete(&[GalleryHardDeletePlan {
                item_id: item.id.as_str().to_owned(),
                artifact_id: item.artifact_id.as_str().to_owned(),
                resource_ids: vec![item.primary_resource.id.as_str().to_owned()],
                transient_owner: GalleryTransientOwner {
                    kind: "job",
                    local_id: "job-delete-atomic".to_owned(),
                },
                force_delete_pending: false,
            }])
            .await
            .unwrap();

        assert_eq!(deleted, 1);
        assert!(
            gallery
                .query_items(GalleryQuery::default())
                .await
                .unwrap()
                .is_empty()
        );
        assert!(
            catalog
                .list_by_owner(&gallery_owner)
                .await
                .unwrap()
                .is_empty()
        );
        assert!(
            resources
                .get_ready_record(&item.primary_resource.id)
                .await
                .unwrap()
                .is_none()
        );
        assert_eq!(
            artifacts
                .delete_artifacts(&[item.artifact_id])
                .await
                .unwrap(),
            0
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

fn gallery_item_ids(items: &[GalleryItem]) -> Vec<GalleryItemId> {
    items.iter().map(|item| item.id.clone()).collect()
}

fn gallery_item(id: &str, indexed_at_ms: u64, safety_score: f32) -> GalleryItem {
    let artifact = artifact_record(
        id,
        i64::try_from(indexed_at_ms).expect("test timestamp should fit i64"),
        ArtifactSource::GenerationJob {
            job_id: format!("job-{id}"),
            batch_id: None,
        },
    );
    GalleryItem {
        id: GalleryItemId::from_artifact_id(&artifact.id),
        artifact_id: artifact.id.clone(),
        artifact_kind: artifact.kind,
        source: artifact.source.clone(),
        primary_resource: artifact.primary_resource.clone(),
        assets: artifact.assets.clone(),
        metadata: artifact.metadata.clone(),
        replay: artifact.replay.clone(),
        safety_assessment: Some(SafetyAssessment::new(
            artifact.primary_resource,
            ImageSafetyScore::new(safety_score).unwrap(),
        )),
        manual_safety_override: None,
        indexed_at_ms,
    }
}
