use std::path::{Path, PathBuf};
use std::sync::Mutex;

use atelier_image_analysis::{
    AnalysisOutputSelection, ImageAnalysis, ImageAnalysisError, ImageAnalysisInput,
    ImageAnalysisModelId, ImageAnalysisModelInfo, ImageAnalysisResult, ImageRatingScores,
    ImageTagCategory, ImageTagScore,
};
use ort::{session::Session, value::Tensor};

use crate::preprocess::{
    DBRATING_INPUT_SIZE, WD_INPUT_SIZE, dbrating_tensor, decode_rgb, wd_tensor,
};
use crate::spec::{ANIME_DBRATING_REVISION, WD_TAGGER_REVISION};

pub enum OnnxImageAnalyzer {
    AnimeDbRating {
        session: Mutex<Session>,
    },
    WdSwinv2TaggerV3 {
        session: Mutex<Session>,
        tags_path: PathBuf,
        rating_indices: [usize; 4],
        tag_count: usize,
    },
}

#[derive(Clone, Debug)]
struct WdTag {
    output_index: usize,
    tag_id: u64,
    name: String,
    category: WdTagCategory,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum WdTagCategory {
    General,
    Character,
}

impl OnnxImageAnalyzer {
    pub(crate) fn load_dbrating(model_path: &Path) -> ImageAnalysisResult<Self> {
        load_session(model_path).map(|session| Self::AnimeDbRating {
            session: Mutex::new(session),
        })
    }

    pub(crate) fn load_wd(model_path: &Path, tags_path: &Path) -> ImageAnalysisResult<Self> {
        let (rating_indices, tag_count) = read_wd_metadata(tags_path)?;
        load_session(model_path).map(|session| Self::WdSwinv2TaggerV3 {
            session: Mutex::new(session),
            tags_path: tags_path.to_owned(),
            rating_indices,
            tag_count,
        })
    }

    pub(crate) fn analyze(
        &self,
        input: ImageAnalysisInput,
        outputs: AnalysisOutputSelection,
    ) -> ImageAnalysisResult<ImageAnalysis> {
        if outputs.is_empty() {
            return Err(ImageAnalysisError::invalid_request(
                "at least one image analysis output must be selected",
            ));
        }
        let image = decode_rgb(&input.bytes)?;
        match self {
            Self::AnimeDbRating { session } => {
                if outputs.general_tags || outputs.character_tags {
                    return Err(ImageAnalysisError::invalid_request(
                        "anime_dbrating only supports rating outputs",
                    ));
                }
                let tensor = Tensor::from_array((
                    [
                        1_usize,
                        3_usize,
                        DBRATING_INPUT_SIZE as usize,
                        DBRATING_INPUT_SIZE as usize,
                    ],
                    dbrating_tensor(&image).into_boxed_slice(),
                ))
                .map_err(inference_error)?;
                let values = run(session, tensor)?;
                Ok(ImageAnalysis {
                    resource: input.resource,
                    model: ImageAnalysisModelInfo {
                        id: ImageAnalysisModelId::AnimeDbRating,
                        revision: ANIME_DBRATING_REVISION.to_owned(),
                    },
                    ratings: Some(rating_scores(&values)?),
                    general_tags: Vec::new(),
                    character_tags: Vec::new(),
                })
            }
            Self::WdSwinv2TaggerV3 {
                session,
                tags_path,
                rating_indices,
                tag_count,
            } => {
                let tensor = Tensor::from_array((
                    [
                        1_usize,
                        WD_INPUT_SIZE as usize,
                        WD_INPUT_SIZE as usize,
                        3_usize,
                    ],
                    wd_tensor(&image).into_boxed_slice(),
                ))
                .map_err(inference_error)?;
                let values = run(session, tensor)?;
                if values.len() != *tag_count {
                    return Err(ImageAnalysisError::inference(format!(
                        "WD output has {} values for {} tags",
                        values.len(),
                        tag_count
                    )));
                }
                let mut general_tags = Vec::new();
                let mut character_tags = Vec::new();
                for tag in read_wd_tags(tags_path, outputs)? {
                    let score = values[tag.output_index];
                    match tag.category {
                        WdTagCategory::General => {
                            general_tags.push(ImageTagScore::new(
                                tag.tag_id,
                                tag.name,
                                ImageTagCategory::General,
                                score,
                            )?);
                        }
                        WdTagCategory::Character => {
                            character_tags.push(ImageTagScore::new(
                                tag.tag_id,
                                tag.name,
                                ImageTagCategory::Character,
                                score,
                            )?);
                        }
                    }
                }
                Ok(ImageAnalysis {
                    resource: input.resource,
                    model: ImageAnalysisModelInfo {
                        id: ImageAnalysisModelId::WdSwinv2TaggerV3,
                        revision: WD_TAGGER_REVISION.to_owned(),
                    },
                    ratings: outputs
                        .ratings
                        .then(|| rating_scores(&rating_indices.map(|index| values[index])))
                        .transpose()?,
                    general_tags,
                    character_tags,
                })
            }
        }
    }
}

fn read_wd_metadata(tags_path: &Path) -> ImageAnalysisResult<([usize; 4], usize)> {
    let mut reader = csv::Reader::from_path(tags_path).map_err(inference_error)?;
    let mut rating_indices = [None; 4];
    let mut count = 0;
    for (index, row) in reader.records().enumerate() {
        let row = row.map_err(inference_error)?;
        let category = row
            .get(2)
            .ok_or_else(|| ImageAnalysisError::inference("WD tag row has no category"))?
            .parse::<u8>()
            .map_err(inference_error)?;
        match category {
            9 => {
                let rating = match row.get(1) {
                    Some("general") => 0,
                    Some("sensitive") => 1,
                    Some("questionable") => 2,
                    Some("explicit") => 3,
                    Some(name) => {
                        return Err(ImageAnalysisError::inference(format!(
                            "unsupported WD rating tag {name}"
                        )));
                    }
                    None => {
                        return Err(ImageAnalysisError::inference("WD rating row has no name"));
                    }
                };
                rating_indices[rating] = Some(index);
            }
            0 | 4 => {}
            value => {
                return Err(ImageAnalysisError::inference(format!(
                    "unsupported WD tag category {value}"
                )));
            }
        }
        count += 1;
    }
    let [general, sensitive, questionable, explicit] = rating_indices;
    let incomplete = || ImageAnalysisError::inference("WD ratings are incomplete");
    let rating_indices = [
        general.ok_or_else(incomplete)?,
        sensitive.ok_or_else(incomplete)?,
        questionable.ok_or_else(incomplete)?,
        explicit.ok_or_else(incomplete)?,
    ];
    Ok((rating_indices, count))
}

fn read_wd_tags(
    tags_path: &Path,
    outputs: AnalysisOutputSelection,
) -> ImageAnalysisResult<Vec<WdTag>> {
    if !outputs.general_tags && !outputs.character_tags {
        return Ok(Vec::new());
    }
    let mut reader = csv::Reader::from_path(tags_path).map_err(inference_error)?;
    let mut tags = Vec::new();
    for (output_index, row) in reader.records().enumerate() {
        let row = row.map_err(inference_error)?;
        let category = row
            .get(2)
            .ok_or_else(|| ImageAnalysisError::inference("WD tag row has no category"))?
            .parse::<u8>()
            .map_err(inference_error)?;
        let category = match category {
            0 if outputs.general_tags => Some(WdTagCategory::General),
            4 if outputs.character_tags => Some(WdTagCategory::Character),
            0 | 4 | 9 => None,
            value => {
                return Err(ImageAnalysisError::inference(format!(
                    "unsupported WD tag category {value}"
                )));
            }
        };
        let Some(category) = category else {
            continue;
        };
        let tag_id = row
            .get(0)
            .ok_or_else(|| ImageAnalysisError::inference("WD tag row has no ID"))?
            .parse::<u64>()
            .map_err(inference_error)?;
        let name = row
            .get(1)
            .ok_or_else(|| ImageAnalysisError::inference("WD tag row has no name"))?
            .to_owned();
        tags.push(WdTag {
            output_index,
            tag_id,
            name,
            category,
        });
    }
    Ok(tags)
}

fn load_session(model_path: &Path) -> ImageAnalysisResult<Session> {
    if !model_path.is_file() {
        return Err(ImageAnalysisError::model_unavailable(format!(
            "model file is missing: {}",
            model_path.display()
        )));
    }
    Session::builder()
        .map_err(inference_error)?
        .commit_from_file(model_path)
        .map_err(inference_error)
}

fn run(session: &Mutex<Session>, tensor: Tensor<f32>) -> ImageAnalysisResult<Vec<f32>> {
    let mut session = session
        .lock()
        .map_err(|_| ImageAnalysisError::inference("ONNX session lock is unavailable"))?;
    let values = {
        let outputs = session.run(ort::inputs![tensor]).map_err(inference_error)?;
        let (_, values) = outputs[0]
            .try_extract_tensor::<f32>()
            .map_err(inference_error)?;
        values.to_vec()
    };
    drop(session);
    Ok(values)
}

fn rating_scores(values: &[f32]) -> ImageAnalysisResult<ImageRatingScores> {
    if values.len() < 4 {
        return Err(ImageAnalysisError::inference(
            "rating model returned fewer than four scores",
        ));
    }
    ImageRatingScores::new(
        values[0].clamp(0.0, 1.0),
        values[1].clamp(0.0, 1.0),
        values[2].clamp(0.0, 1.0),
        values[3].clamp(0.0, 1.0),
    )
}

fn inference_error(error: impl std::fmt::Display) -> ImageAnalysisError {
    ImageAnalysisError::inference(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wd_metadata_keeps_rating_indices_without_materializing_tags_for_rating_only() {
        let temp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(
            temp.path(),
            "tag_id,name,category,count\n\
             9,general,9,1\n\
             8,sensitive,9,1\n\
             7,questionable,9,1\n\
             6,explicit,9,1\n\
             5,one_girl,0,1\n\
             4,some_character,4,1\n",
        )
        .unwrap();

        let (rating_indices, tag_count) = read_wd_metadata(temp.path()).unwrap();
        let rating_only =
            read_wd_tags(temp.path(), AnalysisOutputSelection::ratings_only()).unwrap();
        let all = read_wd_tags(temp.path(), AnalysisOutputSelection::all()).unwrap();

        assert_eq!(rating_indices, [0, 1, 2, 3]);
        assert_eq!(tag_count, 6);
        assert!(rating_only.is_empty());
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].tag_id, 5);
        assert_eq!(all[0].output_index, 4);
        assert_eq!(all[1].category, WdTagCategory::Character);
    }
}
