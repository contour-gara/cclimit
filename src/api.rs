use jiff::Timestamp;
use serde::Deserialize;
use std::fmt;
use std::time::Duration;

#[derive(Debug, PartialEq)]
pub struct Limit {
    pub label: String,
    pub percent: u32,
    pub resets_at: Option<Timestamp>,
}

#[cfg(test)]
pub(crate) fn limit(label: &str, percent: u32, resets_at: Option<&str>) -> Limit {
    Limit {
        label: label.to_string(),
        percent,
        resets_at: resets_at.map(|s| s.parse().unwrap()),
    }
}

#[derive(Deserialize)]
struct Usage {
    five_hour: Option<Window>,
    seven_day: Option<Window>,
    limits: Option<Vec<RawLimit>>,
}

#[derive(Deserialize)]
struct Window {
    utilization: Option<f64>,
    resets_at: Option<Timestamp>,
}

#[derive(Deserialize)]
struct RawLimit {
    kind: Option<String>,
    percent: Option<f64>,
    resets_at: Option<Timestamp>,
    scope: Option<Scope>,
}

#[derive(Deserialize)]
struct Scope {
    model: Option<Model>,
}

#[derive(Deserialize)]
struct Model {
    display_name: Option<String>,
}

#[derive(Debug)]
pub enum Error {
    Unauthorized(u16),
    Status(u16),
    Transport(ureq::Error),
    Body,
    NoLimits,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Unauthorized(status) | Error::Status(status) => write!(f, "HTTP {status}"),
            Error::Transport(e) => write!(f, "unreachable, {e}"),
            // Not the serde message: it can quote values from the response body
            Error::Body => write!(f, "HTTP 200, unreadable response"),
            Error::NoLimits => write!(f, "HTTP 200, no recognizable limits"),
        }
    }
}

impl std::error::Error for Error {}

pub fn fetch(api_base: &str, token: &str) -> Result<Vec<Limit>, Error> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        // Not ureq's default of no timeout: a status bar calling this waits for the answer
        .timeout_global(Some(Duration::from_secs(5)))
        .http_status_as_error(false)
        // Not following redirects: ureq drops Authorization on them, which would look like a 401
        .max_redirects(0)
        .build()
        .into();
    let mut response = agent
        .get(format!("{api_base}/api/oauth/usage"))
        .header("Authorization", format!("Bearer {token}"))
        .header("anthropic-beta", "oauth-2025-04-20")
        .call()
        .map_err(Error::Transport)?;
    match response.status().as_u16() {
        200 => parse(
            &response
                .body_mut()
                .read_to_string()
                .map_err(Error::Transport)?,
        ),
        status @ (401 | 403) => Err(Error::Unauthorized(status)),
        status => Err(Error::Status(status)),
    }
}

fn parse(body: &str) -> Result<Vec<Limit>, Error> {
    let usage: Usage = serde_json::from_str(body).map_err(|_| Error::Body)?;
    let mut limits: Vec<Limit> = usage
        .limits
        .into_iter()
        .flatten()
        .filter_map(normalize)
        .collect();
    if limits.is_empty() {
        limits = [("5h", usage.five_hour), ("7d", usage.seven_day)]
            .into_iter()
            .filter_map(|(label, window)| {
                let window = window?;
                Some(new_limit(
                    label.to_string(),
                    window.utilization?,
                    window.resets_at,
                ))
            })
            .collect();
    }
    if limits.is_empty() {
        return Err(Error::NoLimits);
    }
    Ok(limits)
}

fn normalize(raw: RawLimit) -> Option<Limit> {
    let label = match raw.kind.as_deref()? {
        "session" => "5h".to_string(),
        "weekly_all" => "7d".to_string(),
        "weekly_scoped" => scoped_label(raw.scope.and_then(|scope| scope.model?.display_name)),
        _ => return None,
    };
    Some(new_limit(label, raw.percent?, raw.resets_at))
}

fn scoped_label(display_name: Option<String>) -> String {
    let label: String = display_name
        .unwrap_or_default()
        .to_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect();
    if label.is_empty() {
        "scoped".to_string()
    } else {
        label
    }
}

fn new_limit(label: String, percent: f64, resets_at: Option<Timestamp>) -> Limit {
    Limit {
        label,
        percent: percent.round() as u32,
        resets_at,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn fixture_is_normalized_in_order() {
        let limits = parse(include_str!("../tests/fixtures/usage.json")).unwrap();
        assert_eq!(
            limits,
            vec![
                limit("5h", 11, Some("2026-10-13T21:29:59.728003Z")),
                limit("7d", 38, Some("2026-10-14T22:59:59.728026Z")),
                limit("fable", 68, Some("2026-10-14T22:59:59.728224Z")),
            ]
        );
    }

    #[test]
    fn scoped_label_is_lowercase_ascii_alphanumerics_or_scoped() {
        let body = r#"{"limits": [
            {"kind": "weekly_scoped", "percent": 1, "scope": {"model": {"display_name": "Fable\nX"}}},
            {"kind": "weekly_scoped", "percent": 2, "scope": {"model": {"display_name": "  "}}},
            {"kind": "weekly_scoped", "percent": 3, "scope": null}
        ]}"#;
        assert_eq!(
            parse(body).unwrap(),
            vec![
                limit("fablex", 1, None),
                limit("scoped", 2, None),
                limit("scoped", 3, None)
            ]
        );
    }

    #[test]
    fn without_known_limits_falls_back_to_windows() {
        let windows = r#""five_hour": {"utilization": 10.6, "resets_at": "2026-10-13T21:00:00Z"},
            "seven_day": {"utilization": 38.0, "resets_at": null}"#;
        let expected = vec![
            limit("5h", 11, Some("2026-10-13T21:00:00Z")),
            limit("7d", 38, None),
        ];
        for body in [
            format!("{{{windows}}}"),
            format!(r#"{{"limits": [{{"kind": "monthly", "percent": 5}}], {windows}}}"#),
        ] {
            assert_eq!(parse(&body).unwrap(), expected);
        }
    }

    #[test]
    fn response_without_any_window_is_an_error() {
        let body = r#"{"limits": [{"kind": "monthly", "percent": 5}], "five_hour": null}"#;
        assert!(matches!(parse(body), Err(Error::NoLimits)));
    }

    #[test]
    fn broken_json_is_an_error() {
        assert!(matches!(parse("{"), Err(Error::Body)));
    }
}
