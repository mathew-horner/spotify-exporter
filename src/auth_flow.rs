//! Module that handles the Spotify authentication flow.
use std::sync::{Arc, Mutex};
use std::thread::spawn;
use std::{env, thread::JoinHandle};

use anyhow::{anyhow, Result};
use rouille::{Response, Server};
use url::Url;

use crate::spotify;
use crate::spotify::get_tokens::Response as Tokens;

/// Local URL to listen on for Authorization Code Flow callbacks.
pub const REDIRECT_URL: &str = "http://localhost:3000";

/// This should be the same as `REDIRECT_URL`, just without the `http://` prefix.
pub const REDIRECT_URL_WITHOUT_PROTOCOL: &str = "localhost:3000";

/// Spotify URL base to send the user to in order to authorize the app.
const ENDPOINT: &str = "https://accounts.spotify.com/authorize";

/// Spotify client ID and secret.
#[derive(Clone)]
pub struct ClientCredentials {
    /// Spotify client ID.
    pub id: String,
    /// Spotify client secret.
    pub secret: String,
}

impl ClientCredentials {
    /// Read the credentials from the environment.
    ///
    /// They should be passed in `CLIENT_ID` and `CLIENT_SECRET`.
    pub fn from_env() -> Self {
        let id = env::var("CLIENT_ID").expect("please provide Spotify CLIENT_ID");
        let secret = env::var("CLIENT_SECRET").expect("please provide Spotify CLIENT_SECRET");
        Self { id, secret }
    }
}

/// Put the user through the Authorization Code flow and then fetch access and
/// refresh tokens for them.
pub fn get_tokens(client: &spotify::Client) -> Tokens {
    let mut url: Url = ENDPOINT
        .parse()
        .expect("failed to parse authorize endpoint url");

    url.query_pairs_mut()
        .append_pair("client_id", &client.credentials.id)
        .append_pair("response_type", "code")
        .append_pair("redirect_uri", REDIRECT_URL)
        .append_pair("scope", "user-library-read")
        .finish();

    let server_handle = spawn_authorization_code_callback_server();
    webbrowser::open(url.as_str()).expect("failed to open authorization page in web browser");

    let authorization_code = server_handle
        .join()
        .expect("failed to join on Rouille server thread handle")
        .expect("an error occurred in the Rouille server thread");

    client.get_tokens(&authorization_code)
}

/// Boots up a short lived HTTP server to capture the user's authorization code
/// when Spotify redirects them after their authorize our app and returns it
/// when this thread handle is joined on.
fn spawn_authorization_code_callback_server() -> JoinHandle<Result<String>> {
    spawn(|| {
        let code: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let server = Server::new(REDIRECT_URL_WITHOUT_PROTOCOL, {
            let code = code.clone();
            move |request| {
                *code.lock().unwrap() = request.get_param("code");
                Response::empty_204()
            }
        })
        .map_err(|_| anyhow!("failed to create Rouille server"))?;

        loop {
            // Once our HTTP server has received a code from the authorization callback, we
            // can stop blocking and allow the code to be returned when this
            // thread is joined on.
            if code.lock().unwrap().is_some() {
                break;
            }
            server.poll();
        }

        let code = code.lock().unwrap();
        code.clone()
            .ok_or_else(|| anyhow!("authorization code should not have been None at this point"))
    })
}
