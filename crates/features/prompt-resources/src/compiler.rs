use std::future::Future;
use std::pin::Pin;

use atelier_generation::{ImageModel, QualityPreset};
use atelier_prompt::{ExtensionCall, parse_prompt};

use crate::functions::{PromptFunctionContext, PromptFunctionRegistry, PromptFunctionTraceEntry};
use crate::text::{ExpandedPromptFragment, render_expanded_prompt_fragments};
use crate::{
    PromptPreset, PromptPresetBehavior, PromptPresetId, PromptPresetKind, PromptResourceError,
    PromptResourceReader, PromptResourceResult,
};

const DEFAULT_MAX_EXPANSION_DEPTH: usize = 16;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompilePromptRequest {
    pub prompt: String,
    pub model: ImageModel,
    pub max_depth: usize,
}

impl CompilePromptRequest {
    #[must_use]
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            model: ImageModel::default(),
            max_depth: DEFAULT_MAX_EXPANSION_DEPTH,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompiledPrompt {
    pub expanded_prompt: String,
    pub trace: PromptTrace,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PromptTrace {
    pub raw_prompt: String,
    pub expanded_prompt: String,
    pub function_calls: Vec<PromptFunctionTraceEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompileCharacterPromptRequest {
    pub character_index: u32,
    pub preset_id: Option<PromptPresetId>,
    pub prompt: String,
    pub negative_prompt: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompileGenerationPromptRequest {
    pub model: ImageModel,
    pub main_preset_id: Option<PromptPresetId>,
    pub prompt: String,
    pub negative_prompt: String,
    pub characters: Vec<CompileCharacterPromptRequest>,
    pub max_depth: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompiledCharacterPrompt {
    pub character_index: u32,
    pub prompt: String,
    pub negative_prompt: String,
    pub trace: PromptTrace,
    pub negative_trace: PromptTrace,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompiledGenerationPrompt {
    pub prompt: String,
    pub negative_prompt: String,
    pub characters: Vec<CompiledCharacterPrompt>,
    pub quality_override: Option<QualityPreset>,
    pub uc_preset_override: Option<String>,
    pub trace: PromptOrchestrationTrace,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PromptOrchestrationTrace {
    pub used_presets: Vec<UsedPromptPresetTrace>,
    pub main_prompt: Option<PromptTrace>,
    pub main_negative_prompt: Option<PromptTrace>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UsedPromptPresetTrace {
    pub preset_id: PromptPresetId,
    pub kind: PromptPresetKind,
    pub name: String,
}

struct AppliedPromptFields {
    prompt: String,
    negative_prompt: String,
}

struct AppliedMainPromptFields {
    prompt: String,
    negative_prompt: String,
    quality_override: Option<QualityPreset>,
    uc_preset_override: Option<String>,
}

pub struct PromptCompiler<R> {
    repository: R,
    functions: PromptFunctionRegistry,
}

impl<R> PromptCompiler<R> {
    #[must_use]
    pub fn new(repository: R) -> Self {
        Self {
            repository,
            functions: PromptFunctionRegistry::atelier_defaults(),
        }
    }

    #[must_use]
    pub const fn with_function_registry(repository: R, functions: PromptFunctionRegistry) -> Self {
        Self {
            repository,
            functions,
        }
    }
}

impl<R> PromptCompiler<R>
where
    R: PromptResourceReader,
{
    /// Expands registered prompt functions and returns the compiled prompt.
    ///
    /// # Errors
    /// Returns an error when a function is unknown, invalid, missing its target
    /// resource, exceeds maximum depth, or enters a cycle.
    pub async fn compile(
        &self,
        request: CompilePromptRequest,
    ) -> PromptResourceResult<CompiledPrompt> {
        let mut trace = Vec::new();
        let expanded_prompt = self
            .expand_text(
                &request.prompt,
                request.model,
                0,
                request.max_depth,
                &mut Vec::new(),
                &mut trace,
            )
            .await?;
        Ok(CompiledPrompt {
            expanded_prompt: expanded_prompt.clone(),
            trace: PromptTrace {
                raw_prompt: request.prompt,
                expanded_prompt,
                function_calls: trace,
            },
        })
    }

    /// Applies prompt preset bindings, expands prompt functions, and returns
    /// compiled generation prompt fields.
    ///
    /// # Errors
    /// Returns an error when a referenced preset is missing or has the wrong
    /// kind, or when prompt function expansion fails.
    pub async fn compile_generation_prompt(
        &self,
        request: CompileGenerationPromptRequest,
    ) -> PromptResourceResult<CompiledGenerationPrompt> {
        let mut trace = PromptOrchestrationTrace::default();
        let AppliedMainPromptFields {
            prompt,
            negative_prompt,
            quality_override,
            uc_preset_override,
        } = self
            .apply_main_preset(
                request.main_preset_id.as_ref(),
                request.model,
                request.prompt,
                request.negative_prompt,
                &mut trace,
            )
            .await?;

        let prompt = self
            .compile_prompt_text(prompt, request.model, request.max_depth)
            .await?;
        let negative_prompt = self
            .compile_prompt_text(negative_prompt, request.model, request.max_depth)
            .await?;
        trace.main_prompt = Some(prompt.trace.clone());
        trace.main_negative_prompt = Some(negative_prompt.trace.clone());

        let characters = self
            .compile_character_prompts(
                request.characters,
                request.model,
                request.max_depth,
                &mut trace,
            )
            .await?;

        Ok(CompiledGenerationPrompt {
            prompt: prompt.expanded_prompt,
            negative_prompt: negative_prompt.expanded_prompt,
            characters,
            quality_override,
            uc_preset_override,
            trace,
        })
    }

    async fn compile_character_prompts(
        &self,
        characters: Vec<CompileCharacterPromptRequest>,
        model: ImageModel,
        max_depth: usize,
        trace: &mut PromptOrchestrationTrace,
    ) -> PromptResourceResult<Vec<CompiledCharacterPrompt>> {
        let mut compiled = Vec::with_capacity(characters.len());
        for character in characters {
            let character_index = character.character_index;
            let AppliedPromptFields {
                prompt,
                negative_prompt,
            } = self.apply_character_preset(character, model, trace).await?;
            let compiled_prompt = self.compile_prompt_text(prompt, model, max_depth).await?;
            let compiled_negative_prompt = self
                .compile_prompt_text(negative_prompt, model, max_depth)
                .await?;
            compiled.push(CompiledCharacterPrompt {
                character_index,
                prompt: compiled_prompt.expanded_prompt,
                negative_prompt: compiled_negative_prompt.expanded_prompt,
                trace: compiled_prompt.trace,
                negative_trace: compiled_negative_prompt.trace,
            });
        }
        Ok(compiled)
    }

    async fn apply_main_preset(
        &self,
        preset_id: Option<&PromptPresetId>,
        model: ImageModel,
        prompt: String,
        negative_prompt: String,
        trace: &mut PromptOrchestrationTrace,
    ) -> PromptResourceResult<AppliedMainPromptFields> {
        let Some(preset_id) = preset_id else {
            return Ok(AppliedMainPromptFields {
                prompt,
                negative_prompt,
                quality_override: None,
                uc_preset_override: None,
            });
        };
        let preset = self
            .require_preset(preset_id, PromptPresetKind::Main, model)
            .await?;
        trace_used_preset(trace, &preset);
        let fields = apply_preset_fields(&prompt, &negative_prompt, &preset);
        Ok(AppliedMainPromptFields {
            prompt: fields.prompt,
            negative_prompt: fields.negative_prompt,
            quality_override: preset.quality_override,
            uc_preset_override: preset.uc_preset_override,
        })
    }

    async fn apply_character_preset(
        &self,
        character: CompileCharacterPromptRequest,
        model: ImageModel,
        trace: &mut PromptOrchestrationTrace,
    ) -> PromptResourceResult<AppliedPromptFields> {
        let Some(preset_id) = &character.preset_id else {
            return Ok(AppliedPromptFields {
                prompt: character.prompt,
                negative_prompt: character.negative_prompt,
            });
        };
        let preset = self
            .require_preset(preset_id, PromptPresetKind::Character, model)
            .await?;
        trace_used_preset(trace, &preset);
        Ok(apply_preset_fields(
            &character.prompt,
            &character.negative_prompt,
            &preset,
        ))
    }

    async fn compile_prompt_text(
        &self,
        prompt: String,
        model: ImageModel,
        max_depth: usize,
    ) -> PromptResourceResult<CompiledPrompt> {
        self.compile(CompilePromptRequest {
            prompt,
            model,
            max_depth,
        })
        .await
    }

    async fn require_preset(
        &self,
        id: &PromptPresetId,
        kind: PromptPresetKind,
        model: ImageModel,
    ) -> PromptResourceResult<PromptPreset> {
        let preset = self
            .repository
            .get_preset_by_id(id)
            .await?
            .ok_or_else(|| PromptResourceError::not_found("preset does not exist"))?;
        if preset.kind != kind {
            return Err(PromptResourceError::invalid_request(format!(
                "preset `{}` has the wrong kind",
                id.as_str()
            )));
        }
        if !preset.models.contains(&model) {
            return Err(PromptResourceError::invalid_request(format!(
                "preset `{}` is not bound to model `{}`",
                id.as_str(),
                model.as_str()
            )));
        }
        Ok(preset)
    }

    fn expand_text<'a>(
        &'a self,
        text: &'a str,
        model: ImageModel,
        depth: usize,
        max_depth: usize,
        active_scopes: &'a mut Vec<String>,
        trace: &'a mut Vec<PromptFunctionTraceEntry>,
    ) -> Pin<Box<dyn Future<Output = PromptResourceResult<String>> + Send + 'a>> {
        Box::pin(async move {
            let mut current = text.to_owned();
            loop {
                let expanded = self
                    .expand_one_pass(&current, model, depth, max_depth, active_scopes, trace)
                    .await?;
                if expanded == current {
                    return Ok(expanded);
                }
                current = expanded;
            }
        })
    }

    async fn expand_one_pass(
        &self,
        text: &str,
        model: ImageModel,
        depth: usize,
        max_depth: usize,
        active_scopes: &mut Vec<String>,
        trace: &mut Vec<PromptFunctionTraceEntry>,
    ) -> PromptResourceResult<String> {
        let parsed = parse_prompt(text);
        let ast = parsed.ast();
        if ast.extension_calls().is_empty() {
            return Ok(text.to_owned());
        }

        let mut cursor = 0;
        let mut fragments = Vec::new();
        for call in ast.extension_calls() {
            fragments.push(ExpandedPromptFragment::text(&text[cursor..call.span.start]));
            let replacement = self
                .expand_call(call, text, model, (depth, max_depth), active_scopes, trace)
                .await?;
            fragments.push(ExpandedPromptFragment::expansion(replacement));
            cursor = call.span.end;
        }
        fragments.push(ExpandedPromptFragment::text(&text[cursor..]));
        Ok(render_expanded_prompt_fragments(fragments))
    }

    async fn expand_call(
        &self,
        call: &ExtensionCall,
        source: &str,
        model: ImageModel,
        depth_and_limit: (usize, usize),
        active_scopes: &mut Vec<String>,
        trace: &mut Vec<PromptFunctionTraceEntry>,
    ) -> PromptResourceResult<String> {
        let (depth, max_depth) = depth_and_limit;
        let call_depth = depth + 1;
        if call_depth > max_depth {
            return Err(PromptResourceError::conflict(
                "prompt function expansion depth exceeded",
            ));
        }
        let function = self.functions.get(&call.name)?;
        let context = PromptFunctionContext {
            resources: &self.repository,
            model,
        };
        let output = function.execute(call, &context).await?;
        if let Some(scope) = &output.scope
            && active_scopes.iter().any(|item| item == scope)
        {
            let mut call_chain = active_scopes.clone();
            call_chain.push(scope.clone());
            return Err(PromptResourceError::cycle_detected(call_chain));
        }
        let raw_call = source[call.span.start..call.span.end].to_owned();
        let resolved_arguments = function.resolved_arguments(call)?;
        trace.push(PromptFunctionTraceEntry {
            function_name: function.descriptor().name.clone(),
            raw_call,
            resolved_arguments,
            result_text: output.text.clone(),
            depth: call_depth,
            call_chain: active_scopes.clone(),
        });
        let Some(text) = output.text else {
            return Ok(String::new());
        };
        if let Some(scope) = output.scope {
            active_scopes.push(scope);
            let expanded = self
                .expand_text(&text, model, call_depth, max_depth, active_scopes, trace)
                .await;
            active_scopes.pop();
            expanded
        } else {
            self.expand_text(&text, model, call_depth, max_depth, active_scopes, trace)
                .await
        }
    }
}

fn apply_prompt_behavior(base: &str, behavior: &PromptPresetBehavior) -> String {
    match behavior {
        PromptPresetBehavior::Surround { before, after } => render_expanded_prompt_fragments(
            [before.as_str(), base, after.as_str()]
                .into_iter()
                .filter(|fragment| !fragment.trim().is_empty())
                .map(|fragment| ExpandedPromptFragment::expansion(fragment.to_owned())),
        ),
        PromptPresetBehavior::Replace { text } => text.clone(),
    }
}

fn apply_preset_fields(
    prompt: &str,
    negative_prompt: &str,
    preset: &PromptPreset,
) -> AppliedPromptFields {
    AppliedPromptFields {
        prompt: apply_prompt_behavior(prompt, &preset.prompt_behavior),
        negative_prompt: apply_prompt_behavior(negative_prompt, &preset.uc_behavior),
    }
}

fn trace_used_preset(trace: &mut PromptOrchestrationTrace, preset: &PromptPreset) {
    trace.used_presets.push(UsedPromptPresetTrace {
        preset_id: preset.id.clone(),
        kind: preset.kind,
        name: preset.name.clone(),
    });
}
