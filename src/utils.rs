use std::fs;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;

use serde::de::DeserializeOwned;
use serde::Serialize;

pub fn read_json<T>(path: &Path) -> T
where
    T: DeserializeOwned,
{
    let data = fs::read_to_string(path).expect("failed to read snapshot collection file");
    serde_json::from_str(&data).expect("failed to deserialize snapshot")
}

pub fn write_json<T>(path: &Path, data: T)
where
    T: Serialize,
{
    let mut file = if !path.exists() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("failed to create directories for cache path");
        }
        File::create(path).expect("failed to create file")
    } else {
        OpenOptions::new()
            .write(true)
            .open(path)
            .expect("failed to open file")
    };

    let json = serde_json::to_string_pretty(&data).expect("failed to serialize data");
    file.write_all(json.as_bytes())
        .expect("failed to write to file");
}
