CREATE TABLE spotify_track_cache(
    track_id   TEXT NOT NULL,
    generation INT NOT NULL,
    timestamp  TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (track_id, generation)
);
