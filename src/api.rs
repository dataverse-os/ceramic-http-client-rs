use ceramic_event::{
    Base64String, Jws, MultiBase32String, MultiBase36String, StreamId, StreamIdType,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct BlockHeader {
    pub family: String,
    pub controllers: Vec<String>,
    pub model: Base64String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockData<T: Serialize> {
    pub header: BlockHeader,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jws: Option<Jws>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linked_block: Option<Base64String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cacao_block: Option<MultiBase32String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRequest<T: Serialize> {
    #[serde(rename = "type")]
    pub r#type: StreamIdType,
    #[serde(rename = "genesis")]
    pub block: BlockData<T>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRequest {
    #[serde(rename = "type")]
    pub r#type: StreamIdType,
    #[serde(rename = "commit")]
    pub block: BlockData<Base64String>,
    pub stream_id: MultiBase36String,
}

#[derive(Deserialize)]
pub struct StateLog {
    pub cid: MultiBase36String,
}

#[derive(Deserialize)]
pub struct Metadata {
    pub controllers: Vec<String>,
    pub model: StreamId,
}

#[derive(Deserialize)]
pub struct StreamState {
    pub content: serde_json::Value,
    pub log: Vec<StateLog>,
    pub metadata: Metadata,
}

#[derive(Deserialize)]
pub struct PostResponse {
    #[serde(rename = "streamId")]
    pub stream_id: StreamId,
    pub state: Option<StreamState>,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum PostResponseOrError {
    Error { error: String },
    Ok(PostResponse),
}

impl PostResponseOrError {
    pub fn resolve(self, context: &str) -> anyhow::Result<PostResponse> {
        match self {
            Self::Error { error } => {
                anyhow::bail!(format!("{}: {}", context, error))
            }
            Self::Ok(resp) => Ok(resp),
        }
    }
}

#[derive(Deserialize)]
pub struct Commit {
    pub cid: MultiBase36String,
    pub value: serde_json::Value,
}

#[derive(Deserialize)]
pub struct GetResponse {
    #[serde(rename = "streamId")]
    pub stream_id: StreamId,
    pub commits: Vec<Commit>,
}
