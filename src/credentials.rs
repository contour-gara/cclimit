use anyhow::{Context, Result, anyhow};
use jiff::Timestamp;
use serde::Deserialize;
use std::fmt;
use std::path::Path;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct File {
    claude_ai_oauth: Credentials,
}

#[derive(Deserialize)]
#[serde(try_from = "String")]
pub struct AccessToken(String);

impl TryFrom<String> for AccessToken {
    type Error = &'static str;

    fn try_from(token: String) -> Result<AccessToken, &'static str> {
        if token.is_empty() {
            return Err("access token is empty");
        }
        Ok(AccessToken(token))
    }
}

impl AccessToken {
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for AccessToken {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("AccessToken(<redacted>)")
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Credentials {
    access_token: AccessToken,
    #[serde(default, with = "jiff::fmt::serde::timestamp::millisecond::optional")]
    expires_at: Option<Timestamp>,
}

impl Credentials {
    pub fn access_token(&self) -> &AccessToken {
        &self.access_token
    }

    pub fn expires_at(&self) -> Option<Timestamp> {
        self.expires_at
    }

    pub fn is_expired(&self, now: Timestamp) -> bool {
        self.expires_at.is_some_and(|t| now >= t)
    }
}

pub fn load(override_path: Option<&Path>) -> Result<(&'static str, Credentials)> {
    if let Some(path) = override_path {
        return Ok(("env", read_file(path)?));
    }
    #[cfg(target_os = "macos")]
    if let Some(credentials) = keychain() {
        return Ok(("keychain", credentials));
    }
    let path = std::env::home_dir()
        .context("home directory not found")?
        .join(".claude")
        .join(".credentials.json");
    Ok(("file", read_file(&path)?))
}

#[cfg(target_os = "macos")]
fn keychain() -> Option<Credentials> {
    let output = std::process::Command::new("security")
        .args([
            "find-generic-password",
            "-s",
            "Claude Code-credentials",
            "-w",
        ])
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    parse(std::str::from_utf8(&output.stdout).ok()?).ok()
}

fn read_file(path: &Path) -> Result<Credentials> {
    let json = std::fs::read_to_string(path)
        .with_context(|| format!("credentials not found at {}", path.display()))?;
    parse(&json)
}

fn parse(json: &str) -> Result<Credentials> {
    // Not the serde message: it can quote values from the credentials file
    Ok(serde_json::from_str::<File>(json)
        .map_err(|_| anyhow!("credentials are not readable"))?
        .claude_ai_oauth)
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: &str = "2026-10-13T19:15:00Z";

    fn credentials(expires_at: Option<Timestamp>) -> Credentials {
        Credentials {
            access_token: AccessToken::try_from("test-token".to_string()).unwrap(),
            expires_at,
        }
    }

    #[test]
    fn access_token_debug_hides_the_value() {
        let token = AccessToken::try_from("test-token".to_string()).unwrap();
        assert_eq!(format!("{token:?}"), "AccessToken(<redacted>)");
    }

    #[test]
    fn credentials_debug_hides_the_token() {
        let debug = format!("{:?}", credentials(None));
        assert!(!debug.contains("test-token"));
    }

    #[test]
    fn empty_access_token_is_an_error() {
        let json = r#"{"claudeAiOauth": {"accessToken": ""}}"#;
        assert_eq!(
            parse(json).unwrap_err().to_string(),
            "credentials are not readable"
        );
    }

    #[test]
    fn token_is_expired_at_expires_at() {
        let now: Timestamp = NOW.parse().unwrap();
        assert!(credentials(Some(now)).is_expired(now));
    }

    #[test]
    fn token_is_valid_before_expires_at() {
        let now: Timestamp = NOW.parse().unwrap();
        let later = now + jiff::SignedDuration::from_millis(1);
        assert!(!credentials(Some(later)).is_expired(now));
    }

    #[test]
    fn token_without_expires_at_is_left_to_the_api() {
        assert!(!credentials(None).is_expired(NOW.parse().unwrap()));
    }

    #[test]
    fn credentials_json_is_parsed() {
        let json = r#"{"claudeAiOauth": {"accessToken": "test-token", "expiresAt": 1760000000000, "scopes": []}}"#;
        let credentials = parse(json).unwrap();
        assert_eq!(credentials.access_token().as_str(), "test-token");
        assert_eq!(
            credentials.expires_at(),
            Some(Timestamp::from_millisecond(1_760_000_000_000).unwrap())
        );
    }

    #[test]
    fn credentials_without_expires_at_are_parsed() {
        let json = r#"{"claudeAiOauth": {"accessToken": "test-token"}}"#;
        assert_eq!(parse(json).unwrap().expires_at(), None);
    }

    #[test]
    fn broken_credentials_are_an_error() {
        assert!(parse(r#"{"claudeAiOauth": "#).is_err());
    }
}
