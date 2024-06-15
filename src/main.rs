use config::Config;
use persistence::Persistence;

use crate::http::Client as HttpClient;
use crate::spotify::get_tokens::Response as Tokens;

mod auth_flow;
mod config;
mod http;
mod persistence;
mod spotify;
mod utils;

fn main() {
    env_logger::init();
    let config = Config::from_env();
    let persistence = Persistence::new(config.output_dir);
    let http_client = HttpClient::new();
    let client = spotify::Client::new(http_client, config.credentials);
    let tokens = get_tokens(&client, &persistence);
    let tracks = client.list_all_user_tracks(&tokens.access_token);
    persistence.snapshot(tracks);
}

/// Retrieve up-to-date access and refresh tokens to authenticate with Spotify.
///
/// This function may direct the user through an authorization code flow on the
/// Spotify website if necessary.
fn get_tokens(client: &spotify::Client, persistence: &Persistence) -> Tokens {
    if let Some(tokens) = persistence.get_cached_tokens() {
        // TODO: We need to use the refresh token to get a new access token here and
        // update the cache.
        log::info!("Using tokens from cache");
        tokens
    } else {
        log::info!("Directing user through authorization code flow");
        let tokens = auth_flow::get_tokens(client);
        persistence.cache_tokens(&tokens);
        tokens
    }
}
