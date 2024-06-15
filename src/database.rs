use sqlx::{Pool, QueryBuilder, Sqlite};

use crate::spotify::list_user_tracks::Item as Track;

/// Database client for caching Spotify tracks.
pub struct Database {
    database: Pool<Sqlite>,
}

impl Database {
    pub async fn new() -> Self {
        let database = Pool::connect("sqlite:data.db").await.unwrap();
        Self { database }
    }

    /// Record the current list of tracks in the database.
    pub async fn snapshot(&self, tracks: Vec<Track>) {
        let generation = self.last_generation().await + 1;
        let track_ids: Vec<_> = tracks.into_iter().map(|track| track.track.id).collect();

        let mut query =
            QueryBuilder::new("INSERT INTO spotify_track_cache (generation, track_id) ");

        query.push_values(track_ids, |mut builder, track_id| {
            builder.push_bind(generation);
            builder.push_bind(track_id);
        });

        query.build().execute(&self.database).await.unwrap();
    }

    /// Get the generation of the previous snapshot.
    pub async fn last_generation(&self) -> i32 {
        sqlx::query_scalar("SELECT MAX(generation) FROM spotify_track_cache")
            .fetch_optional(&self.database)
            .await
            .unwrap()
            .unwrap_or(0)
    }

    /// Write the Spotify token data to the database, replacing any pre-existing
    /// entries.
    pub async fn cache_tokens(&self, tokens: Tokens) {
        let mut tx = self.database.begin().await.unwrap();
        sqlx::query("DELETE FROM spotify_tokens")
            .execute(&mut *tx)
            .await
            .unwrap();

        sqlx::query("INSERT INTO spotify_tokens (access_token, refresh_token, expires_in) VALUES ($1, $2, $3)")
            .bind(tokens.access_token)
            .bind(tokens.refresh_token)
            .bind(tokens.expires_in)
            .execute(&mut *tx)
            .await
            .unwrap();

        tx.commit().await.unwrap();
    }

    /// Read the Spotify token data from the database, if it exists.
    pub async fn get_cached_tokens(&self) -> Option<Tokens> {
        sqlx::query_as("SELECT * FROM spotify_tokens LIMIT 1")
            .fetch_optional(&self.database)
            .await
            .unwrap()
    }
}

/// Spotify token data stored in the database.
#[derive(sqlx::FromRow)]
pub struct Tokens {
    /// The Spotify access token.
    pub access_token: String,
    /// The Spotify refresh token.
    pub refresh_token: String,
    /// How many seconds the access token expires in.
    pub expires_in: i32,
}

// Be able to convert to and from the API response type.

impl From<crate::spotify::get_tokens::Response> for Tokens {
    fn from(tokens: crate::spotify::get_tokens::Response) -> Self {
        Self {
            access_token: tokens.access_token,
            refresh_token: tokens.refresh_token,
            expires_in: tokens.expires_in.try_into().unwrap(),
        }
    }
}

impl From<Tokens> for crate::spotify::get_tokens::Response {
    fn from(tokens: Tokens) -> Self {
        Self {
            access_token: tokens.access_token,
            refresh_token: tokens.refresh_token,
            expires_in: tokens.expires_in.try_into().unwrap(),
        }
    }
}
