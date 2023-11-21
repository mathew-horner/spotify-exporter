use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::auth_flow::{ClientCredentials, REDIRECT_URL};
use crate::http::Client as HttpClient;

pub struct Client {
    pub http_client: HttpClient,
    pub credentials: ClientCredentials,
}

impl Client {
    pub fn new(http_client: HttpClient, credentials: ClientCredentials) -> Self {
        Self {
            http_client,
            credentials,
        }
    }

    pub fn get_tokens(&self, authorization_code: &str) -> get_tokens::Response {
        #[allow(deprecated)]
        let credentials = base64::encode(format!(
            "{}:{}",
            self.credentials.id, self.credentials.secret
        ));

        let request = self
            .http_client
            .post(get_tokens::ENDPOINT)
            .header("Authorization", &format!("Basic {credentials}"))
            .form(&json!({
                "code": authorization_code,
                "redirect_uri": REDIRECT_URL,
                "grant_type": "authorization_code"
            }))
            .build()
            .unwrap();

        self.http_client.fetch(request)
    }

    /// This function can make multiple requests as the response from the Spotify API is paginated.
    pub fn list_all_user_tracks(&self, token: &str) -> Vec<list_user_tracks::Item> {
        let mut url = String::from(list_user_tracks::ENDPOINT);
        let mut items = Vec::new();
        loop {
            let request = self
                .http_client
                .get(url)
                .bearer_auth(token)
                .build()
                .unwrap();

            let response: list_user_tracks::Response = self.http_client.fetch(request);
            items.extend(response.items);
            if let Some(next) = response.next {
                url = next;
            } else {
                break;
            }
        }
        items
    }
}

pub mod get_tokens {
    use super::*;

    pub const ENDPOINT: &str = "https://accounts.spotify.com/api/token";

    #[derive(Debug, Deserialize, Serialize)]
    pub struct Response {
        pub access_token: String,
        pub refresh_token: String,
        pub expires_in: u32,
    }
}

pub mod list_user_tracks {
    use super::*;

    pub const ENDPOINT: &str = "https://api.spotify.com/v1/me/tracks";

    #[derive(Debug, Deserialize, Serialize)]
    pub struct Response {
        pub next: Option<String>,
        pub items: Vec<Item>,
    }

    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Item {
        pub track: Track,
    }

    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Track {
        pub artists: Vec<Artist>,
        pub id: String,
        pub name: String,
    }

    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Artist {
        pub name: String,
    }
}
