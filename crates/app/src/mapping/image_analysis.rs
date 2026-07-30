use super::{
    ImageAnalysisModelId, ImageAnalysisModelIdDto, ImageAnalysisModelState,
    ImageAnalysisModelStateDto, ImageAnalysisModelStatus, ImageAnalysisModelStatusDto,
};

pub const fn image_analysis_model_id_to_domain(
    value: ImageAnalysisModelIdDto,
) -> ImageAnalysisModelId {
    match value {
        ImageAnalysisModelIdDto::AnimeDbRating => ImageAnalysisModelId::AnimeDbRating,
        ImageAnalysisModelIdDto::WdSwinv2TaggerV3 => ImageAnalysisModelId::WdSwinv2TaggerV3,
    }
}

const fn image_analysis_model_id_to_dto(value: ImageAnalysisModelId) -> ImageAnalysisModelIdDto {
    match value {
        ImageAnalysisModelId::AnimeDbRating => ImageAnalysisModelIdDto::AnimeDbRating,
        ImageAnalysisModelId::WdSwinv2TaggerV3 => ImageAnalysisModelIdDto::WdSwinv2TaggerV3,
    }
}

pub fn image_analysis_model_status_to_dto(
    value: ImageAnalysisModelStatus,
) -> ImageAnalysisModelStatusDto {
    ImageAnalysisModelStatusDto {
        model_id: image_analysis_model_id_to_dto(value.id),
        required: value.required,
        state: match value.state {
            ImageAnalysisModelState::Missing => ImageAnalysisModelStateDto::Missing,
            ImageAnalysisModelState::Installing => ImageAnalysisModelStateDto::Installing,
            ImageAnalysisModelState::Ready => ImageAnalysisModelStateDto::Ready,
            ImageAnalysisModelState::Corrupt => ImageAnalysisModelStateDto::Corrupt,
            ImageAnalysisModelState::Failed => ImageAnalysisModelStateDto::Failed,
        },
        revision: value.revision,
        size_bytes: value.size_bytes,
        downloaded_bytes: value.downloaded_bytes,
        message: value.message,
    }
}
