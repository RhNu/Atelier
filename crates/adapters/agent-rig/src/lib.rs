//! Rig-backed OpenAI-compatible runtime for Atelier's internal Agent.

mod context_budget;
mod executions;
mod invocation;
use executions::ToolExecutions;

use invocation::{InvocationSlot, RuntimeHook};
use std::sync::Arc;

use async_trait::async_trait;
use atelier_agent::{
    AgentChatRole, AgentError, AgentModelRuntime, AgentResolvedConnection, AgentResult,
    AgentRunObserver, AgentRunOutcome, AgentRunRequest, AgentRuntimeEvent, AgentToolExecutor,
    DiscoveredAgentModel,
};
use futures_util::StreamExt;
use rig_agent::{
    AgentBuilder,
    agent::MultiTurnStreamItem,
    streaming::{StreamedAssistantContent, StreamingChat},
    tool::{DynamicTool, ToolExecutionError, ToolOutput},
};
use rig_core::{
    client::{CompletionClient, ModelListingClient},
    message::Message,
    providers::openai,
};
use rig_memory::{HeuristicTokenCounter, MemoryPolicy, TokenWindowMemory};
use tokio_util::sync::CancellationToken;

const MAX_MODEL_CALLS: usize = 48;
const MIN_HISTORY_BUDGET: usize = 1_024;
const RESERVED_CONTEXT_TOKENS: u32 = 2_048;

#[derive(Clone, Debug, Default)]
pub struct RigAgentRuntime;

#[async_trait]
impl AgentModelRuntime for RigAgentRuntime {
    async fn discover_models(
        &self,
        connection: AgentResolvedConnection,
    ) -> AgentResult<Vec<DiscoveredAgentModel>> {
        let client = openai_client(&connection)?;
        let models = client
            .list_models()
            .await
            .map_err(|error| AgentError::runtime(format!("model discovery failed: {error}")))?;
        let mut discovered = models
            .into_iter()
            .map(|model| DiscoveredAgentModel {
                display_name: model.name.unwrap_or_else(|| model.id.clone()),
                wire_model_id: model.id,
                context_window: model.context_length,
                max_output_tokens: model.max_output_tokens,
            })
            .collect::<Vec<_>>();
        discovered.sort_by(|left, right| left.wire_model_id.cmp(&right.wire_model_id));
        Ok(discovered)
    }

    async fn run(
        &self,
        request: AgentRunRequest,
        tools: Arc<dyn AgentToolExecutor>,
        observer: Arc<dyn AgentRunObserver>,
        cancellation: CancellationToken,
    ) -> AgentResult<AgentRunOutcome> {
        let client = openai_client(&request.connection)?.completions_api();
        let model = client.completion_model(request.model.wire_model_id.clone());
        let invocation = Arc::new(InvocationSlot::default());
        let executions = Arc::new(ToolExecutions::default());
        let dynamic_tools = build_tools(&request, &tools, &observer, &invocation, &executions);
        let agent = AgentBuilder::new(model)
            .name("Atelier Agent")
            .preamble(&request.system_prompt)
            .temperature(f64::from(request.model.temperature))
            .max_tokens(u64::from(request.model.max_output_tokens))
            .default_max_turns(MAX_MODEL_CALLS)
            .dynamic_tools(dynamic_tools)
            .build();
        let history = shape_history(&request);
        let budget = context_budget::ContextBudget::new(&request);
        let mut stream = agent
            .stream_chat(Message::user(request.user_message), history)
            .add_hook(RuntimeHook {
                executor: tools,
                invocation,
                budget,
            })
            .max_turns(MAX_MODEL_CALLS)
            .tool_concurrency(1)
            .await;
        let mut outcome = AgentRunOutcome::default();

        let result = async {
            loop {
                let item = tokio::select! {
                    () = cancellation.cancelled() => return Err(AgentError::cancelled()),
                    item = stream.next() => item,
                };
                let Some(item) = item else {
                    break;
                };
                match item.map_err(|error| AgentError::runtime(error.to_string()))? {
                    MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::Text(
                        text,
                    )) => {
                        outcome.assistant_message.push_str(&text.text);
                        observer.emit(AgentRuntimeEvent::AssistantTextDelta { text: text.text });
                    }
                    MultiTurnStreamItem::CompletionCall(call) => {
                        outcome.model_calls = outcome.model_calls.saturating_add(1);
                        outcome.input_tokens =
                            outcome.input_tokens.saturating_add(call.usage.input_tokens);
                        outcome.output_tokens = outcome
                            .output_tokens
                            .saturating_add(call.usage.output_tokens);
                        observer.emit(AgentRuntimeEvent::Usage {
                            input_tokens: call.usage.input_tokens,
                            output_tokens: call.usage.output_tokens,
                        });
                    }
                    MultiTurnStreamItem::FinalResponse(response)
                        if outcome.assistant_message.is_empty() =>
                    {
                        outcome.assistant_message = response.output;
                    }
                    _ => {}
                }
            }
            Ok(outcome)
        }
        .await;
        if result.is_err() {
            cancellation.cancel();
        }
        drop(stream);
        executions.drain().await?;
        result
    }
}

fn openai_client(
    connection: &AgentResolvedConnection,
) -> AgentResult<openai::Client<reqwest::Client>> {
    openai::Client::builder()
        .api_key(connection.bearer_token.clone().unwrap_or_default())
        .base_url(connection.base_url.trim_end_matches('/'))
        .build()
        .map_err(|error| AgentError::runtime(format!("failed to configure model client: {error}")))
}

fn build_tools(
    request: &AgentRunRequest,
    executor: &Arc<dyn AgentToolExecutor>,
    observer: &Arc<dyn AgentRunObserver>,
    invocation: &Arc<InvocationSlot>,
    executions: &Arc<ToolExecutions>,
) -> Vec<DynamicTool> {
    request
        .tools
        .iter()
        .map(|spec| {
            let executions = executions.clone();
            let executor = executor.clone();
            let invocation = invocation.clone();
            let observer = observer.clone();
            let name = spec.name.clone();
            let callback_name = name.clone();
            let parameters = serde_json::from_str(&spec.parameters_json)
                .unwrap_or_else(|_| serde_json::json!({"type": "object", "properties": {}}));
            DynamicTool::new(
                name,
                spec.description.clone(),
                parameters,
                move |_context, arguments| {
                    let executions = executions.clone();
                    let executor = executor.clone();
                    let invocation = invocation.clone();
                    let observer = observer.clone();
                    let name = callback_name.clone();
                    Box::pin(async move {
                        let call_id = invocation
                            .take(&name, &arguments)
                            .map_err(|error| ToolExecutionError::provider(error.to_string()))?;
                        let arguments_json = arguments.to_string();
                        observer.emit(AgentRuntimeEvent::ToolStarted {
                            name: name.clone(),
                            arguments_json: arguments_json.clone(),
                        });
                        let result = executions
                            .start(executor, observer, call_id, name, arguments_json)
                            .await
                            .map_err(|error| ToolExecutionError::provider(error.to_string()))?;
                        match result {
                            Ok(output) => tool_output(output),
                            Err(error) => {
                                let feedback = error.to_string();
                                Err(ToolExecutionError::provider(feedback.clone())
                                    .with_model_output(ToolOutput::text(feedback)))
                            }
                        }
                    })
                },
            )
        })
        .collect()
}

fn tool_output(output: atelier_agent::AgentToolOutput) -> Result<ToolOutput, ToolExecutionError> {
    use atelier_agent::AgentImageMediaType;
    use rig_core::message::{ImageMediaType, ToolResultContent};
    let mut content = vec![ToolResultContent::text(output.text)];
    content.extend(output.images.into_iter().map(|image| {
        ToolResultContent::image_base64(
            image.base64,
            Some(match image.media_type {
                AgentImageMediaType::Png => ImageMediaType::PNG,
                AgentImageMediaType::Jpeg => ImageMediaType::JPEG,
                AgentImageMediaType::Webp => ImageMediaType::WEBP,
            }),
            None,
        )
    }));
    ToolOutput::content(content).map_err(|error| ToolExecutionError::provider(error.to_string()))
}

fn shape_history(request: &AgentRunRequest) -> Vec<Message> {
    let messages = request
        .history
        .iter()
        .map(|message| match message.role {
            AgentChatRole::User => Message::user(message.content.clone()),
            AgentChatRole::Assistant => Message::assistant(message.content.clone()),
        })
        .collect();
    let available = request
        .model
        .context_window
        .saturating_sub(request.model.max_output_tokens)
        .saturating_sub(RESERVED_CONTEXT_TOKENS) as usize;
    TokenWindowMemory::new(
        available.max(MIN_HISTORY_BUDGET),
        HeuristicTokenCounter::openai(),
    )
    .apply(messages)
    .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use atelier_agent::{AgentChatMessage, AgentConnectionId, AgentModelId, AgentModelSnapshot};

    #[test]
    fn history_window_keeps_the_latest_messages() {
        let request = AgentRunRequest {
            connection: AgentResolvedConnection {
                base_url: "http://localhost:1234/v1".to_owned(),
                bearer_token: None,
            },
            model: AgentModelSnapshot {
                model_id: AgentModelId::new("model"),
                connection_id: AgentConnectionId::new("connection"),
                wire_model_id: "wire".to_owned(),
                display_name: "Wire".to_owned(),
                context_window: 3_200,
                max_output_tokens: 1_000,
                temperature: 0.3,
                supports_vision: false,
            },
            system_prompt: String::new(),
            user_message: "next".to_owned(),
            history: (0..40)
                .map(|index| AgentChatMessage {
                    role: if index % 2 == 0 {
                        AgentChatRole::User
                    } else {
                        AgentChatRole::Assistant
                    },
                    content: "x".repeat(400),
                })
                .collect(),
            tools: Vec::new(),
        };

        let shaped = shape_history(&request);
        assert!(shaped.len() < request.history.len());
        assert!(!shaped.is_empty());
    }
    #[test]
    fn tool_images_are_native_content_blocks() {
        let output = tool_output(atelier_agent::AgentToolOutput {
            text: "output metadata".into(),
            images: vec![atelier_agent::AgentToolImage {
                media_type: atelier_agent::AgentImageMediaType::Png,
                base64: "AA==".into(),
            }],
        })
        .unwrap();
        let content = output.as_content();
        assert_eq!(content.len(), 2);
        assert!(matches!(
            &content[0],
            rig_core::message::ToolResultContent::Text(_)
        ));
        assert!(matches!(
            &content[1],
            rig_core::message::ToolResultContent::Image(_)
        ));
    }
}
