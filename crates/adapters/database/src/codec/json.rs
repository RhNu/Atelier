use super::{DatabaseResult, Deserialize, DeserializeOwned, Serialize};

pub fn encode_json<T: Serialize>(value: &T) -> DatabaseResult<String> {
    serde_json::to_string(value).map_err(Into::into)
}

pub fn decode_json<T: for<'de> Deserialize<'de>>(text: &str) -> DatabaseResult<T> {
    serde_json::from_str(text).map_err(Into::into)
}

pub trait JsonCodec<T>: DeserializeOwned + Serialize + Sized {
    fn from_domain(value: &T) -> Self;

    fn into_domain(self) -> DatabaseResult<T>;

    fn encode_domain(value: &T) -> DatabaseResult<String> {
        encode_json(&Self::from_domain(value))
    }

    fn decode_domain(text: &str) -> DatabaseResult<T> {
        decode_json::<Self>(text)?.into_domain()
    }
}
