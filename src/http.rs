use std::ops::Deref;

use reqwest::{Client as ReqwestClient, Request};
use serde::de::DeserializeOwned;

/// Wrapper for reqwest::Client.
///
/// NOTE: This client is *blocking*, not async.
pub struct Client(ReqwestClient);

impl Client {
    pub fn new() -> Self {
        Self(ReqwestClient::new())
    }

    /// Makes an HTTP request and attempts to deserialize it.
    ///
    /// This function will panic on a non-200 response.
    pub async fn fetch<T>(&self, request: Request) -> T
    where
        T: DeserializeOwned,
    {
        let response = self
            .0
            .execute(request)
            .await
            .expect("failed to send request");
        let status = response.status();
        if !status.is_success() {
            panic!("response was non-OK: {status}");
        }

        let body = response.text().await.expect("failed to get response text");
        serde_json::from_str(&body).expect("failed to deserialize body")
    }
}

impl Deref for Client {
    type Target = ReqwestClient;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
