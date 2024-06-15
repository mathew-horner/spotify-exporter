use clap::{Parser, Subcommand};

use crate::config::Config;
use crate::database::Database;
use crate::http::Client as HttpClient;
use crate::spotify::get_tokens::Response as Tokens;

mod auth_flow;
mod config;
mod database;
mod http;
mod spotify;

/// Manage your Spotify library backups.
#[derive(Parser, Debug)]
#[command(
    version,
    about,
    long_about = "spotify-exporter is a program that keeps backups of your liked on songs on Spotify."
)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

/// The subcommand options for this programm.
#[derive(Debug, Subcommand)]
enum Command {
    /// Create a snapshot of the currently liked songs on Spotify.
    Snapshot,
    /// Show data for the last snapshot that was run.
    LastSnapshot,
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let config = Config::from_env();
    let args = Args::parse();
    let database = Database::new(&config.sqlite_url).await;

    match args.command {
        Command::Snapshot => {
            let http_client = HttpClient::new();
            let client = spotify::Client::new(http_client, config.credentials);
            let tokens = get_tokens(&client, &database).await;
            let tracks = client.list_all_user_tracks(&tokens.access_token).await;
            database.snapshot(tracks).await;
        }
        Command::LastSnapshot => {
            if let Some(snapshot_info) = database.get_last_snapshot_info().await {
                println!("{snapshot_info}");
            }
        }
    }
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
        let tokens = auth_flow::get_tokens(client).await;
        database.cache_tokens(tokens.clone().into()).await;
        tokens
    }
}
