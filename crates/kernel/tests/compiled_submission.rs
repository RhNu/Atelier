mod support;

use atelier_generation::{GenerateImageRequest, GenerationPlanContext};
use atelier_jobs::{BatchId, JobId};
use atelier_kernel::{GenerationWorkRequest, KernelRuntime, SubmitGenerationWork};
use atelier_prompt_resources::{CompiledPrompt, PromptTrace};
use futures_executor::block_on;
use support::MemoryKernelPorts;

#[test]
fn compiled_submission_does_not_reinterpret_function_like_text() {
    block_on(async {
        let ports = MemoryKernelPorts::default()
            .failing_compile_prompt()
            .with_generated_images(vec![atelier_generation::GeneratedImage {
                bytes: vec![1, 2, 3],
                mime_type: Some("image/png".to_owned()),
                metadata: atelier_generation::GeneratedImageMetadata::NotPresent,
            }]);
        let mut runtime = KernelRuntime::new(ports.clone());
        let prompt = "$chunk(literal)";
        runtime
            .submit_generation_work(SubmitGenerationWork {
                batch_id: BatchId::new("compiled-batch"),
                job_id: JobId::new("compiled-job"),
                request: GenerationWorkRequest::Image(GenerateImageRequest {
                    prompt: prompt.to_owned(),
                    ..Default::default()
                }),
                compiled_prompt: Some(CompiledPrompt {
                    expanded_prompt: prompt.to_owned(),
                    trace: PromptTrace {
                        raw_prompt: "original".to_owned(),
                        expanded_prompt: prompt.to_owned(),
                        function_calls: Vec::new(),
                    },
                }),
                context: GenerationPlanContext::default(),
            })
            .await
            .unwrap();
        runtime
            .run_scheduled_generation_job(&JobId::new("compiled-job"))
            .await
            .unwrap();
        assert_eq!(ports.generated_requests()[0].prompt, prompt);
    });
}
