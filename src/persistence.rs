use std::collections::hash_map::RandomState;
use std::collections::HashSet;
use std::env;
use std::fs::{self, DirEntry};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Local};
use regex::Regex;
use serde::Serialize;

use crate::spotify::get_tokens::Response as Tokens;
use crate::spotify::list_user_tracks::Item as Track;
use crate::utils::{read_json, write_json};

const CACHE_DIR: &str = "cache";
const CACHE_TOKEN_FILE: &str = "tokens.json";

// Filenames for all the files that are a part of a snapshot export.
const SNAPSHOT_COLLECTION_FILE: &str = "collection.json";
const SNAPSHOT_DIFF_FILE: &str = "diff.json";
const SNAPSHOT_METADATA_FILE: &str = "metadata.json";

const SNAPSHOT_DIRECTORY_REGEX: &str = r#"snapshot-\d{6}"#;

/// Controls writes to the configured output directory; can be used to store
/// data such as individual "snapshots" (a representation of the Spotify data at
/// a given time), cached tokens, etc.
pub struct Persistence {
    output_dir: PathBuf,
}

impl Persistence {
    pub fn new(output_dir: PathBuf) -> Self {
        Self { output_dir }
    }

    /// Persist Spotify track data, some metadata, and a diff between this
    /// snapshot and the most recent one, if applicable.
    pub fn snapshot(&self, tracks: Vec<Track>) {
        let last_snapshot = self.get_last_snapshot();
        let next_snapshot = last_snapshot.clone().unwrap_or(0) + 1;
        let snapshot = Snapshot::new(&self.output_dir, next_snapshot);
        snapshot.write_collection_file(&tracks);
        snapshot.write_metadata_file();

        if let Some(last_snapshot) = last_snapshot {
            let old_tracks = self.read_snapshot(last_snapshot);
            snapshot.write_diff_file(&old_tracks, &tracks);
        }

        log::info!("Data written to new snapshot directory: {:?}", snapshot.dir);
    }

    fn snapshot_path(&self, snapshot: usize) -> PathBuf {
        self.output_dir.join(snapshot_dir_name(snapshot))
    }

    fn read_snapshot(&self, snapshot: usize) -> Vec<Track> {
        let path = self.snapshot_path(snapshot).join(SNAPSHOT_COLLECTION_FILE);
        read_json(&path)
    }

    fn get_last_snapshot(&self) -> Option<usize> {
        let mut snapshots: Vec<_> = fs::read_dir(&self.output_dir)
            .expect("failed to read dir")
            .filter_map(|entry| {
                let Ok(entry) = entry else { return None };
                extract_snapshot_dir_number(&entry)
            })
            .collect();

        // Our snapshots are autoincremented, so an ascension sort and pop off the top
        // will give us the last one.
        snapshots.sort();
        snapshots.last().map(|s| *s)
    }

    pub fn get_cached_tokens(&self) -> Option<Tokens> {
        let cache_path = self.get_token_cache_path();
        if !cache_path.exists() {
            return None;
        }
        read_json(&cache_path)
    }

    pub fn cache_tokens(&self, tokens: &Tokens) {
        write_json(&self.get_token_cache_path(), tokens);
    }

    fn get_token_cache_path(&self) -> PathBuf {
        self.output_dir.join(CACHE_DIR).join(CACHE_TOKEN_FILE)
    }
}

#[derive(Serialize)]
struct Diff<'a> {
    added: Vec<&'a Track>,
    removed: Vec<&'a Track>,
}

impl<'a> Diff<'a> {
    fn calculate(old_tracks: &'a [Track], new_tracks: &'a [Track]) -> Self {
        let old_track_ids: HashSet<_, RandomState> =
            HashSet::from_iter(old_tracks.iter().map(|t| t.track.id.as_str()));
        let new_track_ids: HashSet<_, RandomState> =
            HashSet::from_iter(new_tracks.iter().map(|t| t.track.id.as_str()));

        // Neat little trick here, A - B will show the elements of A that aren't in B,
        // so we can just do set difference both ways to get the set of added
        // and removed elements.
        //
        // We collect IDs to use for filtering later because hashing the entire struct
        // is not necessary.

        let added_track_ids: HashSet<_> = new_track_ids
            .difference(&old_track_ids)
            .map(|t| *t)
            .collect();

        let removed_track_ids: HashSet<_> = old_track_ids
            .difference(&new_track_ids)
            .map(|t| *t)
            .collect();

        let added_tracks = new_tracks
            .iter()
            .filter(|t| added_track_ids.contains(t.track.id.as_str()))
            .collect();

        let removed_tracks = old_tracks
            .iter()
            .filter(|t| removed_track_ids.contains(t.track.id.as_str()))
            .collect();

        Self {
            added: added_tracks,
            removed: removed_tracks,
        }
    }
}

#[derive(Serialize)]
struct Metadata {
    exported_at: DateTime<Local>,
}

struct Snapshot {
    dir: PathBuf,
}

impl Snapshot {
    fn new(parent_dir: &Path, snapshot: usize) -> Self {
        let dir = parent_dir.join(snapshot_dir_name(snapshot));
        fs::create_dir_all(&dir).expect("failed to create snapshot directory");
        Self { dir }
    }

    fn write_collection_file(&self, tracks: &[Track]) {
        write_json(&self.dir.join(SNAPSHOT_COLLECTION_FILE), &tracks);
    }

    fn write_metadata_file(&self) {
        let metadata = Metadata {
            exported_at: Local::now(),
        };
        write_json(&self.dir.join(SNAPSHOT_METADATA_FILE), &metadata);
    }

    fn write_diff_file(&self, old_tracks: &[Track], new_tracks: &[Track]) {
        let diff = Diff::calculate(old_tracks, new_tracks);
        write_json(&self.dir.join(SNAPSHOT_DIFF_FILE), &diff);
    }
}

pub fn output_dir_from_env() -> PathBuf {
    let env = env::var("OUTPUT_DIR").expect("please provide OUTPUT_DIR");
    let dir = PathBuf::from(env);
    if !dir.exists() {
        fs::create_dir(&dir).expect("failed to create OUTPUT_DIR");
    }
    if !dir.is_dir() {
        panic!("given path for OUTPUT_DIR is not a directory");
    }
    dir
}

fn snapshot_dir_name(snapshot: usize) -> String {
    format!("snapshot-{:0>6}", snapshot)
}

fn extract_snapshot_dir_number(entry: &DirEntry) -> Option<usize> {
    if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
        return None;
    }

    let regex = Regex::new(SNAPSHOT_DIRECTORY_REGEX).expect("failed to compile regex");
    let filename = entry.file_name().to_string_lossy().into_owned();
    let captures = regex.captures(&filename)?;

    if captures.len() != 1 {
        log::warn!(
            "snapshot directory candidate {filename} had the wrong number of captures: {}",
            captures.len(),
        );
        return None;
    }

    match captures[0].parse() {
        Ok(snapshot) => Some(snapshot),
        Err(error) => {
            log::warn!(
                "snapshot directory candidate {filename} had an improperly formatted number: {error}"
            );
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use uuid::Uuid;

    const OUTPUT_DIR_PREFIX: &str = "test-output";

    struct Fixture {
        persistence: Persistence,
    }

    impl Fixture {
        fn new() -> Self {
            let path = PathBuf::from(format!(
                "{}-{}",
                OUTPUT_DIR_PREFIX,
                Uuid::new_v4().to_string()
            ));

            fs::create_dir(&path).expect("failed to create test output directory");
            Self {
                persistence: Persistence::new(path),
            }
        }

        fn assert_snapshot_exists(&self, snapshot: usize, exists: bool) {
            assert_eq!(self.persistence.snapshot_path(snapshot).exists(), exists);
        }

        fn assert_snapshot_file_exists(&self, snapshot: usize, file: &str, exists: bool) {
            let path = self.persistence.snapshot_path(snapshot).join(file);
            assert_eq!(path.exists(), exists);
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.persistence.output_dir)
                .expect("failed to remove test output directory");
        }
    }

    mod snapshot {
        use super::*;

        use crate::spotify::list_user_tracks::{Artist, Track as TrackInner};

        #[test]
        fn it_writes_collection_file() {
            let tracks = vec![
                Track {
                    track: TrackInner {
                        artists: vec![Artist {
                            name: "Van Halen".into(),
                        }],
                        id: "abc123".into(),
                        name: "Ain't Talkin' Bout Love".into(),
                    },
                },
                Track {
                    track: TrackInner {
                        artists: vec![Artist {
                            name: "Def Leppard".into(),
                        }],
                        id: "def456".into(),
                        name: "Bringin' on the Heartbreak".into(),
                    },
                },
            ];

            let fixture = Fixture::new();
            fixture.persistence.snapshot(tracks.clone());
            fixture.assert_snapshot_exists(1, true);
            fixture.assert_snapshot_file_exists(1, SNAPSHOT_COLLECTION_FILE, true);

            let output = fixture.persistence.read_snapshot(1);
            assert_eq!(output, tracks);
        }

        #[test]
        fn it_writes_metadata_file() {
            let fixture = Fixture::new();
            fixture.persistence.snapshot(Vec::new());
            fixture.assert_snapshot_exists(1, true);
            fixture.assert_snapshot_file_exists(1, SNAPSHOT_METADATA_FILE, true);
        }

        #[test]
        fn when_no_previous_snapshot_exists_it_doesnt_write_diff() {
            let fixture = Fixture::new();
            fixture.persistence.snapshot(Vec::new());
            fixture.assert_snapshot_exists(1, true);
            fixture.assert_snapshot_file_exists(1, SNAPSHOT_DIFF_FILE, false);
        }
    }
}
