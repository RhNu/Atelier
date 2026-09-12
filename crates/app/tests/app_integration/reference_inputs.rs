use super::*;
use atelier_app_api::generation::{CharacterReferenceDto, CharacterReferenceTypeDto};

fn reference_request(image: ImageInputDto) -> SubmitGenerationRequestDto {
    let mut request = submit_request("reference-batch", "reference-job", "1girl");
    let GenerationWorkRequestDto::Image(work) = &mut request.work else {
        unreachable!();
    };
    work.character_references = Some(vec![CharacterReferenceDto {
        image,
        reference_type: CharacterReferenceTypeDto::Style,
        fidelity: 0.4,
        strength: 0.6,
    }]);
    request
}

#[test]
fn reference_validation_rejects_empty_inline_input_before_queueing() {
    block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let app = test_app_with_image(&temp, valid_png_bytes(2, 1)).await;
        let error = app
            .generation()
            .submit(reference_request(ImageInputDto::InlineBase64 {
                image_base64: "  ".to_owned(),
            }))
            .await
            .unwrap_err();
        assert_eq!(error.code(), "invalid_reference");
        assert!(
            app.generation()
                .status(None)
                .await
                .unwrap()
                .batch_id
                .is_none()
        );
    });
}

#[test]
fn reference_validation_rejects_non_image_resources_before_queueing() {
    block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let app = test_app_with_image(&temp, valid_png_bytes(2, 1)).await;
        let resource = app
            .resources()
            .import_image(ImportImageResourceRequestDto {
                kind: ImageResourceKindDto::ReferenceImage,
                image_base64: "AQID".to_owned(),
                mime_type: None,
            })
            .await
            .unwrap()
            .resource;
        let connection = rusqlite::Connection::open(workspace_database_path(&WorkspaceRoot::new(
            temp.path().to_path_buf(),
        )))
        .unwrap();
        connection
            .execute(
                "UPDATE resources SET kind = 'vibe_encoding' WHERE id = ?1",
                [&resource.id],
            )
            .unwrap();
        let error = app
            .generation()
            .submit(reference_request(ImageInputDto::ResourceRef { resource }))
            .await
            .unwrap_err();
        assert_eq!(error.code(), "invalid_reference");
        assert!(error.message().contains("invalid_resource_kind"));
    });
}
