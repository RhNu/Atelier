use crate::AgentPersonaSnapshot;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AgentPromptFamily {
    V5,
    Tags,
}

#[must_use]
pub fn build_agent_system_prompt(
    persona: &AgentPersonaSnapshot,
    family: AgentPromptFamily,
    workspace_context: &str,
) -> String {
    let family_guidance = match family {
        AgentPromptFamily::V5 => format!(
            "Write concise NovelAI V5 prompts using natural language, tags, or a useful mix. Keep shared composition, action, and environment in the main prompt and character-specific appearance in character prompts. Do not add quality boilerplate.\n{}",
            persona.v5_prompt_guidance
        ),
        AgentPromptFamily::Tags => format!(
            "Write lowercase Danbooru-style tags for NovelAI V4.x/V3 models. Keep shared composition, action, and environment in the main prompt and, when supported, character-specific appearance in character prompts.\n{}",
            persona.tag_prompt_guidance
        ),
    };
    format!(
        "You are {name}, Atelier's internal NovelAI workflow agent. Help the user refine the current generation draft and use only the supplied Atelier tools. Never claim a change happened unless the matching tool succeeded. Do not request or expose files, shell access, external plugins, credentials, image pixels, or unrelated resources. Reads are safe; mutations may require user approval. Submit at most one generation batch in a user turn.\n\n{family_guidance}\n\nWorkspace instructions:\n{instructions}\n\nCurrent workspace context:\n{workspace_context}",
        name = persona.display_name,
        instructions = persona.instructions,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn persona() -> AgentPersonaSnapshot {
        AgentPersonaSnapshot {
            display_name: "Muse".to_owned(),
            instructions: "Prefer cinematic framing.".to_owned(),
            v5_prompt_guidance: "Use short sentences.".to_owned(),
            tag_prompt_guidance: "Prefer canonical aliases.".to_owned(),
        }
    }

    #[test]
    fn v5_guidance_keeps_shared_and_character_prompts_separate() {
        let prompt =
            build_agent_system_prompt(&persona(), AgentPromptFamily::V5, "draft revision 7");
        assert!(prompt.contains("natural language"));
        assert!(prompt.contains("character-specific appearance"));
        assert!(prompt.contains("draft revision 7"));
        assert!(prompt.contains("at most one generation batch"));
    }

    #[test]
    fn tag_guidance_requests_lowercase_danbooru_tags() {
        let prompt = build_agent_system_prompt(&persona(), AgentPromptFamily::Tags, "draft");
        assert!(prompt.contains("lowercase Danbooru-style tags"));
        assert!(prompt.contains("Prefer canonical aliases."));
    }
}
