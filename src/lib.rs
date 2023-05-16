//! Ceramic HTTP API
//!
//! This crate provides a client for interacting with the Ceramic HTTP API.
#![deny(warnings)]
#![deny(missing_docs)]
/// Structures for working with ceramic http api
pub mod api;
mod model_definition;

use ceramic_event::{
    Base64String, Cid, DagCborEncoded, DidDocument, EventArgs, MultiBase36String, StreamId,
    StreamIdType,
};
use serde::{de::DeserializeOwned, Serialize};
use std::str::FromStr;

pub use ceramic_event;
pub use model_definition::{
    GetRootSchema, ModelAccountRelation, ModelDefinition, ModelRelationDefinition,
    ModelViewDefinition,
};
pub use schemars;

/// Client for interacting with the Ceramic HTTP API
#[derive(Clone, Debug)]
pub struct CeramicHttpClient {
    signer: DidDocument,
    private_key: String,
}

impl CeramicHttpClient {
    /// Create a new client, using a signer and private key
    pub fn new(signer: DidDocument, private_key: &str) -> Self {
        Self {
            signer,
            private_key: private_key.to_string(),
        }
    }

    /// Get the streams endpoint
    pub fn streams_endpoint(&self) -> &'static str {
        "/api/v0/streams"
    }

    /// Get the commits endpoint
    pub fn commits_endpoint(&self) -> &'static str {
        "/api/v0/commits"
    }

    /// Create a serde compatible request for model creation
    pub async fn create_model_request(
        &self,
        model: &ModelDefinition,
    ) -> anyhow::Result<api::CreateRequest<Base64String>> {
        let args = EventArgs::new(&self.signer);
        let commit = args.init_with_data(&model, &self.private_key).await?;
        let controllers: Vec<_> = args.controllers().map(|c| c.id.clone()).collect();
        let data = Base64String::from(commit.linked_block.as_ref());
        let model = Base64String::from(args.parent().to_vec()?);

        Ok(api::CreateRequest {
            r#type: StreamIdType::Model,
            block: api::BlockData {
                header: api::BlockHeader {
                    family: "test".to_string(),
                    controllers,
                    model,
                },
                linked_block: Some(data.clone()),
                jws: Some(commit.jws),
                data: Some(data),
                cacao_block: None,
            },
        })
    }

    /// Create a serde compatible request for a single instance per account creation of a model
    pub async fn create_single_instance_request(
        &self,
        model_id: &StreamId,
    ) -> anyhow::Result<api::CreateRequest<DagCborEncoded>> {
        if !model_id.is_model() {
            anyhow::bail!("StreamId was not a model");
        }
        let args = EventArgs::new_with_parent(&self.signer, model_id);
        let commit = args.init()?;
        let controllers: Vec<_> = args.controllers().map(|c| c.id.clone()).collect();
        let model = Base64String::from(model_id.to_vec()?);
        Ok(api::CreateRequest {
            r#type: StreamIdType::Document,
            block: api::BlockData {
                header: api::BlockHeader {
                    family: "test".to_string(),
                    controllers,
                    model,
                },
                linked_block: None,
                jws: None,
                data: Some(commit.encoded),
                cacao_block: None,
            },
        })
    }

    /// Create a serde compatible request for a list instance per account creation of a model
    pub async fn create_list_instance_request<T: Serialize>(
        &self,
        model_id: &StreamId,
        data: T,
    ) -> anyhow::Result<api::CreateRequest<Base64String>> {
        if !model_id.is_model() {
            anyhow::bail!("StreamId was not a model");
        }
        let args = EventArgs::new_with_parent(&self.signer, model_id);
        let commit = args.init_with_data(&data, &self.private_key).await?;
        let controllers: Vec<_> = args.controllers().map(|c| c.id.clone()).collect();
        let data = Base64String::from(commit.linked_block.as_ref());
        let model = Base64String::from(model_id.to_vec()?);
        Ok(api::CreateRequest {
            r#type: StreamIdType::Document,
            block: api::BlockData {
                header: api::BlockHeader {
                    family: "test".to_string(),
                    controllers,
                    model,
                },
                linked_block: Some(data.clone()),
                jws: Some(commit.jws),
                data: Some(data),
                cacao_block: None,
            },
        })
    }

    /// Create a serde compatible request to update specific parts an existing model instance
    pub async fn create_update_request(
        &self,
        model: &StreamId,
        get: &api::StreamsResponse,
        patch: json_patch::Patch,
    ) -> anyhow::Result<api::UpdateRequest> {
        if !get.stream_id.is_document() {
            anyhow::bail!("StreamId was not a document");
        }
        if let Some(tip) = get.state.as_ref().and_then(|s| s.log.last()) {
            let tip = Cid::from_str(tip.cid.as_ref())?;
            let args = EventArgs::new_with_parent(&self.signer, model);
            let commit = args
                .update(&get.stream_id.cid, &tip, &self.private_key, &patch)
                .await?;
            let controllers: Vec<_> = args.controllers().map(|c| c.id.clone()).collect();
            let data = Base64String::from(commit.linked_block.as_ref());
            let model = Base64String::from(model.to_vec()?);
            let stream = MultiBase36String::try_from(&get.stream_id)?;
            Ok(api::UpdateRequest {
                r#type: StreamIdType::Document,
                block: api::BlockData {
                    header: api::BlockHeader {
                        family: "test".to_string(),
                        controllers,
                        model,
                    },
                    linked_block: Some(data.clone()),
                    jws: Some(commit.jws),
                    data: Some(data),
                    cacao_block: None,
                },
                stream_id: stream,
            })
        } else {
            Err(anyhow::anyhow!("No commits found for stream ",))
        }
    }

    /// Create a serde compatible request to replace an existing model instance completely
    pub async fn create_replace_request<T: Serialize>(
        &self,
        model: &StreamId,
        get: &api::StreamsResponse,
        data: T,
    ) -> anyhow::Result<api::UpdateRequest> {
        let data = serde_json::to_value(data)?;
        let diff = if let Some(existing) = get.state.as_ref().map(|st| &st.content) {
            json_patch::diff(existing, &data)
        } else {
            json_patch::diff(&serde_json::json!({}), &data)
        };
        self.create_update_request(model, get, diff).await
    }
}

/// Remote HTTP Functionality
#[cfg(feature = "remote")]
pub mod remote {
    use super::*;

    /// Ceramic remote http client
    pub struct CeramicRemoteHttpClient {
        cli: CeramicHttpClient,
        remote: reqwest::Client,
        url: url::Url,
    }

    impl CeramicRemoteHttpClient {
        /// Create a new ceramic remote http client for a signer, private key, and url
        pub fn new(signer: DidDocument, private_key: &str, remote: url::Url) -> Self {
            Self {
                cli: CeramicHttpClient::new(signer, private_key),
                remote: reqwest::Client::new(),
                url: remote,
            }
        }

        /// Utility function to get a url for this client's base url, given a path
        pub fn url_for_path(&self, path: &str) -> anyhow::Result<url::Url> {
            let u = self.url.join(path)?;
            Ok(u)
        }

        /// Create a model on the remote ceramic
        pub async fn create_model(&self, model: &ModelDefinition) -> anyhow::Result<StreamId> {
            let req = self.cli.create_model_request(model).await?;
            let resp: api::StreamsResponseOrError = self
                .remote
                .post(self.url_for_path(self.cli.streams_endpoint())?)
                .json(&req)
                .send()
                .await?
                .json()
                .await?;
            Ok(resp.resolve("create_model")?.stream_id)
        }

        /// Create an instance of a model that allows a single instance on the remote ceramic
        pub async fn create_single_instance(
            &self,
            model_id: &StreamId,
        ) -> anyhow::Result<StreamId> {
            let req = self.cli.create_single_instance_request(model_id).await?;
            let resp: api::StreamsResponseOrError = self
                .remote
                .post(self.url_for_path(self.cli.streams_endpoint())?)
                .json(&req)
                .send()
                .await?
                .json()
                .await?;
            Ok(resp.resolve("create_single_instance")?.stream_id)
        }

        /// Create an instance of a model allowing multiple instances on a remote ceramic
        pub async fn create_list_instance<T: Serialize>(
            &self,
            model_id: &StreamId,
            instance: T,
        ) -> anyhow::Result<StreamId> {
            let req = self
                .cli
                .create_list_instance_request(model_id, instance)
                .await?;
            let resp: api::StreamsResponseOrError = self
                .remote
                .post(self.url_for_path(self.cli.streams_endpoint())?)
                .json(&req)
                .send()
                .await?
                .json()
                .await?;
            let resp = resp.resolve("create_list_instance")?;
            Ok(resp.stream_id)
        }

        /// Update an instance that was previously created
        pub async fn update(
            &self,
            model: &StreamId,
            stream_id: &StreamId,
            patch: json_patch::Patch,
        ) -> anyhow::Result<api::StreamsResponse> {
            let resp = self.get(stream_id).await?;
            let req = self.cli.create_update_request(model, &resp, patch).await?;
            let resp: api::StreamsResponseOrError = self
                .remote
                .post(self.url_for_path(self.cli.commits_endpoint())?)
                .json(&req)
                .send()
                .await?
                .json()
                .await?;
            resp.resolve("update")
        }

        /// Replace an instance that was previously created
        pub async fn replace<T: Serialize>(
            &self,
            model: &StreamId,
            stream_id: &StreamId,
            data: T,
        ) -> anyhow::Result<api::StreamsResponse> {
            let resp = self.get(stream_id).await?;
            let req = self.cli.create_replace_request(model, &resp, data).await?;
            let resp: api::StreamsResponseOrError = self
                .remote
                .post(self.url_for_path(self.cli.commits_endpoint())?)
                .json(&req)
                .send()
                .await?
                .json()
                .await?;
            resp.resolve("replace")
        }

        /// Get an instance of model
        pub async fn get(&self, stream_id: &StreamId) -> anyhow::Result<api::StreamsResponse> {
            let endpoint = format!("{}/{}", self.cli.streams_endpoint(), stream_id);
            let endpoint = self.url_for_path(&endpoint)?;
            let resp: api::StreamsResponse = self.remote.get(endpoint).send().await?.json().await?;
            Ok(resp)
        }

        /// Get the content of an instance of a model as a serde compatible type
        pub async fn get_as<T: DeserializeOwned>(&self, stream_id: &StreamId) -> anyhow::Result<T> {
            let resp = self.get(stream_id).await?;
            if let Some(st) = resp.state {
                let resp = serde_json::from_value(st.content)?;
                Ok(resp)
            } else {
                Err(anyhow::anyhow!("No commits for stream {}", stream_id))
            }
        }
    }
}

#[cfg(all(test, feature = "remote"))]
pub mod tests {
    use super::remote::*;
    use super::*;
    use crate::model_definition::{GetRootSchema, ModelAccountRelation, ModelDefinition};
    use json_patch::ReplaceOperation;
    use schemars::JsonSchema;
    use serde::{Deserialize, Serialize};
    use std::time::Duration;

    // See https://github.com/ajv-validator/ajv-formats for information on valid formats
    #[derive(Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
    #[schemars(rename_all = "camelCase", deny_unknown_fields)]
    struct Ball {
        creator: String,
        radius: i32,
        red: i32,
        green: i32,
        blue: i32,
    }

    impl GetRootSchema for Ball {}

    pub fn ceramic_url() -> url::Url {
        let u =
            std::env::var("CERAMIC_URL").unwrap_or_else(|_| "http://localhost:7071".to_string());
        url::Url::parse(&u).unwrap()
    }

    pub fn did() -> DidDocument {
        let s = std::env::var("DID_DOCUMENT").unwrap_or_else(|_| {
            "did:key:z6MkeqCTPhHPVg3HaAAtsR7vZ6FXkAHPXEbTJs7Y4CQABV9Z".to_string()
        });
        DidDocument::new(&s)
    }

    pub fn did_private_key() -> String {
        std::env::var("DID_PRIVATE_KEY").unwrap()
    }

    pub async fn create_model(cli: &CeramicRemoteHttpClient) -> StreamId {
        let model = ModelDefinition::new::<Ball>("TestBall", ModelAccountRelation::List).unwrap();
        cli.create_model(&model).await.unwrap()
    }

    #[tokio::test]
    async fn should_create_model() {
        let ceramic = CeramicRemoteHttpClient::new(did(), &did_private_key(), ceramic_url());
        let model = ModelDefinition::new::<Ball>("TestBall", ModelAccountRelation::List).unwrap();
        ceramic.create_model(&model).await.unwrap();
    }

    // #[tokio::test]
    // async fn should_create_single_instance() {
    //     let ceramic = CeramicRemoteHttpClient::new(did(), &did_private_key(), ceramic_url());
    //     let model = create_model(&ceramic).await;
    //     ceramic.create_single_instance(&model).await.unwrap();
    // }

    #[tokio::test]
    async fn should_create_and_update_list() {
        let d = did();
        let ceramic = CeramicRemoteHttpClient::new(d.clone(), &did_private_key(), ceramic_url());
        let model = create_model(&ceramic).await;
        let stream_id = ceramic
            .create_list_instance(
                &model,
                &Ball {
                    creator: d.id.to_string(),
                    radius: 1,
                    red: 2,
                    green: 3,
                    blue: 4,
                },
            )
            .await
            .unwrap();

        //give anchor time to complete
        tokio::time::sleep(Duration::from_secs(1)).await;

        let patch = json_patch::Patch(vec![json_patch::PatchOperation::Replace(
            ReplaceOperation {
                path: "/red".to_string(),
                value: serde_json::json!(5),
            },
        )]);
        let post_resp = ceramic.update(&model, &stream_id, patch).await.unwrap();
        assert_eq!(post_resp.stream_id, stream_id);
        let post_resp: Ball = serde_json::from_value(post_resp.state.unwrap().content).unwrap();
        assert_eq!(post_resp.red, 5);

        //give anchor time to complete
        tokio::time::sleep(Duration::from_secs(1)).await;

        let patch = json_patch::Patch(vec![json_patch::PatchOperation::Replace(
            ReplaceOperation {
                path: "/blue".to_string(),
                value: serde_json::json!(8),
            },
        )]);
        let post_resp = ceramic.update(&model, &stream_id, patch).await.unwrap();
        assert_eq!(post_resp.stream_id, stream_id);
        let post_resp: Ball = serde_json::from_value(post_resp.state.unwrap().content).unwrap();
        assert_eq!(post_resp.blue, 8);

        //give anchor time to complete
        tokio::time::sleep(Duration::from_secs(1)).await;

        let get_resp: Ball = ceramic.get_as(&stream_id).await.unwrap();
        assert_eq!(get_resp.red, 5);
        assert_eq!(get_resp.blue, 8);
        assert_eq!(get_resp, post_resp);
    }

    #[tokio::test]
    async fn should_create_and_replace_list() {
        let d = did();
        let ceramic = CeramicRemoteHttpClient::new(d.clone(), &did_private_key(), ceramic_url());
        let model = create_model(&ceramic).await;
        let stream_id = ceramic
            .create_list_instance(
                &model,
                &Ball {
                    creator: d.id.to_string(),
                    radius: 1,
                    red: 2,
                    green: 3,
                    blue: 4,
                },
            )
            .await
            .unwrap();

        //give anchor time to complete
        tokio::time::sleep(Duration::from_secs(1)).await;

        let replace = Ball {
            creator: d.id.to_string(),
            radius: 1,
            red: 5,
            green: 3,
            blue: 4,
        };

        let post_resp = ceramic.replace(&model, &stream_id, &replace).await.unwrap();
        assert_eq!(post_resp.stream_id, stream_id);
        let post_resp: Ball = serde_json::from_value(post_resp.state.unwrap().content).unwrap();
        assert_eq!(post_resp, replace);

        //give anchor time to complete
        tokio::time::sleep(Duration::from_secs(1)).await;

        let replace = Ball {
            creator: d.id.to_string(),
            radius: 1,
            red: 0,
            green: 3,
            blue: 10,
        };
        let post_resp = ceramic.replace(&model, &stream_id, &replace).await.unwrap();
        assert_eq!(post_resp.stream_id, stream_id);
        let post_resp: Ball = serde_json::from_value(post_resp.state.unwrap().content).unwrap();
        assert_eq!(post_resp, replace);

        //give anchor time to complete
        tokio::time::sleep(Duration::from_secs(1)).await;

        let get_resp: Ball = ceramic.get_as(&stream_id).await.unwrap();
        assert_eq!(get_resp, post_resp);
    }
}
