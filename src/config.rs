use anyhow::{Context, Result};
use jiff::Timestamp;
use std::env;
use std::path::PathBuf;

pub struct Config {
    pub api_base: String,
    pub credentials_file: Option<PathBuf>,
    pub now: Timestamp,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let now = match env::var("CCLIMIT_NOW") {
            Ok(now) => now.parse().context("CCLIMIT_NOW is not RFC 3339")?,
            Err(_) => Timestamp::now(),
        };
        Ok(Self {
            api_base: env::var("CCLIMIT_API_BASE")
                .unwrap_or_else(|_| "https://api.anthropic.com".to_string()),
            credentials_file: env::var_os("CCLIMIT_CREDENTIALS_FILE").map(PathBuf::from),
            now,
        })
    }
}
