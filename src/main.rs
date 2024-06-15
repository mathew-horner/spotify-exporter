use config::Config;
use database::Database;

use crate::http::Client as HttpClient;
use crate::spotify::get_tokens::Response as Tokens;

mod auth_flow;
mod config;
mod database;
mod http;
mod spotify;

#[tokio::main]
async fn main() {
    env_logger::init();
    let config = Config::from_env();
    let database = Database::new().await;
    // let persistence = Persistence::new(config.output_dir);
    let http_client = HttpClient::new();
    let client = spotify::Client::new(http_client, config.credentials);
    let tokens = get_tokens(&client, &database).await;
    let tracks = client.list_all_user_tracks(&tokens.access_token);
    database.snapshot(tracks).await;
}

/// Retrieve up-to-date access and refresh tokens to authenticate with Spotify.
///
/// This function may direct the user through an authorization code flow on the
/// Spotify website if necessary.
async fn get_tokens(client: &spotify::Client, database: &Database) -> Tokens {
    if let Some(tokens) = database.get_cached_tokens().await {
        // TODO: We need to use the refresh token to get a new access token here and
        // update the cache.
        log::info!("Using tokens from cache");
        tokens.into()
    } else {
        log::info!("Directing user through authorization code flow");
        let tokens = auth_flow::get_tokens(client);
        database.cache_tokens(tokens.clone().into()).await;
        tokens
    }
}
