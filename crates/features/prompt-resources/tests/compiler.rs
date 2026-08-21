mod support;

use async_trait::async_trait;
use atelier_generation::ImageModel;
use atelier_prompt::ExtensionCall;
use atelier_prompt_resources::{
    CompileGenerationPromptRequest, CompilePromptRequest, PromptCompiler, PromptFunction,
    PromptFunctionContext, PromptFunctionDescriptor, PromptFunctionOutput, PromptFunctionRegistry,
    PromptPresetBehavior, PromptPresetKind, PromptPresetService, PromptResourceErrorKind,
    PromptResourceResult, UpsertPromptChunkRequest, UpsertPromptPresetRequest,
};
use futures_executor::block_on;
use support::MemoryPromptResourceRepository;

#[test]
fn compiler_expands_chunk_with_boundary_normalization() {
    block_on(async {
        let repository = repository_with_chunks([("光照", "cinematic lighting")]).await;
        let compiler = PromptCompiler::new(repository);

        let tight = compiler
            .compile(CompilePromptRequest::new("1girl$chunk(光照)solo"))
            .await
            .unwrap();
        let comma = compiler
            .compile(CompilePromptRequest::new("1girl, $chunk(光照), solo"))
            .await
            .unwrap();

        assert_eq!(tight.expanded_prompt, "1girl, cinematic lighting, solo");
        assert_eq!(comma.expanded_prompt, tight.expanded_prompt);
        assert_eq!(
            compiler
                .compile(CompilePromptRequest::new("{ $chunk(光照) }"))
                .await
                .unwrap()
                .expanded_prompt,
            "{ cinematic lighting }"
        );
        assert_eq!(
            compiler
                .compile(CompilePromptRequest::new("||red|$chunk(光照)||"))
                .await
                .unwrap()
                .expanded_prompt,
            "||red|cinematic lighting||"
        );
        assert_eq!(tight.trace.function_calls[0].function_name, "chunk");
        assert_eq!(
            tight.trace.function_calls[0].result_text,
            Some("cinematic lighting".to_owned())
        );
    });
}

#[test]
fn compiler_expands_nested_chunks_and_records_depth() {
    block_on(async {
        let repository = repository_with_chunks([
            ("base", "1girl, $chunk(光照)"),
            ("光照", "cinematic lighting"),
        ])
        .await;
        let compiler = PromptCompiler::new(repository);

        let result = compiler
            .compile(CompilePromptRequest::new("$chunk(base)"))
            .await
            .unwrap();

        assert_eq!(result.expanded_prompt, "1girl, cinematic lighting");
        assert_eq!(
            result
                .trace
                .function_calls
                .iter()
                .map(|call| (call.function_name.as_str(), call.depth))
                .collect::<Vec<_>>(),
            vec![("chunk", 1), ("chunk", 2)]
        );
    });
}

#[test]
fn compiler_rejects_cycles_depth_missing_chunks_and_unknown_functions() {
    block_on(async {
        let cycle_repo = repository_with_chunks([("b", "$chunk(a)"), ("a", "leaf")]).await;
        let cycle_service = atelier_prompt_resources::PromptChunkService::new(cycle_repo.clone());
        let a = cycle_service
            .get_chunk_by_key(&atelier_prompt_resources::PromptChunkKey::parse("a").unwrap())
            .await
            .unwrap()
            .unwrap();
        cycle_service
            .upsert_chunk(UpsertPromptChunkRequest {
                chunk_id: Some(a.id),
                key: atelier_prompt_resources::PromptChunkKey::parse("a").unwrap(),
                content: "$chunk(b)".to_owned(),
                category: None,
                description: None,
                preview_thumb: None,
                models: vec![ImageModel::NaiDiffusion45Full],
            })
            .await
            .unwrap();
        let cycle = PromptCompiler::new(cycle_repo)
            .compile(CompilePromptRequest::new("$chunk(a)"))
            .await
            .unwrap_err();
        assert_eq!(cycle.kind(), PromptResourceErrorKind::Conflict);
        let cycle_chain = vec![
            "chunk:a".to_owned(),
            "chunk:b".to_owned(),
            "chunk:a".to_owned(),
        ];
        assert_eq!(cycle.cycle().unwrap().call_chain(), cycle_chain.as_slice());

        let missing_repo = repository_with_chunks([]).await;
        let missing = PromptCompiler::new(missing_repo)
            .compile(CompilePromptRequest::new("$chunk(not-found)"))
            .await
            .unwrap_err();
        assert_eq!(missing.kind(), PromptResourceErrorKind::NotFound);

        let unknown_repo = repository_with_chunks([]).await;
        let unknown = PromptCompiler::new(unknown_repo)
            .compile(CompilePromptRequest::new("$preset(v4)"))
            .await
            .unwrap_err();
        assert_eq!(unknown.kind(), PromptResourceErrorKind::InvalidRequest);

        let depth_repo = nested_depth_repository(17).await;
        let depth = PromptCompiler::new(depth_repo)
            .compile(CompilePromptRequest::new("$chunk(depth-0)"))
            .await
            .unwrap_err();
        assert_eq!(depth.kind(), PromptResourceErrorKind::Conflict);
    });
}

#[test]
fn compiler_uses_custom_registry_and_traces_empty_outputs() {
    block_on(async {
        let registry = PromptFunctionRegistry::atelier_defaults()
            .with_function(Box::new(EmptyFunction::new()));
        let compiler = PromptCompiler::with_function_registry(
            MemoryPromptResourceRepository::default(),
            registry,
        );

        let result = compiler
            .compile(CompilePromptRequest::new("1girl$empty()solo"))
            .await
            .unwrap();

        assert_eq!(result.expanded_prompt, "1girl, solo");
        assert_eq!(result.trace.function_calls[0].function_name, "empty");
        assert_eq!(result.trace.function_calls[0].result_text, None);
    });
}

#[test]
fn compiler_removes_comments_and_normalizes_boundaries() {
    block_on(async {
        let compiler = PromptCompiler::new(MemoryPromptResourceRepository::default());
        let result = compiler
            .compile(CompilePromptRequest::new(
                r#"1girl, $comment("try composition (B), later"), blue eyes"#,
            ))
            .await
            .unwrap();

        assert_eq!(result.expanded_prompt, "1girl, blue eyes");
        assert_eq!(result.trace.function_calls[0].function_name, "comment");
        assert_eq!(
            result.trace.function_calls[0].resolved_arguments,
            vec!["try composition (B), later".to_owned()]
        );
        assert_eq!(result.trace.function_calls[0].result_text, None);

        let only_comment = compiler
            .compile(CompilePromptRequest::new(r#"$comment("draft")"#))
            .await
            .unwrap();
        assert!(only_comment.expanded_prompt.is_empty());
    });
}

#[test]
fn compiler_rejects_non_string_comment_arguments() {
    block_on(async {
        let compiler = PromptCompiler::new(MemoryPromptResourceRepository::default());
        let error = compiler
            .compile(CompilePromptRequest::new("$comment(draft)"))
            .await
            .unwrap_err();

        assert_eq!(error.kind(), PromptResourceErrorKind::InvalidRequest);
        assert!(error.to_string().contains("expects one string argument"));
    });
}

#[test]
fn compiler_rejects_invalid_chunk_arguments() {
    block_on(async {
        let repository = repository_with_chunks([("face", "portrait")]).await;
        let compiler = PromptCompiler::new(repository);

        let error = compiler
            .compile(CompilePromptRequest::new("$chunk(\"face\")"))
            .await
            .unwrap_err();

        assert_eq!(error.kind(), PromptResourceErrorKind::InvalidRequest);
    });
}

#[test]
fn compiler_rejects_resources_not_bound_to_the_requested_model() {
    block_on(async {
        let repository = repository_with_chunks([("lighting", "cinematic lighting")]).await;
        let compiler = PromptCompiler::new(repository.clone());
        let chunk_error = compiler
            .compile(CompilePromptRequest {
                prompt: "$chunk(lighting)".to_owned(),
                model: ImageModel::NaiDiffusion5Full,
                max_depth: 8,
            })
            .await
            .unwrap_err();
        assert_eq!(chunk_error.kind(), PromptResourceErrorKind::NotFound);

        let preset = PromptPresetService::new(repository.clone())
            .upsert_preset(UpsertPromptPresetRequest {
                preset_id: None,
                kind: PromptPresetKind::Main,
                name: "V4 style".to_owned(),
                category: None,
                description: None,
                order: 0,
                prompt_behavior: PromptPresetBehavior::Surround {
                    before: "detailed".to_owned(),
                    after: String::new(),
                },
                uc_behavior: PromptPresetBehavior::Surround {
                    before: String::new(),
                    after: String::new(),
                },
                quality_override: None,
                uc_preset_override: None,
                preview_thumb: None,
                models: vec![ImageModel::NaiDiffusion45Full],
            })
            .await
            .unwrap();
        let preset_error = compiler
            .compile_generation_prompt(CompileGenerationPromptRequest {
                model: ImageModel::NaiDiffusion5Full,
                main_preset_id: Some(preset.id),
                prompt: "scene".to_owned(),
                negative_prompt: String::new(),
                characters: Vec::new(),
                max_depth: 8,
            })
            .await
            .unwrap_err();
        assert_eq!(preset_error.kind(), PromptResourceErrorKind::InvalidRequest);
    });
}

struct EmptyFunction {
    descriptor: PromptFunctionDescriptor,
}

impl EmptyFunction {
    fn new() -> Self {
        Self {
            descriptor: PromptFunctionDescriptor::new("empty"),
        }
    }
}

#[async_trait]
impl PromptFunction for EmptyFunction {
    fn descriptor(&self) -> &PromptFunctionDescriptor {
        &self.descriptor
    }

    async fn execute(
        &self,
        _call: &ExtensionCall,
        _context: &PromptFunctionContext<'_>,
    ) -> PromptResourceResult<PromptFunctionOutput> {
        Ok(PromptFunctionOutput::default())
    }

    fn resolved_arguments(&self, _call: &ExtensionCall) -> PromptResourceResult<Vec<String>> {
        Ok(Vec::new())
    }
}

async fn repository_with_chunks<const N: usize>(
    chunks: [(&str, &str); N],
) -> MemoryPromptResourceRepository {
    let repository = MemoryPromptResourceRepository::default();
    let service = atelier_prompt_resources::PromptChunkService::new(repository.clone());
    for (key, content) in chunks.into_iter().rev() {
        service
            .upsert_chunk(UpsertPromptChunkRequest {
                chunk_id: None,
                key: atelier_prompt_resources::PromptChunkKey::parse(key).unwrap(),
                content: content.to_owned(),
                category: None,
                description: None,
                preview_thumb: None,
                models: vec![ImageModel::NaiDiffusion45Full],
            })
            .await
            .unwrap();
    }
    repository
}

async fn nested_depth_repository(depth: usize) -> MemoryPromptResourceRepository {
    let repository = MemoryPromptResourceRepository::default();
    let service = atelier_prompt_resources::PromptChunkService::new(repository.clone());
    for index in (0..=depth).rev() {
        let content = if index == depth {
            "leaf".to_owned()
        } else {
            format!("$chunk(depth-{})", index + 1)
        };
        service
            .upsert_chunk(UpsertPromptChunkRequest {
                chunk_id: None,
                key: atelier_prompt_resources::PromptChunkKey::parse(&format!("depth-{index}"))
                    .unwrap(),
                content,
                category: None,
                description: None,
                preview_thumb: None,
                models: vec![ImageModel::NaiDiffusion45Full],
            })
            .await
            .unwrap();
    }
    repository
}
