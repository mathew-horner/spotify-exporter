CREATE TABLE spotify_tokens (
    access_token  TEXT NOT NULL PRIMARY KEY,
    refresh_token TEXT NOT NULL,
    expires_in    INT NOT NULL
);
