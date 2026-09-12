use atelier_app_api::director::DirectorToolDto;
use atelier_app_api::generation::{
    CharacterDto, CharacterReferenceTypeDto, GenerationWorkRequestDto,
};
use atelier_director::DirectorTool;
use atelier_generation::{Character, CharacterPosition, CharacterReferenceType};
pub fn characters_to_domain(value: Vec<CharacterDto>) -> Vec<Character> {
    value
        .into_iter()
        .map(|character| Character {
            prompt: character.prompt,
            negative_prompt: character.negative_prompt,
            position: CharacterPosition {
                x: character.position.x,
                y: character.position.y,
            },
            enabled: character.enabled,
        })
        .collect()
}

pub const fn character_reference_type_to_domain(
    value: CharacterReferenceTypeDto,
) -> CharacterReferenceType {
    match value {
        CharacterReferenceTypeDto::Character => CharacterReferenceType::Character,
        CharacterReferenceTypeDto::Style => CharacterReferenceType::Style,
        CharacterReferenceTypeDto::CharacterAndStyle => CharacterReferenceType::CharacterAndStyle,
        CharacterReferenceTypeDto::Costume => CharacterReferenceType::Costume,
        CharacterReferenceTypeDto::Delta => CharacterReferenceType::Delta,
    }
}

pub const fn director_tool_to_domain(value: DirectorToolDto) -> DirectorTool {
    match value {
        DirectorToolDto::Lineart => DirectorTool::Lineart,
        DirectorToolDto::Sketch => DirectorTool::Sketch,
        DirectorToolDto::BgRemoval => DirectorTool::BgRemoval,
        DirectorToolDto::Emotion => DirectorTool::Emotion,
        DirectorToolDto::Declutter => DirectorTool::Declutter,
        DirectorToolDto::Colorize => DirectorTool::Colorize,
    }
}

pub fn generation_work_title(work: &GenerationWorkRequestDto) -> Option<String> {
    let prompt = match work {
        GenerationWorkRequestDto::Image(request) => &request.prompt,
        GenerationWorkRequestDto::Stream(request) => &request.base.prompt,
    };
    (!prompt.trim().is_empty()).then(|| prompt.clone())
}
