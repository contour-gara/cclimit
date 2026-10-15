use anyhow::{Context, Result, anyhow, bail};
use jiff::Timestamp;
use std::env;
use std::ffi::OsString;
use std::path::PathBuf;

pub struct Config {
    pub api_base: String,
    pub credentials_file: Option<PathBuf>,
    pub now: Timestamp,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let now = match var("CCLIMIT_NOW")? {
            Some(now) => now.parse().context("CCLIMIT_NOW is not RFC 3339")?,
            None => Timestamp::now(),
        };
        Ok(Self {
            api_base: api_base(
                var("CCLIMIT_API_BASE")?.unwrap_or_else(|| "https://api.anthropic.com".to_string()),
            )?,
            credentials_file: var("CCLIMIT_CREDENTIALS_FILE")?.map(PathBuf::from),
            now,
        })
    }
}

fn var(name: &str) -> Result<Option<String>> {
    text(name, env::var_os(name))
}

fn text(name: &str, value: Option<OsString>) -> Result<Option<String>> {
    value
        .map(|v| v.into_string().map_err(|_| anyhow!("{name} is not UTF-8")))
        .transpose()
}

fn api_base(value: String) -> Result<String> {
    let valid = (value.starts_with("http://") || value.starts_with("https://"))
        && !value.ends_with('/')
        && !value.contains(['?', '#']);
    if !valid {
        bail!(
            "CCLIMIT_API_BASE must be an http(s) origin without trailing slash, query, or fragment"
        );
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::ffi::OsStringExt;

    #[test]
    fn unset_variable_is_none() {
        assert_eq!(text("CCLIMIT_NOW", None).unwrap(), None);
    }

    #[test]
    fn utf8_variable_is_its_value() {
        let value = Some(OsString::from("2026-10-13T19:14:59Z"));
        assert_eq!(
            text("CCLIMIT_NOW", value).unwrap().as_deref(),
            Some("2026-10-13T19:14:59Z")
        );
    }

    #[test]
    fn non_utf8_variable_is_an_error() {
        let value = Some(OsString::from_vec(vec![0xff, 0xfe]));
        assert_eq!(
            text("CCLIMIT_NOW", value).unwrap_err().to_string(),
            "CCLIMIT_NOW is not UTF-8"
        );
    }

    #[test]
    fn http_origins_are_api_bases() {
        for value in ["https://api.anthropic.com", "http://127.0.0.1:8080"] {
            assert_eq!(api_base(value.to_string()).unwrap(), value);
        }
    }

    #[test]
    fn other_values_are_not_api_bases() {
        for value in ["https://x/", "https://x?y=1", "https://x#y", "ftp://x"] {
            assert!(api_base(value.to_string()).is_err(), "{value}");
        }
    }
}
