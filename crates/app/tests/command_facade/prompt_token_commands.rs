use super::*;

#[test]
fn prompt_token_count_uses_compiled_effective_request_text() {
    block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let host = test_host();
        open_workspace(&host, &temp).await;
        upsert_hero_chunk(&host).await;

        let count = |prompt: &str, quality| CountPromptTokensRequestDto {
            compile: CompileGenerationPromptRequestDto {
                model: ImageModelDto::NaiDiffusion45Full,
                main_preset_id: None,
                prompt: prompt.to_owned(),
                negative_prompt: None,
                characters: Vec::new(),
                max_depth: 16,
            },
            quality,
            transparent_background: false,
            uc_preset: UcPresetDto::None,
            furry_mode: false,
        };

        let compiled = host
            .count_prompt_tokens(count(
                r#"$chunk(hero), $comment("not sent to NovelAI")"#,
                QualityPresetDto::None,
            ))
            .await
            .unwrap();
        let plain = host
            .count_prompt_tokens(count("1girl", QualityPresetDto::None))
            .await
            .unwrap();
        let with_quality = host
            .count_prompt_tokens(count("1girl", QualityPresetDto::Standard))
            .await
            .unwrap();

        assert_eq!(compiled.prompt, plain.prompt);
        assert!(with_quality.prompt.used > plain.prompt.used);
    });
}
