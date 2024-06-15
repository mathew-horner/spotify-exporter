use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::auth_flow::{ClientCredentials, REDIRECT_URL};
use crate::http::Client as HttpClient;

/// Spotify API client.
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

    /// Retrieve tokens from the token endpoint, using the auth code flow.
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

    /// This function can make multiple requests as the response from the
    /// Spotify API is paginated.
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

    /// The Spotify token endpoint.
    pub const ENDPOINT: &str = "https://accounts.spotify.com/api/token";

    /// Response from the Spotify token endpoint.
    #[derive(Debug, Deserialize, Serialize)]
    pub struct Response {
        /// A Spotify access token.
        pub access_token: String,
        /// A Spotify refresh token.
        pub refresh_token: String,
        /// How many seconds until the access token expires.
        pub expires_in: u32,
    }
}

pub mod list_user_tracks {
    use super::*;

    /// The Spotify list user tracks endpoint.
    pub const ENDPOINT: &str = "https://api.spotify.com/v1/me/tracks";

    /// Response from the Spotify list user tracks endpoint.
    #[derive(Debug, Deserialize, Serialize)]
    pub struct Response {
        /// The next track ID to be used for pagination.
        pub next: Option<String>,
        /// List of tracks.
        pub items: Vec<Item>,
    }

    /// One item in the list tracks response.
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Item {
        /// The actual track data.
        pub track: Track,
    }

    /// A Spotify track.
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Track {
        /// The artists that contributed to the track.
        pub artists: Vec<Artist>,
        /// The ID of the track.
        pub id: String,
        /// The name of the track.
        pub name: String,
    }

    /// A Spotify artist.
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Artist {
        /// The name of the artist.
        pub name: String,
    }
}
