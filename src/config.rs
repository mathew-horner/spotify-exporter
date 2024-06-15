use crate::auth_flow::{self, ClientCredentials};

pub struct Config {
    pub credentials: ClientCredentials,
}

impl Config {
    pub fn from_env() -> Self {
        let credentials = auth_flow::ClientCredentials::from_env();
        Self { credentials }
    }
}
