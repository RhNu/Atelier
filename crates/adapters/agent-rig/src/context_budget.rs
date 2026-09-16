use atelier_agent::{AgentError, AgentImageInputMode, AgentResult, AgentRunRequest};
use rig_core::message::{Message, ToolResultContent, UserContent};
use rig_memory::{HeuristicTokenCounter, TokenCounter};

pub struct ContextBudget {
    available: usize,
    counter: HeuristicTokenCounter,
    image_input: AgentImageInputMode,
}

impl ContextBudget {
    pub fn new(request: &AgentRunRequest) -> Self {
        // Conservative estimates; the actual provider tokenizer remains authoritative.
        let counter = HeuristicTokenCounter::new(3.0, 8, 4_096);
        let static_tokens = counter.count(&Message::user(request.system_prompt.clone()))
            + request
                .tools
                .iter()
                .map(|tool| {
                    counter.count(&Message::user(format!(
                        "{} {} {}",
                        tool.name, tool.description, tool.parameters_json
                    )))
                })
                .sum::<usize>();
        let available = (request.model.context_window as usize)
            .saturating_sub(request.model.max_output_tokens as usize)
            .saturating_sub(static_tokens)
            .saturating_sub(2_048);
        Self {
            available,
            counter,
            image_input: request.model.capabilities.image_input,
        }
    }

    pub fn prepare(&self, history: &[Message], prompt: &Message) -> AgentResult<Vec<Message>> {
        let mut history = history.to_vec();
        if self.image_input == AgentImageInputMode::Message {
            move_tool_images_to_messages(&mut history);
        }
        let keep = 2_usize.saturating_sub(image_count(prompt));
        retain_recent_images(&mut history, keep);
        let used = self.counter.count(prompt)
            + history
                .iter()
                .map(|message| self.counter.count(message))
                .sum::<usize>();
        if used > self.available {
            return Err(AgentError::runtime(
                "Agent context budget reached; continue in a new turn. Applied edits and generated outputs are saved.",
            ));
        }
        Ok(history)
    }
}

fn move_tool_images_to_messages(history: &mut [Message]) {
    for message in history {
        let Message::User { content } = message else {
            continue;
        };
        let mut normalized = Vec::with_capacity(content.len());
        for item in std::mem::take(content) {
            match item {
                UserContent::ToolResult(mut result) => {
                    let mut images = Vec::new();
                    result.content = std::mem::take(&mut result.content)
                        .into_iter()
                        .filter_map(|part| match part {
                            ToolResultContent::Image(image) => {
                                images.push(UserContent::Image(image));
                                None
                            }
                            part => Some(part),
                        })
                        .collect();
                    normalized.push(UserContent::ToolResult(result));
                    normalized.extend(images);
                }
                item => normalized.push(item),
            }
        }
        *content = normalized;
    }
}

fn image_count(message: &Message) -> usize {
    let Message::User { content } = message else {
        return 0;
    };
    content
        .iter()
        .map(|content| match content {
            UserContent::Image(_) => 1,
            UserContent::ToolResult(result) => result
                .content
                .iter()
                .filter(|content| matches!(content, ToolResultContent::Image(_)))
                .count(),
            _ => 0,
        })
        .sum()
}

fn retain_recent_images(history: &mut [Message], mut remaining: usize) {
    for message in history.iter_mut().rev() {
        let Message::User { content } = message else {
            continue;
        };
        for content in content.iter_mut().rev() {
            match content {
                UserContent::Image(_) if remaining > 0 => remaining -= 1,
                UserContent::Image(_) => {
                    *content = UserContent::text(
                        "Earlier image omitted from this request; its metadata remains. Use read_generation_output to inspect its pixels again.",
                    );
                }
                UserContent::ToolResult(result) => {
                    for content in result.content.iter_mut().rev() {
                        if matches!(content, ToolResultContent::Image(_)) {
                            if remaining > 0 {
                                remaining -= 1;
                            } else {
                                *content = ToolResultContent::text(
                                    "Earlier image omitted from this request; its metadata remains. Use read_generation_output to inspect its pixels again.",
                                );
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn output(id: &str) -> Message {
        Message::User {
            content: vec![UserContent::tool_result(
                id,
                "read_generation_output",
                vec![
                    ToolResultContent::text(id),
                    ToolResultContent::image_base64(
                        "AA==",
                        Some(rig_core::message::ImageMediaType::PNG),
                        None,
                    ),
                ],
            )],
        }
    }

    #[test]
    fn latest_images_remain_and_old_outputs_keep_their_metadata() {
        let budget = ContextBudget {
            available: 100_000,
            counter: HeuristicTokenCounter::openai(),
            image_input: AgentImageInputMode::ToolResult,
        };
        let history = vec![output("old"), output("recent")];
        let prepared = budget.prepare(&history, &output("current")).unwrap();
        assert_eq!(image_count(&prepared[0]), 0);
        assert_eq!(prepared[1], history[1]);
        let Message::User { content } = &prepared[0] else {
            panic!("user tool result");
        };
        let UserContent::ToolResult(result) = &content[0] else {
            panic!("tool result");
        };
        assert_eq!(result.content[0], ToolResultContent::text("old"));
    }

    #[test]
    fn message_mode_moves_images_out_of_tool_results() {
        let budget = ContextBudget {
            available: 100_000,
            counter: HeuristicTokenCounter::openai(),
            image_input: AgentImageInputMode::Message,
        };

        let prepared = budget
            .prepare(&[output("current")], &Message::user("next"))
            .unwrap();
        let Message::User { content } = &prepared[0] else {
            panic!("user message");
        };
        assert!(matches!(content[0], UserContent::ToolResult(_)));
        assert!(matches!(content[1], UserContent::Image(_)));
        let UserContent::ToolResult(result) = &content[0] else {
            panic!("tool result");
        };
        assert!(
            result
                .content
                .iter()
                .all(|part| !matches!(part, ToolResultContent::Image(_)))
        );
    }

    #[test]
    fn text_observations_are_never_silently_trimmed() {
        let budget = ContextBudget {
            available: 1,
            counter: HeuristicTokenCounter::openai(),
            image_input: AgentImageInputMode::None,
        };
        assert!(
            budget
                .prepare(&[Message::user("observed draft")], &Message::user("next"))
                .is_err()
        );
        let budget = ContextBudget {
            available: 100,
            counter: HeuristicTokenCounter::openai(),
            image_input: AgentImageInputMode::None,
        };
        let history = vec![Message::user("observed draft")];
        assert_eq!(
            budget.prepare(&history, &Message::user("next")).unwrap(),
            history
        );
    }
}
