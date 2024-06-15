use crate::auth_flow::{self, ClientCredentials};

/// Configuration for the exporter.
pub struct Config {
    /// Spotify client credentials.
    pub credentials: ClientCredentials,
    /// Connection URL for the SQLite database.
    pub sqlite_url: String,
}

impl Config {
    /// Read the configuration from environment variables.
    pub fn from_env() -> Self {
        let credentials = auth_flow::ClientCredentials::from_env();
        let sqlite_url = std::env::var("SQLITE_URL").unwrap_or_else(|_| "data.db".into());
        Self {
            credentials,
            sqlite_url,
        }
    }
}
