use super::*;

#[test]
fn resource_catalog_repository_round_trips_records_links_variants_and_orphans() {
    block_on(async {
        let connection = DatabaseConnection::open_memory().unwrap();
        let repository = DatabaseResourceCatalogRepository::new(connection);
        let blob_store = MemoryBlobStore::default();
        let catalog = ResourceCatalog::new(repository.clone(), blob_store, NullVariantBuilder);
        let job_owner = ResourceOwner::new(ResourceOwnerKind::Job, "job-1");
        let gallery_owner = ResourceOwner::new(ResourceOwnerKind::GalleryItem, "gallery-1");

        let reference = catalog
            .register_resource(RegisterResourceRequest {
                owner: job_owner.clone(),
                ..generated_resource("res-1", vec![9])
            })
            .await
            .unwrap();
        catalog
            .attach_owner(
                &reference.id,
                gallery_owner.clone(),
                ResourceRelation::Reference,
            )
            .await
            .unwrap();
        let variant = catalog
            .create_variant(CreateVariantRequest {
                source: reference.clone(),
                variant_id: VariantId::new("preview-1"),
                kind: ResourceVariantKind::Preview,
            })
            .await
            .unwrap();
        catalog
            .detach_owner(&reference.id, &job_owner, ResourceRelation::Primary)
            .await
            .unwrap();
        repository
            .record_orphan_blob(&BlobId::new("sha256:orphan"))
            .await
            .unwrap();

        assert_eq!(
            catalog.list_by_owner(&gallery_owner).await.unwrap(),
            vec![reference.clone()]
        );
        assert_eq!(
            catalog.list_links_by_owner(&gallery_owner).await.unwrap()[0].relation,
            ResourceRelation::Reference
        );
        assert_eq!(catalog.get_variant(&variant.id).await.unwrap(), variant);
        assert_eq!(
            repository.scan_orphan_blobs().await.unwrap(),
            vec![BlobId::new("sha256:orphan")]
        );
        assert_eq!(
            repository
                .get_ready_record(&reference.id)
                .await
                .unwrap()
                .unwrap()
                .state,
            ResourceState::Ready
        );
    });
}

#[test]
fn resource_catalog_transactions_are_serialized_until_rollback() {
    block_on(async {
        let repository =
            DatabaseResourceCatalogRepository::new(DatabaseConnection::open_memory().unwrap());
        let tx = repository.begin_transaction().await.unwrap();
        let (started_tx, started_rx) = mpsc::channel();
        let (acquired_tx, acquired_rx) = mpsc::channel();
        let repository_for_thread = repository.clone();

        let handle = thread::spawn(move || {
            started_tx.send(()).unwrap();
            let tx = block_on(repository_for_thread.begin_transaction()).unwrap();
            acquired_tx.send(()).unwrap();
            block_on(tx.rollback()).unwrap();
        });

        started_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        assert!(acquired_rx.recv_timeout(Duration::from_millis(50)).is_err());
        tx.rollback().await.unwrap();
        acquired_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        handle.join().unwrap();
    });
}

#[test]
fn generation_payload_store_keeps_submitted_payload_when_prepared_payload_shares_ref() {
    block_on(async {
        let store = DatabaseGenerationPayloadStore::new(DatabaseConnection::open_memory().unwrap());
        let request = GenerateImageRequest {
            prompt: "1girl".to_owned(),
            model: ImageModel::NaiDiffusion45Full,
            size: ImageSize::square(),
            seed: 42,
            ..Default::default()
        };
        let plan =
            plan_generation_request(request.clone(), GenerationPlanContext::default()).unwrap();
        let submitted = SubmittedGenerationPayload {
            payload_ref: JobPayloadRef::new("generation-submitted:job-1"),
            batch_id: BatchId::new("batch-1"),
            job_id: JobId::new("job-1"),
            request: GenerationWorkRequest::Image(request.clone()),
            context: GenerationPlanContext::default(),
        };
        let prepared = PreparedGenerationPayload {
            payload_ref: submitted.payload_ref.clone(),
            submitted_payload_ref: submitted.payload_ref.clone(),
            batch_id: submitted.batch_id.clone(),
            job_id: submitted.job_id.clone(),
            request: GenerationWorkRequest::Image(request),
            compiled_prompt: CompiledPrompt {
                expanded_prompt: "expanded prompt".to_owned(),
                trace: PromptTrace {
                    raw_prompt: "1girl".to_owned(),
                    expanded_prompt: "expanded prompt".to_owned(),
                    function_calls: Vec::new(),
                },
            },
            plan,
        };

        store
            .save_submitted_payload(submitted.clone())
            .await
            .unwrap();
        store.save_prepared_payload(prepared).await.unwrap();

        assert_eq!(
            store
                .get_submitted_payload(&submitted.payload_ref)
                .await
                .unwrap(),
            Some(submitted)
        );
        assert_eq!(
            store
                .get_submitted_payload(&JobPayloadRef::new("missing"))
                .await
                .unwrap(),
            None
        );
    });
}

#[test]
fn artifact_service_rejects_missing_or_mismatched_primary_variant() {
    block_on(async {
        let connection = DatabaseConnection::open_memory().unwrap();
        let resource_repository = DatabaseResourceCatalogRepository::new(connection.clone());
        let catalog = ResourceCatalog::new(
            resource_repository.clone(),
            MemoryBlobStore::default(),
            NullVariantBuilder,
        );
        let artifacts = ArtifactService::new(
            DatabaseArtifactRepository::new(connection),
            resource_repository,
        );
        let first = catalog
            .register_resource(generated_resource("artifact-res-1", vec![1]))
            .await
            .unwrap();
        let second = catalog
            .register_resource(generated_resource("artifact-res-2", vec![2]))
            .await
            .unwrap();
        let second_variant = catalog
            .create_variant(CreateVariantRequest {
                source: second.clone(),
                variant_id: VariantId::new("artifact-preview-2"),
                kind: ResourceVariantKind::Preview,
            })
            .await
            .unwrap();

        let missing_error = artifacts
            .register_artifact(artifact_request(
                "artifact-missing-variant",
                ResourceRef::new(first.id.clone(), Some(VariantId::new("missing-variant"))),
            ))
            .await
            .unwrap_err();
        let mismatch_error = artifacts
            .register_artifact(artifact_request(
                "artifact-mismatched-variant",
                ResourceRef::new(first.id, Some(second_variant.id.clone())),
            ))
            .await
            .unwrap_err();
        let valid = artifacts
            .register_artifact(artifact_request(
                "artifact-valid-variant",
                ResourceRef::new(second.id, Some(second_variant.id)),
            ))
            .await
            .unwrap();

        assert!(missing_error.to_string().contains("variant does not exist"));
        assert!(
            mismatch_error
                .to_string()
                .contains("belongs to another resource")
        );
        assert_eq!(valid.id.as_str(), "artifact-valid-variant");
    });
}
