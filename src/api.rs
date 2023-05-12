use ceramic_event::{
    Base64String, Jws, MultiBase32String, MultiBase36String, StreamId, StreamIdType,
};
use serde::{Deserialize, Serialize};

/// Header for block data
#[derive(Debug, Serialize)]
pub struct BlockHeader {
    /// Family that block belongs to
    pub family: String,
    /// Controllers for block
    pub controllers: Vec<String>,
    /// Model for block
    pub model: Base64String,
}

/// Block data
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockData<T: Serialize> {
    /// Header for block
    pub header: BlockHeader,
    /// Data for block
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    /// Signature for block
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jws: Option<Jws>,
    /// IPFS Linked Block
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linked_block: Option<Base64String>,
    /// Related cacao block
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cacao_block: Option<MultiBase32String>,
}

/// Create request for http api
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRequest<T: Serialize> {
    /// Type of stream to create
    #[serde(rename = "type")]
    pub r#type: StreamIdType,
    /// Data to use when creating stream
    #[serde(rename = "genesis")]
    pub block: BlockData<T>,
}

/// Update request for http api
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRequest {
    /// Type of stream to update
    #[serde(rename = "type")]
    pub r#type: StreamIdType,
    /// Data to update stream to
    #[serde(rename = "commit")]
    pub block: BlockData<Base64String>,
    /// Stream to update
    pub stream_id: MultiBase36String,
}

/// Log entry for stream
#[derive(Debug, Deserialize)]
pub struct StateLog {
    /// CID for stream
    pub cid: MultiBase36String,
}

/// Metadata for stream
#[derive(Debug, Deserialize)]
pub struct Metadata {
    /// Controllers for stream
    pub controllers: Vec<String>,
    /// Model for stream
    pub model: StreamId,
}

/// Current state of stream
#[derive(Debug, Deserialize)]
pub struct StreamState {
    /// Content of stream
    pub content: serde_json::Value,
    /// Log of stream
    pub log: Vec<StateLog>,
    /// Metadata for stream
    pub metadata: Metadata,
}

/// Response from request against streams endpoint
#[derive(Debug, Deserialize)]
pub struct StreamsResponse {
    /// ID of stream requested
    #[serde(rename = "streamId")]
    pub stream_id: StreamId,
    /// State of stream
    pub state: Option<StreamState>,
}

/// Response from request against streams endpoint or error
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum StreamsResponseOrError {
    /// Response was an error
    Error {
        /// Error message
        error: String
    },
    /// Response was ok
    Ok(StreamsResponse),
}

impl StreamsResponseOrError {
    /// Resolve or throw error from response
    pub fn resolve(self, context: &str) -> anyhow::Result<StreamsResponse> {
        match self {
            Self::Error { error } => {
                anyhow::bail!(format!("{}: {}", context, error))
            }
            Self::Ok(resp) => Ok(resp),
        }
    }
}

/// Json wrapper around jws
#[derive(Debug, Deserialize)]
pub struct JwsValue {
    /// Jws for a specific commit
    pub jws: Jws,
}

/// Commit for a specific stream
#[derive(Debug, Deserialize)]
pub struct Commit {
    /// Commit id
    pub cid: MultiBase36String,
    /// Value of commit
    pub value: Option<JwsValue>,
}

/// Response from commits endpoint
#[derive(Debug, Deserialize)]
pub struct CommitsResponse {
    /// ID of stream for commit
    #[serde(rename = "streamId")]
    pub stream_id: StreamId,
    /// Commits of stream
    pub commits: Vec<Commit>,
}
