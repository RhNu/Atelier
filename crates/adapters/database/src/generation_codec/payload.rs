use super::{
    BatchId, CompiledPromptDto, DatabaseResult, Deserialize, GenerationPlanContextDto,
    GenerationRequestPlanDto, GenerationWorkRequestDto, JSON_SCHEMA_VERSION, JobId, JobPayloadRef,
    JsonCodec, PreparedGenerationPayload, Serialize, SubmittedGenerationPayload, ensure_schema,
};

const COMPILED_PAYLOAD_VERSION: u32 = 4;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SubmittedGenerationPayloadDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    compiled_prompt: Option<CompiledPromptDto>,
    schema_version: u32,
    payload_ref: String,
    batch_id: String,
    job_id: String,
    request: GenerationWorkRequestDto,
    context: GenerationPlanContextDto,
}

impl JsonCodec<SubmittedGenerationPayload> for SubmittedGenerationPayloadDto {
    fn from_domain(value: &SubmittedGenerationPayload) -> Self {
        Self {
            schema_version: if value.compiled_prompt.is_some() {
                COMPILED_PAYLOAD_VERSION
            } else {
                JSON_SCHEMA_VERSION
            },
            compiled_prompt: value.compiled_prompt.as_ref().map(CompiledPromptDto::from),
            payload_ref: value.payload_ref.as_str().to_owned(),
            batch_id: value.batch_id.as_str().to_owned(),
            job_id: value.job_id.as_str().to_owned(),
            request: GenerationWorkRequestDto::from(&value.request),
            context: GenerationPlanContextDto::from(&value.context),
        }
    }

    fn into_domain(self) -> DatabaseResult<SubmittedGenerationPayload> {
        let compiled_prompt = match (self.schema_version, self.compiled_prompt) {
            (JSON_SCHEMA_VERSION, None) => None,
            (COMPILED_PAYLOAD_VERSION, Some(prompt)) => Some(prompt.into_domain()),
            _ => {
                return Err(super::decode_error(
                    "submitted payload version or compiled snapshot",
                    &self.schema_version.to_string(),
                ));
            }
        };
        let request = self.request.into_domain()?;
        if let Some(prompt) = &compiled_prompt
            && (prompt.expanded_prompt != request.prompt()
                || prompt.trace.expanded_prompt != prompt.expanded_prompt)
        {
            return Err(super::decode_error("compiled snapshot", "prompt mismatch"));
        }
        Ok(SubmittedGenerationPayload {
            compiled_prompt,
            payload_ref: JobPayloadRef::new(self.payload_ref),
            batch_id: BatchId::new(self.batch_id),
            job_id: JobId::new(self.job_id),
            request,
            context: self.context.into_domain(),
        })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PreparedGenerationPayloadDto {
    schema_version: u32,
    payload_ref: String,
    submitted_payload_ref: String,
    batch_id: String,
    job_id: String,
    request: GenerationWorkRequestDto,
    compiled_prompt: CompiledPromptDto,
    plan: GenerationRequestPlanDto,
}

impl JsonCodec<PreparedGenerationPayload> for PreparedGenerationPayloadDto {
    fn from_domain(value: &PreparedGenerationPayload) -> Self {
        Self {
            schema_version: JSON_SCHEMA_VERSION,
            payload_ref: value.payload_ref.as_str().to_owned(),
            submitted_payload_ref: value.submitted_payload_ref.as_str().to_owned(),
            batch_id: value.batch_id.as_str().to_owned(),
            job_id: value.job_id.as_str().to_owned(),
            request: GenerationWorkRequestDto::from(&value.request),
            compiled_prompt: CompiledPromptDto::from(&value.compiled_prompt),
            plan: GenerationRequestPlanDto::from(&value.plan),
        }
    }

    fn into_domain(self) -> DatabaseResult<PreparedGenerationPayload> {
        ensure_schema(self.schema_version)?;
        Ok(PreparedGenerationPayload {
            payload_ref: JobPayloadRef::new(self.payload_ref),
            submitted_payload_ref: JobPayloadRef::new(self.submitted_payload_ref),
            batch_id: BatchId::new(self.batch_id),
            job_id: JobId::new(self.job_id),
            request: self.request.into_domain()?,
            compiled_prompt: self.compiled_prompt.into_domain(),
            plan: self.plan.into_domain()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use atelier_generation::{GenerateImageRequest, GenerationPlanContext};
    use atelier_kernel::GenerationWorkRequest;
    use atelier_prompt_resources::{CompiledPrompt, PromptTrace};

    fn submitted(compiled: bool) -> SubmittedGenerationPayload {
        SubmittedGenerationPayload {
            compiled_prompt: compiled.then(|| CompiledPrompt {
                expanded_prompt: "resolved".to_owned(),
                trace: PromptTrace {
                    raw_prompt: "$chunk(hero)".to_owned(),
                    expanded_prompt: "resolved".to_owned(),
                    function_calls: Vec::new(),
                },
            }),
            payload_ref: JobPayloadRef::new("payload"),
            batch_id: BatchId::new("batch"),
            job_id: JobId::new("job"),
            request: GenerationWorkRequest::Image(GenerateImageRequest {
                prompt: "resolved".to_owned(),
                ..Default::default()
            }),
            context: GenerationPlanContext::default(),
        }
    }

    #[test]
    fn submitted_versions_preserve_the_compilation_stage() {
        for compiled in [false, true] {
            let payload = submitted(compiled);
            let json =
                serde_json::to_value(SubmittedGenerationPayloadDto::from_domain(&payload)).unwrap();
            assert_eq!(json["schema_version"], if compiled { 4 } else { 3 });
            assert_eq!(json.get("compiled_prompt").is_some(), compiled);
            let decoded: SubmittedGenerationPayloadDto = serde_json::from_value(json).unwrap();
            assert_eq!(decoded.into_domain().unwrap(), payload);
        }
    }

    #[test]
    fn submitted_rejects_ambiguous_versions_and_inconsistent_snapshots() {
        for (compiled, version) in [(false, 4), (true, 3), (true, 5)] {
            let mut dto = SubmittedGenerationPayloadDto::from_domain(&submitted(compiled));
            dto.schema_version = version;
            assert!(dto.into_domain().is_err());
        }
        let mut payload = submitted(true);
        payload.compiled_prompt.as_mut().unwrap().expanded_prompt = "different".to_owned();
        assert!(
            SubmittedGenerationPayloadDto::from_domain(&payload)
                .into_domain()
                .is_err()
        );
    }
}
