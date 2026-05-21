use super::{
    BatchId, CompiledPromptDto, DatabaseResult, Deserialize, GenerationPlanContextDto,
    GenerationRequestPlanDto, GenerationWorkRequestDto, JSON_SCHEMA_VERSION, JobId, JobPayloadRef,
    JsonCodec, PreparedGenerationPayload, Serialize, SubmittedGenerationPayload, ensure_schema,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SubmittedGenerationPayloadDto {
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
            schema_version: JSON_SCHEMA_VERSION,
            payload_ref: value.payload_ref.as_str().to_owned(),
            batch_id: value.batch_id.as_str().to_owned(),
            job_id: value.job_id.as_str().to_owned(),
            request: GenerationWorkRequestDto::from(&value.request),
            context: GenerationPlanContextDto::from(&value.context),
        }
    }

    fn into_domain(self) -> DatabaseResult<SubmittedGenerationPayload> {
        ensure_schema(self.schema_version)?;
        Ok(SubmittedGenerationPayload {
            payload_ref: JobPayloadRef::new(self.payload_ref),
            batch_id: BatchId::new(self.batch_id),
            job_id: JobId::new(self.job_id),
            request: self.request.into_domain()?,
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
