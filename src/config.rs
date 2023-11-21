use std::path::PathBuf;

use crate::auth_flow::{self, ClientCredentials};
use crate::persistence;

pub struct Config {
    pub credentials: ClientCredentials,
    pub output_dir: PathBuf,
}

impl Config {
    pub fn from_env() -> Self {
        let credentials = auth_flow::ClientCredentials::from_env();
        let output_dir = persistence::output_dir_from_env();
        Self {
            credentials,
            output_dir,
        }
    }
}
