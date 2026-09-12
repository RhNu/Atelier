//! Compatibility for version-3 submitted payloads. These may contain raw prompts.
//! Current app submissions already contain a complete compiled snapshot.
use crate::{GenerationWorkRequest, KernelGenerationPorts};
use atelier_generation::{GenerateImageRequest, ImageModel};
use atelier_prompt_resources::{CompilePromptRequest, CompiledPrompt, PromptResourceResult};

pub async fn resolve<P: KernelGenerationPorts>(
    ports: &P,
    request: &GenerationWorkRequest,
) -> PromptResourceResult<(GenerationWorkRequest, CompiledPrompt)> {
    let compiled = compile_generation_prompts(ports, request).await?;
    Ok((apply_work(request.clone(), &compiled), compiled.prompt))
}
async fn compile_generation_prompts<P>(
    ports: &P,
    request: &GenerationWorkRequest,
) -> PromptResourceResult<CompiledGenerationPrompts>
where
    P: KernelGenerationPorts,
{
    let model = request.model();
    let prompt = ports
        .compile_legacy_prompt(CompilePromptRequest::new(request.prompt(), model))
        .await?;
    let negative_prompt = compile_optional_prompt(ports, request.negative_prompt(), model).await?;
    let mut characters = Vec::new();
    if let Some(request_characters) = request.characters() {
        characters.reserve(request_characters.len());
        for character in request_characters {
            characters.push(CompiledGenerationCharacterPrompts {
                prompt: compile_optional_prompt(ports, Some(character.prompt.as_str()), model)
                    .await?,
                negative_prompt: compile_optional_prompt(
                    ports,
                    character.negative_prompt.as_deref(),
                    model,
                )
                .await?,
            });
        }
    }
    Ok(CompiledGenerationPrompts {
        prompt,
        negative_prompt,
        characters,
    })
}

async fn compile_optional_prompt<P>(
    ports: &P,
    prompt: Option<&str>,
    model: ImageModel,
) -> PromptResourceResult<Option<atelier_prompt_resources::CompiledPrompt>>
where
    P: KernelGenerationPorts,
{
    let Some(prompt) = prompt else {
        return Ok(None);
    };
    if prompt.trim().is_empty() {
        return Ok(None);
    }
    ports
        .compile_legacy_prompt(CompilePromptRequest::new(prompt, model))
        .await
        .map(Some)
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CompiledGenerationCharacterPrompts {
    prompt: Option<CompiledPrompt>,
    negative_prompt: Option<CompiledPrompt>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CompiledGenerationPrompts {
    prompt: CompiledPrompt,
    negative_prompt: Option<CompiledPrompt>,
    characters: Vec<CompiledGenerationCharacterPrompts>,
}

fn apply_compiled_prompts(
    request: &mut GenerateImageRequest,
    compiled: &CompiledGenerationPrompts,
) {
    request.prompt.clone_from(&compiled.prompt.expanded_prompt);
    if let Some(negative_prompt) = &compiled.negative_prompt {
        request.negative_prompt = Some(negative_prompt.expanded_prompt.clone());
    }
    if let Some(characters) = &mut request.characters {
        for (character, compiled_character) in characters.iter_mut().zip(&compiled.characters) {
            if let Some(prompt) = &compiled_character.prompt {
                character.prompt.clone_from(&prompt.expanded_prompt);
            }
            if let Some(negative_prompt) = &compiled_character.negative_prompt {
                character.negative_prompt = Some(negative_prompt.expanded_prompt.clone());
            }
        }
    }
}

fn apply_work(
    mut request: GenerationWorkRequest,
    compiled: &CompiledGenerationPrompts,
) -> GenerationWorkRequest {
    match &mut request {
        GenerationWorkRequest::Image(value) => apply_compiled_prompts(value, compiled),
        GenerationWorkRequest::Stream(value) => apply_compiled_prompts(&mut value.base, compiled),
    }
    request
}
